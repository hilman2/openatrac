/// Big-endian bit packer matching the original FUN_004376e0.
///
/// Writes `num_bits` bits of `value` at bit position `bit_pos` into `buf`.
/// Bit position 0 is the MSB of byte 0.
pub fn write_bits(buf: &mut [u8], bit_pos: u32, value: u32, num_bits: u8) {
    if num_bits == 0 {
        return;
    }
    let byte_pos = (bit_pos >> 3) as usize;
    let bit_offset = (bit_pos & 7) as u32;

    // Match the original exactly:
    // shifted = value << ((16 - bit_offset) - num_bits)
    // ptr[1] = shifted & 0xFF
    // ptr[0] |= (shifted >> 8) & 0xFF
    let shift = 16u32.wrapping_sub(bit_offset).wrapping_sub(num_bits as u32) & 0x1F;
    let shifted = value << shift;

    if byte_pos + 1 < buf.len() {
        buf[byte_pos] |= ((shifted >> 8) & 0xFF) as u8;
        buf[byte_pos + 1] |= (shifted & 0xFF) as u8;
    } else if byte_pos < buf.len() {
        buf[byte_pos] |= ((shifted >> 8) & 0xFF) as u8;
    }
}

/// Read `num_bits` bits from `buf` at `bit_pos`.
pub fn read_bits(buf: &[u8], bit_pos: u32, num_bits: u8) -> u32 {
    if num_bits == 0 {
        return 0;
    }
    let byte_pos = (bit_pos >> 3) as usize;
    let bit_offset = (bit_pos & 7) as u32;

    let mut val: u32 = 0;
    if byte_pos < buf.len() {
        val = (buf[byte_pos] as u32) << 8;
    }
    if byte_pos + 1 < buf.len() {
        val |= buf[byte_pos + 1] as u32;
    }
    if byte_pos + 2 < buf.len() {
        val |= (buf[byte_pos + 2] as u32) >> 8; // not needed for <= 8 bits
    }

    let shift = 16u32.wrapping_sub(bit_offset).wrapping_sub(num_bits as u32) & 0x1F;
    let mask = (1u32 << num_bits) - 1;
    (val >> shift) & mask
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_write_bits_basic() {
        let mut buf = [0u8; 4];
        write_bits(&mut buf, 0, 0b101, 3);
        // Should write 101 at bits 0-2
        assert_eq!(buf[0], 0b10100000);
    }

    #[test]
    fn test_write_bits_offset() {
        let mut buf = [0u8; 4];
        write_bits(&mut buf, 3, 0b1101, 4);
        // Bits 3-6 = 1101 → byte 0 = 000_1101_0 = 0x1A? Let's check
        // bit_pos=3, num_bits=4: shift = 16-3-4 = 9
        // value << 9 = 0b1101 << 9 = 0b1_1010_0000_0000 = 0x1A00
        // buf[0] |= 0x1A >> 0 = but (0x1A00 >> 8) = 0x1A
        // buf[1] |= 0x00
        assert_eq!(buf[0], 0b00011010);
    }

    #[test]
    fn test_write_read_roundtrip() {
        let mut buf = [0u8; 8];
        write_bits(&mut buf, 0, 0xA2, 8);
        write_bits(&mut buf, 8, 5, 3);
        write_bits(&mut buf, 11, 7, 5);

        assert_eq!(buf[0], 0xA2);
        assert_eq!(read_bits(&buf, 0, 8), 0xA2);
        assert_eq!(read_bits(&buf, 8, 3), 5);
        assert_eq!(read_bits(&buf, 11, 5), 7);
    }
}
