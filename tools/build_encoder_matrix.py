#!/usr/bin/env python3
"""
Build the encoder matrix by measuring the real decoder's impulse response.

For each spectral coefficient position, we write an AT3 file with an impulse
at that position, decode it with psp_at3tool.exe, and measure the PCM output.
The resulting 1024x1024 matrix maps spectral coefficients to PCM samples.
Its pseudo-inverse is the encoder matrix (PCM → spectral coefficients).
"""

import struct, subprocess, sys, io, os, wave, math, time
import numpy as np

sys.stdout = io.TextIOWrapper(sys.stdout.buffer, encoding='utf-8', errors='replace')

SCRIPT_DIR = os.path.dirname(os.path.abspath(__file__))
PROJECT_DIR = os.path.dirname(SCRIPT_DIR)
TEST_DIR = os.path.join(os.path.dirname(PROJECT_DIR), 'testsong')
AT3TOOL = os.path.join(os.path.dirname(PROJECT_DIR), 'psp_at3tool.exe')

SB_STARTS = [0,8,16,24,32,40,48,56,64,80,96,112,128,144,160,176,192,224,256,288,320,352,384,416,448,480,512,576,640,704,768,896]
SB_ENDS   = [8,16,24,32,40,48,56,64,80,96,112,128,144,160,176,192,224,256,288,320,352,384,416,448,480,512,576,640,704,768,896,1024]

def write_bits_be(buf, bp, val, nb):
    for i in range(nb):
        bit = (val >> (nb - 1 - i)) & 1
        by = (bp + i) >> 3
        bi = 7 - ((bp + i) & 7)
        if by < len(buf):
            if bit: buf[by] |= (1 << bi)

def build_impulse_frame(spec_positions, sf_idx=45, wl=4):
    """Build an AT3 frame with impulses at the given spectral positions.
    spec_positions: list of (subband_index, local_coeff_index) pairs.
    Returns 384-byte frame.
    """
    cu = bytearray(192)
    cu[0] = 0xA2
    bp = 8
    # Gain: 0 points per band
    for _ in range(3): write_bits_be(cu, bp, 0, 3); bp += 3
    # Tonal: 0
    write_bits_be(cu, bp, 0, 5); bp += 5

    # Determine which subbands need coding
    needed_sbs = set(sb for sb, _ in spec_positions)
    max_sb = max(needed_sbs) + 1 if needed_sbs else 1

    # num_subbands
    write_bits_be(cu, bp, max_sb - 1, 5); bp += 5
    # VLC mode
    write_bits_be(cu, bp, 0, 1); bp += 1

    # Word lengths: wl=4 for needed subbands, 0 for others
    for sb in range(max_sb):
        if sb in needed_sbs:
            write_bits_be(cu, bp, wl + 1, 3); bp += 3
        else:
            write_bits_be(cu, bp, 0, 3); bp += 3

    # VLC wl=4 codes: sym0=(0b00, 2bits), sym7=(0b1100, 4bits)
    # Scale factors + coefficients
    impulse_set = set((sb, idx) for sb, idx in spec_positions)
    for sb in range(max_sb):
        if sb not in needed_sbs: continue
        write_bits_be(cu, bp, sf_idx, 6); bp += 6
        n_coeffs = SB_ENDS[sb] - SB_STARTS[sb]
        for i in range(n_coeffs):
            if (sb, i) in impulse_set:
                write_bits_be(cu, bp, 0b1100, 4); bp += 4  # sym7 = 4557
            else:
                write_bits_be(cu, bp, 0b00, 2); bp += 2    # sym0 = 0

    frame = bytearray(384)
    frame[:192] = cu
    frame[0] = 0xA2
    frame[192] = 0xA2  # CU1 = silence
    return bytes(frame)

def read_wav_samples(path):
    with open(path, 'rb') as f:
        data = f.read()
    pos = 12
    while pos + 8 <= len(data):
        chunk_id = data[pos:pos+4]
        chunk_size = struct.unpack_from('<I', data, pos+4)[0]
        if chunk_id == b'data':
            raw = data[pos+8:pos+8+chunk_size]
            return np.array([struct.unpack_from('<h', raw, i)[0]
                           for i in range(0, len(raw)-1, 2)], dtype=np.float32)
        pos += 8 + chunk_size + (chunk_size % 2)
    return np.array([])

def main():
    # Create template AT3 (2 seconds silence at 132kbps)
    template_wav = os.path.join(TEST_DIR, 'template_matrix.wav')
    template_at3 = os.path.join(TEST_DIR, 'template_matrix.at3')

    sr = 44100
    with wave.open(template_wav, 'w') as w:
        w.setnchannels(2); w.setsampwidth(2); w.setframerate(sr)
        w.writeframes(b'\x00' * sr * 2 * 4)

    subprocess.run([AT3TOOL, '-e', '-br', '132', template_wav, template_at3],
                   capture_output=True)

    with open(template_at3, 'rb') as f:
        tmpl = f.read()
    header = tmpl[:76]
    nf = (len(tmpl) - 76) // 384
    silence_frame = bytearray(384)
    silence_frame[0] = 0xA2
    silence_frame[192] = 0xA2

    print(f"Template: {nf} frames")

    # For each spectral position, measure the decoder's impulse response
    # We can test 4 positions simultaneously (one per subband, they're independent)
    # Total: 29 subbands × max 128 coeffs = ~700 positions, but subbands vary in size

    # Build the full list of spectral positions
    all_positions = []
    for sb in range(29):
        n = SB_ENDS[sb] - SB_STARTS[sb]
        for i in range(n):
            all_positions.append((sb, i, SB_STARTS[sb] + i))  # (subband, local_idx, global_idx)

    print(f"Total spectral positions: {len(all_positions)}")

    # Measure active frame region
    active_start = 10  # start impulse at frame 10
    active_end = 30    # end at frame 30 (20 frames active)
    measure_frame = 20 # measure at frame 20 (middle, overlap settled)

    D = np.zeros((1024, 1024), dtype=np.float32)

    tmp_at3 = os.path.join(TEST_DIR, 'impulse_matrix.at3')
    tmp_wav = os.path.join(TEST_DIR, 'impulse_matrix.wav')

    # Process one position at a time (simple, reliable)
    t0 = time.time()
    for pos_idx, (sb, local_i, global_i) in enumerate(all_positions):
        # Build AT3 with impulse at this position
        imp_frame = build_impulse_frame([(sb, local_i)])

        with open(tmp_at3, 'wb') as f:
            f.write(header)
            for fi in range(nf):
                if active_start <= fi <= active_end:
                    f.write(imp_frame)
                else:
                    f.write(silence_frame)

        # Decode
        if os.path.exists(tmp_wav): os.remove(tmp_wav)
        subprocess.run([AT3TOOL, '-d', tmp_at3, tmp_wav], capture_output=True)

        # Read PCM
        pcm = read_wav_samples(tmp_wav)
        if len(pcm) == 0:
            print(f"  WARNING: no output for position {global_i}")
            continue

        # Extract 1024 samples from the measurement frame (left channel only, stride 2)
        sample_start = measure_frame * 1024 * 2  # interleaved stereo
        if sample_start + 2048 <= len(pcm):
            left_channel = pcm[sample_start:sample_start+2048:2]  # 1024 left samples
            D[:, global_i] = left_channel[:1024]

        if (pos_idx + 1) % 50 == 0:
            elapsed = time.time() - t0
            rate = (pos_idx + 1) / elapsed
            remaining = (len(all_positions) - pos_idx - 1) / rate
            print(f"  {pos_idx+1}/{len(all_positions)} ({elapsed:.0f}s elapsed, ~{remaining:.0f}s remaining)")

    elapsed = time.time() - t0
    print(f"\nDecoder measurement complete: {elapsed:.0f}s for {len(all_positions)} positions")

    # Check matrix quality
    rank = np.linalg.matrix_rank(D, tol=1e-3)
    print(f"Decoder matrix: shape={D.shape}, rank={rank}")
    print(f"  Max value: {np.max(np.abs(D)):.1f}")
    print(f"  Non-zero columns: {np.sum(np.any(D != 0, axis=0))}")

    # Compute encoder matrix via regularized pseudo-inverse
    U, s, Vt = np.linalg.svd(D, full_matrices=False)
    print(f"  SVD: min_s={s.min():.6f}, max_s={s.max():.6f}, cond={s.max()/max(s.min(),1e-30):.1f}")

    # Regularize: keep singular values above threshold
    threshold = s.max() * 1e-4
    s_inv = np.where(s > threshold, 1.0/s, 0.0)
    effective_rank = np.sum(s > threshold)
    print(f"  Effective rank (threshold={threshold:.4f}): {effective_rank}")

    E = (Vt.T @ np.diag(s_inv) @ U.T).astype(np.float32)

    # Verify roundtrip
    np.random.seed(42)
    test_pcm = np.random.randn(1024).astype(np.float32) * 5000
    test_spec = E @ test_pcm
    test_recon = D @ test_spec

    noise = np.sum((test_pcm - test_recon)**2)
    sig = np.sum(test_pcm**2)
    snr = 10 * np.log10(sig / max(noise, 1e-30))
    corr = np.corrcoef(test_pcm, test_recon)[0, 1]
    print(f"\nRoundtrip verification (random signal):")
    print(f"  SNR = {snr:.1f} dB")
    print(f"  Correlation = {corr:.4f}")

    # Save
    output_path = os.path.join(PROJECT_DIR, 'encoder_matrix.bin')
    E.tofile(output_path)
    print(f"\nSaved encoder matrix to {output_path} ({os.path.getsize(output_path)} bytes)")

    # Also save decoder matrix for debugging
    D.astype(np.float32).tofile(os.path.join(PROJECT_DIR, 'decoder_matrix.bin'))

if __name__ == '__main__':
    main()
