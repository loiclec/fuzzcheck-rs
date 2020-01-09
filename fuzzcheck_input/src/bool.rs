use std::hash::Hash;
use std::hash::Hasher;

extern crate fuzzcheck;
use fuzzcheck::input::*;

use crate::FuzzedJsonInput;

#[derive(Clone)]
pub enum FuzzedBool {}

impl FuzzedJsonInput for FuzzedBool {
    fn from_json(json: &json::JsonValue) -> Option<Self::Value> {
        json.as_bool()
    }
    fn to_json(value: &Self::Value) -> json::JsonValue {
        json::JsonValue::Boolean(*value)
    }
}

impl FuzzedInput for FuzzedBool {
    type Value = bool;
    type State = bool; // true if it has been mutated, false otherwise
    type UnmutateToken = ();

    fn default() -> Self::Value {
        false
    }

    fn state_from_value(_value: &Self::Value) -> Self::State {
        false
    }

    fn arbitrary(seed: usize, _max_cplx: f64) -> Self::Value {
        seed % 2 == 0
    }

    fn max_complexity() -> f64 {
        1.0
    }

    fn min_complexity() -> f64 {
        1.0
    }

    fn hash_value<H: Hasher>(value: &Self::Value, state: &mut H) {
        value.hash(state);
    }

    fn complexity(_value: &Self::Value, _state: &Self::State) -> f64 {
        1.0
    }

    fn mutate(value: &mut Self::Value, state: &mut Self::State, _max_cplx: f64) -> Self::UnmutateToken {
        *value = !*value;
        *state = true;
    }

    fn unmutate(value: &mut Self::Value, _state: &mut Self::State, _t: Self::UnmutateToken) {
        *value = !*value;
    }

    fn from_data(data: &[u8]) -> Option<Self::Value> {
        <Self as FuzzedJsonInput>::from_data(data)
    }
    fn to_data(value: &Self::Value) -> Vec<u8> {
        <Self as FuzzedJsonInput>::to_data(value)
    }
}
