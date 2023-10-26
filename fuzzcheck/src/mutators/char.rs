use std::any::Any;
use std::ops::{Bound, RangeBounds};

use crate::mutators::integer::binary_search_arbitrary_u32;
use crate::{DefaultMutator, Mutator, MutatorExt};

const INITIAL_MUTATION_STEP: u64 = 0;

// quickly written but inefficient implementation of a general mutator for char
//
// does not lean towards any particular char. Use CharWithinRangeMutator or CharacterMutator
// for more focused mutators
impl DefaultMutator for char {
    type Mutator = impl Mutator<char>;

    #[coverage(off)]
    fn default_mutator() -> Self::Mutator {
        u32::default_mutator()
            .filter(
                #[coverage(off)]
                |x| char::from_u32(*x).is_some(),
            )
            .map(
                #[coverage(off)]
                |x| char::from_u32(*x).unwrap(),
                |c| Some(*c as u32),
            )
    }
}

/// Mutator for a `char` within a given range
pub struct CharWithinRangeMutator {
    start_range: u32,
    len_range: u32,
    rng: fastrand::Rng,
    search_space_complexity: f64,
}
impl CharWithinRangeMutator {
    #[coverage(off)]
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
        let search_space_complexity = crate::mutators::size_to_cplxity(len_range as usize);
        Self {
            start_range: start,
            len_range: len_range as u32,
            rng: fastrand::Rng::default(),
            search_space_complexity,
        }
    }
}

impl Mutator<char> for CharWithinRangeMutator {
    #[doc(hidden)]
    type Cache = f64; // complexity of the character
    #[doc(hidden)]
    type MutationStep = u64; // mutation step
    #[doc(hidden)]
    type ArbitraryStep = u64;
    #[doc(hidden)]
    type UnmutateToken = char; // old value

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
    fn is_valid(&self, value: &char) -> bool {
        (self.start_range..=self.start_range + self.len_range).contains(&(*value as u32))
    }

    #[doc(hidden)]
    #[coverage(off)]
    fn validate_value(&self, value: &char) -> Option<Self::Cache> {
        if (self.start_range..=self.start_range + self.len_range).contains(&(*value as u32)) {
            Some((value.len_utf8() * 8) as f64)
        } else {
            None
        }
    }
    #[doc(hidden)]
    #[coverage(off)]
    fn default_mutation_step(&self, _value: &char, _cache: &Self::Cache) -> Self::MutationStep {
        INITIAL_MUTATION_STEP
    }

    #[doc(hidden)]
    #[coverage(off)]
    fn global_search_space_complexity(&self) -> f64 {
        self.search_space_complexity
    }

    #[doc(hidden)]
    #[coverage(off)]
    fn max_complexity(&self) -> f64 {
        32.0
    }
    #[doc(hidden)]
    #[coverage(off)]
    fn min_complexity(&self) -> f64 {
        8.0
    }
    #[doc(hidden)]
    #[coverage(off)]
    fn complexity(&self, _value: &char, cache: &Self::Cache) -> f64 {
        *cache
    }
    #[doc(hidden)]
    #[coverage(off)]
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
                Some((c, (c.len_utf8() * 8) as f64))
            } else {
                *step += 1;
                self.ordered_arbitrary(step, max_cplx)
            }
        }
    }

    #[doc(hidden)]
    #[coverage(off)]
    fn random_arbitrary(&self, max_cplx: f64) -> (char, f64) {
        let value = self
            .rng
            .u32(self.start_range..=self.start_range.wrapping_add(self.len_range));
        if let Some(value) = char::from_u32(value) {
            (value, (value.len_utf8() * 8) as f64)
        } else {
            // try again
            self.random_arbitrary(max_cplx)
        }
    }
    #[doc(hidden)]
    #[coverage(off)]
    fn ordered_mutate(
        &self,
        value: &mut char,
        cache: &mut Self::Cache,
        step: &mut Self::MutationStep,
        subvalue_provider: &dyn crate::SubValueProvider,
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
        // TODO: loop instead of recurse
        if let Some(result) = char::from_u32(self.start_range.wrapping_add(result)) {
            *step += 1;
            if result == *value {
                return self.ordered_mutate(value, cache, step, subvalue_provider, max_cplx);
            }

            *value = result;

            Some((token, (value.len_utf8() * 8) as f64))
        } else {
            *step += 1;
            self.ordered_mutate(value, cache, step, subvalue_provider, max_cplx)
        }
    }
    #[doc(hidden)]
    #[coverage(off)]
    fn random_mutate(&self, value: &mut char, _cache: &mut Self::Cache, _max_cplx: f64) -> (Self::UnmutateToken, f64) {
        let old_value = std::mem::replace(
            value,
            char::from_u32(
                self.rng
                    .u32(self.start_range..=self.start_range.wrapping_add(self.len_range)),
            )
            .unwrap_or(*value),
        );
        (old_value, (value.len_utf8() * 8) as f64)
    }
    #[doc(hidden)]
    #[coverage(off)]
    fn unmutate(&self, value: &mut char, _cache: &mut Self::Cache, t: Self::UnmutateToken) {
        *value = t;
    }

    #[doc(hidden)]
    #[coverage(off)]
    fn visit_subvalues<'a>(&self, _value: &'a char, _cache: &'a Self::Cache, _visit: &mut dyn FnMut(&'a dyn Any, f64)) {
    }
}
