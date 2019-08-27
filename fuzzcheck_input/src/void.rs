use std::hash::Hasher;

extern crate fuzzcheck;
use fuzzcheck::input::*;

pub struct VoidGenerator {}

impl InputGenerator for VoidGenerator {
    type Input = ();

    fn complexity(_input: &Self::Input) -> f64 {
        0.0
    }

    fn hash<H>(_input: &Self::Input, _state: &mut H)
    where
        H: Hasher,
    {
    }

    fn base_input() -> Self::Input {}
    fn new_input(&mut self, _max_cplx: f64) -> Self::Input {}

    fn mutate(&mut self, _input: &mut Self::Input, _spare_cplx: f64) -> bool {
        true
    }

    fn from_data(_data: &[u8]) -> Option<Self::Input> {
        Some(())
    }
    fn to_data(_input: &Self::Input) -> Vec<u8> {
        vec![]
    }
}
