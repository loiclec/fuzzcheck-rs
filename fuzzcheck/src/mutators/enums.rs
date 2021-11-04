use crate::Mutator;

/// Trait used by the [DefaultMutator derive macro](fuzzcheck_mutators_derive::DefaultMutator)
/// for enums without associated data
pub trait BasicEnumStructure {
    fn from_item_index(item_index: usize) -> Self;
    fn get_item_index(&self) -> usize;
}

extern crate self as fuzzcheck;

/// A mutator used for enums implementing [BasicEnumStructure]
pub struct BasicEnumMutator {
    rng: fastrand::Rng,
    cplx: f64,
}
impl BasicEnumMutator {
    #[no_coverage]
    pub fn new<T>() -> Self {
        Self {
            rng: <_>::default(),
            cplx: crate::mutators::size_to_cplxity(std::mem::variant_count::<T>()),
        }
    }
}

const INITIAL_MUTATION_STEP: usize = 1;

impl<T> Mutator<T> for BasicEnumMutator
where
    T: Clone + BasicEnumStructure,
{
    type Cache = ();
    type MutationStep = usize;
    type ArbitraryStep = usize;
    type UnmutateToken = usize;

    #[no_coverage]
    fn default_arbitrary_step(&self) -> Self::ArbitraryStep {
        0
    }

    #[no_coverage]
    fn validate_value(&self, _value: &T) -> Option<(Self::Cache, Self::MutationStep)> {
        Some(((), INITIAL_MUTATION_STEP))
    }

    #[no_coverage]
    fn max_complexity(&self) -> f64 {
        self.cplx
    }

    #[no_coverage]
    fn min_complexity(&self) -> f64 {
        self.cplx
    }

    #[no_coverage]
    fn complexity(&self, _value: &T, _cache: &Self::Cache) -> f64 {
        self.cplx
    }

    #[no_coverage]
    fn ordered_arbitrary(&self, step: &mut Self::ArbitraryStep, max_cplx: f64) -> Option<(T, f64)> {
        if max_cplx < <Self as Mutator<T>>::min_complexity(self) {
            return None;
        }
        if *step < std::mem::variant_count::<T>() {
            let old_step = *step;
            *step += 1;
            Some((T::from_item_index(old_step), self.cplx))
        } else {
            None
        }
    }

    #[no_coverage]
    fn random_arbitrary(&self, _max_cplx: f64) -> (T, f64) {
        let item_idx = self.rng.usize(..std::mem::variant_count::<T>());
        (T::from_item_index(item_idx), self.cplx)
    }

    #[no_coverage]
    fn ordered_mutate(
        &self,
        value: &mut T,
        _cache: &mut Self::Cache,
        step: &mut Self::MutationStep,
        max_cplx: f64,
    ) -> Option<(Self::UnmutateToken, f64)> {
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
            Some((old_index, self.cplx))
        } else {
            None
        }
    }

    #[no_coverage]
    fn random_mutate(&self, value: &mut T, _cache: &mut Self::Cache, _max_cplx: f64) -> (Self::UnmutateToken, f64) {
        let old_index = value.get_item_index();
        let item_idx = self.rng.usize(..std::mem::variant_count::<T>());
        *value = T::from_item_index(item_idx);
        (old_index, self.cplx)
    }

    #[no_coverage]
    fn unmutate(&self, value: &mut T, _cache: &mut Self::Cache, t: Self::UnmutateToken) {
        *value = T::from_item_index(t);
    }
}
