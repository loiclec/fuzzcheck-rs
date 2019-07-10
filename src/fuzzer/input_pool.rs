use std::collections::HashMap;

use crate::fuzzer::input::FuzzerInput;

// TODO: think through derive
#[derive(PartialEq, Eq, Hash, Clone)]
pub enum Feature {
    Edge(EdgeFeature),
    Comparison(ComparisonFeature)
}

#[derive(PartialEq, Eq, Hash, Clone)]
pub struct EdgeFeature {
    pc_guard: usize,
    intensity: u8
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
            pc_guard: pc_guard,
            intensity: score_from_counter(counter)
        }
    }
}

#[derive(PartialEq, Eq, Hash, Copy, Clone, PartialOrd, Ord)]
pub struct ComparisonFeature {
    pc: usize,
    id: u8
}

impl ComparisonFeature {
    pub fn new(pc: usize, arg1: u64, arg2: u64) -> ComparisonFeature {
        ComparisonFeature {
            pc: pc,
            id: score_from_counter(arg1.wrapping_sub(arg2).count_ones() as u16)
        }
    }
    /*
    init(pc: UInt, arg1: UInt64, arg2: UInt64) {
            self.init(pc: pc, argxordist: scoreFromCounter(UInt8((arg1 &- arg2).nonzeroBitCount)))
        }
    */
}

impl Feature {
    fn score(self) -> f64 {
        match self {
            Feature::Edge(_) => 1.0,
            Feature::Comparison(_) => 0.5
        }
    }
}

pub enum InputPoolIndex {
    Normal(usize),
    Favored
}

pub struct FuzzerState <Input> {
    input: Input
}

#[derive(Clone)]
pub struct InputPoolElement <Input: Clone> {
    input: Input,
    complexity: f64,
    features: Vec<Feature>,
    score: f64,
    flagged_for_deletion: bool
}

// TODO: think of req for Input
impl<Input: FuzzerInput> InputPoolElement<Input> {
    fn new(input: Input, complexity: f64, features: Vec<Feature>) -> InputPoolElement<Input> {
        InputPoolElement {
            input: input, 
            complexity: complexity,
            features: features,
            score: -1.0, 
            flagged_for_deletion: false
        }
    }
}

pub struct InputPool <Input: FuzzerInput> {
    inputs: Vec<InputPoolElement<Input>>,
    favored_input: Option<InputPoolElement<Input>>,
    cumulative_weights: Vec<f64>,
    score: f64,
    smallest_input_complexity_for_feature: HashMap<Feature, f64>
}

impl<Input: FuzzerInput> InputPool<Input> {
    fn get(&self, idx: InputPoolIndex) -> &InputPoolElement<Input> {
        match idx {
            InputPoolIndex::Normal(idx) => &self.inputs[idx],
            InputPoolIndex::Favored => &self.favored_input.as_ref().unwrap()
        }
    }
    fn set(&mut self, idx: InputPoolIndex, element: InputPoolElement<Input>) {
        match idx {
            InputPoolIndex::Normal(idx) => self.inputs[idx] = element,
            InputPoolIndex::Favored => panic!("Cannot change the favored input")
        }
    }

    fn update_scores(&mut self) -> impl FnOnce(&mut f64) -> () {
        // TODO
        |x: &mut f64| -> () { *x += 1.0 }
    }

    fn add(&mut self, elements: Vec<InputPoolElement<Input>>) -> impl FnOnce(&mut f64) -> () {
        for element in elements.iter() {
            for f in element.features.iter() {
                let complexity = self.smallest_input_complexity_for_feature.get(&f);
                if complexity == Option::None || element.complexity < *complexity.unwrap() {
                    let _ = self.smallest_input_complexity_for_feature.insert(f.clone(), element.complexity);
                }
            }
            self.inputs.push(element.clone());
        }
        let world_update_1 = self.update_scores();

        self.cumulative_weights = elements.iter().scan(0.0, |state, x| {
            *state = *state + x.score;
            Some(*state)
        }).collect();

        |x: &mut f64| -> () { 
            // TODO
            world_update_1(x);
            *x += 1.0 
        }
    }
}