use std::borrow::BorrowMut;
use std::cmp::Ordering;

use fuzzcheck_traits::Mutator;

pub trait BasicEnumStructure {
    fn from_item_index(item_index: usize) -> Self;
    fn get_item_index(&self) -> usize;
}

#[derive(Default)]
pub struct BasicEnumMutator {
    rng: fastrand::Rng,
}

impl<T> Mutator<T> for BasicEnumMutator
where
    T: Clone + BasicEnumStructure,
{
    type Cache = ();
    type MutationStep = usize;
    type ArbitraryStep = usize;
    type UnmutateToken = usize;

    fn cache_from_value(&self, _value: &T) -> Self::Cache {}

    fn initial_step_from_value(&self, _value: &T) -> Self::MutationStep {
        1
    }

    fn max_complexity(&self) -> f64 {
        crate::size_to_cplxity(std::mem::variant_count::<T>())
    }

    fn min_complexity(&self) -> f64 {
        crate::size_to_cplxity(std::mem::variant_count::<T>())
    }

    fn complexity(&self, _value: &T, _cache: &Self::Cache) -> f64 {
        crate::size_to_cplxity(std::mem::variant_count::<T>())
    }

    fn ordered_arbitrary(&self, step: &mut Self::ArbitraryStep, max_cplx: f64) -> Option<(T, Self::Cache)> {
        if max_cplx < <Self as Mutator<T>>::min_complexity(self) {
            return None;
        }
        if *step < std::mem::variant_count::<T>() {
            let old_step = *step;
            *step += 1;
            Some((T::from_item_index(old_step), ()))
        } else {
            None
        }
    }

    fn random_arbitrary(&self, _max_cplx: f64) -> (T, Self::Cache) {
        let item_idx = self.rng.usize(..std::mem::variant_count::<T>());
        (T::from_item_index(item_idx), ())
    }

    fn ordered_mutate(
        &self,
        value: &mut T,
        _cache: &mut Self::Cache,
        step: &mut Self::MutationStep,
        max_cplx: f64,
    ) -> Option<Self::UnmutateToken> {
        if max_cplx < <Self as Mutator<T>>::min_complexity(self) {
            return None;
        }
        // starts at step = 1
        // create new from (get_item_index + step) % nbr_of_items
        if *step < std::mem::variant_count::<T>() {
            let old_index = value.get_item_index();
            let old_step = *step;
            *step += 1;
            *value = T::from_item_index((old_index + old_step) % std::mem::variant_count::<T>());
            Some(old_index)
        } else {
            None
        }
    }

    fn random_mutate(&self, value: &mut T, _cache: &mut Self::Cache, _max_cplx: f64) -> Self::UnmutateToken {
        let old_index = value.get_item_index();
        let item_idx = self.rng.usize(..std::mem::variant_count::<T>());
        *value = T::from_item_index(item_idx);
        old_index
    }

    fn unmutate(&self, value: &mut T, _cache: &mut Self::Cache, t: Self::UnmutateToken) {
        *value = T::from_item_index(t);
    }
}

extern crate self as fuzzcheck_mutators;

fuzzcheck_mutators_derive::make_basic_enum_mutators!(1);
fuzzcheck_mutators_derive::make_basic_enum_mutators!(2);
fuzzcheck_mutators_derive::make_basic_enum_mutators!(3);
fuzzcheck_mutators_derive::make_basic_enum_mutators!(4);
fuzzcheck_mutators_derive::make_basic_enum_mutators!(5);
fuzzcheck_mutators_derive::make_basic_enum_mutators!(6);
fuzzcheck_mutators_derive::make_basic_enum_mutators!(7);
fuzzcheck_mutators_derive::make_basic_enum_mutators!(8);
fuzzcheck_mutators_derive::make_basic_enum_mutators!(9);
fuzzcheck_mutators_derive::make_basic_enum_mutators!(10);
