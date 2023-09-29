use std::any::Any;
use std::ops::{Bound, RangeBounds};

use crate::mutators::integer::{
    binary_search_arbitrary_u16, binary_search_arbitrary_u32, binary_search_arbitrary_u64, binary_search_arbitrary_u8,
};
use crate::Mutator;
const INITIAL_MUTATION_STEP: u64 = 0;

macro_rules! impl_int_mutator_constrained {
    ($name:ident,$name_unsigned:ident, $name_mutator:ident, $name_binary_arbitrary_function: ident) => {
        pub struct $name_mutator {
            start_range: $name,
            len_range: $name_unsigned,
            search_space_complexity: f64,
            rng: fastrand::Rng,
        }
        impl $name_mutator {
            #[coverage(off)]
            pub fn new<RB: RangeBounds<$name>>(range: RB) -> Self {
                let start = match range.start_bound() {
                    Bound::Included(b) => *b,
                    Bound::Excluded(b) => {
                        assert_ne!(*b, <$name>::MAX);
                        *b + 1
                    }
                    Bound::Unbounded => <$name>::MIN,
                };
                let end = match range.end_bound() {
                    Bound::Included(b) => *b,
                    Bound::Excluded(b) => {
                        assert_ne!(*b, <$name>::MIN);
                        *b - 1
                    }
                    Bound::Unbounded => <$name>::MAX,
                };
                if !(start <= end) {
                    panic!(
                        "You have provided a character range where the value of the start of the range \
                        is larger than the end of the range!\nRange start: {:#?}\nRange end: {:#?}",
                        range.start_bound(),
                        range.end_bound()
                    )
                }
                let length = end.wrapping_sub(start);
                Self {
                    start_range: start,
                    len_range: end.wrapping_sub(start) as $name_unsigned,
                    search_space_complexity: super::size_to_cplxity(length as usize),
                    rng: fastrand::Rng::default(),
                }
            }
        }

        impl Mutator<$name> for $name_mutator {
            #[doc(hidden)]
            type Cache = ();
            #[doc(hidden)]
            type MutationStep = u64; // mutation step
            #[doc(hidden)]
            type ArbitraryStep = u64;
            #[doc(hidden)]
            type UnmutateToken = $name; // old value

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
            fn is_valid(&self, value: &$name) -> bool {
                (self.start_range..=self.start_range.wrapping_add(self.len_range as _)).contains(value)
            }
            #[doc(hidden)]
            #[coverage(off)]
            fn validate_value(&self, value: &$name) -> Option<Self::Cache> {
                if (self.start_range..=self.start_range.wrapping_add(self.len_range as _)).contains(value) {
                    Some(())
                } else {
                    None
                }
            }

            #[doc(hidden)]
            #[coverage(off)]
            fn default_mutation_step(&self, _value: &$name, _cache: &Self::Cache) -> Self::MutationStep {
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
                <$name>::BITS as f64
            }

            #[doc(hidden)]
            #[coverage(off)]
            fn min_complexity(&self) -> f64 {
                <$name>::BITS as f64
            }

            #[doc(hidden)]
            #[coverage(off)]
            fn complexity(&self, _value: &$name, _cache: &Self::Cache) -> f64 {
                <$name>::BITS as f64
            }

            #[doc(hidden)]
            #[coverage(off)]
            fn ordered_arbitrary(&self, step: &mut Self::ArbitraryStep, max_cplx: f64) -> Option<($name, f64)> {
                if max_cplx < self.min_complexity() {
                    return None;
                }
                if *step > self.len_range as u64 {
                    None
                } else {
                    let result = $name_binary_arbitrary_function(0, self.len_range, *step);
                    *step = step.wrapping_add(1);
                    Some((
                        self.start_range.wrapping_add(result as $name),
                        <$name>::BITS as f64,
                    ))
                }
            }

            #[doc(hidden)]
            #[coverage(off)]
            fn random_arbitrary(&self, _max_cplx: f64) -> ($name, f64) {
                let value = self
                    .rng
                    .$name(self.start_range..=self.start_range.wrapping_add(self.len_range as $name));
                (value, <$name>::BITS as f64)
            }

            #[doc(hidden)]
            #[coverage(off)]
            fn ordered_mutate(
                &self,
                value: &mut $name,
                _cache: &mut Self::Cache,
                step: &mut Self::MutationStep,
                _subvalue_provider: &dyn crate::SubValueProvider,
                max_cplx: f64,
            ) -> Option<(Self::UnmutateToken, f64)> {
                if max_cplx < self.min_complexity() {
                    return None;
                }
                if *step > self.len_range as u64 {
                    return None;
                }
                let token = *value;

                let result = $name_binary_arbitrary_function(0, self.len_range, *step);
                *value = self.start_range.wrapping_add(result as $name);
                *step = step.wrapping_add(1);

                Some((token, <$name>::BITS as f64))
            }

            #[doc(hidden)]
            #[coverage(off)]
            fn random_mutate(
                &self,
                value: &mut $name,
                _cache: &mut Self::Cache,
                _max_cplx: f64,
            ) -> (Self::UnmutateToken, f64) {
                (
                    std::mem::replace(
                        value,
                        self.rng
                            .$name(self.start_range..=self.start_range.wrapping_add(self.len_range as $name)),
                    ),
                    <$name>::BITS as f64,
                )
            }

            #[doc(hidden)]
            #[coverage(off)]
            fn unmutate(&self, value: &mut $name, _cache: &mut Self::Cache, t: Self::UnmutateToken) {
                *value = t;
            }

            #[doc(hidden)]
            #[coverage(off)]
            fn visit_subvalues<'a>(
                &self,
                _value: &'a $name,
                _cache: &'a Self::Cache,
                _visit: &mut dyn FnMut(&'a dyn Any, f64),
            ) {
            }
        }
    };
}

impl_int_mutator_constrained!(u8, u8, U8WithinRangeMutator, binary_search_arbitrary_u8);
impl_int_mutator_constrained!(u16, u16, U16WithinRangeMutator, binary_search_arbitrary_u16);
impl_int_mutator_constrained!(u32, u32, U32WithinRangeMutator, binary_search_arbitrary_u32);
impl_int_mutator_constrained!(u64, u64, U64WithinRangeMutator, binary_search_arbitrary_u64);
impl_int_mutator_constrained!(i8, u8, I8WithinRangeMutator, binary_search_arbitrary_u8);
impl_int_mutator_constrained!(i16, u16, I16WithinRangeMutator, binary_search_arbitrary_u16);
impl_int_mutator_constrained!(i32, u32, I32WithinRangeMutator, binary_search_arbitrary_u32);
impl_int_mutator_constrained!(i64, u64, I64WithinRangeMutator, binary_search_arbitrary_u64);

#[cfg(test)]
mod tests {
    use super::U8WithinRangeMutator;
    use crate::Mutator;

    #[test]
    fn test_int_constrained() {
        let m = U8WithinRangeMutator::new(1..2);
        assert!(m.is_valid(&1));
        assert!(!m.is_valid(&2));
    }
}
