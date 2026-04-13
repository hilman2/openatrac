use crate::tables::mdct_native::{POST_COS, POST_SCALE, POST_INDEX};

const SUBBAND_SAMPLES: usize = 256;
const NUM_SUBBANDS: usize = 4;
const FRAME_SAMPLES: usize = 1024;

pub struct MdctContext {
    /// Previous frame's interleaved work buffer [channel][1024]
    overlap: Vec<[f32; 1024]>,
    num_channels: usize,
}

impl MdctContext {
    pub fn new(num_channels: usize) -> Self {
        MdctContext {
            overlap: vec![[0.0; 1024]; num_channels],
            num_channels,
        }
    }

    /// Transform 1024 PCM samples into 4×256 spectral coefficients.
    /// This replicates the algorithm in FUN_0043a220 from the decompiled encoder.
    pub fn analyze_frame(&mut self, channel: usize, samples: &[f32]) -> [[f32; SUBBAND_SAMPLES]; NUM_SUBBANDS] {
        assert!(samples.len() >= FRAME_SAMPLES);

        // Step 1: Input rearrangement (4-band interleaving)
        // Creates a 1024-element work buffer with bands interleaved
        let mut work = [0.0f32; FRAME_SAMPLES];
        for i in 0..256 {
            work[4 * i]           = samples[i];       // Band 0 forward
            work[4 * i + 2]       = samples[i + 512]; // Band 2 forward
            work[1023 - 4 * i - 2] = samples[i + 256]; // Band 1 reverse
            work[1023 - 4 * i]     = samples[i + 768]; // Band 3 reverse
        }

        // Step 2: The MDCT needs 512 samples per subband (256 overlap + 256 current)
        // At stride 4, that's 1024 + 1024 = 2048 interleaved samples
        // Previous frame's interleaved data (1024 floats) + current work (1024 floats)
        let mut combined = vec![0.0f32; 2048];
        combined[..1024].copy_from_slice(&self.overlap[channel][..]);
        combined[1024..].copy_from_slice(&work[..]);
        // Save current frame for next overlap
        self.overlap[channel][..].copy_from_slice(&work[..]);

        // Step 3: Per-subband forward MDCT
        // Each subband is extracted at stride 4 from the combined buffer,
        // giving 256 samples per subband (128 from overlap + 128 from current)
        let mut output = [[0.0f32; SUBBAND_SAMPLES]; NUM_SUBBANDS];

        // Sine window for 512-point MDCT
        let n = 512;

        for sb in 0..NUM_SUBBANDS {
            // Extract subband samples from interleaved combined buffer
            let mut block = [0.0f64; 512];
            for i in 0..256 {
                // First half (overlap): from previous frame's interleaved data
                block[i] = combined[4 * i + sb] as f64;
                // Second half (current): from current frame's interleaved data
                block[256 + i] = combined[1024 + 4 * i + sb] as f64;
            }

            // Apply sine window
            for i in 0..n {
                let w = ((i as f64 + 0.5) * std::f64::consts::PI / n as f64).sin();
                block[i] *= w;
            }

            // Forward MDCT: X[k] = sum x[n] * cos(PI/N * (n + 0.5 + N/4) * (k + 0.5))
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

        // Step 4: Apply post-processing twiddle rotation
        // The reference encoder applies rotation using POST_COS and POST_SCALE
        // tables indexed by POST_INDEX
        // This corrects the phase relationship between the QMF subbands
        for idx in 0..128 {
            let k = POST_INDEX[idx] as usize;
            if k >= 128 { continue; }

            let cos_val = POST_COS[k];
            let scale_val = POST_SCALE[k]; // approximately 2*cos(theta)

            // Rotate pairs between subbands 0 and 2 (low and mid-hi bands)
            let a = output[0][k];
            let b = output[2][127 - k];
            let half_scale = scale_val * 0.5;
            output[0][k] = a * cos_val + b * half_scale;
            output[2][127 - k] = -a * half_scale + b * cos_val;
        }

        output
    }
}
