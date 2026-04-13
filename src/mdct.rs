const SUBBAND_SAMPLES: usize = 256;
const NUM_SUBBANDS: usize = 4;
const FRAME_SAMPLES: usize = 1024;

pub struct MdctContext {
    /// Encode matrix (1024×1024): maps PCM directly to spectral coefficients
    /// (combines QMF analysis + MDCT in one step)
    encode_matrix: Vec<f32>,
    num_channels: usize,
}

impl MdctContext {
    pub fn new(num_channels: usize) -> Self {
        let matrix_data = std::fs::read("qmf_analysis.bin")
            .or_else(|_| std::fs::read("D:/test2/openatrac/qmf_analysis.bin"))
            .expect("Failed to load qmf_analysis.bin");
        assert_eq!(matrix_data.len(), FRAME_SAMPLES * FRAME_SAMPLES * 4);
        let encode_matrix: Vec<f32> = matrix_data.chunks(4)
            .map(|c| f32::from_le_bytes([c[0], c[1], c[2], c[3]]))
            .collect();
        MdctContext { encode_matrix, num_channels }
    }

    /// Transform PCM → 4×256 spectral coefficients via matrix multiplication.
    /// The matrix encodes the complete inverse of (IMDCT + QMF synthesis).
    pub fn analyze_frame(&mut self, _channel: usize, samples: &[f32]) -> [[f32; SUBBAND_SAMPLES]; NUM_SUBBANDS] {
        assert!(samples.len() >= FRAME_SAMPLES);

        // Matrix-vector multiply: spectral = encode_matrix * pcm
        let mut flat = [0.0f32; FRAME_SAMPLES];
        for i in 0..FRAME_SAMPLES {
            let mut sum = 0.0f64;
            let row = i * FRAME_SAMPLES;
            for j in 0..FRAME_SAMPLES {
                sum += self.encode_matrix[row + j] as f64 * samples[j] as f64;
            }
            flat[i] = sum as f32;
        }

        // Split into 4 subbands
        let mut output = [[0.0f32; SUBBAND_SAMPLES]; NUM_SUBBANDS];
        for sb in 0..NUM_SUBBANDS {
            output[sb].copy_from_slice(&flat[sb * SUBBAND_SAMPLES..(sb + 1) * SUBBAND_SAMPLES]);
        }
        output
    }
}
