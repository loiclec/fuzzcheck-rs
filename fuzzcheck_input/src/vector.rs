use std::hash::Hasher;
use std::marker::PhantomData;

extern crate fuzzcheck;
use fuzzcheck::input::*;

extern crate rand;
use rand::rngs::SmallRng;
use rand::{Rng, SeedableRng};

use crate::FuzzedJsonInput;

// TODO: should go back to InputFuzzer trait and put rng in it?
// it acts as the “configuration” thing, common to all inputs
pub struct FuzzedVector<V: FuzzedInput> {
    phantom: PhantomData<V>,
}

struct FuzzedVectorArbitrarySeed {
    complexity_step: usize,
    len_step: usize,
    rng: SmallRng,
}

impl FuzzedVectorArbitrarySeed {
    fn new(step: usize) -> Self {
        let mut rng = SmallRng::from_entropy();
        if step == 0 {
            Self {
                complexity_step: 0,
                len_step: 0,
                rng,
            }
        } else {
            let (complexity_step, len_step) = if step < 100 {
                // deterministic phase for 100 first steps
                (step % 10, step / 10)
            } else {
                // default
                (step, rng.gen())
            };
            Self {
                complexity_step,
                len_step,
                rng,
            }
        }
    }
}

#[derive(Clone, Debug)]
struct MutationStep {
    category: MutationCategory,
    remove_idx: usize,
    insert_idx: usize,
    vec_operations: Vec<VecOperation>,
    cycle: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum VecOperation {
    Remove,
    Insert,
}

impl MutationStep {
    fn new(len: usize) -> Self {
        let (category, vec_operations) = if len > 0 {
            (
                MutationCategory::Element(0),
                vec![VecOperation::Insert, VecOperation::Remove],
            )
        } else {
            (MutationCategory::Empty, vec![VecOperation::Insert])
        };
        Self {
            category,
            remove_idx: len.saturating_sub(1),
            insert_idx: 0,
            vec_operations,
            cycle: 0,
        }
    }
}

#[derive(Debug, Clone)]
enum MutationCategory {
    Empty,
    Element(usize),
    Vector(usize),
}
use crate::vector::MutationCategory::*;

#[derive(Debug, Clone)]
pub struct FuzzedVectorState<State> {
    inner_states: Vec<State>,
    sum_cplx: f64,
    step: MutationStep,
    rng: SmallRng,
}
impl<State> FuzzedVectorState<State> {
    fn increment_mutation_step_category(&mut self) {
        match self.step.category {
            Empty => {
                if !self.inner_states.is_empty() {
                    self.step.category = MutationCategory::Element(0)
                } else {
                    self.step.category = MutationCategory::Vector(0)
                }
            }
            Element(idx) => {
                let new_idx = idx + 1;
                if new_idx < self.inner_states.len() {
                    self.step.category = MutationCategory::Element(new_idx)
                } else {
                    self.step.category = MutationCategory::Vector(0)
                }
            }
            Vector(step) => {
                let new_step = step + 1;
                if new_step < self.step.vec_operations.len() {
                    self.step.category = MutationCategory::Vector(new_step)
                } else {
                    self.step.cycle += 1;
                    if !self.inner_states.is_empty() {
                        self.step.category = MutationCategory::Element(0)
                    } else {
                        self.step.category = MutationCategory::Vector(0)
                    }
                }
            }
        }
    }
}

pub enum UnmutateVecToken<V, T> {
    Element(usize, T, f64),
    Remove(usize, f64),
    Insert(usize, V, f64),
    Replace(Vec<V>, f64),
}

impl<V: FuzzedJsonInput> FuzzedVector<V> {
    fn mutate_element(
        value: &mut Vec<V::Value>,
        state: &mut FuzzedVectorState<V::State>,
        idx: usize,
        spare_cplx: f64,
    ) -> UnmutateVecToken<V::Value, V::UnmutateToken> {
        let el = &mut value[idx];
        let el_state = &mut state.inner_states[idx];

        let old_cplx = V::complexity(el, el_state);

        let token = V::mutate(el, el_state, spare_cplx);

        let new_cplx = V::complexity(el, el_state);

        state.sum_cplx += new_cplx - old_cplx;
        state.increment_mutation_step_category();

        // TODO: what to do with inner_states (for now: nothing)
        UnmutateVecToken::Element(idx, token, old_cplx - new_cplx)
    }

    fn insert_element(
        value: &mut Vec<V::Value>,
        state: &mut FuzzedVectorState<V::State>,
        spare_cplx: f64,
    ) -> UnmutateVecToken<V::Value, V::UnmutateToken> {
        let (idx, cycle) = (state.step.insert_idx, state.step.cycle);

        // TODO: For now I assume that the complexity given by the length of the vector does not change
        // Should I take it into account instead?
        let el = V::arbitrary(cycle, spare_cplx);
        let el_state = V::state_from_value(&el);
        let el_cplx = V::complexity(&el, &el_state);

        value.insert(idx, el);
        let token = UnmutateVecToken::Remove(idx, el_cplx); // TODO: is that always right?

        state.sum_cplx += el_cplx;
        // TODO: what is happening here?
        state.step.insert_idx = (state.step.insert_idx + 1) % (state.inner_states.len() + 1);
        state.increment_mutation_step_category();
        // TODO: what to do with inner_states? (for now: nothing)
        token
    }

    fn remove_element(
        value: &mut Vec<V::Value>,
        state: &mut FuzzedVectorState<V::State>,
    ) -> UnmutateVecToken<V::Value, V::UnmutateToken> {
        let idx = state.step.remove_idx;

        let el = &value[idx];
        let el_cplx = V::complexity(&el, &state.inner_states[idx]);

        let removed = value.remove(idx);
        let token = UnmutateVecToken::Insert(idx, removed, el_cplx);

        state.sum_cplx -= el_cplx;

        if state.step.remove_idx == 0 {
            state.step.vec_operations.remove_item(&VecOperation::Remove);
        } else {
            state.step.remove_idx -= 1;
        }

        state.increment_mutation_step_category();

        // TODO: what to do to state.inner_states?

        token
    }
}

impl<V: FuzzedJsonInput> FuzzedJsonInput for FuzzedVector<V> {
    fn from_json(json: &json::JsonValue) -> Option<Self::Value> {
        if let json::JsonValue::Array(json_values) = json {
            let mut result = vec![];
            for json_value in json_values {
                if let Some(value) = V::from_json(json_value) {
                    result.push(value);
                } else {
                    return None;
                }
            }
            Some(result)
        } else {
            None
        }
    }
    fn to_json(value: &Self::Value) -> json::JsonValue {
        json::JsonValue::Array(value.iter().map(|x| V::to_json(x)).collect())
    }
}

impl<V: FuzzedJsonInput> FuzzedInput for FuzzedVector<V> {
    type Value = Vec<V::Value>;
    type State = FuzzedVectorState<V::State>;
    type UnmutateToken = UnmutateVecToken<V::Value, V::UnmutateToken>;

    fn max_complexity() -> f64 {
        std::f64::INFINITY
    }

    fn min_complexity() -> f64 {
        1.0
    }

    fn complexity(value: &Self::Value, state: &Self::State) -> f64 {
        // TODO: should I really bother with this?
        1.0 + state.sum_cplx + crate::size_to_cplxity(value.len() + 1)
    }

    fn hash_value<H: Hasher>(value: &Self::Value, state: &mut H) {
        for e in value.iter() {
            V::hash_value(e, state);
        }
    }

    fn default() -> Self::Value {
        vec![]
    }

    fn state_from_value(value: &Self::Value) -> Self::State {
        let inner_states: Vec<_> = value.iter().map(|x| V::state_from_value(x)).collect();
        let sum_cplx = value
            .iter()
            .zip(inner_states.iter())
            .fold(0.0, |c, (v, s)| c + V::complexity(v, s));
        let step = MutationStep::new(value.len());
        let rng = SmallRng::from_entropy();

        Self::State {
            inner_states,
            sum_cplx,
            step,
            rng,
        }
    }

    fn arbitrary(seed: usize, max_cplx: f64) -> Self::Value {
        let FuzzedVectorArbitrarySeed {
            complexity_step,
            len_step,
            mut rng,
        } = FuzzedVectorArbitrarySeed::new(seed);

        if seed == 0 || max_cplx <= 1.0 {
            return Self::default();
        }

        let target_cplx = {
            let increments_target_cplx = (max_cplx * 100.0).round() as usize;
            let multiplied_target_cplx = crate::arbitrary_binary(0, increments_target_cplx, complexity_step) as f64;
            multiplied_target_cplx / 100.0
        };
        let min_cplx_el = V::min_complexity();

        // slight underestimate of the maximum number of elements required to produce an input of max_cplx
        let max_len_most_complex = {
            let overestimated_max_len: f64 = target_cplx / min_cplx_el;
            let max_len = if overestimated_max_len.is_infinite() {
                // min_cplx_el is 0, so the max length is the maximum complexity of the length component of the vector
                crate::cplxity_to_size(target_cplx)
            } else {
                // an underestimate of the true max_length, but not by much
                (overestimated_max_len - overestimated_max_len.log2()) as usize
            };
            if max_len > 10_000 {
                /* TODO */
                // 10_000?
                target_cplx.trunc() as usize
            } else {
                max_len
            }
        };
        let max_cplx_el = V::max_complexity();
        // slight underestimate of the minimum number of elements required to produce an input of max_cplx
        let min_len_most_complex = target_cplx / max_cplx_el - (target_cplx / max_cplx_el).log2();
        if !min_len_most_complex.is_finite() {
            // in this case, the elements are always of cplx 0, so we can only vary the length of the vector
            let len = crate::arbitrary_binary(0, max_len_most_complex, len_step);
            let mut v = Self::default();
            for _ in 0..len {
                // no point in adding valid step and max_cplx argument, the elements have only one possible value
                let el = V::arbitrary(0, 0.0);
                v.push(el);
            }
            v
        } else {
            let min_len_most_complex = min_len_most_complex.trunc() as usize;
            // arbitrary restriction on the length of the generated number, to avoid creating absurdly large vectors
            // of very simple elements, that take up too much memory
            let max_len_most_complex = if max_len_most_complex > 10_000 {
                /* TODO */
                // 10_000?
                target_cplx.trunc() as usize
            } else {
                max_len_most_complex
            };

            // choose a length between min_len_most_complex and max_len_most_complex
            let target_len = crate::arbitrary_binary(min_len_most_complex, max_len_most_complex, len_step);
            // TODO: create a new_input_with_complexity method
            let mut v = Self::default();
            let mut remaining_cplx = target_cplx;
            for i in 0..target_len {
                let max_cplx_element = remaining_cplx / ((target_len - i) as f64);
                if max_cplx_element <= min_cplx_el {
                    break;
                }
                let cplx_element = rng.gen_range(min_cplx_el, max_cplx_element);
                let x = V::arbitrary(rng.gen(), cplx_element);
                let x_state = V::state_from_value(&x);
                let x_cplx = V::complexity(&x, &x_state);
                v.push(x);
                remaining_cplx -= x_cplx;
            }
            v
        }
    }

    fn mutate(value: &mut Self::Value, state: &mut Self::State, max_cplx: f64) -> Self::UnmutateToken {
        let spare_cplx = max_cplx - Self::complexity(value, state);

        match state.step.category {
            MutationCategory::Empty => {
                state.increment_mutation_step_category();
                let mut old_value = vec![];
                std::mem::swap(value, &mut old_value);
                let old_sum_cplx = state.sum_cplx;
                state.sum_cplx = 0.0;
                // TODO: anything else?
                UnmutateVecToken::Replace(old_value, old_sum_cplx)
            }
            MutationCategory::Element(idx) => Self::mutate_element(value, state, idx, spare_cplx),
            MutationCategory::Vector(step) => {
                let operation_idx = step % state.step.vec_operations.len();
                let operation = state.step.vec_operations[operation_idx];
                match operation {
                    VecOperation::Insert => Self::insert_element(value, state, spare_cplx),
                    VecOperation::Remove => Self::remove_element(value, state),
                }
            }
        }
    }

    fn unmutate(value: &mut Self::Value, state: &mut Self::State, t: Self::UnmutateToken) {
        match t {
            UnmutateVecToken::Element(idx, inner_t, diff_cplx) => {
                let el = &mut value[idx];
                let el_state = &mut state.inner_states[idx];
                V::unmutate(el, el_state, inner_t);
                state.sum_cplx += diff_cplx;
            }
            UnmutateVecToken::Insert(idx, el, el_cplx) => {
                value.insert(idx, el);
                state.sum_cplx += el_cplx;
                // TODO: anything else?
            }
            UnmutateVecToken::Remove(idx, el_cplx) => {
                value.remove(idx);
                state.sum_cplx -= el_cplx;
            }
            UnmutateVecToken::Replace(vec, sum_cplx) => {
                let _ = std::mem::replace(value, vec);
                state.sum_cplx = sum_cplx;
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
