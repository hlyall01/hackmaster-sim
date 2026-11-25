use hackmaster_sim::core::rng::SimRng;
use rand::{RngCore, SeedableRng, rngs::StdRng};

#[cfg_attr(target_arch = "wasm32", wasm_bindgen::prelude::wasm_bindgen)]
pub fn compare_rng_streams() {
    for seed in [0, 9876, u32::MAX as u64, u64::MAX] {
        let mut actual = SimRng::from_seed(seed);
        let mut expected = StdRng::seed_from_u64(seed);
        for i in 0..4096 {
            assert_eq!(actual.next_u32(), expected.next_u32());
            if i % 5 == 0 {
                assert_eq!(actual.next_u64(), expected.next_u64());
            }
            if i % 17 == 0 {
                let mut a = [0; 65];
                let mut b = [0; 65];
                actual.fill_bytes(&mut a[..i % 66]);
                expected.fill_bytes(&mut b[..i % 66]);
                assert_eq!(a, b);
                assert_eq!(actual.clone().next_u64(), expected.clone().next_u64());
            }
        }
    }
}

#[test]
fn refill_matches_std_rng_across_word_and_block_boundaries() {
    compare_rng_streams();
}
