use crate::mutators::integer::binary_search_arbitrary_u32;
use crate::Mutator;
use std::ops::{Bound, RangeBounds};

const INITIAL_MUTATION_STEP: u64 = 0;

/// Mutator for a `char` within a given range
pub struct CharWithinRangeMutator {
    start_range: u32,
    len_range: u32,
    rng: fastrand::Rng,
    cplx: f64,
}
impl CharWithinRangeMutator {
    #[no_coverage]
    pub fn new<RB: RangeBounds<char>>(range: RB) -> Self {
        let start = match range.start_bound() {
            Bound::Included(b) => *b as u32,
            Bound::Excluded(b) => {
                assert_ne!(*b as u32, <u32>::MAX);
                *b as u32 + 1
            }
            Bound::Unbounded => <u32>::MIN,
        };
        let end = match range.end_bound() {
            Bound::Included(b) => *b as u32,
            Bound::Excluded(b) => {
                assert_ne!(*b as u32, <u32>::MIN);
                (*b as u32) - 1
            }
            Bound::Unbounded => <u32>::MAX,
        };
        if !(start <= end) {
            panic!(
                "You have provided a character range where the value of the start of the range \
                is larger than the end of the range!\nRange start: {:#?}\nRange end: {:#?}",
                range.start_bound(),
                range.end_bound()
            )
        }
        let len_range = end.wrapping_sub(start);
        let cplx = crate::mutators::size_to_cplxity(len_range as usize);
        Self {
            start_range: start,
            len_range: len_range as u32,
            rng: fastrand::Rng::default(),
            cplx,
        }
    }
}

impl Mutator<char> for CharWithinRangeMutator {
    #[doc(hidden)]
    type Cache = ();
    #[doc(hidden)]
    type MutationStep = u64; // mutation step
    #[doc(hidden)]
    type ArbitraryStep = u64;
    #[doc(hidden)]
    type UnmutateToken = char; // old value

    #[doc(hidden)]
    #[no_coverage]
    fn default_arbitrary_step(&self) -> Self::ArbitraryStep {
        0
    }
    #[doc(hidden)]
    #[no_coverage]
    fn validate_value(&self, value: &char) -> Option<Self::Cache> {
        if (self.start_range..=self.start_range + self.len_range).contains(&(*value as u32)) {
            Some(())
        } else {
            None
        }
    }
    #[doc(hidden)]
    #[no_coverage]
    fn default_mutation_step(&self, _value: &char, _cache: &Self::Cache) -> Self::MutationStep {
        INITIAL_MUTATION_STEP
    }
    #[doc(hidden)]
    #[no_coverage]
    fn max_complexity(&self) -> f64 {
        self.cplx
    }
    #[doc(hidden)]
    #[no_coverage]
    fn min_complexity(&self) -> f64 {
        self.cplx
    }
    #[doc(hidden)]
    #[no_coverage]
    fn complexity(&self, _value: &char, _cache: &Self::Cache) -> f64 {
        self.cplx
    }
    #[doc(hidden)]
    #[no_coverage]
    fn ordered_arbitrary(&self, step: &mut Self::ArbitraryStep, max_cplx: f64) -> Option<(char, f64)> {
        if max_cplx < self.min_complexity() {
            return None;
        }
        if *step > self.len_range as u64 {
            None
        } else {
            let result = binary_search_arbitrary_u32(0, self.len_range, *step);
            *step += 1;
            if let Some(c) = char::from_u32(self.start_range.wrapping_add(result)) {
                Some((c, self.cplx))
            } else {
                *step += 1;
                self.ordered_arbitrary(step, max_cplx)
            }
        }
    }

    #[doc(hidden)]
    #[no_coverage]
    fn random_arbitrary(&self, max_cplx: f64) -> (char, f64) {
        let value = self
            .rng
            .u32(self.start_range..=self.start_range.wrapping_add(self.len_range));
        if let Some(value) = char::from_u32(value) {
            (value, self.cplx)
        } else {
            // try again
            self.random_arbitrary(max_cplx)
        }
    }
    #[doc(hidden)]
    #[no_coverage]
    fn ordered_mutate(
        &self,
        value: &mut char,
        cache: &mut Self::Cache,
        step: &mut Self::MutationStep,
        max_cplx: f64,
    ) -> Option<(Self::UnmutateToken, f64)> {
        if max_cplx < self.min_complexity() {
            return None;
        }
        if *step > self.len_range as u64 {
            return None;
        }
        let token = *value;

        let result = binary_search_arbitrary_u32(0, self.len_range, *step);
        if let Some(result) = char::from_u32(self.start_range.wrapping_add(result)) {
            *step += 1;
            if result == *value {
                return self.ordered_mutate(value, cache, step, max_cplx);
            }

            *value = result;

            Some((token, self.cplx))
        } else {
            *step += 1;
            self.ordered_mutate(value, cache, step, max_cplx)
        }
    }
    #[doc(hidden)]
    #[no_coverage]
    fn random_mutate(&self, value: &mut char, _cache: &mut Self::Cache, _max_cplx: f64) -> (Self::UnmutateToken, f64) {
        (
            std::mem::replace(
                value,
                char::from_u32(
                    self.rng
                        .u32(self.start_range..=self.start_range.wrapping_add(self.len_range)),
                )
                .unwrap_or(*value),
            ),
            self.cplx,
        )
    }
    #[doc(hidden)]
    #[no_coverage]
    fn unmutate(&self, value: &mut char, _cache: &mut Self::Cache, t: Self::UnmutateToken) {
        *value = t;
    }

    #[doc(hidden)]
    type LensPath = !;
    #[doc(hidden)]
    #[no_coverage]
    fn lens<'a>(&self, _value: &'a char, _cache: &Self::Cache, _path: &Self::LensPath) -> &'a dyn std::any::Any {
        unreachable!()
    }

    #[doc(hidden)]
    #[no_coverage]
    fn all_paths(&self, _value: &char, _cache: &Self::Cache, _register_path: &mut dyn FnMut(std::any::TypeId, Self::LensPath))
    {
    }

    #[doc(hidden)]
    #[no_coverage]
    fn crossover_mutate(
        &self,
        value: &mut char,
        cache: &mut Self::Cache,
        _subvalue_provider: &dyn crate::SubValueProvider,
        max_cplx: f64,
    ) -> (Self::UnmutateToken, f64) {
        self.random_mutate(value, cache, max_cplx)
    }
}
