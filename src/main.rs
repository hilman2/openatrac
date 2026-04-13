mod bitstream;
mod encoder;
mod mdct;
mod tables;
mod wav;

use encoder::{Atrac3Config, Atrac3Encoder};
use wav::reader::WavReader;
use wav::writer::Atrac3WavWriter;

fn main() {
    let args: Vec<String> = std::env::args().collect();

    if args.len() < 3 {
        eprintln!("Usage: openatrac <input.wav> <output.at3> [--bitrate <kbps>]");
        std::process::exit(1);
    }

    let input_path = &args[1];
    let output_path = &args[2];
    let bitrate = if args.len() >= 5 && args[3] == "--bitrate" {
        args[4].parse::<u32>().unwrap_or(132)
    } else {
        132
    };

    let wav = WavReader::open(input_path).expect("Failed to read WAV file");
    eprintln!("Read {} ({} ch, {} Hz, {} samples/ch)", input_path, wav.channels, wav.sample_rate, wav.num_samples);

    let frame_size: u16 = match bitrate {
        66 => 192, 105 => 304, 132 => 384,
        _ => { eprintln!("Unsupported bitrate, using 132"); 384 }
    };

    let config = Atrac3Config {
        channels: wav.channels, sample_rate: wav.sample_rate,
        bitrate_kbps: bitrate, frame_size,
    };
    let spf = config.samples_per_frame();
    let mut encoder = Atrac3Encoder::new(config);
    let ch = wav.channels as usize;
    let ipf = spf * ch; // interleaved samples per frame

    // Encode all frames into memory first, so we know the exact count
    let mut encoded_frames: Vec<Vec<u8>> = Vec::new();

    // Audio frames (1 sample per channel per frame = 1024)
    let audio_frames = (wav.num_samples as usize + spf - 1) / spf;
    // Reference encoder produces audio_frames + 2 total frames
    // (1 priming + audio_frames + 1 trailing)
    let total_target = audio_frames + 2;

    for fi in 0..total_target {
        // Frame 0 = priming silence, frames 1..=audio_frames = audio, last = trailing
        let audio_idx = fi.wrapping_sub(1); // which audio frame (0-based)
        let start = audio_idx * ipf;
        let mut buf = vec![0i16; ipf];
        if fi > 0 && fi <= audio_frames && start < wav.data.len() {
            let end = (start + ipf).min(wav.data.len());
            buf[..end - start].copy_from_slice(&wav.data[start..end]);
        }
        encoded_frames.push(encoder.encode_frame(&buf));
    }

    let total_frames = encoded_frames.len() as u32;
    eprintln!("Encoded {} frames at {} kbps", total_frames, bitrate);

    // Write output with correct frame count
    let mut writer = Atrac3WavWriter::create(
        output_path, wav.channels, wav.sample_rate, frame_size,
        wav.num_samples, total_frames,
    ).expect("Failed to create output");

    for frame in &encoded_frames {
        writer.write_frame(frame).expect("Write failed");
    }
    writer.finish().expect("Finish failed");
    eprintln!("Written to {}", output_path);
}

#[cfg(test)]
mod integration_tests {
    use crate::bitstream::write_bits;

    #[test]
    fn test_encode_known_frame() {
        // Reproduce the Python test: 29 subbands, wl=1, sf=15, sym=0 for all coeffs
        let sb_starts: [usize; 32] = [0,8,16,24,32,40,48,56,64,80,96,112,128,144,160,176,192,224,256,288,320,352,384,416,448,480,512,576,640,704,768,896];
        let sb_ends: [usize; 32] = [8,16,24,32,40,48,56,64,80,96,112,128,144,160,176,192,224,256,288,320,352,384,416,448,480,512,576,640,704,768,896,1024];

        let mut cu = [0u8; 192];
        cu[0] = 0xA2;
        let mut bp: u32 = 8;

        // Gain: 0,0,0
        for _ in 0..3 { write_bits(&mut cu, bp, 0, 3); bp += 3; }
        // Tonal: 0
        write_bits(&mut cu, bp, 0, 5); bp += 5;
        // 29 subbands
        write_bits(&mut cu, bp, 28, 5); bp += 5;
        // VLC mode
        write_bits(&mut cu, bp, 0, 1); bp += 1;
        // word_lens: all 2 (wl=1)
        for _ in 0..29 { write_bits(&mut cu, bp, 2, 3); bp += 3; }
        // sf + coeffs
        for sb in 0..29 {
            write_bits(&mut cu, bp, 15, 6); bp += 6;
            let n = sb_ends[sb] - sb_starts[sb];
            for _ in 0..n {
                write_bits(&mut cu, bp, 0, 1); bp += 1; // sym 0
            }
        }
        assert_eq!(bp, 993);
        
        // Compare with Python output
        // Python produced: a2 00 03 84 00 00 00 00 ... (with VLC codes embedded)
        // Let's just print the first 20 bytes
        let hex: String = cu[..20].iter().map(|b| format!("{:02x}", b)).collect::<Vec<_>>().join("");
        eprintln!("Rust CU: {}", hex);
        
        // Known-good Python output: 
        // The Python frame has bits starting same way
        // Let's verify key bytes
        // After header (8 bits) + gain (9) + tonal (5) + nsb (5) + mode (1) = 28 bits
        // Then 29 * 3 = 87 bits of word_lens starting at bit 28
        // bit 28: word_len for sb0 = 010 (=2)
        // byte 3 bits: 24-31
        // bit 24: 0 (last bit of nsb?)
        // Actually: bp starts at 8, gain=9 -> 17, tonal=5 -> 22, nsb=5 -> 27, mode=1 -> 28
        // bit 28: start of word_lens
        // byte 3 (bits 24-31): bits 24,25,26,27 = last of nsb(0), mode(0) then 28,29,30 = first wl = 010, bit 31 = next wl
        // = 00 0 010 0 = 0b00001000? No: 
        // bit 24 = 0 (nsb bit 3 of 28=11100, bit 24 is... let me recalculate)
        
        // nsb-1 = 28 = 0b11100
        // Written at bp=22: bits 22-26 = 11100
        // bit 22=1, 23=1, 24=1, 25=0, 26=0
        // mode at bp=27: bit 27=0
        // wl[0] at bp=28: bits 28,29,30 = 010
        
        // byte 2 (bits 16-23): bit16=0(tonal), 17-21=tonal(00000), 22=1(nsb), 23=1(nsb)
        // = 00000011 = 0x03
        // byte 3 (bits 24-31): 24=1(nsb), 25=0(nsb), 26=0(nsb), 27=0(mode), 28=0(wl), 29=1(wl), 30=0(wl), 31=0(next wl)
        // = 10000100 = 0x84
        
        assert_eq!(cu[2], 0x03, "byte 2");
        assert_eq!(cu[3], 0x84, "byte 3");
        eprintln!("Frame bytes match expected pattern!");
    }
}
