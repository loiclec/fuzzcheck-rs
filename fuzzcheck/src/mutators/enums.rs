use std::any::Any;

use crate::Mutator;

/// Trait used by the [DefaultMutator derive macro](fuzzcheck_mutators_derive::DefaultMutator)
/// for enums without associated data
pub trait BasicEnumStructure {
    fn from_variant_index(item_index: usize) -> Self;
    fn get_variant_index(&self) -> usize;
}

/// A mutator used for enums implementing [BasicEnumStructure]
pub struct BasicEnumMutator {
    non_ignored_variant_count: usize,
    rng: fastrand::Rng,
    cplx: f64,
}
impl BasicEnumMutator {
    #[coverage(off)]
    pub fn new<T>(non_ignored_variant_count: usize) -> Self
    where
        T: BasicEnumStructure,
    {
        Self {
            non_ignored_variant_count,
            rng: <_>::default(),
            cplx: crate::mutators::size_to_cplxity(non_ignored_variant_count),
        }
    }
}

const INITIAL_MUTATION_STEP: usize = 1;

impl<T> Mutator<T> for BasicEnumMutator
where
    T: Clone + BasicEnumStructure + 'static,
{
    #[doc(hidden)]
    type Cache = ();
    #[doc(hidden)]
    type MutationStep = usize;
    #[doc(hidden)]
    type ArbitraryStep = usize;
    #[doc(hidden)]
    type UnmutateToken = usize;

    #[doc(hidden)]
    #[coverage(off)]
    fn initialize(&self) {}

    #[doc(hidden)]
    #[coverage(off)]
    fn default_arbitrary_step(&self) -> Self::ArbitraryStep {
        0
    }

    #[doc(hidden)]
    #[coverage(off)]
    fn is_valid(&self, _value: &T) -> bool {
        true
    }

    #[doc(hidden)]
    #[coverage(off)]
    fn validate_value(&self, _value: &T) -> Option<Self::Cache> {
        Some(())
    }

    #[doc(hidden)]
    #[coverage(off)]
    fn default_mutation_step(&self, _value: &T, _cache: &Self::Cache) -> Self::MutationStep {
        INITIAL_MUTATION_STEP
    }

    #[doc(hidden)]
    #[coverage(off)]
    fn global_search_space_complexity(&self) -> f64 {
        self.cplx
    }

    #[doc(hidden)]
    #[coverage(off)]
    fn max_complexity(&self) -> f64 {
        self.cplx
    }

    #[doc(hidden)]
    #[coverage(off)]
    fn min_complexity(&self) -> f64 {
        self.cplx
    }

    #[doc(hidden)]
    #[coverage(off)]
    fn complexity(&self, _value: &T, _cache: &Self::Cache) -> f64 {
        self.cplx
    }

    #[doc(hidden)]
    #[coverage(off)]
    fn ordered_arbitrary(&self, step: &mut Self::ArbitraryStep, max_cplx: f64) -> Option<(T, f64)> {
        if max_cplx < <Self as Mutator<T>>::min_complexity(self) {
            return None;
        }
        if *step < self.non_ignored_variant_count {
            let old_step = *step;
            *step += 1;
            Some((T::from_variant_index(old_step), self.cplx))
        } else {
            None
        }
    }

    #[doc(hidden)]
    #[coverage(off)]
    fn random_arbitrary(&self, _max_cplx: f64) -> (T, f64) {
        let item_idx = self.rng.usize(..self.non_ignored_variant_count);
        (T::from_variant_index(item_idx), self.cplx)
    }

    #[doc(hidden)]
    #[coverage(off)]
    fn ordered_mutate(
        &self,
        value: &mut T,
        _cache: &mut Self::Cache,
        step: &mut Self::MutationStep,
        _subvalue_provider: &dyn crate::SubValueProvider,
        max_cplx: f64,
    ) -> Option<(Self::UnmutateToken, f64)> {
        if max_cplx < <Self as Mutator<T>>::min_complexity(self) {
            return None;
        }
        // starts at step = 1
        // create new from (get_item_index + step) % nbr_of_items
        if *step < self.non_ignored_variant_count {
            let old_index = value.get_variant_index();
            let old_step = *step;
            *step += 1;
            *value = T::from_variant_index((old_index + old_step) % self.non_ignored_variant_count);
            Some((old_index, self.cplx))
        } else {
            None
        }
    }

    #[doc(hidden)]
    #[coverage(off)]
    fn random_mutate(&self, value: &mut T, _cache: &mut Self::Cache, _max_cplx: f64) -> (Self::UnmutateToken, f64) {
        let old_index = value.get_variant_index();
        let item_idx = self.rng.usize(..self.non_ignored_variant_count);
        *value = T::from_variant_index(item_idx);
        (old_index, self.cplx)
    }

    #[doc(hidden)]
    #[coverage(off)]
    fn unmutate(&self, value: &mut T, _cache: &mut Self::Cache, t: Self::UnmutateToken) {
        *value = T::from_variant_index(t);
    }

    #[doc(hidden)]
    #[coverage(off)]
    fn visit_subvalues<'a>(&self, _value: &'a T, _cache: &'a Self::Cache, _visit: &mut dyn FnMut(&'a dyn Any, f64)) {}
}
