// Auto-generated VLC encode tables (reverse of decoder lookup)
// Each entry: (code_bits, code_length)

/// VLC encode table for word_len=1: 5 symbols
pub const VLC_ENC_WL1: [(u32, u8); 5] = [
    (0x0, 1), // sym 0
    (0x4, 3), // sym 1
    (0x6, 3), // sym 2
    (0x7, 3), // sym 3
    (0x5, 3), // sym 4
];

/// VLC encode table for word_len=2: 7 symbols
pub const VLC_ENC_WL2: [(u32, u8); 7] = [
    (0xE, 4), // sym 0
    (0x4, 3), // sym 1
    (0xC, 4), // sym 2
    (0x0, 1), // sym 3
    (0xF, 4), // sym 4
    (0xD, 4), // sym 5
    (0x5, 3), // sym 6
];

/// VLC encode table for word_len=3: 9 symbols
pub const VLC_ENC_WL3: [(u32, u8); 9] = [
    (0x1E, 5), // sym 0
    (0x4, 3), // sym 1
    (0xC, 4), // sym 2
    (0x1C, 5), // sym 3
    (0x0, 1), // sym 4
    (0x1F, 5), // sym 5
    (0x1D, 5), // sym 6
    (0xD, 4), // sym 7
    (0x5, 3), // sym 8
];

/// VLC encode table for word_len=4: 15 symbols
pub const VLC_ENC_WL4: [(u32, u8); 15] = [
    (0x0, 2), // sym 0
    (0x2, 3), // sym 1
    (0x8, 4), // sym 2
    (0xA, 4), // sym 3
    (0x1C, 5), // sym 4
    (0x3C, 6), // sym 5
    (0x3E, 6), // sym 6
    (0xC, 4), // sym 7
    (0xD, 4), // sym 8
    (0x3F, 6), // sym 9
    (0x3D, 6), // sym 10
    (0x1D, 5), // sym 11
    (0xB, 4), // sym 12
    (0x9, 4), // sym 13
    (0x3, 3), // sym 14
];

/// VLC encode table for word_len=5: 31 symbols
pub const VLC_ENC_WL5: [(u32, u8); 31] = [
    (0x0, 3), // sym 0
    (0x2, 4), // sym 1
    (0x4, 4), // sym 2
    (0x6, 4), // sym 3
    (0x14, 5), // sym 4
    (0x16, 5), // sym 5
    (0x18, 5), // sym 6
    (0x34, 6), // sym 7
    (0x36, 6), // sym 8
    (0x38, 6), // sym 9
    (0x3A, 6), // sym 10
    (0x78, 7), // sym 11
    (0x7A, 7), // sym 12
    (0x7C, 7), // sym 13
    (0x7E, 7), // sym 14
    (0x8, 4), // sym 15
    (0x9, 4), // sym 16
    (0x7F, 7), // sym 17
    (0x7D, 7), // sym 18
    (0x7B, 7), // sym 19
    (0x79, 7), // sym 20
    (0x3B, 6), // sym 21
    (0x39, 6), // sym 22
    (0x37, 6), // sym 23
    (0x35, 6), // sym 24
    (0x19, 5), // sym 25
    (0x17, 5), // sym 26
    (0x15, 5), // sym 27
    (0x7, 4), // sym 28
    (0x5, 4), // sym 29
    (0x3, 4), // sym 30
];

/// VLC encode table for word_len=6: 63 symbols
pub const VLC_ENC_WL6: [(u32, u8); 63] = [
    (0x10, 5), // sym 0
    (0x8, 5), // sym 1
    (0xA, 5), // sym 2
    (0xC, 5), // sym 3
    (0xE, 5), // sym 4
    (0x0, 3), // sym 5
    (0x24, 6), // sym 6
    (0x26, 6), // sym 7
    (0x28, 6), // sym 8
    (0x2A, 6), // sym 9
    (0x2C, 6), // sym 10
    (0x2E, 6), // sym 11
    (0x30, 6), // sym 12
    (0x32, 6), // sym 13
    (0x68, 7), // sym 14
    (0x6A, 7), // sym 15
    (0x6C, 7), // sym 16
    (0x6E, 7), // sym 17
    (0x70, 7), // sym 18
    (0x72, 7), // sym 19
    (0x74, 7), // sym 20
    (0xEC, 8), // sym 21
    (0xEE, 8), // sym 22
    (0xF0, 8), // sym 23
    (0xF2, 8), // sym 24
    (0xF4, 8), // sym 25
    (0xF6, 8), // sym 26
    (0xF8, 8), // sym 27
    (0xFA, 8), // sym 28
    (0xFC, 8), // sym 29
    (0xFE, 8), // sym 30
    (0x2, 4), // sym 31
    (0x3, 4), // sym 32
    (0xFF, 8), // sym 33
    (0xFD, 8), // sym 34
    (0xFB, 8), // sym 35
    (0xF9, 8), // sym 36
    (0xF7, 8), // sym 37
    (0xF5, 8), // sym 38
    (0xF3, 8), // sym 39
    (0xF1, 8), // sym 40
    (0xEF, 8), // sym 41
    (0xED, 8), // sym 42
    (0x75, 7), // sym 43
    (0x73, 7), // sym 44
    (0x71, 7), // sym 45
    (0x6F, 7), // sym 46
    (0x6D, 7), // sym 47
    (0x6B, 7), // sym 48
    (0x69, 7), // sym 49
    (0x33, 6), // sym 50
    (0x31, 6), // sym 51
    (0x2F, 6), // sym 52
    (0x2D, 6), // sym 53
    (0x2B, 6), // sym 54
    (0x29, 6), // sym 55
    (0x27, 6), // sym 56
    (0x25, 6), // sym 57
    (0x11, 5), // sym 58
    (0xF, 5), // sym 59
    (0xD, 5), // sym 60
    (0xB, 5), // sym 61
    (0x9, 5), // sym 62
];

/// CLC encode table for word_len=1: 5 symbols
pub const CLC_ENC_WL1: [(u32, u8); 5] = [
    (0x0, 3), // sym 0
    (0x1, 3), // sym 1
    (0x2, 3), // sym 2
    (0x6, 3), // sym 3
    (0x7, 3), // sym 4
];

/// CLC encode table for word_len=2: 7 symbols
pub const CLC_ENC_WL2: [(u32, u8); 7] = [
    (0x3, 3), // sym 0
    (0x1, 3), // sym 1
    (0x2, 3), // sym 2
    (0x0, 3), // sym 3
    (0x5, 3), // sym 4
    (0x6, 3), // sym 5
    (0x7, 3), // sym 6
];

/// CLC encode table for word_len=3: 9 symbols
pub const CLC_ENC_WL3: [(u32, u8); 9] = [
    (0x4, 4), // sym 0
    (0x1, 4), // sym 1
    (0x2, 4), // sym 2
    (0x3, 4), // sym 3
    (0x0, 4), // sym 4
    (0xC, 4), // sym 5
    (0xD, 4), // sym 6
    (0xE, 4), // sym 7
    (0xF, 4), // sym 8
];

/// CLC encode table for word_len=4: 15 symbols
pub const CLC_ENC_WL4: [(u32, u8); 15] = [
    (0x0, 4), // sym 0
    (0x1, 4), // sym 1
    (0x2, 4), // sym 2
    (0x3, 4), // sym 3
    (0x4, 4), // sym 4
    (0x5, 4), // sym 5
    (0x6, 4), // sym 6
    (0x7, 4), // sym 7
    (0x9, 4), // sym 8
    (0xA, 4), // sym 9
    (0xB, 4), // sym 10
    (0xC, 4), // sym 11
    (0xD, 4), // sym 12
    (0xE, 4), // sym 13
    (0xF, 4), // sym 14
];

/// CLC encode table for word_len=5: 31 symbols
pub const CLC_ENC_WL5: [(u32, u8); 31] = [
    (0x0, 5), // sym 0
    (0x1, 5), // sym 1
    (0x2, 5), // sym 2
    (0x3, 5), // sym 3
    (0x4, 5), // sym 4
    (0x5, 5), // sym 5
    (0x6, 5), // sym 6
    (0x7, 5), // sym 7
    (0x8, 5), // sym 8
    (0x9, 5), // sym 9
    (0xA, 5), // sym 10
    (0xB, 5), // sym 11
    (0xC, 5), // sym 12
    (0xD, 5), // sym 13
    (0xE, 5), // sym 14
    (0xF, 5), // sym 15
    (0x11, 5), // sym 16
    (0x12, 5), // sym 17
    (0x13, 5), // sym 18
    (0x14, 5), // sym 19
    (0x15, 5), // sym 20
    (0x16, 5), // sym 21
    (0x17, 5), // sym 22
    (0x18, 5), // sym 23
    (0x19, 5), // sym 24
    (0x1A, 5), // sym 25
    (0x1B, 5), // sym 26
    (0x1C, 5), // sym 27
    (0x1D, 5), // sym 28
    (0x1E, 5), // sym 29
    (0x1F, 5), // sym 30
];

/// CLC encode table for word_len=6: 63 symbols
pub const CLC_ENC_WL6: [(u32, u8); 63] = [
    (0x5, 6), // sym 0
    (0x1, 6), // sym 1
    (0x2, 6), // sym 2
    (0x3, 6), // sym 3
    (0x4, 6), // sym 4
    (0x0, 6), // sym 5
    (0x6, 6), // sym 6
    (0x7, 6), // sym 7
    (0x8, 6), // sym 8
    (0x9, 6), // sym 9
    (0xA, 6), // sym 10
    (0xB, 6), // sym 11
    (0xC, 6), // sym 12
    (0xD, 6), // sym 13
    (0xE, 6), // sym 14
    (0xF, 6), // sym 15
    (0x10, 6), // sym 16
    (0x11, 6), // sym 17
    (0x12, 6), // sym 18
    (0x13, 6), // sym 19
    (0x14, 6), // sym 20
    (0x15, 6), // sym 21
    (0x16, 6), // sym 22
    (0x17, 6), // sym 23
    (0x18, 6), // sym 24
    (0x19, 6), // sym 25
    (0x1A, 6), // sym 26
    (0x1B, 6), // sym 27
    (0x1C, 6), // sym 28
    (0x1D, 6), // sym 29
    (0x1E, 6), // sym 30
    (0x1F, 6), // sym 31
    (0x21, 6), // sym 32
    (0x22, 6), // sym 33
    (0x23, 6), // sym 34
    (0x24, 6), // sym 35
    (0x25, 6), // sym 36
    (0x26, 6), // sym 37
    (0x27, 6), // sym 38
    (0x28, 6), // sym 39
    (0x29, 6), // sym 40
    (0x2A, 6), // sym 41
    (0x2B, 6), // sym 42
    (0x2C, 6), // sym 43
    (0x2D, 6), // sym 44
    (0x2E, 6), // sym 45
    (0x2F, 6), // sym 46
    (0x30, 6), // sym 47
    (0x31, 6), // sym 48
    (0x32, 6), // sym 49
    (0x33, 6), // sym 50
    (0x34, 6), // sym 51
    (0x35, 6), // sym 52
    (0x36, 6), // sym 53
    (0x37, 6), // sym 54
    (0x38, 6), // sym 55
    (0x39, 6), // sym 56
    (0x3A, 6), // sym 57
    (0x3B, 6), // sym 58
    (0x3C, 6), // sym 59
    (0x3D, 6), // sym 60
    (0x3E, 6), // sym 61
    (0x3F, 6), // sym 62
];

