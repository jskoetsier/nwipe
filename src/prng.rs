/*
 *  prng.rs: Pseudo-random number generation for nwipe.
 *
 *  Copyright Darik Horn <dajhorn-dban@vanadac.com>.
 *  Modifications to original dwipe Copyright Andy Beverley <andy@andybev.com>
 *  Rust conversion: 2023
 *
 *  This program is free software; you can redistribute it and/or modify it under
 *  the terms of the GNU General Public License as published by the Free Software
 *  Foundation, version 2.
 */

use std::io;
use rand::{SeedableRng, RngCore};
use rand::rngs::StdRng;
use rand_isaac::Isaac64Rng;
use rand_mt::Mt64;

/// A trait for PRNGs used by nwipe.
pub trait NwipePrng {
    /// Fill a buffer with random bytes.
    fn fill_bytes(&mut self, dest: &mut [u8]);
}

/// Initialize a PRNG based on the given name.
pub fn init_prng(name: &str) -> Result<Box<dyn NwipePrng>, io::Error> {
    match name {
        "isaac" => Ok(Box::new(IsaacPrng::new())),
        "mt19937" => Ok(Box::new(Mt19937Prng::new())),
        "twister" => Ok(Box::new(Mt19937Prng::new())), // Alias for mt19937
        "random" => Ok(Box::new(StdPrng::new())),
        _ => Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("Unknown PRNG: {}", name),
        )),
    }
}

/// ISAAC PRNG implementation.
pub struct IsaacPrng {
    rng: Isaac64Rng,
}

impl IsaacPrng {
    /// Create a new ISAAC PRNG.
    pub fn new() -> Self {
        // Create a seed from the system entropy source
        let mut seed = [0u8; 32];
        getrandom::getrandom(&mut seed).expect("Failed to get random seed");

        Self {
            rng: Isaac64Rng::from_seed(seed),
        }
    }
}

impl NwipePrng for IsaacPrng {
    fn fill_bytes(&mut self, dest: &mut [u8]) {
        self.rng.fill_bytes(dest);
    }
}

/// MT19937 PRNG implementation.
pub struct Mt19937Prng {
    rng: Mt64,
}

impl Mt19937Prng {
    /// Create a new MT19937 PRNG.
    pub fn new() -> Self {
        // Create a seed from the system entropy source
        let mut seed_bytes = [0u8; 8];
        getrandom::getrandom(&mut seed_bytes).expect("Failed to get random seed");
        let seed = u64::from_le_bytes(seed_bytes);

        Self {
            rng: Mt64::new(seed),
        }
    }
}

impl NwipePrng for Mt19937Prng {
    fn fill_bytes(&mut self, dest: &mut [u8]) {
        // Fill the buffer with random bytes
        for chunk in dest.chunks_mut(8) {
            let random_value = self.rng.next_u64();
            let bytes = random_value.to_le_bytes();

            // Copy as many bytes as needed (handles the last chunk which might be smaller than 8 bytes)
            for (i, byte) in chunk.iter_mut().enumerate() {
                if i < bytes.len() {
                    *byte = bytes[i];
                }
            }
        }
    }
}

/// Standard library PRNG implementation.
pub struct StdPrng {
    rng: StdRng,
}

impl StdPrng {
    /// Create a new standard library PRNG.
    pub fn new() -> Self {
        Self {
            rng: StdRng::from_entropy(),
        }
    }
}

impl NwipePrng for StdPrng {
    fn fill_bytes(&mut self, dest: &mut [u8]) {
        self.rng.fill_bytes(dest);
    }
}
