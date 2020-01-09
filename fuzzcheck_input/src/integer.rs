use std::hash::Hash;
use std::hash::Hasher;

extern crate json;

extern crate fuzzcheck;
use fuzzcheck::input::*;

use crate::FuzzedJsonInput;

#[derive(Clone)]
pub enum FuzzedU8 {}

impl FuzzedJsonInput for FuzzedU8 {
    fn from_json(json: &json::JsonValue) -> Option<Self::Value> {
        json.as_u8()
    }
    fn to_json(value: &Self::Value) -> json::JsonValue {
        json::JsonValue::Number((*value).into())
    }
}

impl FuzzedInput for FuzzedU8 {
    type Value = u8;
    type State = u16; // mutation step
    type UnmutateToken = u8; // old value

    fn default() -> Self::Value {
        0
    }

    fn state_from_value(_value: &Self::Value) -> Self::State {
        0
    }

    fn arbitrary(seed: usize, _max_cplx: f64) -> Self::Value {
        ((seed % std::u8::MAX as usize) as u8)
    }

    fn max_complexity() -> f64 {
        8.0
    }

    fn min_complexity() -> f64 {
        8.0
    }

    fn complexity(_value: &Self::Value, _state: &Self::State) -> f64 {
        8.0
    }

    fn hash_value<H: Hasher>(value: &Self::Value, state: &mut H) {
        value.hash(state);
    }

    fn mutate(value: &mut Self::Value, state: &mut Self::State, _max_cplx: f64) -> Self::UnmutateToken {
        let token = *value;
        *value = {
            let mut tmp_step = *state;
            if tmp_step < 8 {
                let nudge = tmp_step + 2;
                let nudge_u8 = nudge as u8;
                if nudge % 2 == 0 {
                    value.wrapping_add(nudge_u8 / 2)
                } else {
                    value.wrapping_sub(nudge_u8 / 2)
                }
            } else {
                tmp_step -= 7;
                let low = value.wrapping_sub(std::u8::MAX / 2);
                let high = value.wrapping_add(std::u8::MAX / 2 + 1);
                arbitrary_u8(low, high, tmp_step)
            }
        };
        *state = state.wrapping_add(1);

        token
    }

    fn unmutate(value: &mut Self::Value, _state: &mut Self::State, t: Self::UnmutateToken) {
        *value = t;
    }

    fn from_data(data: &[u8]) -> Option<Self::Value> {
        <Self as FuzzedJsonInput>::from_data(data)
    }
    fn to_data(value: &Self::Value) -> Vec<u8> {
        <Self as FuzzedJsonInput>::to_data(value)
    }
}

pub fn arbitrary_u8(low: u8, high: u8, step: u16) -> u8 {
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
        arbitrary_u8(next.wrapping_add(1), high, step / 2)
    } else {
        // step % 2 == 0
        arbitrary_u8(low, next.wrapping_sub(1), (step - 1) / 2)
    }
}
