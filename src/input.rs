use core::hash::Hash;

pub trait InputGenerator {
    type Input: Hash + Clone;

    fn complexity(input: &Self::Input) -> f64;

    fn adjusted_complexity(input: &Self::Input) -> f64 {
        Self::complexity(input) + 1.0
    }

    fn new_input(&mut self, max_cplx: f64) -> Self::Input;

    fn initial_inputs(&mut self, max_cplx: f64) -> Vec<Self::Input> {
        (0..10).map(|_| self.new_input(max_cplx)).collect()
    }

    fn mutate(&mut self, input: &mut Self::Input, spare_cplx: f64) -> bool;

    fn from_data(data: &Vec<u8>) -> Option<Self::Input>;
    fn to_data(input: &Self::Input) -> Vec<u8>;
}
