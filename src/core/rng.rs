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

pub fn derive_seed(base: u64, domain: &str, index: u64) -> u64 {
    let domain_hash = fnv1a64(domain.as_bytes());
    let mixed = base ^ domain_hash ^ index.wrapping_mul(0x9E3779B97F4A7C15);
    splitmix64(mixed)
}

fn fnv1a64(bytes: &[u8]) -> u64 {
    let mut hash = 0xcbf29ce484222325u64;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

fn splitmix64(mut state: u64) -> u64 {
    state = state.wrapping_add(0x9E3779B97F4A7C15);
    let mut z = state;
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D049BB133111EB);
    z ^ (z >> 31)
}
