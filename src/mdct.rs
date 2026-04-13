const NUM_SUBBANDS: usize = 4;
const SUBBAND_SAMPLES: usize = 256;
const FRAME_SAMPLES: usize = 1024;
const MDCT_N: usize = 512;

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
    qmf_window: [f32; 48],
    qmf_delay: Vec<[[f32; 46]; 3]>,
    mdct_overlap: Vec<[[f32; SUBBAND_SAMPLES]; NUM_SUBBANDS]>,
    /// MDCT window (512 points) — from Sony decoder's init_imdct_window()
    mdct_window: [f32; MDCT_N],
    num_channels: usize,
}

impl MdctContext {
    pub fn new(num_channels: usize) -> Self {
        let mut qmf_window = [0.0f32; 48];
        for i in 0..24 {
            qmf_window[i] = TAP_HALF[i] * 2.0;
            qmf_window[47 - i] = TAP_HALF[i] * 2.0;
        }

        // MDCT window from Sony decoder: sin(((i+0.5)/256 - 0.5)*PI) + 1.0, normalized
        let mut mdct_window = [0.0f32; MDCT_N];
        for i in 0..MDCT_N {
            mdct_window[i] = ((i as f32 + 0.5) / 256.0 - 0.5).mul_add(std::f32::consts::PI, 0.0).sin() + 1.0;
        }
        // Normalize: w[i] = w[i] / sqrt(0.5*(w[i]^2 + w[N-1-i]^2))
        for i in 0..MDCT_N / 2 {
            let j = MDCT_N - 1 - i;
            let wi = mdct_window[i];
            let wj = mdct_window[j];
            let norm = (0.5 * (wi * wi + wj * wj)).sqrt();
            if norm > 1e-10 { mdct_window[i] = wi / norm; mdct_window[j] = wj / norm; }
        }

        MdctContext {
            qmf_window,
            qmf_delay: vec![[[0.0; 46]; 3]; num_channels],
            mdct_overlap: vec![[[0.0; SUBBAND_SAMPLES]; NUM_SUBBANDS]; num_channels],
            mdct_window,
            num_channels,
        }
    }

    pub fn analyze_frame(&mut self, channel: usize, samples: &[f32]) -> [[f32; SUBBAND_SAMPLES]; NUM_SUBBANDS] {
        // QMF analysis tree: 1024 → 4 × 256
        let (low, high) = self.qmf_analysis_pair(channel, 2, samples, 512);
        let (band0, band1) = self.qmf_analysis_pair(channel, 0, &low, 256);
        let (band3, band2) = self.qmf_analysis_pair(channel, 1, &high, 256);

        let bands = [band0, band1, band2, band3];
        let mut output = [[0.0f32; SUBBAND_SAMPLES]; NUM_SUBBANDS];

        for sb in 0..NUM_SUBBANDS {
            // MDCT with sine window + overlap (standard for ATRAC3)
            let mut block = [0.0f64; MDCT_N];
            for i in 0..SUBBAND_SAMPLES {
                let w0 = ((i as f64 + 0.5) * std::f64::consts::PI / MDCT_N as f64).sin();
                let w1 = (((SUBBAND_SAMPLES + i) as f64 + 0.5) * std::f64::consts::PI / MDCT_N as f64).sin();
                block[i] = self.mdct_overlap[channel][sb][i] as f64 * w0;
                block[SUBBAND_SAMPLES + i] = bands[sb][i] as f64 * w1;
            }
            self.mdct_overlap[channel][sb].copy_from_slice(&bands[sb]);

            // Forward MDCT: cos(PI/N * (n+0.5+N/2) * (k+0.5)), scale=1.0
            let n = MDCT_N as f64;
            for k in 0..SUBBAND_SAMPLES {
                let mut sum = 0.0f64;
                let kf = std::f64::consts::PI / n * (k as f64 + 0.5);
                for i in 0..MDCT_N {
                    sum += block[i] * ((i as f64 + 0.5 + n / 2.0) * kf).cos();
                }
                output[sb][k] = sum as f32;
            }

            // Swap odd bands (from atracdenc: reverses spectral order for QMF mirroring)
            if sb & 1 != 0 {
                for i in 0..SUBBAND_SAMPLES / 2 {
                    output[sb].swap(i, SUBBAND_SAMPLES - 1 - i);
                }
            }
        }

        output
    }

    fn qmf_analysis_pair(&mut self, channel: usize, stage: usize,
                         input: &[f32], n_out: usize) -> (Vec<f32>, Vec<f32>) {
        let n_in = n_out * 2;
        let delay = &mut self.qmf_delay[channel][stage];

        let mut pcm_buf = vec![0.0f32; 46 + n_in];
        pcm_buf[..46].copy_from_slice(delay);
        for i in 0..n_in.min(input.len()) {
            pcm_buf[46 + i] = input[i];
        }

        let mut lower = vec![0.0f32; n_out];
        let mut upper = vec![0.0f32; n_out];

        for j in (0..n_in).step_by(2) {
            let out_idx = j / 2;
            let mut lo = 0.0f32;
            let mut up = 0.0f32;
            for i in 0..24 {
                let buf_idx = 48 - 1 + j;
                lo += self.qmf_window[2 * i] * pcm_buf[buf_idx - 2 * i];
                up += self.qmf_window[2 * i + 1] * pcm_buf[buf_idx - 2 * i - 1];
            }
            let temp = up;
            upper[out_idx] = lo - up;
            lower[out_idx] = lo + temp;
        }

        delay.copy_from_slice(&pcm_buf[n_in..n_in + 46]);
        (lower, upper)
    }
}
