use std::fs::File;
use std::io::{self, Write, BufWriter};

pub struct Atrac3WavWriter {
    writer: BufWriter<File>,
    frame_size: u16,
}

impl Atrac3WavWriter {
    pub fn create(
        path: &str,
        channels: u16,
        sample_rate: u32,
        frame_size: u16,
        total_samples: u32,
        total_frames: u32,
    ) -> io::Result<Self> {
        let file = File::create(path)?;
        let mut writer = BufWriter::new(file);

        let samples_per_frame: u32 = 1024;
        let data_size = total_frames * frame_size as u32;
        // byte_rate must match reference: ceiling division
        let byte_rate = ((frame_size as u64 * sample_rate as u64) + samples_per_frame as u64 - 1) / samples_per_frame as u64;
        let byte_rate = byte_rate as u32;

        // RIFF size = 4 (WAVE) + fmt chunk (8+32) + fact chunk (8+8) + data chunk header (8) + data
        let riff_size = 4u32 + 40 + 16 + 8 + data_size;

        // === RIFF header ===
        writer.write_all(b"RIFF")?;
        writer.write_all(&riff_size.to_le_bytes())?;
        writer.write_all(b"WAVE")?;

        // === fmt chunk (32 bytes of data) ===
        writer.write_all(b"fmt ")?;
        writer.write_all(&32u32.to_le_bytes())?;
        writer.write_all(&0x0270u16.to_le_bytes())?;  // ATRAC3
        writer.write_all(&channels.to_le_bytes())?;
        writer.write_all(&sample_rate.to_le_bytes())?;
        writer.write_all(&byte_rate.to_le_bytes())?;
        writer.write_all(&frame_size.to_le_bytes())?;
        writer.write_all(&0u16.to_le_bytes())?;        // bits_per_sample = 0
        writer.write_all(&14u16.to_le_bytes())?;       // cbSize = 14

        // Extra data (14 bytes) - exact match with reference
        writer.write_all(&[0x01, 0x00, 0x00, 0x10, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00])?;

        // === fact chunk ===
        writer.write_all(b"fact")?;
        writer.write_all(&8u32.to_le_bytes())?;
        writer.write_all(&total_samples.to_le_bytes())?;
        writer.write_all(&samples_per_frame.to_le_bytes())?;

        // === data chunk ===
        writer.write_all(b"data")?;
        writer.write_all(&data_size.to_le_bytes())?;

        Ok(Atrac3WavWriter { writer, frame_size })
    }

    pub fn write_frame(&mut self, frame_data: &[u8]) -> io::Result<()> {
        assert_eq!(frame_data.len(), self.frame_size as usize);
        self.writer.write_all(frame_data)?;
        Ok(())
    }

    pub fn finish(mut self) -> io::Result<()> {
        self.writer.flush()
    }
}
