pub type LinearI = u8;
pub type LinearW = i8;
pub type LinearB = i32;

#[derive(Debug, Clone)]
pub struct Linear<const INPUTS: usize, const OUTPUTS: usize> {
    pub weights: [[LinearW; INPUTS]; OUTPUTS],
    pub biases: [LinearB; OUTPUTS]
}

impl<const INPUTS: usize, const OUTPUTS: usize> Linear<INPUTS, OUTPUTS> {
    pub fn activate(&self, inputs: &[LinearI; INPUTS], outputs: &mut [LinearB; OUTPUTS]) {
        *outputs = self.biases;
        for (o, w) in outputs.iter_mut().zip(&self.weights) {
            *o += dot_product(inputs, w);
        }
    }
}

pub type BitLinearWB = i16;

#[derive(Debug, Clone)]
pub struct BitLinear<const INPUTS: usize, const OUTPUTS: usize> {
    pub weights: [[BitLinearWB; OUTPUTS]; INPUTS],
    pub biases: [BitLinearWB; OUTPUTS]
}

impl<const INPUTS: usize, const OUTPUTS: usize> BitLinear<INPUTS, OUTPUTS> {
    pub fn empty(&self, outputs: &mut [BitLinearWB; OUTPUTS]) {
        *outputs = self.biases;
    }

    pub fn add(&self, index: usize, outputs: &mut [BitLinearWB; OUTPUTS]) {
        vec_add(outputs, &self.weights[index]);
    }

    pub fn sub(&self, index: usize, outputs: &mut [BitLinearWB; OUTPUTS]) {
        vec_sub(outputs, &self.weights[index]);
    }
}

fn dot_product<const LEN: usize>(vec: &[LinearI; LEN], other: &[LinearW; LEN]) -> LinearB {
    #[cfg(all(target_feature = "avx2", not(debug_assertions)))] {
        use std::arch::x86_64::*;

        const VEC_SIZE: usize = std::mem::size_of::<__m256i>() / std::mem::size_of::<LinearI>();
        // lmao rip if this isn't true
        if LEN % VEC_SIZE == 0 {
            unsafe {
                let mut sum = _mm256_setzero_si256(); // i32x8
                for (l, r) in vec.chunks_exact(VEC_SIZE).zip(other.chunks_exact(VEC_SIZE)) {
                    let l = _mm256_loadu_si256(l.as_ptr() as *const __m256i);
                    let r = _mm256_loadu_si256(r.as_ptr() as *const __m256i);

                    // u8x32 * i8x32 -> i16x32 horizontal add -> i16x16
                    let partial = _mm256_maddubs_epi16(l, r);
                    // i16x16 * i16x16 -> i32x16 horizontal add -> i32x8
                    // We only want the horizontal add, so we no-op multiply with 1
                    let partial = _mm256_madd_epi16(partial, _mm256_set1_epi16(1));
                    // i32x8 + i32x8 -> i32x8
                    sum = _mm256_add_epi32(sum, partial);
                }

                // Sum i32x8 to i32.
                // i32x8 lower half -> i32x4
                let lower = _mm256_castsi256_si128(sum);
                // i32x8 upper half -> i32x4
                let upper = _mm256_extracti128_si256::<1>(sum);
                // i32x4 + i32x4 -> i32x4
                let sum = _mm_add_epi32(lower, upper);
                // i32x4 reversed -> i32x4
                let reversed = _mm_shuffle_epi32(sum, 0b_00_01_10_11);
                // i32x4 + i32x4 reversed -> i32x2 + ...
                let sum = _mm_add_epi32(sum, reversed);
                // i32x2 + ... element 0 -> i32
                let lower = _mm_cvtsi128_si32(sum);
                // i32x2 + ... element 1 -> i32
                let upper = _mm_extract_epi32::<1>(sum);
                return lower + upper;
            }
        }
    }

    // Fallback impl
    vec.iter().zip(other).map(|(&v, &o)| v as LinearB * o as LinearB).sum()
}

fn vec_add<const LEN: usize>(vec: &mut [BitLinearWB; LEN], other: &[BitLinearWB; LEN]) {
    for (v, o) in vec.iter_mut().zip(other) {
        *v += o;
    }
}

fn vec_sub<const LEN: usize>(vec: &mut [BitLinearWB; LEN], other: &[BitLinearWB; LEN]) {
    for (v, o) in vec.iter_mut().zip(other) {
        *v -= o;
    }
}
