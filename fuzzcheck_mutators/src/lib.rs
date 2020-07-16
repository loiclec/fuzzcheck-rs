pub extern crate fuzzcheck_traits;
pub extern crate fastrand;
pub extern crate fuzzcheck_mutators_derive;
pub use fuzzcheck_mutators_derive::*;

pub mod bool;
pub mod either;
pub mod integer;
pub mod option;
pub mod tuples;
pub mod vector;
pub mod void;

use std::ops::Range;
use fuzzcheck_traits::Mutator;

pub trait HasDefaultMutator: Clone {
    type Mutator: Mutator<Value=Self> + Default;
    fn default_mutator() -> Self::Mutator;
}

/// Generate a random f64 within the given range
/// The start and end of the range must be finite
/// This is a very naive implementation
pub fn gen_f64(rng: &fastrand::Rng, range: Range<f64>) -> f64 {
    assert!(range.start.is_finite() && range.end.is_finite());

    let granularity = u32::MAX;
    let granularity_f = granularity as f64;

    let x = rng.u32(0 .. granularity);

    range.start + ((range.end - range.start) / granularity_f) * (x as f64)
}

#[must_use]
pub fn arbitrary_binary(low: usize, high: usize, step: usize) -> usize {
    if high == low {
        return low;
    }
    let step = step % (high - low);
    let next = low.wrapping_add(high.wrapping_sub(low) / 2);
    if low.wrapping_add(1) == high {
        if step % 2 == 0 {
            high
        } else {
            low
        }
    } else if step == 0 {
        next
    } else if step % 2 == 1 {
        arbitrary_binary(next.wrapping_add(1), high, step / 2)
    } else {
        // step % 2 == 0
        arbitrary_binary(low, next.wrapping_sub(1), (step - 1) / 2)
    }
}

#[must_use]
pub fn cplxity_to_size(cplx: f64) -> usize {
    let size_f: f64 = 2.0_f64.powf(cplx).round();
    if std::usize::MAX as f64 > size_f {
        size_f as usize
    } else {
        std::usize::MAX
    }
}
#[must_use]
pub fn size_to_cplxity(size: usize) -> f64 {
    (size as f64).log2().round()
}
