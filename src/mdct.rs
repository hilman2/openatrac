const SUBBAND_SAMPLES: usize = 256;
const NUM_SUBBANDS: usize = 4;
const FRAME_SAMPLES: usize = 1024;

/// ATRAC3 forward transform: PCM → spectral coefficients.
/// Uses an empirically derived encoder matrix (pseudo-inverse of the real decoder).
pub struct MdctContext {
    /// Encoder matrix: 1024×1024 (maps PCM to spectral coefficients)
    encoder_matrix: Vec<f32>,
    num_channels: usize,
}

impl MdctContext {
    pub fn new(num_channels: usize) -> Self {
        let matrix_data = std::fs::read("encoder_matrix.bin")
            .or_else(|_| std::fs::read("D:/test2/openatrac/encoder_matrix.bin"))
            .expect("Failed to load encoder_matrix.bin - run tools/build_encoder_matrix.py first");

        assert_eq!(matrix_data.len(), FRAME_SAMPLES * FRAME_SAMPLES * 4,
            "Encoder matrix must be 1024x1024 f32 = 4194304 bytes");

        let encoder_matrix: Vec<f32> = matrix_data.chunks(4)
            .map(|c| f32::from_le_bytes([c[0], c[1], c[2], c[3]]))
            .collect();

        MdctContext { encoder_matrix, num_channels }
    }

    /// Transform 1024 PCM samples into 4×256 spectral coefficients
    /// via matrix-vector multiplication: spectral = E × pcm
    pub fn analyze_frame(&mut self, _channel: usize, samples: &[f32]) -> [[f32; SUBBAND_SAMPLES]; NUM_SUBBANDS] {
        assert!(samples.len() >= FRAME_SAMPLES);

        let mut flat = [0.0f32; FRAME_SAMPLES];
        for i in 0..FRAME_SAMPLES {
            let mut sum = 0.0f64;
            let row = i * FRAME_SAMPLES;
            for j in 0..FRAME_SAMPLES {
                sum += self.encoder_matrix[row + j] as f64 * samples[j] as f64;
            }
            flat[i] = sum as f32;
        }

        let mut output = [[0.0f32; SUBBAND_SAMPLES]; NUM_SUBBANDS];
        for sb in 0..NUM_SUBBANDS {
            output[sb].copy_from_slice(&flat[sb * SUBBAND_SAMPLES..(sb + 1) * SUBBAND_SAMPLES]);
        }
        output
    }
}
