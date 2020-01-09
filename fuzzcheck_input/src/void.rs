use std::hash::Hasher;

extern crate fuzzcheck;
use fuzzcheck::input::*;

#[derive(Clone)]
pub enum FuzzedVoid { }

impl FuzzedInput for FuzzedVoid {
    type Value = ();
    type State = ();
    type UnmutateToken = ();

    fn default() -> Self::Value {
        
    }

    fn state_from_value(_value: &Self::Value) -> Self::State {
        
    }

    fn arbitrary(_seed: usize, _max_cplx: f64) -> Self::Value {
        
    }

    fn max_complexity() -> f64 {
        0.0
    }

    fn min_complexity() -> f64 {
        0.0
    }

    fn hash_value<H: Hasher>(_value: &Self::Value, _state: &mut H) {

    }

    fn complexity(_value: &Self::Value, _state: &Self::State) -> f64 {
        0.0
    }

    fn mutate(_value: &mut Self::Value, _state: &mut Self::State, _max_cplx: f64) -> Self::UnmutateToken {

    }

    fn unmutate(_value: &mut Self::Value, _state: &mut Self::State, _t: Self::UnmutateToken) {
    }

    fn from_data(_data: &[u8]) -> Option<Self::Value> {
        Some(())
    }
    fn to_data(_value: &Self::Value) -> Vec<u8> {
        vec![] // TODO: not good
    }
}

