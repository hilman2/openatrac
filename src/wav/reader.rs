use std::fs::File;
use std::io::{self, Read, Seek, SeekFrom};

pub struct WavReader {
    pub channels: u16,
    pub sample_rate: u32,
    pub bits_per_sample: u16,
    pub num_samples: u32, // per channel
    pub data: Vec<i16>,
}

impl WavReader {
    pub fn open(path: &str) -> io::Result<Self> {
        let mut file = File::open(path)?;
        let mut buf = Vec::new();
        file.read_to_end(&mut buf)?;

        if buf.len() < 44 {
            return Err(io::Error::new(io::ErrorKind::InvalidData, "File too small"));
        }
        if &buf[0..4] != b"RIFF" || &buf[8..12] != b"WAVE" {
            return Err(io::Error::new(io::ErrorKind::InvalidData, "Not a WAV file"));
        }

        // Find fmt chunk
        let mut pos = 12;
        let mut channels = 0u16;
        let mut sample_rate = 0u32;
        let mut bits_per_sample = 0u16;
        let mut data_offset = 0usize;
        let mut data_size = 0usize;

        while pos + 8 <= buf.len() {
            let chunk_id = &buf[pos..pos + 4];
            let chunk_size = u32::from_le_bytes([buf[pos + 4], buf[pos + 5], buf[pos + 6], buf[pos + 7]]) as usize;

            if chunk_id == b"fmt " {
                let fmt_tag = u16::from_le_bytes([buf[pos + 8], buf[pos + 9]]);
                if fmt_tag != 1 {
                    return Err(io::Error::new(io::ErrorKind::InvalidData,
                        format!("Unsupported format tag: 0x{:04X} (need PCM=1)", fmt_tag)));
                }
                channels = u16::from_le_bytes([buf[pos + 10], buf[pos + 11]]);
                sample_rate = u32::from_le_bytes([buf[pos + 12], buf[pos + 13], buf[pos + 14], buf[pos + 15]]);
                bits_per_sample = u16::from_le_bytes([buf[pos + 22], buf[pos + 23]]);
            } else if chunk_id == b"data" {
                data_offset = pos + 8;
                data_size = chunk_size;
            }

            pos += 8 + chunk_size;
            if chunk_size % 2 != 0 {
                pos += 1; // padding
            }
        }

        if data_offset == 0 || channels == 0 {
            return Err(io::Error::new(io::ErrorKind::InvalidData, "Missing fmt or data chunk"));
        }

        if bits_per_sample != 16 {
            return Err(io::Error::new(io::ErrorKind::InvalidData,
                format!("Unsupported bits per sample: {} (need 16)", bits_per_sample)));
        }

        let num_samples_total = data_size / 2; // 16-bit samples
        let num_samples_per_channel = num_samples_total / channels as usize;

        let mut samples = Vec::with_capacity(num_samples_total);
        let end = (data_offset + data_size).min(buf.len());
        let mut i = data_offset;
        while i + 1 < end {
            samples.push(i16::from_le_bytes([buf[i], buf[i + 1]]));
            i += 2;
        }

        Ok(WavReader {
            channels,
            sample_rate,
            bits_per_sample,
            num_samples: num_samples_per_channel as u32,
            data: samples,
        })
    }

    /// Get deinterleaved channel data as floats.
    /// Returns Vec<Vec<f32>> where [channel][sample].
    pub fn get_channels_f32(&self) -> Vec<Vec<f32>> {
        let ch = self.channels as usize;
        let mut channels = vec![Vec::with_capacity(self.num_samples as usize); ch];
        for (i, &sample) in self.data.iter().enumerate() {
            channels[i % ch].push(sample as f32);
        }
        channels
    }
}
