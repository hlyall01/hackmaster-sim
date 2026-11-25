//! Four ChaCha12 blocks per refill, one independent block per SIMD lane.
//! BlockRng supplies the same buffering and mixed-width reads as StdRng.
use core::arch::wasm32::*;
use rand_core::{SeedableRng, block::BlockRngCore};

#[derive(Clone, Debug)]
pub(super) struct ChaCha12 {
    key: [u32; 8],
    counter: u64,
}

#[derive(Clone)]
pub(super) struct Words([u32; 64]);
impl Default for Words {
    fn default() -> Self {
        Self([0; 64])
    }
}
impl AsRef<[u32]> for Words {
    fn as_ref(&self) -> &[u32] {
        &self.0
    }
}
impl AsMut<[u32]> for Words {
    fn as_mut(&mut self) -> &mut [u32] {
        &mut self.0
    }
}

impl SeedableRng for ChaCha12 {
    type Seed = [u8; 32];
    fn from_seed(seed: Self::Seed) -> Self {
        Self {
            key: core::array::from_fn(|i| {
                u32::from_le_bytes(seed[4 * i..4 * i + 4].try_into().unwrap())
            }),
            counter: 0,
        }
    }
}

impl BlockRngCore for ChaCha12 {
    type Item = u32;
    type Results = Words;
    fn generate(&mut self, words: &mut Words) {
        // The web loader selects this module only after validating SIMD support.
        unsafe {
            refill(&self.key, self.counter, &mut words.0);
        }
        self.counter = self.counter.wrapping_add(4);
    }
}

#[target_feature(enable = "simd128")]
unsafe fn refill(key: &[u32; 8], counter: u64, output: &mut [u32; 64]) {
    let counters = [
        counter,
        counter.wrapping_add(1),
        counter.wrapping_add(2),
        counter.wrapping_add(3),
    ];
    let lo = counters.map(|c| c as u32);
    let hi = counters.map(|c| (c >> 32) as u32);
    let constants = [0x61707865, 0x3320646e, 0x79622d32, 0x6b206574];
    let mut initial = [u32x4_splat(0); 16];
    for (v, word) in initial.iter_mut().zip(constants.iter().chain(key)) {
        *v = u32x4_splat(*word);
    }
    initial[12] = u32x4(lo[0], lo[1], lo[2], lo[3]);
    initial[13] = u32x4(hi[0], hi[1], hi[2], hi[3]);
    let mut x = initial;
    macro_rules! rot {
        ($v:expr, $bits:literal) => {
            v128_or(u32x4_shl($v, $bits), u32x4_shr($v, 32 - $bits))
        };
    }
    macro_rules! quarter {
        ($a:literal,$b:literal,$c:literal,$d:literal) => {{
            x[$a] = u32x4_add(x[$a], x[$b]);
            x[$d] = rot!(v128_xor(x[$d], x[$a]), 16);
            x[$c] = u32x4_add(x[$c], x[$d]);
            x[$b] = rot!(v128_xor(x[$b], x[$c]), 12);
            x[$a] = u32x4_add(x[$a], x[$b]);
            x[$d] = rot!(v128_xor(x[$d], x[$a]), 8);
            x[$c] = u32x4_add(x[$c], x[$d]);
            x[$b] = rot!(v128_xor(x[$b], x[$c]), 7);
        }};
    }
    for _ in 0..6 {
        quarter!(0, 4, 8, 12);
        quarter!(1, 5, 9, 13);
        quarter!(2, 6, 10, 14);
        quarter!(3, 7, 11, 15);
        quarter!(0, 5, 10, 15);
        quarter!(1, 6, 11, 12);
        quarter!(2, 7, 8, 13);
        quarter!(3, 4, 9, 14);
    }
    for i in 0..16 {
        let word = u32x4_add(x[i], initial[i]);
        output[i] = u32x4_extract_lane::<0>(word);
        output[16 + i] = u32x4_extract_lane::<1>(word);
        output[32 + i] = u32x4_extract_lane::<2>(word);
        output[48 + i] = u32x4_extract_lane::<3>(word);
    }
}
