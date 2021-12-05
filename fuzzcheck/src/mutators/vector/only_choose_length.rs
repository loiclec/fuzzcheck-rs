//! This mutation is chosen when the element mutator is a “unit” mutator,
//! meaning that it can only produce a single value. In this case, the
//! vector mutator’s role is simply to choose a length.
//!
//! For example, if we have:
//! ```
//! use fuzzcheck::{Mutator, DefaultMutator};
//! use fuzzcheck::mutators::vector::VecMutator;
//!
//! let m /* : impl Mutator<Vec<()>> */ = VecMutator::new(<()>::default_mutator(), 2..=5);
//! ```
//! Then the values that `m` can produce are only:
//! ```txt
//! [(), ()]
//! [(), (), ()]
//! [(), (), (), ()]
//! [(), (), (), (), ()]
//! ```
//! and nothing else.
//!
//! We can detect if the element mutator is a unit mutator by calling `m.max_complexity()`.
//! If the maximum complexity is `0.0`, then the mutator is only capable of producing a
//! single value.

use super::VecMutator;
use crate::mutators::mutations::{Mutation, RevertMutation};
use crate::Mutator;

pub struct OnlyChooseLength;

#[derive(Clone)]
pub struct OnlyChooseLengthStep {
    length: usize,
}
#[derive(Clone)]
pub struct OnlyChooseLengthRandomStep;

pub struct ConcreteOnlyChooseLength {
    length: usize,
}
pub struct RevertOnlyChooseLength<T> {
    replace_by: Vec<T>,
}

impl<T, M> RevertMutation<Vec<T>, VecMutator<T, M>> for RevertOnlyChooseLength<T>
where
    T: Clone + 'static,
    M: Mutator<T>,
{
    fn revert(
        mut self,
        _mutator: &VecMutator<T, M>,
        value: &mut Vec<T>,
        _cache: &mut <VecMutator<T, M> as Mutator<Vec<T>>>::Cache,
    ) {
        let _ = std::mem::swap(value, &mut self.replace_by);
    }
}

impl<T, M> Mutation<Vec<T>, VecMutator<T, M>> for OnlyChooseLength
where
    T: Clone + 'static,
    M: Mutator<T>,
{
    type RandomStep = OnlyChooseLengthRandomStep;
    type Step = OnlyChooseLengthStep;
    type Concrete<'a> = ConcreteOnlyChooseLength;
    type Revert = RevertOnlyChooseLength<T>;

    fn default_random_step(&self, mutator: &VecMutator<T, M>, _value: &Vec<T>) -> Option<Self::RandomStep> {
        if mutator.m.max_complexity() <= 0.0 {
            Some(OnlyChooseLengthRandomStep)
        } else {
            None
        }
    }

    fn random<'a>(
        mutator: &VecMutator<T, M>,
        _value: &Vec<T>,
        _cache: &<VecMutator<T, M> as Mutator<Vec<T>>>::Cache,
        _random_step: &Self::RandomStep,
        max_cplx: f64,
    ) -> Self::Concrete<'a> {
        let upperbound = std::cmp::max(*mutator.len_range.start(), max_cplx as usize);
        ConcreteOnlyChooseLength {
            length: mutator.rng.usize(*mutator.len_range.start()..=upperbound),
        }
    }

    fn default_step(
        &self,
        mutator: &VecMutator<T, M>,
        _value: &Vec<T>,
        _cache: &<VecMutator<T, M> as Mutator<Vec<T>>>::Cache,
    ) -> Option<Self::Step> {
        if mutator.m.max_complexity() <= 0.0 {
            Some(OnlyChooseLengthStep {
                length: *mutator.len_range.start(),
            })
        } else {
            None
        }
    }

    fn from_step<'a>(
        mutator: &VecMutator<T, M>,
        _value: &Vec<T>,
        _cache: &<VecMutator<T, M> as Mutator<Vec<T>>>::Cache,
        step: &'a mut Self::Step,
        max_cplx: f64,
    ) -> Option<Self::Concrete<'a>> {
        if step.length <= *mutator.len_range.end() && mutator.complexity_from_inner(0.0, step.length) < max_cplx {
            let x = ConcreteOnlyChooseLength { length: step.length };
            step.length += 1;
            Some(x)
        } else {
            None
        }
    }

    fn apply<'a>(
        mutation: Self::Concrete<'a>,
        mutator: &VecMutator<T, M>,
        value: &mut Vec<T>,
        _cache: &mut <VecMutator<T, M> as Mutator<Vec<T>>>::Cache,
        _max_cplx: f64,
    ) -> (Self::Revert, f64) {
        let (el, _) = mutator.m.random_arbitrary(0.0);
        let mut value_2 = std::iter::repeat(el).take(mutation.length).collect();
        std::mem::swap(value, &mut value_2);
        let cplx = mutator.complexity_from_inner(0.0, mutation.length);
        (RevertOnlyChooseLength { replace_by: value_2 }, cplx)
    }
}
