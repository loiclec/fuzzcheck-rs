use num::cast::NumCast;
use num::{Bounded, Num, Signed, Unsigned};
use rand::distributions::WeightedIndex;
use rand::distributions::{Distribution, Standard};
use rand::rngs::ThreadRng;
use rand::seq::SliceRandom;
use rand::Rng;
use std::cmp::PartialEq;
use std::mem;
use std::num::Wrapping;
use std::ops::{Add, BitOr, Shl, Shr, Sub};

use crate::input::*;

impl<T: FuzzerInput> InputProperties for IntegerGenerator<T> {
    type Input = T;
    fn complexity(_input: &T) -> f64 {
        mem::size_of::<T>() as f64
    }
}

pub struct IntegerGenerator<T> {
    max_nudge: usize,
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
    T: NumCast + PartialEq + Copy,
    Wrapping<T>: Add<Output = Wrapping<T>> + Sub<Output = Wrapping<T>>,
    Standard: Distribution<T>,
{
    fn new_with_special_values(max_nudge: usize, special_values: Vec<T>) -> Self {
        Self {
            max_nudge,
            special_values,
            mutators: MUTATORS.to_vec(),
            weighted_index: WeightedIndex::new(WEIGHTS.to_vec()).unwrap(),
        }
    }

    fn nudge(&self, input: &mut T, rng: &mut ThreadRng) -> bool {
        let nudge: T = num::cast(rng.gen_range(0, self.max_nudge)).unwrap();
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
    T: Num + FuzzerInput + NumCast + PartialEq + Copy,
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
    T: Num + FuzzerInput + NumCast + PartialEq + Copy,
    Wrapping<T>: Add<Output = Wrapping<T>> + Sub<Output = Wrapping<T>>,
    Standard: Distribution<T>,
{
    fn base_input(&self) -> T {
        T::zero()
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

impl<T> IntegerGenerator<T>
where
    T: Signed + FuzzerInput + NumCast + PartialEq + Copy,
    Wrapping<T>: Add<Output = Wrapping<T>> + Sub<Output = Wrapping<T>>,
    Standard: Distribution<T>,
{
    fn special_values_signed<U, F>(from_bits: F) -> Vec<T>
    where
        U: Unsigned + Bounded + BitOr<Output = U> + Shr<usize, Output = U> + Shl<usize, Output = U> + Copy,
        F: Fn(U) -> T,
    {
        let mut result = vec![T::zero(), T::one()];
        let mut i = 8;
        let bit_width = std::mem::size_of::<T>() * 8;
        while i < bit_width {
            i *= 2;
            let ones = U::max_value();
            let zeros = U::min_value();

            let umax = zeros | (ones >> (bit_width - i));
            let umin = zeros | (ones << i);

            let max = from_bits(umax);
            let lesser_max = max / num::cast(2).unwrap();
            let min = from_bits(umin);
            let lesser_min = min / num::cast(2).unwrap();

            result.push(max);
            result.push(lesser_max);
            result.push(min);
            result.push(lesser_min);
        }
        result
    }
}

impl<T> IntegerGenerator<T>
where
    T: Unsigned + Bounded + BitOr<Output = T> + Shr<usize, Output = T> + FuzzerInput + NumCast + PartialEq + Copy,
    Wrapping<T>: Add<Output = Wrapping<T>> + Sub<Output = Wrapping<T>>,
    Standard: Distribution<T>,
{
    fn special_values_unsigned() -> Vec<T> {
        let mut result = vec![T::zero(), T::one()];
        let mut i = 8;
        let bit_width = std::mem::size_of::<T>() * 8;
        while i < bit_width {
            i *= 2;
            let ones = T::max_value();
            let zeros = T::min_value();

            let umax = zeros | (ones >> (bit_width - i));
            let umax_lesser = umax / num::cast(2).unwrap();

            result.push(umax);
            result.push(umax_lesser);
        }
        result
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
    pub fn new(max_nudge: usize) -> Self {
        Self::new_with_special_values(
            max_nudge,
            Self::special_values_unsigned()
        )
    }
}
impl IntegerGenerator<u16> {
    pub fn new(max_nudge: usize) -> Self {
        Self::new_with_special_values(
            max_nudge,
            Self::special_values_unsigned()
        )
    }
}
impl IntegerGenerator<u32> {
    pub fn new(max_nudge: usize) -> Self {
        Self::new_with_special_values(
            max_nudge,
            Self::special_values_unsigned()
        )
    }
}
impl IntegerGenerator<u64> {
    pub fn new(max_nudge: usize) -> Self {
        Self::new_with_special_values(
            max_nudge,
            Self::special_values_unsigned()
        )
    }
}
impl IntegerGenerator<u128> {
    pub fn new(max_nudge: usize) -> Self {
        Self::new_with_special_values(
            max_nudge,
            Self::special_values_unsigned()
        )
    }
}
impl IntegerGenerator<usize> {
    pub fn new(max_nudge: usize) -> Self {
        Self::new_with_special_values(
            max_nudge,
            Self::special_values_unsigned()
        )
    }
}
impl IntegerGenerator<i8> {
    pub fn new(max_nudge: usize) -> Self {
        Self::new_with_special_values(
            max_nudge,
            Self::special_values_signed(|x: u8| x as i8)
        )
    }
}
impl IntegerGenerator<i16> {
    pub fn new(max_nudge: usize) -> Self {
        Self::new_with_special_values(
            max_nudge,
            Self::special_values_signed(|x: u16| x as i16)
        )
    }
}
impl IntegerGenerator<i32> {
    pub fn new(max_nudge: usize) -> Self {
        Self::new_with_special_values(
            max_nudge,
            Self::special_values_signed(|x: u32| x as i32)
        )
    }
}
impl IntegerGenerator<i64> {
    pub fn new(max_nudge: usize) -> Self {
        Self::new_with_special_values(
            max_nudge,
            Self::special_values_signed(|x: u64| x as i64)
        )
    }
}
impl IntegerGenerator<i128> {
    pub fn new(max_nudge: usize) -> Self {
        Self::new_with_special_values(
            max_nudge,
            Self::special_values_signed(|x: u128| x as i128)
        )
    }
}
impl IntegerGenerator<isize> {
    pub fn new(max_nudge: usize) -> Self {
        Self::new_with_special_values(
            max_nudge,
            Self::special_values_signed(|x: usize| x as isize)
        )
    }
}
