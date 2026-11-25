//! RNG abstractions for deterministic simulation and testing.

use rand::{RngCore, SeedableRng};

#[cfg(all(target_arch = "wasm32", feature = "wasm-simd-rng"))]
mod wasm_chacha;
#[cfg(all(target_arch = "wasm32", feature = "wasm-simd-rng"))]
type Engine = rand_core::block::BlockRng<wasm_chacha::ChaCha12>;
#[cfg(not(all(target_arch = "wasm32", feature = "wasm-simd-rng")))]
type Engine = rand::rngs::StdRng;

#[derive(Clone, Debug)]
pub struct SimRng {
    rng: Engine,
}

impl SimRng {
    pub fn from_seed(seed: u64) -> Self {
        Self {
            rng: Engine::seed_from_u64(seed),
        }
    }
}

impl Default for SimRng {
    fn default() -> Self {
        Self {
            rng: Engine::from_entropy(),
        }
    }
}

impl RngCore for SimRng {
    fn next_u32(&mut self) -> u32 {
        self.rng.next_u32()
    }

    fn next_u64(&mut self) -> u64 {
        self.rng.next_u64()
    }

    fn fill_bytes(&mut self, dest: &mut [u8]) {
        self.rng.fill_bytes(dest);
    }

    fn try_fill_bytes(&mut self, dest: &mut [u8]) -> Result<(), rand::Error> {
        self.rng.try_fill_bytes(dest)
    }
}
