use core::hash::Hash;
use rand::rngs::ThreadRng;

pub trait FuzzerInput: Hash + Clone {}

pub trait InputProperties {
    type Input: FuzzerInput;

    fn complexity(input: &Self::Input) -> f64;

    fn adjusted_complexity(input: &Self::Input) -> f64 {
        Self::complexity(input) + 1.0
    }
}

pub trait InputGenerator: InputProperties {
    fn base_input(&self) -> Self::Input;
    fn new_input(&self, max_cplx: f64, rand: &mut ThreadRng) -> Self::Input;

    fn initial_inputs(&self, max_cplx: f64, rand: &mut ThreadRng) -> Vec<Self::Input> {
        (0..10).map(|_| self.new_input(max_cplx, rand)).collect()
    }

    fn mutate(&self, input: &mut Self::Input, spare_cplx: f64, rand: &mut ThreadRng) -> bool;
}
