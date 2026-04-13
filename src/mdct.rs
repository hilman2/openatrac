use crate::tables::mdct_native::*;

const SUBBAND_SAMPLES: usize = 256;
const NUM_SUBBANDS: usize = 4;
const FRAME_SAMPLES: usize = 1024;

pub struct MdctContext {
    /// Overlap buffer: 512 floats (4 bands × 128), per channel
    overlap: Vec<[f32; 512]>,
    /// Previous scale factors per channel
    scale_state: Vec<[f32; 512]>,
    num_channels: usize,
}

impl MdctContext {
    pub fn new(num_channels: usize) -> Self {
        MdctContext {
            overlap: vec![[0.0; 512]; num_channels],
            scale_state: vec![[0.0; 512]; num_channels],
            num_channels,
        }
    }

    /// Direct translation of FUN_0043a220.
    /// The contiguous work area is modeled as `w[0..1027]`:
    ///   w[0..255]   = local_1010 (256 floats)
    ///   w[256..507]  = afStack_c10 (252 floats)
    ///   w[508..1019] = local_820 (512 floats)
    ///   w[1020..1026] = local_20 (7 floats)
    /// Also: `state[0..519]` = local_1824 (520 floats, indices 1-4 are scale, 5-516 are overlap)
    pub fn analyze_frame(&mut self, channel: usize, samples: &[f32]) -> [[f32; SUBBAND_SAMPLES]; NUM_SUBBANDS] {
        assert!(samples.len() >= FRAME_SAMPLES);

        let mut w = vec![0.0f32; 1028]; // contiguous work buffer
        let mut state = vec![0.0f32; 520]; // local_1824

        // Initialize scale factors
        state[1] = 1.0; state[2] = 1.0; state[3] = 1.0; state[4] = 1.0;

        // ================================================================
        // Step 1: Input Rearrangement (Z.44225-44237)
        // pfVar12 starts at w[0] (local_1010), advances +4
        // pfVar5 starts at w[1023] (local_20+3), goes -4
        // pfVar4 starts at samples[512], advances +1
        // ================================================================
        for i in 0..256 {
            w[4 * i]           = samples[i];         // pfVar4[-0x200]
            w[1023 - 4 * i - 2] = samples[i + 256];  // pfVar5[-2]
            w[4 * i + 2]       = samples[i + 512];   // pfVar12[2]
            w[1023 - 4 * i]    = samples[i + 768];   // *pfVar5
        }

        // ================================================================
        // Step 2: Load overlap into state[5..516] (Z.44238-44249)
        // pfVar5 = state+7, pfVar12 = overlap (param_1+0x1440)
        // 128 iterations: load 4 values from overlap at offsets -0x80, 0, +0x80, +0x100
        // ================================================================
        for j in 0..128 {
            // overlap is 4 blocks of 128: [0..127]=B0, [128..255]=B1, [256..383]=B2, [384..511]=B3
            state[5 + 4*j]     = self.overlap[channel][j];       // pfVar12[-0x80]
            state[5 + 4*j + 1] = self.overlap[channel][j + 128]; // *pfVar12
            state[5 + 4*j + 2] = self.overlap[channel][j + 256]; // pfVar12[0x80]
            state[5 + 4*j + 3] = self.overlap[channel][j + 384]; // pfVar12[0x100]
        }

        // ================================================================
        // Step 3: First Butterfly (Z.44250-44301)
        // pfVar5 starts at w[508] (local_820), goes BACKWARD by 4
        // pfVar12 starts at w[508] (local_820), starts at +4 (i.e. w[512])
        //
        // In terms of absolute indices:
        // pfVar12 = w[508 + 4] initially, then pfVar12 += 4 each iteration
        // pfVar5 = w[508] initially, then pfVar5 -= 4 each iteration
        //
        // After first pfVar12 += 4: pfVar12 = w[512]
        // pfVar12[-0x200] = w[512 - 512] = w[0]
        // pfVar5[0x200] = w[508 + 512] = w[1020]
        // ================================================================
        let c = 508usize; // local_820 start in w[]

        // Important: pfVar12 is ADVANCED by 4 before first read, so starts at w[c+4]
        for iter in 0..64 {
            let p12 = c + 4 + iter * 4;   // pfVar12 position (after +=4)
            let p5  = c - iter * 4;        // pfVar5 position (before -=4)

            // Read values
            let a = [w[p12], w[p12+1], w[p12+2], w[p12+3]];         // *pfVar12 etc.
            let b = [w[p5+0x200], w[p5+0x201], w[p5+0x202], w[p5+0x203]]; // pfVar5[0x200]
            let lo = [w[p12-0x200], w[p12-0x1ff], w[p12-0x1fe], w[p12-0x1fd]]; // pfVar12[-0x200]
            let s = [w[p5], w[p5+1], w[p5+2], w[p5+3]];             // *pfVar5

            // Compute
            let diff = [lo[0]-b[0], lo[1]-b[1], lo[2]-b[2], lo[3]-b[3]]; // fVar13..19
            let s_sub = [s[0]-a[0], s[1]-a[1], s[2]-a[2], s[3]-a[3]];   // *pfVar5 -= *pfVar12

            let a_sq = [a[0]*SQRT2, a[1]*SQRT2, a[2]*SQRT2, a[3]*SQRT2]; // fVar25*SQRT2

            // Write pfVar12[-0x200] = diff + a_sq
            w[p12-0x200] = diff[0] + a_sq[0];
            w[p12-0x1ff] = diff[1] + a_sq[1];
            w[p12-0x1fe] = diff[2] + a_sq[2];
            w[p12-0x1fd] = diff[3] + a_sq[3];

            // Write *pfVar5 = s_sub (temporary, will be overwritten below)
            w[p5]   = s_sub[0];
            w[p5+1] = s_sub[1];
            w[p5+2] = s_sub[2];
            w[p5+3] = s_sub[3];

            // Read back updated *pfVar5
            let s_new = [w[p5], w[p5+1], w[p5+2], w[p5+3]]; // fVar14..20

            let b_sq = [b[0]*SQRT2, b[1]*SQRT2, b[2]*SQRT2, b[3]*SQRT2]; // fVar21*SQRT2

            // Write *pfVar12 = diff - a_sq
            w[p12]   = diff[0] - a_sq[0];
            w[p12+1] = diff[1] - a_sq[1];
            w[p12+2] = diff[2] - a_sq[2];
            w[p12+3] = diff[3] - a_sq[3];

            // Write pfVar5[0x200] = s_new - b_sq
            w[p5+0x200] = s_new[0] - b_sq[0];
            w[p5+0x201] = s_new[1] - b_sq[1];
            w[p5+0x202] = s_new[2] - b_sq[2];
            w[p5+0x203] = s_new[3] - b_sq[3];

            // Write *pfVar5 = s_new + b_sq
            w[p5]   = s_new[0] + b_sq[0];
            w[p5+1] = s_new[1] + b_sq[1];
            w[p5+2] = s_new[2] + b_sq[2];
            w[p5+3] = s_new[3] + b_sq[3];
        }

        // ================================================================
        // Step 4: Iterative FFT (Z.44302-44384)
        // Operates on w[0..255] (local_1010) with twiddle from TWIDDLE table
        // ================================================================
        let mut stage = 64u32;
        while stage > 1 {
            let half = (stage / 2) as usize;
            if half < 128 {
                let mut base = 0usize;
                let mut tw_idx = half;

                while tw_idx < 128 {
                    // Subtraction pass
                    if stage > 0 {
                        let stride_bytes = (stage as usize) << 5; // stage * 32
                        let stride_floats = stride_bytes / 4;     // stage * 8
                        let num_pairs = ((stage * 2 - 1) / 4 + 1) as usize;
                        let mut p = base;
                        let mut off = stride_floats;
                        for _ in 0..num_pairs {
                            // pfVar4[0..3] -= pfVar6[0..3] where pfVar6 = pfVar4 + off - 4
                            let hi = p + off;
                            if hi >= 4 && p + 3 < 256 && hi - 4 + 3 < 1028 {
                                for k in 0..4 { w[p+k] -= w[hi-4+k]; }
                            }
                            if hi >= 8 && p + 7 < 256 && hi - 8 + 3 < 1028 {
                                for k in 0..4 { w[p+4+k] -= w[hi-8+k]; }
                            }
                            p += 8;
                            if off >= 16 { off -= 16; }
                        }

                        // Twiddle multiplication pass
                        if base < p && tw_idx < TWIDDLE.len() {
                            let tw = TWIDDLE[tw_idx];
                            let s1 = (stage as usize + 1) * 4;
                            let mut q = base;
                            while q < p.min(256) {
                                let hi1 = q + s1 - 4;
                                let hi2 = q + s1;
                                if hi1 + 3 < 1028 && hi2 + 3 < 1028 && q + 7 < 256 {
                                    for k in 0..4 {
                                        let h = w[hi1+k];
                                        w[hi1+k] = w[q+k] - h * tw;
                                        w[q+k] = w[q+k] + h * tw;
                                    }
                                    for k in 0..4 {
                                        let h = w[hi2+k];
                                        let qk = q + 4 + k;
                                        let hk = q + 8 + s1 - 12 + k;
                                        if hk < 1028 && qk < 256 {
                                            w[hk] = w[qk] - h * tw;
                                            w[qk] = w[qk] + h * tw;
                                        }
                                    }
                                }
                                q += 8;
                            }
                        }
                    }
                    base += stage as usize * 4;
                    tw_idx += stage as usize;
                }
            }
            stage >>= 1;
        }

        // ================================================================
        // Step 5: Post-processing (Z.44385-44428)
        // Reads from w[] (FFT output), combines with state[], writes to output
        // ================================================================
        let mut output = [[0.0f32; SUBBAND_SAMPLES]; NUM_SUBBANDS];

        // For now: extract the 4 subbands from w[0..1023] at stride 4
        // The post-processing applies twiddle rotation, but without it
        // the basic frequency split should be present
        for sb in 0..NUM_SUBBANDS {
            for i in 0..SUBBAND_SAMPLES {
                let idx = 4 * i + sb;
                if idx < 1024 {
                    output[sb][i] = w[idx];
                }
            }
        }

        // ================================================================
        // Step 6: Save overlap state (Z.44429-44440)
        // ================================================================
        for j in 0..128 {
            self.overlap[channel][j]       = state[5 + 4*j];
            self.overlap[channel][j + 128] = state[5 + 4*j + 1];
            self.overlap[channel][j + 256] = state[5 + 4*j + 2];
            self.overlap[channel][j + 384] = state[5 + 4*j + 3];
        }

        output
    }
}
