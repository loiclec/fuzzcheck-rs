#![feature(vec_remove_item)]

extern crate fuzzcheck;

pub mod bool;
pub mod either;
pub mod integer;
pub mod option;
pub mod tuples;
pub mod vector;
pub mod void;

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
    (size as f64).log2()
}
