const NUM_SUBBANDS: usize = 4;
const SUBBAND_SAMPLES: usize = 256;
const FRAME_SAMPLES: usize = 1024;

/// QMF 48-tap half-coefficients (from FFmpeg/atracdenc)
const TAP_HALF: [f32; 24] = [
   -0.00001461907, -0.00009205479,-0.000056157569,0.00030117269,
    0.0002422519,  -0.00085293897,-0.0005205574,  0.0020340169,
    0.00078333891, -0.0042153862, -0.00075614988, 0.0078402944,
   -0.000061169922,-0.01344162,    0.0024626821,  0.021736089,
   -0.007801671,   -0.034090221,   0.01880949,    0.054326009,
   -0.043596379,   -0.099384367,   0.13207909,    0.46424159,
];

pub struct MdctContext {
    /// QMF window = TapHalf * 2, mirrored to 48 taps
    qmf_window: [f32; 48],
    /// QMF PCM delay buffers per channel, 3 stages (46 samples each)
    qmf_delay: Vec<[[f32; 46]; 3]>,
    /// MDCT overlap per channel per subband (256 samples)
    mdct_overlap: Vec<[[f32; SUBBAND_SAMPLES]; NUM_SUBBANDS]>,
    num_channels: usize,
}

impl MdctContext {
    pub fn new(num_channels: usize) -> Self {
        // Build QMF window: mirrored TapHalf * 2 (matching atracdenc)
        let mut qmf_window = [0.0f32; 48];
        for i in 0..24 {
            qmf_window[i] = TAP_HALF[i] * 2.0;
            qmf_window[47 - i] = TAP_HALF[i] * 2.0;
        }

        MdctContext {
            qmf_window,
            qmf_delay: vec![[[0.0; 46]; 3]; num_channels],
            mdct_overlap: vec![[[0.0; SUBBAND_SAMPLES]; NUM_SUBBANDS]; num_channels],
            num_channels,
        }
    }

    pub fn analyze_frame(&mut self, channel: usize, samples: &[f32]) -> [[f32; SUBBAND_SAMPLES]; NUM_SUBBANDS] {
        // Step 1: QMF analysis tree — split 1024 PCM → 4 × 256 subbands
        // Tree structure (same as decoder synthesis but reversed):
        //   1024 → (512 low, 512 high)
        //   512 low → (256 band0, 256 band1)
        //   512 high → (256 band2, 256 band3)
        // QMF tree: note the decoder uses IQMF(band0,band1)→low, IQMF(band2,band3)→high,
        // IQMF(low,high)→output. The analysis reverses this.
        // In atracdenc: Analysis(input, lower, upper) where lower=lowpass, upper=highpass
        // QMF analysis tree. The decoder synthesis does:
        //   IQMF(band0, band1, 256) → p1 (low 512)
        //   IQMF(band2, band3, 256) → p3 (high 512)
        //   IQMF(p1, p3, 512) → output 1024
        // Our analysis reverses this. The key: in the analysis butterfly,
        // 'lower' = lo+up (sum = lowpass), 'upper' = lo-up (diff = highpass)
        // But the decoder's IQMF interleaves as: temp[2i]=lo+hi, temp[2i+1]=lo-hi
        // So the decoder's 'lo' input becomes the SUM in the output,
        // and 'hi' becomes the DIFFERENCE.
        // For perfect reconstruction: analysis(synthesis(lo,hi)) = (lo, hi)
        // This means: lower output of analysis = the 'lo' input of synthesis = band0/band2
        //             upper output of analysis = the 'hi' input of synthesis = band1/band3
        let (low, high) = self.qmf_analysis_pair(channel, 2, samples, 512);
        let (band0, band1) = self.qmf_analysis_pair(channel, 0, &low, 256);
        // atracdenc: Qmf3 → subs[3]=lower, subs[2]=upper (reversed!)
        let (band3_lo, band2_up) = self.qmf_analysis_pair(channel, 1, &high, 256);
        let band2 = band2_up;
        let band3 = band3_lo;

        // Step 2: Forward MDCT per subband (512-point → 256 coefficients)
        let bands = [band0, band1, band2, band3];
        let mut output = [[0.0f32; SUBBAND_SAMPLES]; NUM_SUBBANDS];

        for sb in 0..NUM_SUBBANDS {
            // MDCT with overlap — NO windowing (atracdenc applies window externally)
            let mut block = [0.0f64; 512];
            let win_n = 512.0f64;
            for i in 0..256 {
                block[i] = self.mdct_overlap[channel][sb][i] as f64;
                block[256 + i] = bands[sb][i] as f64;
            }
            // Save current for next overlap
            self.mdct_overlap[channel][sb].copy_from_slice(&bands[sb]);

            // Forward MDCT: atracdenc uses cos((PI/N)*(n+0.5+N/2)*(k+0.5))
            // Note: N/2 phase offset, NOT N/4! And scale=1.0 (no normalization)
            let scale = 1.0;
            for k in 0..256 {
                let mut sum = 0.0f64;
                let kf = std::f64::consts::PI / win_n * (k as f64 + 0.5);
                for n in 0..512 {
                    sum += block[n] * ((n as f64 + 0.5 + win_n / 2.0) * kf).cos();
                }
                output[sb][k] = (sum * scale) as f32;
            }

            // Step 3: Swap odd bands (critical! from atracdenc: if band & 1, swap)
            if sb & 1 != 0 {
                let half = SUBBAND_SAMPLES / 2;
                for i in 0..half {
                    output[sb].swap(i, SUBBAND_SAMPLES - 1 - i);
                }
            }
        }

        output
    }

    /// QMF analysis: split N*2 input samples into N lower + N upper frequency samples.
    /// Direct port from atracdenc's Analysis() function.
    fn qmf_analysis_pair(&mut self, channel: usize, stage: usize,
                         input: &[f32], n_out: usize) -> (Vec<f32>, Vec<f32>) {
        let n_in = n_out * 2;
        let delay = &mut self.qmf_delay[channel][stage];

        // Build PcmBuffer: [delay_46 | input]
        let mut pcm_buf = vec![0.0f32; 46 + n_in];
        pcm_buf[..46].copy_from_slice(delay);
        for i in 0..n_in.min(input.len()) {
            pcm_buf[46 + i] = input[i];
        }

        let mut lower = vec![0.0f32; n_out];
        let mut upper = vec![0.0f32; n_out];

        // atracdenc's Analysis() — the core QMF filter
        for j in (0..n_in).step_by(2) {
            let out_idx = j / 2;
            let mut lo = 0.0f32;
            let mut up = 0.0f32;

            for i in 0..24 {
                // QmfWindow[2*i] applied to even-indexed samples
                // QmfWindow[2*i+1] applied to odd-indexed samples
                let buf_idx = 48 - 1 + j; // = 47 + j
                lo += self.qmf_window[2 * i] * pcm_buf[buf_idx - 2 * i];
                up += self.qmf_window[2 * i + 1] * pcm_buf[buf_idx - 2 * i - 1];
            }

            // Butterfly: sum and difference
            let temp = up;
            upper[out_idx] = lo - up;
            lower[out_idx] = lo + temp;
        }

        // Save delay: last 46 samples of pcm_buf
        delay.copy_from_slice(&pcm_buf[n_in..n_in + 46]);

        (lower, upper)
    }
}
