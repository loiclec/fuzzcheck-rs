use rand::distributions::uniform::SampleUniform;
use rand::distributions::WeightedIndex;
use rand::distributions::{Distribution, Standard};
use rand::rngs::ThreadRng;
use rand::seq::SliceRandom;
use rand::Rng;

use std::cmp::PartialEq;
use std::hash::Hash;
use std::hash::Hasher;
use std::mem;
use std::num::Wrapping;
use std::ops::{Add, Sub};

use miniserde::{json, Deserialize, Serialize};

extern crate fuzzcheck;
use fuzzcheck::input::*;

// Let's be honest, everything in this file is guesswork

pub struct IntegerGenerator<T> {
    max_nudge: T,
    special_values: Vec<T>,
    rng: ThreadRng,
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
    T: Default + Hash + Clone + PartialEq + Copy + SampleUniform + Serialize + Deserialize,
    Wrapping<T>: Add<Output = Wrapping<T>> + Sub<Output = Wrapping<T>>,
    Standard: Distribution<T>,
{
    fn new_with_special_values(max_nudge: T, special_values: Vec<T>) -> Self {
        Self {
            max_nudge,
            special_values,
            rng: rand::thread_rng(),
            weighted_index: WeightedIndex::new(WEIGHTS.to_vec()).unwrap(),
        }
    }

    fn nudge(&mut self, input: &mut T) -> bool {
        let nudge: T = self.rng.gen_range(<T as Default>::default(), self.max_nudge);
        let plus = self.rng.gen::<bool>();
        if plus {
            *input = (Wrapping(*input) + Wrapping(nudge)).0;
        } else {
            *input = (Wrapping(*input) - Wrapping(nudge)).0;
        }
        true
    }
    fn random(&mut self, input: &mut T) -> bool {
        *input = self.rng.gen();
        true
    }
    fn special(&mut self, input: &mut T) -> bool {
        let old = *input;
        *input = *self.special_values.choose(&mut self.rng).unwrap();
        old != *input
    }
}

impl<T> IntegerGenerator<T>
where
    T: Default + Hash + Clone + PartialEq + Copy + SampleUniform + Serialize + Deserialize,
    Wrapping<T>: Add<Output = Wrapping<T>> + Sub<Output = Wrapping<T>>,
    Standard: Distribution<T>,
{
    fn mutate_with(&mut self, input: &mut T, mutator: IntegerMutatorKind, _spare_cplx: f64) -> bool {
        match mutator {
            IntegerMutatorKind::Special => self.special(input),
            IntegerMutatorKind::Random => self.random(input),
            IntegerMutatorKind::Nudge => self.nudge(input),
        }
    }
}

impl<T> InputGenerator for IntegerGenerator<T>
where
    T: Default + Hash + Clone + PartialEq + Copy + SampleUniform + Serialize + Deserialize,
    Wrapping<T>: Add<Output = Wrapping<T>> + Sub<Output = Wrapping<T>>,
    Standard: Distribution<T>,
{
    type Input = T;

    fn hash<H>(input: &Self::Input, state: &mut H)
    where
        H: Hasher,
    {
        input.hash(state);
    }

    fn complexity(_input: &T) -> f64 {
        mem::size_of::<T>() as f64
    }

    fn base_input() -> T {
        <T as Default>::default()
    }

    fn new_input(&mut self, _max_cplx: f64) -> T {
        self.rng.gen()
    }

    fn mutate(&mut self, input: &mut Self::Input, spare_cplx: f64) -> bool {
        for _ in 0..MUTATORS.len() {
            let pick = self.weighted_index.sample(&mut self.rng);
            if self.mutate_with(input, MUTATORS[pick], spare_cplx) {
                return true;
            }
        }
        false
    }

    fn from_data(data: &[u8]) -> Option<Self::Input> {
        if let Ok(s) = std::str::from_utf8(data) {
            json::from_str(s).ok()
        } else {
            None
        }
    }
    fn to_data(input: &Self::Input) -> Vec<u8> {
        json::to_string(input).into_bytes()
    }
}

impl Default for IntegerGenerator<u8> {
    fn default() -> Self {
        Self::new_with_special_values(10, vec![0x0, 0x1, 0xff, 0x7f])
    }
}
impl Default for IntegerGenerator<u16> {
    fn default() -> Self {
        Self::new_with_special_values(10, vec![0x0, 0x1, 0xff, 0x7f, 0xffff, 0x7fff])
    }
}
impl Default for IntegerGenerator<u32> {
    fn default() -> Self {
        Self::new_with_special_values(10, vec![0x0, 0x1, 0xff, 0x7f, 0xffff, 0x7fff, 0xffff_ffff, 0x7fff_ffff])
    }
}
impl Default for IntegerGenerator<u64> {
    fn default() -> Self {
        Self::new_with_special_values(
            10,
            vec![
                0x0,
                0x1,
                0xff,
                0x7f,
                0xffff,
                0x7fff,
                0xffff_ffff,
                0x7fff_ffff,
                0xffff_ffff_ffff_ffff,
                0x7fff_ffff_ffff_ffff,
            ],
        )
    }
}

impl Default for IntegerGenerator<usize> {
    fn default() -> Self {
        Self::new_with_special_values(
            10,
            vec![
                0x0,
                0x1,
                0xff,
                0x7f,
                0xffff,
                0x7fff,
                0xffff_ffff,
                0x7fff_ffff,
                0xffff_ffff_ffff_ffff,
                0x7fff_ffff_ffff_ffff,
            ],
        )
    }
}
impl Default for IntegerGenerator<i8> {
    fn default() -> Self {
        Self::new_with_special_values(10, vec![0x0, -0x1, 0x7f, -0x80])
    }
}
impl Default for IntegerGenerator<i16> {
    fn default() -> Self {
        Self::new_with_special_values(10, vec![0x0, -0x1, 0xff, 0x7f, -0x100, -0x80, 0x7fff, -0x8000])
    }
}
impl Default for IntegerGenerator<i32> {
    fn default() -> Self {
        Self::new_with_special_values(
            10,
            vec![
                0x0,
                -0x1,
                0xff,
                0x7f,
                -0x100,
                -0x80,
                0xffff,
                0x7fff,
                -0x10000,
                -0x8000,
                0x7fff_ffff,
                -0x8000_0000,
            ],
        )
    }
}
impl Default for IntegerGenerator<i64> {
    fn default() -> Self {
        Self::new_with_special_values(
            10,
            vec![
                0x0,
                -0x1,
                0xff,
                0x7f,
                -0x100,
                -0x80,
                0xffff,
                0x7fff,
                -0x10000,
                -0x8000,
                0xffff_ffff,
                0x7fff_ffff,
                -0x1_0000_0000,
                -0x8000_0000,
                0x7fff_ffff_ffff_ffff,
                -0x8000_0000_0000_0000,
            ],
        )
    }
}
impl Default for IntegerGenerator<isize> {
    fn default() -> Self {
        Self::new_with_special_values(
            10,
            vec![
                0x0,
                -0x1,
                0xff,
                0x7f,
                -0x100,
                -0x80,
                0xffff,
                0x7fff,
                -0x10000,
                -0x8000,
                0xffff_ffff,
                0x7fff_ffff,
                -0x1_0000_0000,
                -0x8000_0000,
                0x7fff_ffff_ffff_ffff,
                -0x8000_0000_0000_0000,
            ],
        )
    }
}
