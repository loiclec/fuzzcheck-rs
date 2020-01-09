#![feature(saturating_neg)]
#![feature(vec_remove_item)]

extern crate fuzzcheck;
use fuzzcheck::input::FuzzedInput;

pub mod bool;
pub mod integer;
pub mod option;
pub mod vector;
pub mod void;

pub trait FuzzedJsonInput: FuzzedInput {
    fn from_json(json: &json::JsonValue) -> Option<Self::Value>;
    fn to_json(value: &Self::Value) -> json::JsonValue;

    fn from_data(data: &[u8]) -> Option<Self::Value> {
        if let Ok(s) = std::str::from_utf8(data) {
            json::parse(s).ok().and_then(|x| Self::from_json(&x))
        } else {
            None
        }
    }
    fn to_data(value: &Self::Value) -> Vec<u8> {
        let json = Self::to_json(&value);
        json.dump().into_bytes()
    }
}


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

pub fn cplxity_to_size(cplx: f64) -> usize {
    let size_f = 2.0_f64.powf(cplx).round();
    if std::usize::MAX as f64 > size_f {
        size_f as usize
    } else {
        std::usize::MAX
    }
}
pub fn size_to_cplxity(size: usize) -> f64 {
    (size as f64).log2()
}
