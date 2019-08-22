

use rand::distributions::Distribution;
use rand::distributions::WeightedIndex;
use rand::rngs::ThreadRng;
use rand::seq::index;
use rand::seq::SliceRandom;
use rand::Rng;

use miniserde::json;

use std::hash::Hash;
use std::hash::Hasher;

use rand_distr::Exp1;

extern crate fuzzcheck;
use fuzzcheck::input::*;

// Let's be honest, everything in this file is guesswork

static MUTATORS: &[VectorMutator] = &[
    VectorMutator::AppendNew,
    VectorMutator::AppendRecycled,
    VectorMutator::InsertNew,
    VectorMutator::InsertRecycled,
    VectorMutator::MutateElement,
    VectorMutator::Swap,
    VectorMutator::RemoveLast,
    VectorMutator::RemoveRandom,
];
static WEIGHTS: &[usize] = &[5, 5, 5, 5, 15, 5, 5, 5];

pub struct VectorGenerator<G>
where
    G: InputGenerator,
{
    g: G,
    rng: ThreadRng,
    weighted_index: WeightedIndex<usize>,
}

impl<G> VectorGenerator<G>
where
    G: InputGenerator,
{
    pub fn new(g: G) -> Self {
        Self {
            g,
            rng: rand::thread_rng(),
            weighted_index: WeightedIndex::new(WEIGHTS).unwrap(),
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub enum VectorMutator {
    AppendNew,
    AppendRecycled,
    InsertNew,
    InsertRecycled,
    MutateElement,
    Swap,
    RemoveLast,
    RemoveRandom,
}

impl<G> VectorGenerator<G>
where
    G: InputGenerator,
{
    fn mutate_with(
        &mut self,
        mutator: VectorMutator,
        input: &mut Vec<G::Input>,
        spare_cplx: f64
    ) -> bool {
        match mutator {
            VectorMutator::AppendNew => {
                let add_cplx = self.rng.sample(Exp1);
                input.push(self.g.new_input(add_cplx));
                true
            }
            VectorMutator::AppendRecycled => {
                if input.is_empty() {
                    false
                } else {
                    let pick = input.choose(&mut self.rng).unwrap().clone();
                    input.push(pick);
                    true
                }
            }
            VectorMutator::InsertNew => {
                if input.is_empty() {
                    self.mutate_with(VectorMutator::AppendNew, input, spare_cplx)
                } else {
                    let add_cplx = self.rng.sample(Exp1);
                    let idx = self.rng.gen_range(0, input.len());
                    input.insert(idx, self.g.new_input(add_cplx));
                    true
                }
            }
            VectorMutator::InsertRecycled => {
                if input.is_empty() {
                    false
                } else {
                    let pick = input.choose(&mut self.rng).unwrap().clone();
                    let idx = self.rng.gen_range(0, input.len());
                    input.insert(idx, pick);
                    true
                }
            }
            VectorMutator::MutateElement => {
                if input.is_empty() {
                    false
                } else {
                    let idx = self.rng.gen_range(0, input.len());
                    self.g.mutate(&mut input[idx], spare_cplx)
                }
            }
            VectorMutator::Swap => {
                if input.len() < 2 {
                    false
                } else {
                    let idxs = index::sample(&mut self.rng, input.len(), 2);
                    input.swap(idxs.index(0), idxs.index(1));
                    true
                }
            }
            VectorMutator::RemoveLast => input.pop().is_some(),
            VectorMutator::RemoveRandom => {
                if input.is_empty() {
                    false
                } else {
                    let idx = self.rng.gen_range(0, input.len());
                    input.remove(idx);
                    true
                }
            }
        }
    }
}

impl<G> InputGenerator for VectorGenerator<G>
where
    G: InputGenerator,
    Vec<G::Input>: Hash + miniserde::Serialize + miniserde::Deserialize
{
    type Input = Vec<G::Input>;

    fn hash<H>(input: &Self::Input, state: &mut H) where H: Hasher {
        input.hash(state);
    }

    fn complexity(input: &Self::Input) -> f64 {
        input.iter().fold(0.0, |c, n| c + G::complexity(n))
    }

    fn base_input() -> Self::Input {
        Self::Input::default()
    }

    fn new_input(&mut self, max_cplx: f64) -> Self::Input {
        if max_cplx <= 0.0 {
            return vec![];
        }
        let target_cplx: f64 = self.rng.gen_range(0.0, max_cplx);
        let mut result: Self::Input = vec![];
        let mut cur_cplx = Self::complexity(&result);
        loop {
            self.mutate_with(VectorMutator::AppendNew, &mut result, target_cplx - cur_cplx);
            cur_cplx = Self::complexity(&result);
            while cur_cplx >= target_cplx {
                self.mutate_with(VectorMutator::RemoveRandom, &mut result, target_cplx - cur_cplx);
                cur_cplx = Self::complexity(&result);
                if cur_cplx <= target_cplx {
                    result.shuffle(&mut self.rng);
                    return result;
                }
            }
        }
    }

    fn mutate(&mut self, input: &mut Self::Input, spare_cplx: f64) -> bool {
        for _ in 0..MUTATORS.len() {
            let pick = self.weighted_index.sample(&mut self.rng);
            if self.mutate_with(MUTATORS[pick], input, spare_cplx) {
                return true;
            }
        }
        false
    }
    fn from_data(data: &Vec<u8>) -> Option<Self::Input> {
        if let Some(s) = std::str::from_utf8(data).ok() {
            json::from_str(s).ok()
        } else {
            None
        }
    }
    fn to_data(input: &Self::Input) -> Vec<u8> {
        json::to_string(input).into_bytes()
    }
}
