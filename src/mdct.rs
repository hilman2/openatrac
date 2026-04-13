const SUBBAND_SAMPLES: usize = 256;
const NUM_SUBBANDS: usize = 4;
const FRAME_SAMPLES: usize = 1024;

pub struct MdctContext {
    /// Previous frame's PCM samples for overlap [channel][1024]
    prev_pcm: Vec<[f32; FRAME_SAMPLES]>,
    num_channels: usize,
}

impl MdctContext {
    pub fn new(num_channels: usize) -> Self {
        MdctContext {
            prev_pcm: vec![[0.0; FRAME_SAMPLES]; num_channels],
            num_channels,
        }
    }

    pub fn analyze_frame(&mut self, channel: usize, samples: &[f32]) -> [[f32; SUBBAND_SAMPLES]; NUM_SUBBANDS] {
        assert!(samples.len() >= FRAME_SAMPLES);

        // The ATRAC3 encoder uses interleaved 4-band processing:
        // 1024 PCM samples are rearranged into 4 interleaved bands
        // Then per-band 512-point MDCT (256 overlap + 256 current) produces 256 spectral coefficients

        // Rearrange BOTH previous and current frames
        let prev = &self.prev_pcm[channel];

        // Interleaving pattern from decompiled FUN_0043a220:
        // work[4*i+0] = pcm[i]        (band 0, samples 0-255, forward)
        // work[4*i+2] = pcm[i+512]    (band 2, samples 512-767, forward)
        // work[1023-4*i-2] = pcm[i+256] (band 1, samples 256-511, reverse)
        // work[1023-4*i] = pcm[i+768]   (band 3, samples 768-1023, reverse)

        let mut prev_il = [0.0f32; FRAME_SAMPLES];
        let mut curr_il = [0.0f32; FRAME_SAMPLES];
        for i in 0..256 {
            prev_il[4*i]       = prev[i];
            prev_il[4*i + 2]   = prev[i + 512];
            prev_il[1023-4*i-2] = prev[i + 256];
            prev_il[1023-4*i]   = prev[i + 768];

            curr_il[4*i]       = samples[i];
            curr_il[4*i + 2]   = samples[i + 512];
            curr_il[1023-4*i-2] = samples[i + 256];
            curr_il[1023-4*i]   = samples[i + 768];
        }

        // Save current PCM for next frame
        self.prev_pcm[channel].copy_from_slice(&samples[..FRAME_SAMPLES]);

        // Per-subband MDCT with overlap
        let mut output = [[0.0f32; SUBBAND_SAMPLES]; NUM_SUBBANDS];
        let n = 512usize;

        for sb in 0..NUM_SUBBANDS {
            // Extract 256 samples from previous and current interleaved buffers at stride 4
            // Bands 1 and 3 were written in REVERSE order in the interleaving step,
            // so we need to read them in reverse to get correct time-domain order
            let mut block = [0.0f64; 512];
            for i in 0..256 {
                let idx = if sb == 1 || sb == 3 {
                    4 * (255 - i) + sb  // reverse for odd bands
                } else {
                    4 * i + sb          // forward for even bands
                };
                block[i] = prev_il[idx] as f64;

                let idx2 = if sb == 1 || sb == 3 {
                    4 * (255 - i) + sb
                } else {
                    4 * i + sb
                };
                block[256 + i] = curr_il[idx2] as f64;
            }

            // Apply sine window
            for i in 0..n {
                let w = ((i as f64 + 0.5) * std::f64::consts::PI / n as f64).sin();
                block[i] *= w;
            }

            // Forward MDCT: 512 → 256 coefficients
            let scale = 2.0 / n as f64;
            for k in 0..256 {
                let mut sum = 0.0f64;
                let kf = std::f64::consts::PI / n as f64 * (k as f64 + 0.5);
                for i in 0..n {
                    sum += block[i] * ((i as f64 + 0.5 + n as f64 / 4.0) * kf).cos();
                }
                output[sb][k] = (sum * scale) as f32;
            }
        }

        output
    }
}
