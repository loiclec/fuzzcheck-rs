use crate::mutators::integer::{
    binary_search_arbitrary_u16, binary_search_arbitrary_u32, binary_search_arbitrary_u64, binary_search_arbitrary_u8,
};
use crate::Mutator;
use std::ops::Bound;
use std::ops::RangeBounds;

const INITIAL_MUTATION_STEP: u64 = 0;

macro_rules! impl_int_mutator_constrained {
    ($name:ident,$name_unsigned:ident, $name_mutator:ident, $name_binary_arbitrary_function: ident) => {
        pub struct $name_mutator {
            start_range: $name,
            len_range: $name_unsigned,
            rng: fastrand::Rng,
        }
        impl $name_mutator {
            #[no_coverage]
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
                Self {
                    start_range: start,
                    len_range: end.wrapping_sub(start) as $name_unsigned,
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
            #[no_coverage]
            fn default_arbitrary_step(&self) -> Self::ArbitraryStep {
                0
            }

            #[doc(hidden)]
            #[no_coverage]
            fn validate_value(&self, _value: &$name) -> Option<Self::Cache> {
                Some(())
            }

            #[doc(hidden)]
            #[no_coverage]
            fn default_mutation_step(&self, _value: &$name, _cache: &Self::Cache) -> Self::MutationStep {
                INITIAL_MUTATION_STEP
            }

            #[doc(hidden)]
            #[no_coverage]
            fn max_complexity(&self) -> f64 {
                <$name>::BITS as f64
            }

            #[doc(hidden)]
            #[no_coverage]
            fn min_complexity(&self) -> f64 {
                <$name>::BITS as f64
            }

            #[doc(hidden)]
            #[no_coverage]
            fn complexity(&self, _value: &$name, _cache: &Self::Cache) -> f64 {
                <$name>::BITS as f64
            }

            #[doc(hidden)]
            #[no_coverage]
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
            #[no_coverage]
            fn random_arbitrary(&self, _max_cplx: f64) -> ($name, f64) {
                let value = self
                    .rng
                    .$name(self.start_range..=self.start_range.wrapping_add(self.len_range as $name));
                (value, <$name>::BITS as f64)
            }

            #[doc(hidden)]
            #[no_coverage]
            fn ordered_mutate(
                &self,
                value: &mut $name,
                _cache: &mut Self::Cache,
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

                let result = $name_binary_arbitrary_function(0, self.len_range, *step);
                *value = self.start_range.wrapping_add(result as $name);
                *step = step.wrapping_add(1);

                Some((token, <$name>::BITS as f64))
            }

            #[doc(hidden)]
            #[no_coverage]
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
            #[no_coverage]
            fn unmutate(&self, value: &mut $name, _cache: &mut Self::Cache, t: Self::UnmutateToken) {
                *value = t;
            }

            #[doc(hidden)]
            type RecursingPartIndex = ();
            #[doc(hidden)]
            #[no_coverage]
            fn default_recursing_part_index(&self, _value: &$name, _cache: &Self::Cache) -> Self::RecursingPartIndex {}
            #[doc(hidden)]
            #[no_coverage]
            fn recursing_part<'a, V, N>(
                &self,
                _parent: &N,
                _value: &'a $name,
                _index: &mut Self::RecursingPartIndex,
            ) -> Option<&'a V>
            where
                V: Clone + 'static,
                N: Mutator<V>,
            {
                None
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
