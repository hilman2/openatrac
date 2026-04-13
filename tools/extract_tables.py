#!/usr/bin/env python3
"""Extract lookup tables from psp_at3tool.exe and generate Rust source files."""

import struct
import sys
import os

EXE_PATH = os.path.join(os.path.dirname(__file__), '..', '..', 'psp_at3tool.exe')
IMAGE_BASE = 0x400000

def va_to_offset(va):
    return va - IMAGE_BASE

def read_exe():
    with open(EXE_PATH, 'rb') as f:
        return f.read()

def read_floats(data, va, count):
    off = va_to_offset(va)
    return [struct.unpack_from('<f', data, off + i*4)[0] for i in range(count)]

def read_u32s(data, va, count):
    off = va_to_offset(va)
    return [struct.unpack_from('<I', data, off + i*4)[0] for i in range(count)]

def read_i32s(data, va, count):
    off = va_to_offset(va)
    return [struct.unpack_from('<i', data, off + i*4)[0] for i in range(count)]

def read_u16s(data, va, count):
    off = va_to_offset(va)
    return [struct.unpack_from('<H', data, off + i*2)[0] for i in range(count)]

def read_bytes(data, va, count):
    off = va_to_offset(va)
    return list(data[off:off+count])

def fmt_float(f):
    if f == 0.0:
        return "0.0"
    return f"{f:.10e}"

def fmt_float_arr(arr, name, rust_type="f32"):
    lines = [f"pub const {name}: [{rust_type}; {len(arr)}] = ["]
    for i in range(0, len(arr), 8):
        chunk = arr[i:i+8]
        line = "    " + ", ".join(f"{fmt_float(v)}" for v in chunk) + ","
        lines.append(line)
    lines.append("];")
    return "\n".join(lines)

def fmt_u32_arr(arr, name):
    lines = [f"pub const {name}: [u32; {len(arr)}] = ["]
    for i in range(0, len(arr), 8):
        chunk = arr[i:i+8]
        line = "    " + ", ".join(f"0x{v:08X}" for v in chunk) + ","
        lines.append(line)
    lines.append("];")
    return "\n".join(lines)

def fmt_i32_arr(arr, name):
    lines = [f"pub const {name}: [i32; {len(arr)}] = ["]
    for i in range(0, len(arr), 8):
        chunk = arr[i:i+8]
        line = "    " + ", ".join(f"{v}" for v in chunk) + ","
        lines.append(line)
    lines.append("];")
    return "\n".join(lines)

def main():
    data = read_exe()
    print(f"Loaded {len(data)} bytes from {EXE_PATH}")

    tables_dir = os.path.join(os.path.dirname(__file__), '..', 'src', 'tables')
    os.makedirs(tables_dir, exist_ok=True)

    # === 1. Scale Factors (DAT_0048d6c0) ===
    # Used heavily in quantization, indexed by value >> 3
    scale_factors = read_floats(data, 0x0048d6c0, 64)
    print(f"Scale factors (first 8): {scale_factors[:8]}")

    # === 2. Stereo Balance Factors ===
    stereo_a = read_floats(data, 0x0048be38, 16)
    stereo_b = read_floats(data, 0x0048be50, 32)
    print(f"Stereo balance A (first 8): {stereo_a[:8]}")
    print(f"Stereo balance B (first 8): {stereo_b[:8]}")

    # === 3. Global float constants ===
    const_half = read_floats(data, 0x0048b384, 1)[0]
    const_one = read_floats(data, 0x0048b3c0, 1)[0]
    const_eighth = read_floats(data, 0x0048b394, 1)[0]
    print(f"Constants: half={const_half}, one={const_one}, eighth={const_eighth}")

    # Stereo thresholds
    stereo_max_delta = read_floats(data, 0x0048e830, 1)[0]
    stereo_diff_scale = read_floats(data, 0x0048e838, 1)[0]
    stereo_threshold = read_floats(data, 0x0048e840, 1)[0]
    print(f"Stereo: max_delta={stereo_max_delta}, diff_scale={stereo_diff_scale}, threshold={stereo_threshold}")

    # === 4. Quantization levels (DAT_0048e908, 33 entries) ===
    quant_levels = read_u32s(data, 0x0048e908, 33)
    print(f"Quant levels (first 8): {[hex(x) for x in quant_levels[:8]]}")

    # === 5. Bitrate config table (DAT_004519b8, 19 entries, stride 36 bytes = 9 u32) ===
    bitrate_config_raw = read_u32s(data, 0x004519b8, 19 * 9)
    print(f"Bitrate config entry 0: {[hex(x) for x in bitrate_config_raw[:9]]}")
    print(f"Bitrate config entry 1: {[hex(x) for x in bitrate_config_raw[9:18]]}")

    # === 6. ATRAC3 encode config (DAT_00452250, 3 entries, stride 20 bytes = 5 u32) ===
    # The table starts at 00452250 (offset -8 from 00452254+4=00452258)
    # Actually DAT_00452254 is the comparison start, but the struct base is 8 bytes before
    # Let's read from 0x00452248 (base - 0xC) or calculate properly
    # From the code: piVar1 = &DAT_00452254, compares frame_size=piVar1[-1], sample_rate=piVar1[0], channels=piVar1[1]
    # So struct is: [-2]=?, [-1]=frame_size, [0]=sample_rate, [1]=channels, [2]=?
    # Stride is 0x14 = 5 ints
    # Base for index is UNK_0045224c which is DAT_00452254 - 8 = 0x0045224C
    atrac3_config = read_u32s(data, 0x0045224C, 3 * 5)
    print(f"ATRAC3 config entry 0: {[hex(x) for x in atrac3_config[:5]]}")
    print(f"ATRAC3 config entry 1: {[hex(x) for x in atrac3_config[5:10]]}")
    print(f"ATRAC3 config entry 2: {[hex(x) for x in atrac3_config[10:15]]}")

    # === 7. Spectrum encoder config (16 entries across 5 arrays) ===
    spec_e6e8 = read_i32s(data, 0x0048e6e8, 16)  # mdct_mode
    spec_e728 = read_i32s(data, 0x0048e728, 16)    # sample_rate config
    spec_e768 = read_i32s(data, 0x0048e768, 16)    # bitrate config
    spec_e7a8 = read_i32s(data, 0x0048e7a8, 16)    # frame_size
    spec_e7e8 = read_i32s(data, 0x0048e7e8, 16)    # mdct_precision
    print(f"Spectrum e728 (sample rates): {spec_e728}")
    print(f"Spectrum e768 (bitrates): {spec_e768}")
    print(f"Spectrum e7a8 (frame sizes): {spec_e7a8}")

    # === 8. Subband default table pointer target (DAT_0048d634) ===
    subband_default = read_floats(data, 0x0048d634, 16)
    print(f"Subband default (first 8): {subband_default[:8]}")

    # === 9. MDCT window / twiddle factors ===
    # The SSE function uses tables that are part of the working buffer init
    # FUN_0043a0e0 sets up the MDCT context
    # Let's look at what's at 0x4c006c (referenced in analysis as twiddle table location)
    # This is in .data section
    if 0x4c006c - IMAGE_BASE < len(data):
        twiddle_start = read_floats(data, 0x004c006c, 32)
        print(f"Twiddle factors (first 8): {twiddle_start[:8]}")
    else:
        print("Twiddle table address out of range")
        twiddle_start = []

    # === 10. Psychoacoustic constants ===
    psy_constants = read_floats(data, 0x0048b388, 4)
    print(f"Psychoacoustic constants at b388: {psy_constants}")

    # DAT_0048b3ac region
    psy_b3ac = read_floats(data, 0x0048b3ac, 8)
    print(f"Psy constants at b3ac: {psy_b3ac}")

    # === 11. Bit allocation thresholds ===
    bit_alloc = read_floats(data, 0x0048cc90, 32)
    print(f"Bit alloc thresholds (first 8): {bit_alloc[:8]}")

    # === 12. ATRAC3 LP config (DAT_00451dd8, 41 entries, stride 28 = 7 u32) ===
    lp_config = read_u32s(data, 0x00451dd8, 41 * 7)

    # === 13. VLC/Huffman tables ===
    # s_UnknownVendr_004be9bc - this is a pointer-based structure
    vlc_base = read_u32s(data, 0x004be9bc, 32)
    print(f"VLC base data (first 8): {[hex(x) for x in vlc_base[:8]]}")

    # === GENERATE RUST SOURCE FILES ===

    # --- scale_factors.rs ---
    with open(os.path.join(tables_dir, 'scale_factors.rs'), 'w') as f:
        f.write("// Auto-generated from psp_at3tool.exe\n")
        f.write("// Scale factor table (DAT_0048d6c0)\n\n")
        f.write(fmt_float_arr(scale_factors, "SCALE_FACTORS"))
        f.write("\n")

    # --- stereo_balance.rs ---
    with open(os.path.join(tables_dir, 'stereo_balance.rs'), 'w') as f:
        f.write("// Auto-generated from psp_at3tool.exe\n")
        f.write("// Stereo balance tables\n\n")
        f.write(fmt_float_arr(stereo_a, "STEREO_BALANCE_A"))
        f.write("\n\n")
        f.write(fmt_float_arr(stereo_b, "STEREO_BALANCE_B"))
        f.write("\n\n")
        f.write(f"pub const STEREO_MAX_DELTA: f32 = {fmt_float(stereo_max_delta)};\n")
        f.write(f"pub const STEREO_DIFF_SCALE: f32 = {fmt_float(stereo_diff_scale)};\n")
        f.write(f"pub const STEREO_THRESHOLD: f32 = {fmt_float(stereo_threshold)};\n")

    # --- constants.rs ---
    with open(os.path.join(tables_dir, 'constants.rs'), 'w') as f:
        f.write("// Auto-generated from psp_at3tool.exe\n")
        f.write("// Global constants\n\n")
        f.write(f"pub const HALF: f32 = {const_half};\n")
        f.write(f"pub const ONE: f32 = {const_one};\n")
        f.write(f"pub const EIGHTH: f32 = {const_eighth};\n\n")
        f.write(fmt_float_arr(psy_constants, "PSY_CONSTANTS_B388"))
        f.write("\n\n")
        f.write(fmt_float_arr(psy_b3ac, "PSY_CONSTANTS_B3AC"))
        f.write("\n\n")
        f.write(fmt_float_arr(subband_default, "SUBBAND_DEFAULT"))
        f.write("\n\n")
        f.write(fmt_float_arr(bit_alloc, "BIT_ALLOC_THRESHOLDS"))
        f.write("\n")

    # --- quantization.rs ---
    with open(os.path.join(tables_dir, 'quantization.rs'), 'w') as f:
        f.write("// Auto-generated from psp_at3tool.exe\n")
        f.write("// Quantization tables\n\n")
        f.write(fmt_u32_arr(quant_levels, "QUANT_LEVELS"))
        f.write("\n")

    # --- bitrate_config.rs ---
    with open(os.path.join(tables_dir, 'bitrate_config.rs'), 'w') as f:
        f.write("// Auto-generated from psp_at3tool.exe\n")
        f.write("// Bitrate configuration tables\n\n")

        # Parse the 19-entry bitrate config into a struct-like format
        f.write("/// Bitrate configuration entry (from DAT_004519b8)\n")
        f.write("/// Fields: [codec_info, unknown1, bitrate_kbps, channels, unknown2, sample_rate, ...]\n")
        f.write(f"pub const BITRATE_CONFIG: [[u32; 9]; 19] = [\n")
        for i in range(19):
            entry = bitrate_config_raw[i*9:(i+1)*9]
            f.write(f"    [{', '.join(f'0x{v:08X}' for v in entry)}],\n")
        f.write("];\n\n")

        # ATRAC3 encode config (3 entries)
        f.write("/// ATRAC3 encode configuration (from DAT_0045224C)\n")
        f.write(f"pub const ATRAC3_ENCODE_CONFIG: [[u32; 5]; 3] = [\n")
        for i in range(3):
            entry = atrac3_config[i*5:(i+1)*5]
            f.write(f"    [{', '.join(f'0x{v:08X}' for v in entry)}],\n")
        f.write("];\n")

    # --- spectrum_config.rs ---
    with open(os.path.join(tables_dir, 'spectrum_config.rs'), 'w') as f:
        f.write("// Auto-generated from psp_at3tool.exe\n")
        f.write("// Spectrum encoder configuration tables\n\n")
        f.write(fmt_i32_arr(spec_e6e8, "SPEC_MDCT_MODE"))
        f.write("\n\n")
        f.write(fmt_i32_arr(spec_e728, "SPEC_SAMPLE_RATE"))
        f.write("\n\n")
        f.write(fmt_i32_arr(spec_e768, "SPEC_BITRATE"))
        f.write("\n\n")
        f.write(fmt_i32_arr(spec_e7a8, "SPEC_FRAME_SIZE"))
        f.write("\n\n")
        f.write(fmt_i32_arr(spec_e7e8, "SPEC_MDCT_PRECISION"))
        f.write("\n")

    # --- vlc.rs ---
    with open(os.path.join(tables_dir, 'vlc.rs'), 'w') as f:
        f.write("// Auto-generated from psp_at3tool.exe\n")
        f.write("// VLC/Huffman code tables\n\n")
        f.write(fmt_u32_arr(vlc_base, "VLC_BASE"))
        f.write("\n\n")

        # Try to follow pointers and extract VLC sub-tables
        f.write("// VLC sub-tables (following pointers from VLC_BASE)\n")
        for i, ptr in enumerate(vlc_base):
            if IMAGE_BASE < ptr < IMAGE_BASE + len(data):
                sub = read_u32s(data, ptr, 16)
                f.write(f"pub const VLC_SUB_{i}: [u32; 16] = [{', '.join(f'0x{v:08X}' for v in sub)}];\n")

    # --- mdct_window.rs ---
    with open(os.path.join(tables_dir, 'mdct_window.rs'), 'w') as f:
        f.write("// Auto-generated from psp_at3tool.exe\n")
        f.write("// MDCT window and twiddle factors\n\n")
        if twiddle_start:
            f.write(fmt_float_arr(twiddle_start, "TWIDDLE_START"))
        else:
            f.write("pub const TWIDDLE_START: [f32; 0] = [];\n")
        f.write("\n")

    print("\n=== Rust table files generated in src/tables/ ===")
    print("Files created:")
    for fname in sorted(os.listdir(tables_dir)):
        fpath = os.path.join(tables_dir, fname)
        if os.path.isfile(fpath):
            print(f"  {fname}: {os.path.getsize(fpath)} bytes")

if __name__ == '__main__':
    main()
