use crate::fuzzcheck_traits::Mutator;
use crate::integer::binary_search_arbitrary_u32;
use std::ops::{Bound, RangeBounds};

const INITIAL_MUTATION_STEP: u64 = 0;

pub struct CharWithinRangeMutator {
    start_range: u32,
    len_range: u32,
    rng: fastrand::Rng,
    cplx: f64,
}
impl CharWithinRangeMutator {
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
        assert!(start <= end);
        let len_range = end.wrapping_sub(start);
        let cplx = 1.0 + crate::size_to_cplxity(len_range as usize);
        Self {
            start_range: start,
            len_range: len_range as u32,
            rng: fastrand::Rng::default(),
            cplx,
        }
    }
}

impl Mutator<char> for CharWithinRangeMutator {
    type Cache = ();
    type MutationStep = u64; // mutation step
    type ArbitraryStep = u64;
    type UnmutateToken = char; // old value

    fn default_arbitrary_step(&self) -> Self::ArbitraryStep {
        0
    }

    fn validate_value(&self, value: &char) -> Option<(Self::Cache, Self::MutationStep)> {
        if (self.start_range..=self.start_range + self.len_range).contains(&(*value as u32)) {
            Some(((), INITIAL_MUTATION_STEP))
        } else {
            None
        }
    }

    fn max_complexity(&self) -> f64 {
        self.cplx
    }

    fn min_complexity(&self) -> f64 {
        self.cplx
    }

    fn complexity(&self, _value: &char, _cache: &Self::Cache) -> f64 {
        self.cplx
    }

    fn ordered_arbitrary(&self, step: &mut Self::ArbitraryStep, max_cplx: f64) -> Option<(char, f64)> {
        if max_cplx < self.min_complexity() {
            return None;
        }
        if *step > self.len_range as u64 {
            None
        } else {
            let result = binary_search_arbitrary_u32(0, self.len_range, *step);
            *step = step.wrapping_add(1);
            let c = char::from_u32(self.start_range.wrapping_add(result)).unwrap();
            Some((c, self.cplx))
        }
    }

    fn random_arbitrary(&self, _max_cplx: f64) -> (char, f64) {
        let value = self
            .rng
            .u32(self.start_range..=self.start_range.wrapping_add(self.len_range));
        let value = char::from_u32(value).unwrap();
        (value, self.cplx)
    }

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
        let result = char::from_u32(self.start_range.wrapping_add(result)).unwrap();
        *step = step.wrapping_add(1);
        if result == *value {
            return self.ordered_mutate(value, cache, step, max_cplx);
        }

        *value = result;

        Some((token, self.cplx))
    }

    fn random_mutate(&self, value: &mut char, _cache: &mut Self::Cache, _max_cplx: f64) -> (Self::UnmutateToken, f64) {
        (
            std::mem::replace(
                value,
                char::from_u32(
                    self.rng
                        .u32(self.start_range..=self.start_range.wrapping_add(self.len_range)),
                )
                .unwrap(),
            ),
            self.cplx,
        )
    }

    fn unmutate(&self, value: &mut char, _cache: &mut Self::Cache, t: Self::UnmutateToken) {
        *value = t;
    }
}
