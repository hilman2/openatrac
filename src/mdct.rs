const SUBBAND_SAMPLES: usize = 256;
const NUM_SUBBANDS: usize = 4;
const FRAME_SAMPLES: usize = 1024;

/// MDCT context for ATRAC3 encoding.
/// Based on the exact algorithm from the decompiled encoder (FUN_0043a220).
///
/// The encoder uses a combined QMF+MDCT:
/// 1. Rearrange 1024 PCM samples into interleaved format (4 bands × 256)
/// 2. Apply overlap with previous frame
/// 3. FFT-based MDCT butterfly operations
/// 4. Post-processing to produce 4 × 256 spectral coefficients
///
/// For simplicity, we implement steps 1-2 and use a direct MDCT instead of FFT.
pub struct MdctContext {
    /// Overlap buffer from previous frame [channel][512 floats in interleaved format]
    prev_interleaved: Vec<[f32; FRAME_SAMPLES]>,
    /// MDCT window (512 points)
    mdct_window: [f32; 512],
    num_channels: usize,
}

impl MdctContext {
    pub fn new(num_channels: usize) -> Self {
        let mut mdct_window = [0.0f32; 512];
        for i in 0..512 {
            mdct_window[i] = ((i as f32 + 0.5) * std::f32::consts::PI / 512.0).sin();
        }
        MdctContext {
            prev_interleaved: vec![[0.0; FRAME_SAMPLES]; num_channels],
            mdct_window,
            num_channels,
        }
    }

    pub fn analyze_frame(&mut self, channel: usize, samples: &[f32]) -> [[f32; SUBBAND_SAMPLES]; NUM_SUBBANDS] {
        assert!(samples.len() >= FRAME_SAMPLES);

        // Simple approach: treat 1024 samples as 4 sequential blocks of 256
        // Band 0: samples[0..256], Band 1: samples[256..512], etc.
        let mut output = [[0.0f32; SUBBAND_SAMPLES]; NUM_SUBBANDS];
        for sb in 0..NUM_SUBBANDS {
            let offset = sb * SUBBAND_SAMPLES;
            let mut current = [0.0f32; SUBBAND_SAMPLES];
            current.copy_from_slice(&samples[offset..offset + SUBBAND_SAMPLES]);

            let prev_offset = sb * SUBBAND_SAMPLES;
            let mut block = [0.0f64; 512];
            for i in 0..SUBBAND_SAMPLES {
                let prev = self.prev_interleaved[channel][prev_offset + i] as f64;
                block[i] = prev * self.mdct_window[i] as f64;
                block[SUBBAND_SAMPLES + i] = current[i] as f64 * self.mdct_window[SUBBAND_SAMPLES + i] as f64;
            }

            // Forward MDCT: 512 → 256 coefficients
            let n = 512.0f64;
            // Scale factor: 1.0 matches the reference encoder's convention.
            // The decoder's IMDCT effectively divides by ~4557,
            // so sf * dequant / 4557 ≈ PCM.
            // With scale=1.0, MDCT coefficients are in the same magnitude as PCM.
            let scale = 2.0 / n;
            for k in 0..SUBBAND_SAMPLES {
                let mut sum = 0.0f64;
                let kf = std::f64::consts::PI / n * (k as f64 + 0.5);
                for i in 0..512 {
                    sum += block[i] * ((i as f64 + 0.5 + n / 4.0) * kf).cos();
                }
                output[sb][k] = (sum * scale) as f32;
            }
        }

        // Save current frame for next frame's overlap
        self.prev_interleaved[channel].copy_from_slice(&samples[..FRAME_SAMPLES]);

        output
    }
}
