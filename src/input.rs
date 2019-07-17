use core::hash::Hash;

pub trait InputGenerator {
    type Input: Hash + Clone;
    type Rng;

    fn complexity(input: &Self::Input) -> f64;

    fn adjusted_complexity(input: &Self::Input) -> f64 {
        Self::complexity(input) + 1.0
    }

    fn base_input(&self) -> Self::Input;
    fn new_input(&self, max_cplx: f64, rand: &mut Self::Rng) -> Self::Input;

    fn initial_inputs(&self, max_cplx: f64, rand: &mut Self::Rng) -> Vec<Self::Input> {
        (0..10).map(|_| self.new_input(max_cplx, rand)).collect()
    }

    fn mutate(&self, input: &mut Self::Input, spare_cplx: f64, rand: &mut Self::Rng) -> bool;

    fn from_data(data: &Vec<u8>) -> Option<Self::Input>;
    fn to_data(input: &Self::Input) -> Vec<u8>;
}
