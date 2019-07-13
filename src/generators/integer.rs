
use rand::distributions::WeightedIndex;
use rand::distributions::uniform::SampleUniform;
use rand::distributions::{Distribution, Standard};
use rand::rngs::ThreadRng;
use rand::seq::SliceRandom;
use rand::Rng;
use std::cmp::PartialEq;
use std::mem;
use std::num::Wrapping;
use std::ops::{Add, Sub};

use crate::input::*;

impl<T: FuzzerInput> InputProperties for IntegerGenerator<T> {
    type Input = T;
    fn complexity(_input: &T) -> f64 {
        mem::size_of::<T>() as f64
    }
}

pub struct IntegerGenerator<T> {
    max_nudge: T,
    special_values: Vec<T>,
    mutators: Vec<IntegerMutatorKind>,
    weighted_index: WeightedIndex<usize>,
}

#[derive(Clone, Copy)]
enum IntegerMutatorKind {
    Special,
    Random,
    Nudge,
}

static MUTATORS: &[IntegerMutatorKind] = &[
    IntegerMutatorKind::Special,
    IntegerMutatorKind::Random,
    IntegerMutatorKind::Nudge,
];
static WEIGHTS: &[usize] = &[1, 10, 10];


impl<T> IntegerGenerator<T>
where
    T: Default + FuzzerInput + PartialEq + Copy + SampleUniform,
    Wrapping<T>: Add<Output = Wrapping<T>> + Sub<Output = Wrapping<T>>,
    Standard: Distribution<T>,
{
    fn new_with_special_values(max_nudge: T, special_values: Vec<T>) -> Self {
        Self {
            max_nudge,
            special_values,
            mutators: MUTATORS.to_vec(),
            weighted_index: WeightedIndex::new(WEIGHTS.to_vec()).unwrap(),
        }
    }

    fn nudge(&self, input: &mut T, rng: &mut ThreadRng) -> bool {
        let nudge: T = rng.gen_range(T::default(), self.max_nudge);
        let plus = rng.gen::<bool>();
        if plus {
            *input = (Wrapping(*input) + Wrapping(nudge)).0;
        } else {
            *input = (Wrapping(*input) - Wrapping(nudge)).0;
        }
        true
    }
    fn random(&self, input: &mut T, rng: &mut ThreadRng) -> bool {
        *input = rng.gen();
        true
    }
    fn special(&self, input: &mut T, rng: &mut ThreadRng) -> bool {
        let old = *input;
        *input = *self.special_values.choose(rng).unwrap();
        old != *input
    }
}

impl<T> IntegerGenerator<T>
where
    T: Default + FuzzerInput + PartialEq + Copy + SampleUniform,
    Wrapping<T>: Add<Output = Wrapping<T>> + Sub<Output = Wrapping<T>>,
    Standard: Distribution<T>,
{
    fn mutate_with(
        &self,
        input: &mut T,
        mutator: IntegerMutatorKind,
        _spare_cplx: f64,
        rng: &mut ThreadRng,
    ) -> bool {
        match mutator {
            IntegerMutatorKind::Special => self.special(input, rng),
            IntegerMutatorKind::Random => self.random(input, rng),
            IntegerMutatorKind::Nudge => self.nudge(input, rng),
        }
    }
}

impl<T> InputGenerator for IntegerGenerator<T>
where
    T: Default + FuzzerInput + PartialEq + Copy + SampleUniform,
    Wrapping<T>: Add<Output = Wrapping<T>> + Sub<Output = Wrapping<T>>,
    Standard: Distribution<T>,
{
    fn base_input(&self) -> T {
        T::default()
    }

    fn new_input(&self, _max_cplx: f64, rng: &mut ThreadRng) -> T {
        rng.gen()
    }

    fn mutate(&self, input: &mut Self::Input, spare_cplx: f64, rng: &mut ThreadRng) -> bool {
        for _ in 0..MUTATORS.len() {
            let pick = self.weighted_index.sample(rng);
            if self.mutate_with(input, MUTATORS[pick], spare_cplx, rng) {
                return true;
            }
        }
        false
    }
}

impl FuzzerInput for u8 {}
impl FuzzerInput for u16 {}
impl FuzzerInput for u32 {}
impl FuzzerInput for u64 {}
impl FuzzerInput for u128 {}
impl FuzzerInput for usize {}

impl FuzzerInput for i8 {}
impl FuzzerInput for i16 {}
impl FuzzerInput for i32 {}
impl FuzzerInput for i64 {}
impl FuzzerInput for i128 {}
impl FuzzerInput for isize {}

impl IntegerGenerator<u8> {
    pub fn new(max_nudge: u8) -> Self {
        Self::new_with_special_values(
            max_nudge,
            vec![0x0, 0x1, 0xff, 0x7f]
        )
    }
}
impl IntegerGenerator<u16> {
    pub fn new(max_nudge: u16) -> Self {
        Self::new_with_special_values(
            max_nudge,
            vec![0x0, 0x1, 0xff, 0x7f, 0xffff, 0x7fff]
        )
    }
}
impl IntegerGenerator<u32> {
    pub fn new(max_nudge: u32) -> Self {
        Self::new_with_special_values(
            max_nudge,
            vec![0x0, 0x1, 0xff, 0x7f, 0xffff, 0x7fff, 0xffff_ffff, 0x7fff_ffff]
        )
    }
}
impl IntegerGenerator<u64> {
    pub fn new(max_nudge: u64) -> Self {
        Self::new_with_special_values(
            max_nudge,
            vec![0x0, 0x1, 0xff, 0x7f, 0xffff, 0x7fff, 0xffff_ffff, 0x7fff_ffff, 0xffff_ffff_ffff_ffff, 0x7fff_ffff_ffff_ffff]
        )
    }
}
impl IntegerGenerator<u128> {
    pub fn new(max_nudge: u128) -> Self {
        Self::new_with_special_values(
            max_nudge,
            vec![0x0, 0x1, 0xff, 0x7f, 0xffff, 0x7fff, 0xffff_ffff, 0x7fff_ffff, 0xffff_ffff_ffff_ffff, 0x7fff_ffff_ffff_ffff, 0xffff_ffff_ffff_ffff_ffff_ffff_ffff_ffff, 0x7fff_ffff_ffff_ffff_ffff_ffff_ffff_ffff]
        )
    }
}
impl IntegerGenerator<usize> {
    pub fn new(max_nudge: usize) -> Self {
        Self::new_with_special_values(
            max_nudge,
            vec![0x0, 0x1, 0xff, 0x7f, 0xffff, 0x7fff, 0xffff_ffff, 0x7fff_ffff, 0xffff_ffff_ffff_ffff, 0x7fff_ffff_ffff_ffff]
        )
    }
}
impl IntegerGenerator<i8> {
    pub fn new(max_nudge: i8) -> Self {
        Self::new_with_special_values(
            max_nudge,
            vec![0x0, -0x1, 0x7f, -0x80]
        )
    }
}
impl IntegerGenerator<i16> {
    pub fn new(max_nudge: i16) -> Self {
        Self::new_with_special_values(
            max_nudge,
            vec![0x0, -0x1, 0xff, 0x7f, -0x100, -0x80, 0x7fff, -0x8000]
        )
    }
}
impl IntegerGenerator<i32> {
    pub fn new(max_nudge: i32) -> Self {
        Self::new_with_special_values(
            max_nudge,
            vec![0x0, -0x1, 0xff, 0x7f, -0x100, -0x80, 0xffff, 0x7fff, -0x10000, -0x8000, 0x7fff_ffff, -0x8000_0000]
        )
    }
}
impl IntegerGenerator<i64> {
    pub fn new(max_nudge: i64) -> Self {
        Self::new_with_special_values(
            max_nudge,
            vec![0x0, -0x1, 0xff, 0x7f, -0x100, -0x80, 0xffff, 0x7fff, -0x10000, -0x8000, 0xffff_ffff, 0x7fff_ffff, -0x1_0000_0000, -0x8000_0000, 0x7fff_ffff_ffff_ffff, -0x8000_0000_0000_0000]
        )
    }
}
impl IntegerGenerator<i128> {
    pub fn new(max_nudge: i128) -> Self {
        Self::new_with_special_values(
            max_nudge,
            vec![0x0, -0x1, 0xff, 0x7f, -0x100, -0x80, 0xffff, 0x7fff, -0x10000, -0x8000, 0xffff_ffff, 0x7fff_ffff, -0x1_0000_0000, -0x8000_0000, 0xffff_ffff_ffff_ffff, 0x7fff_ffff_ffff_ffff, -0x1_0000_0000_0000_0000, -0x8000_0000_0000_0000, 0x7fff_ffff_ffff_ffff_ffff_ffff_ffff_ffff, -0x8000_0000_0000_0000_0000_0000_0000_0000]
        )
    }
}
impl IntegerGenerator<isize> {
    pub fn new(max_nudge: isize) -> Self {
        Self::new_with_special_values(
            max_nudge,
            vec![0x0, -0x1, 0xff, 0x7f, -0x100, -0x80, 0xffff, 0x7fff, -0x10000, -0x8000, 0xffff_ffff, 0x7fff_ffff, -0x1_0000_0000, -0x8000_0000, 0x7fff_ffff_ffff_ffff, -0x8000_0000_0000_0000]
        )
    }
}
