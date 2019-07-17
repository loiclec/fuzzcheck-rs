use std::collections::HashMap;
use std::hash::Hash;
use std::marker::PhantomData;

use rand::rngs::ThreadRng;
use rand::Rng;

use rand::distributions::uniform::{UniformFloat, UniformSampler};
use rand::distributions::Distribution;

use crate::input::InputGenerator;
use crate::weighted_index::WeightedIndex;
use crate::world::FuzzerEvent;
use crate::world::World;

#[derive(PartialEq, Eq, Hash, Clone)]
pub enum Feature {
    Edge(EdgeFeature),
    Comparison(ComparisonFeature),
    Indir(IndirFeature),
}

#[derive(PartialEq, Eq, Hash, Clone)]
pub struct EdgeFeature {
    pc_guard: usize,
    intensity: u8,
}
#[derive(PartialEq, Eq, Hash, Clone)]
pub struct IndirFeature {
    pub caller: usize,
    pub callee: usize,
}

fn score_from_counter(counter: u16) -> u8 {
    if counter == core::u16::MAX {
        16
    } else if counter <= 3 {
        counter as u8
    } else {
        (16 - counter.leading_zeros() + 1) as u8
    }
}

impl EdgeFeature {
    pub fn new(pc_guard: usize, counter: u16) -> EdgeFeature {
        EdgeFeature {
            pc_guard,
            intensity: score_from_counter(counter),
        }
    }
}

#[derive(PartialEq, Eq, Hash, Copy, Clone, PartialOrd, Ord)]
pub struct ComparisonFeature {
    pc: usize,
    id: u8,
}

impl ComparisonFeature {
    pub fn new(pc: usize, arg1: u64, arg2: u64) -> ComparisonFeature {
        ComparisonFeature {
            pc,
            id: score_from_counter(arg1.wrapping_sub(arg2).count_ones() as u16),
        }
    }
}

impl Feature {
    fn score(&self) -> f64 {
        match self {
            Feature::Edge(_) => 1.0,
            Feature::Comparison(_) => 0.1,
            Feature::Indir(_) => 1.0,
        }
    }
}

pub enum InputPoolIndex {
    Normal(usize),
    Favored,
}

#[derive(Clone)]
pub struct InputPoolElement<T: Hash + Clone> {
    pub input: T,
    pub complexity: f64,
    features: Vec<Feature>,
    score: f64,
    flagged_for_deletion: bool,
}

impl<T: Hash + Clone> InputPoolElement<T> {
    pub fn new(input: T, complexity: f64, features: Vec<Feature>) -> InputPoolElement<T> {
        InputPoolElement {
            input,
            complexity,
            features,
            score: -1.0,
            flagged_for_deletion: false,
        }
    }
}

pub struct InputPool<T: Hash + Clone, R> {
    pub inputs: Vec<InputPoolElement<T>>,
    pub favored_input: Option<InputPoolElement<T>>,
    cumulative_weights: Vec<f64>,
    pub score: f64,
    pub smallest_input_complexity_for_feature: HashMap<Feature, f64>,
    rng: ThreadRng,
    phantom: PhantomData<R>,
}

impl<T: Hash + Clone, R> InputPool<T, R> {
    pub fn new() -> Self {
        InputPool {
            inputs: vec![],
            favored_input: None,
            cumulative_weights: vec![],
            score: 0.0,
            smallest_input_complexity_for_feature: HashMap::new(),
            rng: rand::thread_rng(),
            phantom: PhantomData,
        }
    }

    pub fn get(&self, idx: InputPoolIndex) -> &InputPoolElement<T> {
        match idx {
            InputPoolIndex::Normal(idx) => &self.inputs[idx],
            InputPoolIndex::Favored => &self.favored_input.as_ref().unwrap(),
        }
    }
    fn set(&mut self, idx: InputPoolIndex, element: InputPoolElement<T>) {
        match idx {
            InputPoolIndex::Normal(idx) => self.inputs[idx] = element,
            InputPoolIndex::Favored => panic!("Cannot change the favored input"),
        }
    }

    fn complexity_ratio(simplest: f64, other: f64) -> f64 {
        let square = |x| x * x;
        square(simplest / other)
    }

    pub fn update_scores<G>(&mut self) -> impl FnOnce(&mut World<T, G>) -> ()
    where
        G: InputGenerator<Input = T>,
    {
        let mut sum_cplx_ratios: HashMap<Feature, f64> = HashMap::new();
        for input in self.inputs.iter_mut() {
            input.flagged_for_deletion = true;
            input.score = 0.0;
            for f in input.features.iter() {
                let simplest_cplx = self.smallest_input_complexity_for_feature[f];
                let ratio = Self::complexity_ratio(simplest_cplx, input.complexity);
                assert!(ratio <= 1.0);
                if (simplest_cplx - input.complexity).abs() < std::f64::EPSILON {
                    input.flagged_for_deletion = false;
                }
            }
            if input.flagged_for_deletion {
                continue;
            }
            for f in input.features.iter() {
                let simplest_cplx = self.smallest_input_complexity_for_feature[f];
                let ratio = Self::complexity_ratio(simplest_cplx, input.complexity);
                *sum_cplx_ratios.entry(f.clone()).or_insert(0.0) += ratio;
            }
        }

        for input in self.inputs.iter_mut() {
            if input.flagged_for_deletion {
                continue;
            }
            for f in input.features.iter() {
                let simplest_cplx = self.smallest_input_complexity_for_feature[f];
                let sum_ratios = sum_cplx_ratios[f];
                let base_score = f.score() / sum_ratios;
                let ratio = Self::complexity_ratio(simplest_cplx, input.complexity);
                let score = base_score * ratio;
                input.score += score;
            }
        }

        let inputs_to_delete: Vec<T> = self
            .inputs
            .iter()
            .filter_map(|i| {
                if i.flagged_for_deletion {
                    Some(i.input.clone())
                } else {
                    None
                }
            })
            .collect();

        let _ = self.inputs.drain_filter(|i| i.flagged_for_deletion);
        self.score = self.inputs.iter().fold(0.0, |x, next| x + next.score);
        let deleted_some = !inputs_to_delete.is_empty();
        move |w| {
            for i in &inputs_to_delete {
                // TODO: handle error
                let _ = w.remove_from_output_corpus(i.clone());
            }
            if deleted_some {
                w.report_event(FuzzerEvent::Deleted(inputs_to_delete.len()), Option::None);
            }
        }
    }

    pub fn add<G>(&mut self, elements: Vec<InputPoolElement<T>>) -> impl FnOnce(&mut World<T, G>) -> ()
    where
        G: InputGenerator<Input = T>,
    {
        for element in elements.iter() {
            for f in element.features.iter() {
                let complexity = self.smallest_input_complexity_for_feature.get(&f);
                if complexity == Option::None || element.complexity < *complexity.unwrap() {
                    let _ = self
                        .smallest_input_complexity_for_feature
                        .insert(f.clone(), element.complexity);
                }
            }
            self.inputs.push(element.clone());
        }
        let world_update_1 = self.update_scores();

        self.cumulative_weights = self
            .inputs
            .iter()
            .scan(0.0, |state, x| {
                *state += x.score;
                Some(*state)
            })
            .collect();

        |w: &mut World<T, G>| {
            world_update_1(w);
            for i in elements {
                // TODO
                let _ = w.add_to_output_corpus(i.input);
            }
        }
    }
    fn add_one<G>(&mut self, element: InputPoolElement<T>) -> impl FnOnce(&mut World<T, G>) -> ()
    where
        G: InputGenerator<Input = T>,
    {
        for f in element.features.iter() {
            let complexity = self.smallest_input_complexity_for_feature.get(&f);
            if complexity == Option::None || element.complexity < *complexity.unwrap() {
                let _ = self
                    .smallest_input_complexity_for_feature
                    .insert(f.clone(), element.complexity);
            }
        }
        self.inputs.push(element.clone());
        let world_update_1 = self.update_scores();

        self.cumulative_weights = self
            .inputs
            .iter()
            .scan(0.0, |state, x| {
                *state += x.score;
                Some(*state)
            })
            .collect();

        |w: &mut World<T, G>| {
            world_update_1(w);
            // TODO
            let _ = w.add_to_output_corpus(element.input);
        }
    }

    pub fn random_index(&mut self) -> InputPoolIndex {
        if self.favored_input.is_some() && (self.rng.gen_bool(0.25) || self.inputs.is_empty()) {
            InputPoolIndex::Favored
        } else {
            let weight_distr = UniformFloat::new(0.0, self.cumulative_weights.last().unwrap_or(&0.0));
            let dist = WeightedIndex {
                cumulative_weights: self.cumulative_weights.clone(),
                weight_distribution: weight_distr,
            };
            let x = dist.sample(&mut self.rng);
            InputPoolIndex::Normal(x)
        }
    }

    fn empty(&mut self) {
        self.inputs.clear();
        self.score = 0.0;
        self.cumulative_weights.clear();
        self.smallest_input_complexity_for_feature.clear();
    }
}
