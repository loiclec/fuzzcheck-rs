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
//! We can detect if the element mutator is a unit mutator by calling
//! `m.global_search_space_complexity()`. If the complexity is `0.0`, then the
//! mutator is only capable of producing a single value.

use super::VecMutator;
use crate::mutators::mutations::{Mutation, RevertMutation};
use crate::{Mutator, SubValueProvider};

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
    #[coverage(off)]
    fn revert(
        mut self,
        _mutator: &VecMutator<T, M>,
        value: &mut Vec<T>,
        _cache: &mut <VecMutator<T, M> as Mutator<Vec<T>>>::Cache,
    ) {
        std::mem::swap(value, &mut self.replace_by);
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
    #[coverage(off)]
    fn default_random_step(&self, mutator: &VecMutator<T, M>, _value: &Vec<T>) -> Option<Self::RandomStep> {
        if mutator.m.global_search_space_complexity() <= 0.0 {
            Some(OnlyChooseLengthRandomStep)
        } else {
            None
        }
    }
    #[coverage(off)]
    fn random<'a>(
        mutator: &VecMutator<T, M>,
        _value: &Vec<T>,
        _cache: &<VecMutator<T, M> as Mutator<Vec<T>>>::Cache,
        _random_step: &Self::RandomStep,
        max_cplx: f64,
    ) -> Self::Concrete<'a> {
        let cplx_element = mutator.m.min_complexity();
        assert_eq!(cplx_element, mutator.m.max_complexity(), "A mutator of type {:?} has a global_search_space_complexity of 0.0 (indicating that it can produce only one value), but its min_complexity() is different than its max_complexity(), which is a contradiction.", std::any::type_name::<M>());

        let cplx_element = if mutator.inherent_complexity {
            // then each element adds an additional 1.0 of complexity
            1.0 + cplx_element
        } else {
            cplx_element
        };

        let upperbound = std::cmp::max(
            std::cmp::min(*mutator.len_range.end(), ((max_cplx - 1.0) / cplx_element) as usize),
            *mutator.len_range.start(),
        );
        ConcreteOnlyChooseLength {
            length: mutator.rng.usize(*mutator.len_range.start()..=upperbound),
        }
    }
    #[coverage(off)]
    fn default_step(
        &self,
        mutator: &VecMutator<T, M>,
        _value: &Vec<T>,
        _cache: &<VecMutator<T, M> as Mutator<Vec<T>>>::Cache,
    ) -> Option<Self::Step> {
        if mutator.m.global_search_space_complexity() <= 0.0 {
            Some(OnlyChooseLengthStep {
                length: *mutator.len_range.start(),
            })
        } else {
            None
        }
    }
    #[coverage(off)]
    fn from_step<'a>(
        mutator: &VecMutator<T, M>,
        _value: &Vec<T>,
        _cache: &<VecMutator<T, M> as Mutator<Vec<T>>>::Cache,
        step: &'a mut Self::Step,
        _subvalue_provider: &dyn SubValueProvider,
        max_cplx: f64,
    ) -> Option<Self::Concrete<'a>> {
        let cplx_element = mutator.m.min_complexity();
        if step.length <= *mutator.len_range.end()
            && mutator.complexity_from_inner(cplx_element * step.length as f64, step.length) < max_cplx
        {
            let x = ConcreteOnlyChooseLength { length: step.length };
            step.length += 1;
            Some(x)
        } else {
            None
        }
    }
    #[coverage(off)]
    fn apply<'a>(
        mutation: Self::Concrete<'a>,
        mutator: &VecMutator<T, M>,
        value: &mut Vec<T>,
        _cache: &mut <VecMutator<T, M> as Mutator<Vec<T>>>::Cache,
        _subvalue_provider: &dyn SubValueProvider,
        _max_cplx: f64,
    ) -> (Self::Revert, f64) {
        let (el, el_cplx) = mutator.m.random_arbitrary(0.0);
        let mut value_2 = std::iter::repeat(el).take(mutation.length).collect();
        std::mem::swap(value, &mut value_2);
        let cplx = mutator.complexity_from_inner(el_cplx * mutation.length as f64, mutation.length);
        (RevertOnlyChooseLength { replace_by: value_2 }, cplx)
    }
}
