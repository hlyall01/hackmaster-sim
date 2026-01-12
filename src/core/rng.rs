//! RNG abstractions for deterministic simulation and testing.

use rand::{RngCore, SeedableRng};

#[derive(Clone, Debug)]
pub struct SimRng {
    rng: rand::rngs::StdRng,
}

impl SimRng {
    pub fn from_seed(seed: u64) -> Self {
        Self {
            rng: rand::rngs::StdRng::seed_from_u64(seed),
        }
    }

    pub fn from_rng(rng: rand::rngs::StdRng) -> Self {
        Self { rng }
    }
}

impl Default for SimRng {
    fn default() -> Self {
        Self {
            rng: rand::rngs::StdRng::from_entropy(),
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
