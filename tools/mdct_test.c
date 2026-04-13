// Direct translation of FUN_0043a220 from decompiled psp_at3tool.exe
// This is a standalone test program that applies the ATRAC3 MDCT to input samples
// and outputs the spectral coefficients.

#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <math.h>

// DAT_0048e5e4 = sqrt(2)
static const float SQRT2 = 1.4142135381698608f;

// Placeholder for the context structure
// param_1 is a pointer to the encoder context
// param_1+8 points to the output buffer
// param_1+0x1440 points to the overlap buffer (512 floats = 4 blocks of 128)

typedef struct {
    float output[1024 + 256]; // spectral output, accessed at offset 8 + 0x228 = 0x230
    float overlap[512];       // overlap at offset 0x1440 (4 × 128 floats)
    // ... other fields we don't need
} EncoderContext;

// Read float from EXE at given virtual address
static float exe_float(const unsigned char* exe, unsigned int va) {
    unsigned int offset = va - 0x400000;
    float val;
    memcpy(&val, exe + offset, 4);
    return val;
}

int main(int argc, char* argv[]) {
    if (argc < 3) {
        fprintf(stderr, "Usage: mdct_test <psp_at3tool.exe> <input_pcm_f32>\n");
        fprintf(stderr, "  Reads 1024 float32 samples from stdin\n");
        fprintf(stderr, "  Outputs 1024 float32 spectral coefficients to stdout\n");
        return 1;
    }

    // Load the EXE for table access
    FILE* fexe = fopen(argv[1], "rb");
    if (!fexe) { fprintf(stderr, "Cannot open %s\n", argv[1]); return 1; }
    fseek(fexe, 0, SEEK_END);
    long exe_size = ftell(fexe);
    fseek(fexe, 0, SEEK_SET);
    unsigned char* exe = malloc(exe_size);
    fread(exe, 1, exe_size, fexe);
    fclose(fexe);

    // Read input samples
    FILE* fin = fopen(argv[2], "rb");
    if (!fin) { fprintf(stderr, "Cannot open %s\n", argv[2]); return 1; }
    float samples[1024];
    fread(samples, sizeof(float), 1024, fin);
    fclose(fin);

    // The actual function FUN_0043a220 takes:
    // param_1: encoder context pointer
    // param_2: number of scale factors (we'll set to 0)
    // param_3: scale factor indices (unused with param_2=0)
    // param_4: pointer to input PCM (1024 floats, accessed with offsets -0x200, -0x100, 0, +0x100 from center)

    // Stack-local variables (matching Ghidra names)
    float local_1824[520]; // [0..519], indices 1-4 are scale factors, 5-516 are overlap state
    float local_1010[256];
    float afStack_c10[252];
    float local_820[512];
    float local_20[8];

    // These are contiguous in memory:
    // &local_1010[0] is followed by &afStack_c10[0] is followed by &local_820[0] is followed by &local_20[0]
    // Total: 256+252+512+8 = 1028 floats

    // Initialize
    memset(local_1824, 0, sizeof(local_1824));
    memset(local_1010, 0, sizeof(local_1010));
    memset(afStack_c10, 0, sizeof(afStack_c10));
    memset(local_820, 0, sizeof(local_820));
    memset(local_20, 0, sizeof(local_20));

    // Scale factors (local_1824[1..4]) initialized to 1.0
    local_1824[1] = local_1824[2] = local_1824[3] = local_1824[4] = 1.0f;

    // param_2 = 0 (no pre-computed scale factors)
    int param_2 = 0;

    // Output buffer pointer
    float output_buf[1280]; // enough for output at offset 0x228
    memset(output_buf, 0, sizeof(output_buf));
    int iVar2 = (int)output_buf + 0; // simplified: output starts at output_buf[0]
    // Actually: iVar2 = *(int*)(param_1 + 8) + 0x228
    // We'll store output starting at output_buf[0x228/4] = output_buf[138]

    // ================================================================
    // Step 1: Input rearrangement (lines 44225-44237)
    // ================================================================
    float* pfVar12 = local_1010;
    float* pfVar5 = local_20 + 3; // Points near end of contiguous area
    float* pfVar4 = samples + 512; // param_4 + 0x800 bytes = param_4 + 512 floats

    int iVar3 = 0x100; // 256 iterations
    do {
        *pfVar12      = pfVar4[-0x200];  // = samples[i - 512 + 512] = samples[i]
        pfVar5[-2]    = pfVar4[-0x100];  // = samples[i + 256]
        pfVar12[2]    = *pfVar4;         // = samples[i + 512]
        *pfVar5       = pfVar4[0x100];   // = samples[i + 768]
        pfVar12 += 4;
        pfVar5 -= 4;
        iVar3--;
        pfVar4++;
    } while (iVar3 != 0);

    fprintf(stderr, "Rearrangement done. local_1010[0]=%f, local_820[0]=%f\n",
            local_1010[0], local_820[0]);

    // ================================================================
    // Step 2: Load overlap (lines 44238-44249) - Skip for first frame
    // ================================================================
    // (overlap is all zeros)

    // ================================================================
    // Step 3: First butterfly (lines 44250-44301)
    // ================================================================
    pfVar5 = local_820;
    iVar3 = 0x40; // 64 iterations
    pfVar12 = local_820;

    do {
        pfVar12 += 4;
        float fVar25 = *pfVar12;
        float fVar26 = pfVar12[1];
        float fVar27 = pfVar12[2];
        float fVar29 = pfVar12[3];
        float fVar21 = pfVar5[0x200];
        float fVar22 = pfVar5[0x201];
        float fVar23 = pfVar5[0x202];
        float fVar24 = pfVar5[0x203];
        float fVar13 = pfVar12[-0x200] - fVar21;
        float fVar15 = pfVar12[-0x1ff] - fVar22;
        float fVar17 = pfVar12[-0x1fe] - fVar23;
        float fVar19 = pfVar12[-0x1fd] - fVar24;
        *pfVar5 = *pfVar5 - fVar25;
        pfVar5[1] = pfVar5[1] - fVar26;
        pfVar5[2] = pfVar5[2] - fVar27;
        pfVar5[3] = pfVar5[3] - fVar29;
        fVar25 *= SQRT2;
        fVar26 *= SQRT2;
        fVar27 *= SQRT2;
        fVar29 *= SQRT2;
        pfVar12[-0x200] = fVar13 + fVar25;
        pfVar12[-0x1ff] = fVar15 + fVar26;
        pfVar12[-0x1fe] = fVar17 + fVar27;
        pfVar12[-0x1fd] = fVar19 + fVar29;
        float fVar14 = *pfVar5;
        float fVar16 = pfVar5[1];
        float fVar18 = pfVar5[2];
        float fVar20 = pfVar5[3];
        fVar21 *= SQRT2;
        fVar22 *= SQRT2;
        fVar23 *= SQRT2;
        fVar24 *= SQRT2;
        *pfVar12 = fVar13 - fVar25;
        pfVar12[1] = fVar15 - fVar26;
        pfVar12[2] = fVar17 - fVar27;
        pfVar12[3] = fVar19 - fVar29;
        pfVar5[0x200] = fVar14 - fVar21;
        pfVar5[0x201] = fVar16 - fVar22;
        pfVar5[0x202] = fVar18 - fVar23;
        pfVar5[0x203] = fVar20 - fVar24;
        *pfVar5 = fVar14 + fVar21;
        pfVar5[1] = fVar16 + fVar22;
        pfVar5[2] = fVar18 + fVar23;
        pfVar5[3] = fVar20 + fVar24;
        pfVar5 -= 4;
        iVar3--;
    } while (iVar3 != 0);

    fprintf(stderr, "Butterfly done. local_1010[0]=%f\n", local_1010[0]);

    // ================================================================
    // Step 4: Iterative FFT (lines 44302-44384)
    // ================================================================
    unsigned int local_1834 = 0x40;
    do {
        pfVar5 = local_1010;
        unsigned int uVar10 = local_1834 / 2;
        if (uVar10 < 0x80) {
            // Twiddle table: &DAT_0048e6e4 + uVar10 * -4
            // We read directly from the EXE
            float* pfVar12_tw = NULL; // We'll compute twiddle inline
            unsigned int tw_base_offset = 0x48e6e4;

            do {
                if (local_1834 != 0) {
                    int iVar11 = local_1834 << 5;
                    iVar3 = (local_1834 * 2 - 1) / 4 + 1;
                    pfVar4 = pfVar5;
                    do {
                        float* pfVar6 = (float*)((char*)pfVar4 + iVar11 - 0x10);
                        pfVar4[0] -= pfVar6[0];
                        pfVar4[1] -= pfVar6[1];
                        pfVar4[2] -= pfVar6[2];
                        pfVar4[3] -= pfVar6[3];
                        pfVar6 = (float*)((char*)pfVar4 + iVar11 - 0x20);
                        pfVar4[4] -= pfVar6[0];
                        pfVar4[5] -= pfVar6[1];
                        pfVar4[6] -= pfVar6[2];
                        pfVar4[7] -= pfVar6[3];
                        pfVar4 += 8;
                        iVar11 -= 0x40;
                        iVar3--;
                    } while (iVar3 != 0);

                    if (pfVar5 < pfVar4) {
                        iVar3 = local_1834 + 1;
                        float* pfVar6_it = pfVar5;
                        float tw = exe_float(exe, tw_base_offset - uVar10 * 4);
                        do {
                            float* hi1 = pfVar6_it + iVar3 * 4 - 4;
                            float* hi2 = pfVar6_it + iVar3 * 4;
                            float* next = pfVar6_it + 8;
                            float* out1 = next + iVar3 * 4 - 12;
                            float* out2 = next + iVar3 * 4 - 8;

                            out1[0] = pfVar6_it[0] - hi1[0] * tw;
                            out1[1] = pfVar6_it[1] - hi1[1] * tw;
                            out1[2] = pfVar6_it[2] - hi1[2] * tw;
                            out1[3] = pfVar6_it[3] - hi1[3] * tw;
                            out2[0] = pfVar6_it[4] - hi2[0] * tw;
                            out2[1] = pfVar6_it[5] - hi2[1] * tw;
                            out2[2] = pfVar6_it[6] - hi2[2] * tw;
                            out2[3] = pfVar6_it[7] - hi2[3] * tw;
                            pfVar6_it[0] += hi1[0] * tw;
                            pfVar6_it[1] += hi1[1] * tw;
                            pfVar6_it[2] += hi1[2] * tw;
                            pfVar6_it[3] += hi1[3] * tw;
                            pfVar6_it[4] += hi2[0] * tw;
                            pfVar6_it[5] += hi2[1] * tw;
                            pfVar6_it[6] += hi2[2] * tw;
                            pfVar6_it[7] += hi2[3] * tw;
                            pfVar6_it = next;
                        } while (next < pfVar4);
                    }
                }
                pfVar5 += local_1834 * 4;
                uVar10 += local_1834;
            } while ((int)uVar10 < 0x80);
        }
        local_1834 >>= 1;
    } while (local_1834 > 1);

    fprintf(stderr, "FFT done. local_1010[0]=%f\n", local_1010[0]);

    // ================================================================
    // Step 5: Post-processing (lines 44385-44428)
    // ================================================================
    // This step reads from the FFT output and writes spectral coefficients
    // using twiddle rotation indexed by POST_INDEX (DAT_004bec04)

    // For now, just output the FFT result as-is for verification
    // The spectral coefficients are in local_1010 (and the contiguous buffer)

    // Output 256 floats from local_1010 (the FFT result)
    FILE* fout = fopen("mdct_output.bin", "wb");
    fwrite(local_1010, sizeof(float), 256, fout);
    fclose(fout);

    fprintf(stderr, "Output written to mdct_output.bin (256 floats)\n");
    fprintf(stderr, "First 4 values: %f %f %f %f\n",
            local_1010[0], local_1010[1], local_1010[2], local_1010[3]);

    free(exe);
    return 0;
}
