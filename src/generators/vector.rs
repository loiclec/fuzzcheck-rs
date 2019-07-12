
use rand::Rng;
use rand::seq::SliceRandom;
use rand::seq::index;
use rand::rngs::ThreadRng;
use rand::distributions::WeightedIndex;
use rand::distributions::{Distribution};

use rand_distr::Exp1;

use crate::input::*;

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
static WEIGHTS: &[usize] = &[
    5,
    5,
    5,
    5,
    15,
    5,
    5,
    5,
];

pub struct VectorGenerator<G> where G: InputGenerator {
    g: G,
    weighted_index: WeightedIndex<usize>
}

impl<G> VectorGenerator<G> where G: InputGenerator {
    pub fn new(g: G) -> Self {
        Self {
            g,
            weighted_index: WeightedIndex::new(WEIGHTS).unwrap()
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

impl<G> VectorGenerator<G> where G: InputGenerator {
    fn mutate_with(&self, mutator: VectorMutator, input: &mut Vec<G::Input>, spare_cplx: f64, rng: &mut ThreadRng) -> bool {
        match mutator {
            VectorMutator::AppendNew => {
                let add_cplx = rng.sample(Exp1);
                input.push(self.g.new_input(add_cplx, rng));
                true
            },
            VectorMutator::AppendRecycled => {
                if input.is_empty() { 
                    false 
                }
                else {
                    let pick = input.choose(rng).unwrap().clone();
                    input.push(pick);
                    true
                }
            },
            VectorMutator::InsertNew => {
                if input.is_empty() { 
                    self.mutate_with(VectorMutator::AppendNew, input, spare_cplx, rng)
                }
                else {
                    let add_cplx = rng.sample(Exp1);
                    let idx = rng.gen_range(0, input.len());
                    input.insert(idx, self.g.new_input(add_cplx, rng));
                    true
                }
            },
            VectorMutator::InsertRecycled => {
                if input.is_empty() { 
                    false 
                }
                else {
                    let pick = input.choose(rng).unwrap().clone();
                    let idx = rng.gen_range(0, input.len());
                    input.insert(idx, pick);
                    true
                }
            },
            VectorMutator::MutateElement => {
                if input.is_empty() { 
                    false 
                }
                else {
                    let idx = rng.gen_range(0, input.len());
                    self.g.mutate(&mut input[idx], spare_cplx, rng)
                }
            },
            VectorMutator::Swap => {
                if input.len() < 2 { 
                    false 
                }
                else {
                    let idxs = index::sample(rng, input.len(), 2);
                    input.swap(idxs.index(0), idxs.index(1));
                    true
                }
            },
            VectorMutator::RemoveLast => {    
                input.pop().is_some()
            },
            VectorMutator::RemoveRandom => {
                if input.is_empty() { 
                    false 
                }
                else {
                    let idx = rng.gen_range(0, input.len());
                    input.remove(idx);
                    true
                }
            },
        }
    }
}

impl<T: FuzzerInput> FuzzerInput for Vec<T> {}

impl<G> InputProperties for VectorGenerator<G> where G: InputGenerator {
    type Input = Vec<G::Input>;

    fn complexity(input: &Self::Input) -> f64 {
        input.iter().fold(0.0, |c, n| c + G::complexity(n))
    }
}
impl<G> InputGenerator for VectorGenerator<G> where G: InputGenerator {
    fn base_input(&self) -> Self::Input {
        vec![]
    }

    fn new_input(&self, max_cplx: f64, rng: &mut ThreadRng) -> Self::Input {
        if max_cplx <= 0.0 { return vec![]; }
        let target_cplx: f64 = rng.gen_range(0.0, max_cplx);
        let mut result: Self::Input = vec![];
        let mut cur_cplx = Self::complexity(&result);
        loop {
            self.mutate_with(VectorMutator::AppendNew, &mut result, target_cplx - cur_cplx, rng);
            cur_cplx = Self::complexity(&result);
            while cur_cplx >= target_cplx {
                self.mutate_with(VectorMutator::RemoveRandom, &mut result, target_cplx - cur_cplx, rng);
                cur_cplx = Self::complexity(&result);
                if cur_cplx <= target_cplx {
                    result.shuffle(rng);
                    return result
                }
            }
        }
    }

    fn mutate(&self, input: &mut Self::Input, spare_cplx: f64, rng: &mut ThreadRng) -> bool {
        for _ in 0..MUTATORS.len() {
            let pick = self.weighted_index.sample(rng);
            if self.mutate_with(MUTATORS[pick], input, spare_cplx, rng) {
                return true;
            }
        }
        false
    }
}

