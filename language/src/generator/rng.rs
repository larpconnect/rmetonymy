//! Random number generator utilities for word generators.

pub use rand::Rng;
pub use rand::RngExt;
pub use rand::SeedableRng;
pub use rand::rngs::StdRng;

/// Returns a thread-local random number generator.
#[must_use]
pub fn thread_rng() -> rand::rngs::ThreadRng {
    rand::rng()
}
