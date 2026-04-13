use crate::bitstream::write_bits;
use crate::mdct::MdctContext;
use crate::tables::spectral::{SCALE_FACTOR_VALUES, SUBBAND_STARTS, SUBBAND_ENDS};
use crate::tables::vlc_encode::*;

pub struct Atrac3Config {
    pub channels: u16,
    pub sample_rate: u32,
    pub bitrate_kbps: u32,
    pub frame_size: u16,
}

impl Atrac3Config {
    pub fn samples_per_frame(&self) -> usize { 1024 }
    pub fn coding_unit_size(&self) -> usize {
        if self.channels == 2 { self.frame_size as usize / 2 } else { self.frame_size as usize }
    }
}

pub struct Atrac3Encoder {
    pub config: Atrac3Config,
    mdct: MdctContext,
    frame_count: u32,
}

const NUM_QMF_BANDS: u8 = 3; // 0xA3 header: 4 QMF bands (matching atracdenc)
const TOTAL_COEFFS: usize = 1024;

// Dequantization tables (sym -> dequant value, from EXE)
const DEQUANT_WL1: [f32; 5] = [0.0, 1953.0, 3906.0, -3906.0, -1953.0];

impl Atrac3Encoder {
    pub fn new(config: Atrac3Config) -> Self {
        let ch = config.channels as usize;
        Atrac3Encoder { config, mdct: MdctContext::new(ch), frame_count: 0 }
    }

    pub fn encode_frame(&mut self, samples: &[i16]) -> Vec<u8> {
        let fs = self.config.frame_size as usize;
        let mut frame = vec![0u8; fs];
        let ch = self.config.channels as usize;
        let spf = self.config.samples_per_frame();
        let cu = self.config.coding_unit_size();

        let mut ch_data = vec![vec![0.0f32; spf]; ch];
        for i in 0..spf {
            for c in 0..ch {
                let idx = i * ch + c;
                if idx < samples.len() { ch_data[c][i] = samples[idx] as f32; }
            }
        }

        let mut spectra = Vec::with_capacity(ch);
        for c in 0..ch { spectra.push(self.mdct.analyze_frame(c, &ch_data[c])); }

        for c in 0..ch {
            let mut flat = [0.0f32; TOTAL_COEFFS];
            for sb in 0..4 { for i in 0..256 { flat[sb * 256 + i] = spectra[c][sb][i]; } }
            Self::encode_coding_unit(&mut frame[c * cu..(c + 1) * cu], &flat);
        }

        self.frame_count += 1;
        frame
    }

    fn encode_coding_unit(buf: &mut [u8], spectrum: &[f32; TOTAL_COEFFS]) {
        let cu_bits = (buf.len() * 8) as u32;
        let num_subbands = 29usize;

        // DEBUG: Write the known-good frame format (29sb, wl=1, sf=15, all sym=0)
        // to verify that the issue is in encoding logic, not frame structure
        if false { // KNOWN-GOOD DEBUG FRAME
            buf[0] = 0xA2;
            let mut bp: u32 = 8;
            for _ in 0..3 { write_bits(buf, bp, 0, 3); bp += 3; }
            write_bits(buf, bp, 0, 5); bp += 5;
            write_bits(buf, bp, 28, 5); bp += 5;
            write_bits(buf, bp, 0, 1); bp += 1;
            for _ in 0..29 { write_bits(buf, bp, 2, 3); bp += 3; }
            for sb in 0..29 {
                write_bits(buf, bp, 15, 6); bp += 6;
                let n = SUBBAND_ENDS[sb] - SUBBAND_STARTS[sb];
                for _ in 0..n { write_bits(buf, bp, 0, 1); bp += 1; }
            }
            return;
        }

        // Header
        buf[0] = 0xA0 | NUM_QMF_BANDS;
        let mut bp: u32 = 8;

        // Gain control: 0 points per band
        for _ in 0..=NUM_QMF_BANDS as u32 { write_bits(buf, bp, 0, 3); bp += 3; }
        // Tonal: 0
        write_bits(buf, bp, 0, 5); bp += 5;

        // num_subbands - 1
        write_bits(buf, bp, (num_subbands - 1) as u32, 5); bp += 5;
        // VLC mode
        write_bits(buf, bp, 0, 1); bp += 1;

        // Determine word_len and scale factor per subband
        let mut word_lens = [-1i32; 29];
        let mut sf_indices = [0u32; 29];

        // Max dequant values per word_len
        let max_dq: [f32; 7] = [0.0, 3906.0, 4185.0, 4340.0, 4557.0, 4725.0, 4805.0];
        // Average VLC code lengths per word_len
        let avg_bits: [u32; 7] = [0, 2, 3, 4, 4, 5, 6];

        let header_bits = bp + (num_subbands as u32 * 3);
        let available_bits = cu_bits.saturating_sub(header_bits + 16);

        // First pass: assign word_lens based on subband importance
        let mut used_bits = 0u32;
        for sb in 0..num_subbands {
            let s = SUBBAND_STARTS[sb];
            let e = SUBBAND_ENDS[sb];
            let n = (e - s) as u32;
            let max_val = spectrum[s..e].iter().map(|&x| x.abs()).fold(0.0f32, f32::max);
            if max_val < 0.001 { continue; }

            // Scale factor: sf * max_dequant ≈ max_spectral_value
            // So sf ≈ max_val / max_dequant
            // For wl=1: max_dequant = 3906
            let needed_sf = max_val / 3906.0;
            let sf_idx = find_scale_factor(needed_sf);
            sf_indices[sb] = sf_idx;

            // Use wl=1 for now (5 symbols, reliable budget)
            let wl = 1i32;
            let est = 6 + n * 3;
            if used_bits + est <= available_bits {
                word_lens[sb] = wl;
                used_bits += est;
            }
        }

        // If still over budget, reduce from the back
        while used_bits > available_bits {
            let mut reduced = false;
            for sb in (0..num_subbands).rev() {
                if word_lens[sb] > 1 {
                    let s = SUBBAND_STARTS[sb];
                    let e = SUBBAND_ENDS[sb];
                    let n = (e - s) as u32;
                    used_bits -= n * avg_bits[word_lens[sb] as usize];
                    word_lens[sb] -= 1;
                    used_bits += n * avg_bits[word_lens[sb] as usize];
                    reduced = true;
                    break;
                } else if word_lens[sb] == 1 {
                    let s = SUBBAND_STARTS[sb];
                    let e = SUBBAND_ENDS[sb];
                    let n = (e - s) as u32;
                    used_bits -= 6 + n * avg_bits[1];
                    word_lens[sb] = -1;
                    reduced = true;
                    break;
                }
            }
            if !reduced { break; }
        }

        // Write word_lens
        for sb in 0..num_subbands {
            let v = (word_lens[sb] + 1) as u32;
            write_bits(buf, bp, v, 3);
            bp += 3;
        }

        // Track bits used for debugging
        let after_wl_bp = bp;

        // Write scale factors + VLC-coded mantissas
        for sb in 0..num_subbands {
            if word_lens[sb] < 0 { continue; }

            let sf_idx = sf_indices[sb];
            write_bits(buf, bp, sf_idx, 6);
            bp += 6;

            let wl = word_lens[sb] as usize;
            let sf = SCALE_FACTOR_VALUES[sf_idx as usize];
            let s = SUBBAND_STARTS[sb];
            let e = SUBBAND_ENDS[sb];
            let dq = get_dequant_table(wl);
            let num_syms = dq.len();

            for i in s..e {
                // Quantize: spectrum[i] / sf gives normalized value in [-1, 1]
                // Multiply by max_dequant to get into the dequant table range
                let normalized = spectrum[i] / sf;
                let target = normalized * max_dq[wl];
                let sym = nearest_symbol(target, dq, num_syms);
                let (code, len) = get_vlc_code(wl, sym);

                if len == 0 || bp + len as u32 > cu_bits - 2 {
                    let zsym = find_zero_symbol(dq, num_syms);
                    let (c, l) = get_vlc_code(wl, zsym);
                    if l > 0 { write_bits(buf, bp, c, l); bp += l as u32; }
                } else {
                    write_bits(buf, bp, code, len);
                    bp += len as u32;
                }
            }
        }

        // Verify we didn't overflow
        if bp > cu_bits {
            // Buffer overflow - rewrite as silence
            for b in buf.iter_mut() { *b = 0; }
            buf[0] = 0xA2;
        }
    }
}

fn find_scale_factor(min_sf: f32) -> u32 {
    for i in 0..64 {
        if SCALE_FACTOR_VALUES[i] >= min_sf { return i as u32; }
    }
    63
}

fn nearest_symbol(target: f32, dq: &[f32], num_syms: usize) -> usize {
    let mut best = 0;
    let mut best_dist = f32::MAX;
    for i in 0..num_syms {
        let dist = (target - dq[i]).abs();
        if dist < best_dist { best_dist = dist; best = i; }
    }
    best
}

fn find_zero_symbol(dq: &[f32], num_syms: usize) -> usize {
    for i in 0..num_syms { if dq[i] == 0.0 { return i; } }
    0
}

fn get_dequant_table(wl: usize) -> &'static [f32] {
    match wl {
        1 => &DEQUANT_WL1, 2 => &DEQUANT_WL2, 3 => &DEQUANT_WL3,
        4 => &DEQUANT_WL4, 5 => &DEQUANT_WL5, 6 => &DEQUANT_WL6,
        _ => &DEQUANT_WL1,
    }
}

fn get_vlc_code(wl: usize, sym: usize) -> (u32, u8) {
    match wl {
        1 => if sym < VLC_ENC_WL1.len() { VLC_ENC_WL1[sym] } else { VLC_ENC_WL1[0] },
        2 => if sym < VLC_ENC_WL2.len() { VLC_ENC_WL2[sym] } else { VLC_ENC_WL2[3] },
        3 => if sym < VLC_ENC_WL3.len() { VLC_ENC_WL3[sym] } else { VLC_ENC_WL3[4] },
        4 => if sym < VLC_ENC_WL4.len() { VLC_ENC_WL4[sym] } else { VLC_ENC_WL4[0] },
        5 => if sym < VLC_ENC_WL5.len() { VLC_ENC_WL5[sym] } else { VLC_ENC_WL5[0] },
        6 => if sym < VLC_ENC_WL6.len() { VLC_ENC_WL6[sym] } else { VLC_ENC_WL6[5] },
        _ => (0, 1),
    }
}

// Dequantization tables for wl 2-6
const DEQUANT_WL2: [f32; 7] = [4185.0, 1395.0, 2790.0, 0.0, -4185.0, -2790.0, -1395.0];
const DEQUANT_WL3: [f32; 9] = [4340.0, 1085.0, 2170.0, 3255.0, 0.0, -4340.0, -3255.0, -2170.0, -1085.0];
const DEQUANT_WL4: [f32; 15] = [0.0, 651.0, 1302.0, 1953.0, 2604.0, 3255.0, 3906.0, 4557.0, -4557.0, -3906.0, -3255.0, -2604.0, -1953.0, -1302.0, -651.0];
const DEQUANT_WL5: [f32; 31] = [0.0, 315.0, 630.0, 945.0, 1260.0, 1575.0, 1890.0, 2205.0, 2520.0, 2835.0, 3150.0, 3465.0, 3780.0, 4095.0, 4410.0, 4725.0, -4725.0, -4410.0, -4095.0, -3780.0, -3465.0, -3150.0, -2835.0, -2520.0, -2205.0, -1890.0, -1575.0, -1260.0, -945.0, -630.0, -315.0];
const DEQUANT_WL6: [f32; 63] = [775.0, 155.0, 310.0, 465.0, 620.0, 0.0, 930.0, 1085.0, 1240.0, 1395.0, 1550.0, 1705.0, 1860.0, 2015.0, 2170.0, 2325.0, 2480.0, 2635.0, 2790.0, 2945.0, 3100.0, 3255.0, 3410.0, 3565.0, 3720.0, 3875.0, 4030.0, 4185.0, 4340.0, 4495.0, 4650.0, 4805.0, -4805.0, -4650.0, -4495.0, -4340.0, -4185.0, -4030.0, -3875.0, -3720.0, -3565.0, -3410.0, -3255.0, -3100.0, -2945.0, -2790.0, -2635.0, -2480.0, -2325.0, -2170.0, -2015.0, -1860.0, -1705.0, -1550.0, -1395.0, -1240.0, -1085.0, -930.0, -775.0, -620.0, -465.0, -310.0, -155.0];
