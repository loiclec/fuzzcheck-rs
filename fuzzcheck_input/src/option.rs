use std::hash::Hash;
use std::hash::Hasher;

extern crate fuzzcheck;
use fuzzcheck::input::*;

use crate::FuzzedJsonInput;

use std::marker::PhantomData;

pub struct FuzzedOption<V: FuzzedInput>
where
    V::Value: Hash,
{
    phantom: PhantomData<V>,
}

pub enum FuzzedOptionUnmutateToken<Value, Token> {
    UnmutateSome(Token),
    ToSome(Value),
    ToNone,
}
use crate::option::FuzzedOptionUnmutateToken::*;

#[derive(Debug, Clone)]
pub struct FuzzedOptionState<State> {
    inner_state: Option<State>,
    did_check_none: bool,
    some_mutation_step: usize,
}

struct FuzzedOptionArbitrarySeed {
    check_none: bool,
    inner_seed: usize,
}

impl FuzzedOptionArbitrarySeed {
    fn new(seed: usize) -> Self {
        Self {
            check_none: seed == 0,
            inner_seed: seed.saturating_sub(1),
        }
    }
}

impl<V: FuzzedJsonInput> FuzzedJsonInput for FuzzedOption<V>
where
    V::Value: Hash,
{
    fn from_json(json: &json::JsonValue) -> Option<Self::Value> {
        if json.is_null() {
            Some(None)
        } else if let Some(value) = V::from_json(json) {
            Some(Some(value))
        } else {
            None
        }
    }
    fn to_json(value: &Self::Value) -> json::JsonValue {
        if let Some(value) = value {
            V::to_json(&value)
        } else {
            json::JsonValue::Null
        }
    }
}

impl<V: FuzzedJsonInput> FuzzedInput for FuzzedOption<V>
where
    V::Value: Hash,
{
    type Value = Option<V::Value>;
    type State = FuzzedOptionState<V::State>;
    type UnmutateToken = FuzzedOptionUnmutateToken<V::Value, V::UnmutateToken>;

    fn default() -> Self::Value {
        None
    }

    fn state_from_value(value: &Self::Value) -> Self::State {
        if let Some(inner) = value {
            Self::State {
                inner_state: Some(V::state_from_value(inner)),
                did_check_none: false,
                some_mutation_step: 0,
            }
        } else {
            Self::State {
                inner_state: None,
                did_check_none: true,
                some_mutation_step: 0,
            }
        }
    }

    fn arbitrary(seed: usize, max_cplx: f64) -> Self::Value {
        let seed = FuzzedOptionArbitrarySeed::new(seed);
        if seed.check_none {
            None
        } else {
            let inner_value = V::arbitrary(seed.inner_seed, max_cplx - 1.0);
            Some(inner_value)
        }
    }

    fn max_complexity() -> f64 {
        1.0 + V::max_complexity()
    }

    fn min_complexity() -> f64 {
        1.0 + V::min_complexity()
    }

    fn complexity(value: &Self::Value, state: &Self::State) -> f64 {
        if let Some(inner_value) = value {
            let inner_state = state.inner_state.as_ref().unwrap();
            1.0 + V::complexity(inner_value, &inner_state)
        } else {
            1.0
        }
    }

    fn hash_value<H: Hasher>(value: &Self::Value, state: &mut H) {
        value.hash(state);
    }

    fn mutate(value: &mut Self::Value, state: &mut Self::State, max_cplx: f64) -> Self::UnmutateToken {
        let inner_max_cplx = max_cplx - 1.0;

        if let Some(inner_value) = value {
            if !state.did_check_none {
                let mut old_value = None;
                std::mem::swap(value, &mut old_value);
                state.did_check_none = true;
                ToSome(old_value.unwrap())
            } else {
                let inner_state = state.inner_state.as_mut().unwrap();
                let inner_token = V::mutate(inner_value, inner_state, inner_max_cplx);
                UnmutateSome(inner_token)
            }
        } else {
            *value = Self::arbitrary(state.some_mutation_step, inner_max_cplx);
            state.some_mutation_step = state.some_mutation_step.wrapping_add(1);

            ToNone
        }
    }

    fn unmutate(value: &mut Self::Value, state: &mut Self::State, t: Self::UnmutateToken) {
        match t {
            UnmutateSome(t) => {
                let inner_value = value.as_mut().unwrap();
                let inner_state = state.inner_state.as_mut().unwrap();
                V::unmutate(inner_value, inner_state, t);
            }
            ToSome(v) => {
                *value = Some(v);
            }
            ToNone => {
                *value = None;
            }
        }
    }

    fn from_data(data: &[u8]) -> Option<Self::Value> {
        <Self as FuzzedJsonInput>::from_data(data)
    }
    fn to_data(value: &Self::Value) -> Vec<u8> {
        <Self as FuzzedJsonInput>::to_data(value)
    }
}
