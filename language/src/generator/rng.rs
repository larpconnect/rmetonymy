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

const ZERO_F64: f64 = 0.0;
const ONE_F64: f64 = 1.0;

/// Samples a random index using Zipf's law.
#[expect(clippy::cast_precision_loss, reason = "safe direct cast of choice index in hot path")]
pub fn sample_zipf<R: Rng + ?Sized>(num_choices: usize, a: f64, b: f64, rng: &mut R) -> usize {
    if num_choices <= 1 {
        return 0;
    }
    let mut weights = Vec::with_capacity(num_choices);
    let mut sum = ZERO_F64;
    for i in 1..=num_choices {
        let w = ONE_F64 / (i as f64 + b).powf(a);
        weights.push(w);
        sum += w;
    }

    let r = rng.random::<f64>() * sum;
    let mut accum = ZERO_F64;
    for (i, w) in weights.into_iter().enumerate() {
        accum += w;
        if accum >= r {
            return i;
        }
    }
    num_choices - 1
}
