#![feature(prelude_import)]
#![no_std]
#[prelude_import]
use std::prelude::v1::*;
#[macro_use]
extern crate std;
extern crate basic_example;
extern crate decent_serde_json_alternative;
extern crate fuzzcheck_mutators;
pub extern crate fuzzcheck_serializer;
use decent_serde_json_alternative::{FromJson, ToJson};
use fuzzcheck_mutators::{fuzzcheck_derive_mutator, fuzzcheck_make_mutator};
struct A {
    x: u8,
    y: u16,
}
pub use _A::AMutator;
mod _A {
    use super::*;
    pub struct AMutator<xType, yType>
    where
        u8: ::core::clone::Clone,
        xType: fuzzcheck_mutators::fuzzcheck_traits::Mutator<Value = u8>,
        u16: ::core::clone::Clone,
        yType: fuzzcheck_mutators::fuzzcheck_traits::Mutator<Value = u16>,
        A: ::core::clone::Clone,
    {
        pub x: xType,
        pub y: yType,
        pub rng: fuzzcheck_mutators::fastrand::Rng,
    }
    #[allow(non_camel_case_types)]
    pub struct AMutatorCache<xType, yType> {
        pub x: xType,
        pub y: yType,
        pub cplx: f64,
    }

    #[allow(non_camel_case_types)]
    pub enum AInnerMutationStep {
        x,
        y,
    }

    #[allow(non_camel_case_types)]
    pub struct AMutationStep<xType, yType> {
        pub x: xType,
        pub y: yType,
        pub step: usize,
        pub inner: ::std::vec::Vec<AInnerMutationStep>,
    }

    #[allow(non_camel_case_types)]
    pub struct AArbitraryStep<xType, yType> {
        x: xType,
        y: yType,
    }

    #[allow(non_camel_case_types)]
    pub struct AUnmutateToken<xType, yType> {
        pub x: ::std::option::Option<xType>,
        pub y: ::std::option::Option<yType>,
        pub cplx: f64,
    }

    impl<xType, yType> fuzzcheck_mutators::fuzzcheck_traits::Mutator for AMutator<xType, yType>
    where
        u8: ::core::clone::Clone,
        xType: fuzzcheck_mutators::fuzzcheck_traits::Mutator<Value = u8>,
        u16: ::core::clone::Clone,
        yType: fuzzcheck_mutators::fuzzcheck_traits::Mutator<Value = u16>,
        A: ::core::clone::Clone,
    {
        type Value = A;
        #[doc(hidden)]
type Cache = AMutatorCache<
            <xType as fuzzcheck_mutators::fuzzcheck_traits::Mutator>::Cache,
            <yType as fuzzcheck_mutators::fuzzcheck_traits::Mutator>::Cache,
        >;
        #[doc(hidden)]
type MutationStep = AMutationStep<
            <xType as fuzzcheck_mutators::fuzzcheck_traits::Mutator>::MutationStep,
            <yType as fuzzcheck_mutators::fuzzcheck_traits::Mutator>::MutationStep,
        >;
        #[doc(hidden)]
type ArbitraryStep = AArbitraryStep<
            <xType as fuzzcheck_mutators::fuzzcheck_traits::Mutator>::ArbitraryStep,
            <yType as fuzzcheck_mutators::fuzzcheck_traits::Mutator>::ArbitraryStep,
        >;
        #[doc(hidden)]
type UnmutateToken = AUnmutateToken<
            <xType as fuzzcheck_mutators::fuzzcheck_traits::Mutator>::UnmutateToken,
            <yType as fuzzcheck_mutators::fuzzcheck_traits::Mutator>::UnmutateToken,
        >;
        #[coverage(off)] fn max_complexity(&self) -> f64 {
            self.x.max_complexity() + self.y.max_complexity()
        }
        #[coverage(off)] fn min_complexity(&self) -> f64 {
            self.x.min_complexity() + self.y.min_complexity()
        }
        #[coverage(off)] fn complexity(&self, value: &Self::Value, cache: &Self::Cache) -> f64 {
            cache.cplx
        }
        #[coverage(off)] fn cache_from_value(&self, value: &Self::Value) -> Self::Cache {
            let x = self.x.cache_from_value(&value.x);
            let y = self.y.cache_from_value(&value.y);
            let cplx = self.x.complexity(&value.x, &x) + self.y.complexity(&value.y, &y);
            Self::Cache { x, y, cplx }
        }
        #[coverage(off)] fn initial_step_from_value(&self, value: &Self::Value) -> Self::MutationStep {
            let x = self.x.initial_step_from_value(&value.x);
            let y = self.y.initial_step_from_value(&value.y);
            let step = 0;
            Self::MutationStep {
                x,
                y,
                inner: <[_]>::into_vec(box [AInnerMutationStep::x, AInnerMutationStep::y]),
                step,
            }
        }
        #[coverage(off)] fn ordered_arbitrary(
            &mut self,
            step: &mut Self::ArbitraryStep,
            max_cplx: f64,
        ) -> ::std::option::Option<(Self::Value, Self::Cache)> {
            ::std::option::Option::Some(self.random_arbitrary(max_cplx))
        }
        #[coverage(off)] fn random_arbitrary(&mut self, max_cplx: f64) -> (Self::Value, Self::Cache) {
            let mut x_value: ::std::option::Option<_> = ::std::option::Option::None;
            let mut x_cache: ::std::option::Option<_> = ::std::option::Option::None;
            let mut y_value: ::std::option::Option<_> = ::std::option::Option::None;
            let mut y_cache: ::std::option::Option<_> = ::std::option::Option::None;
            let mut indices = (0..2).collect::<::std::vec::Vec<_>>();
            fuzzcheck_mutators::fastrand::shuffle(&mut indices);
            let seed = fuzzcheck_mutators::fastrand::usize(..);
            let mut cplx = f64::default();
            for idx in indices.iter() {
                match idx {
                    0 => {
                        let (value, cache) = self.x.random_arbitrary(max_cplx - cplx);
                        cplx += self.x.complexity(&value, &cache);
                        x_value = ::std::option::Option::Some(value);
                        x_cache = ::std::option::Option::Some(cache);
                    }
                    1 => {
                        let (value, cache) = self.y.random_arbitrary(max_cplx - cplx);
                        cplx += self.y.complexity(&value, &cache);
                        y_value = ::std::option::Option::Some(value);
                        y_cache = ::std::option::Option::Some(cache);
                    }
                    _ => ::core::panicking::panic("internal error: entered unreachable code"),
                }
            }
            (
                Self::Value {
                    x: x_value.unwrap(),
                    y: y_value.unwrap(),
                },
                Self::Cache {
                    x: x_cache.unwrap(),
                    y: y_cache.unwrap(),
                    cplx,
                },
            )
        }
        #[coverage(off)] fn ordered_mutate(
            &mut self,
            value: &mut Self::Value,
            cache: &mut Self::Cache,
            step: &mut Self::MutationStep,
            max_cplx: f64,
        ) -> ::std::option::Option<Self::UnmutateToken> {
            if step.inner.is_empty() {
                return ::std::option::Option::None;
            }
            let orig_step = step.step;
            step.step += 1;
            let current_cplx = self.complexity(value, cache);
            let mut inner_step_to_remove: ::std::option::Option<usize> = ::std::option::Option::None;
            let mut recurse = false;
            match step.inner[orig_step % step.inner.len()] {
                AInnerMutationStep::x => {
                    let current_field_cplx = self.x.complexity(&value.x, &cache.x);
                    let max_field_cplx = max_cplx - current_cplx + current_field_cplx;
                    if let ::std::option::Option::Some(token) =
                        self.x
                            .ordered_mutate(&mut value.x, &mut cache.x, &mut step.x, max_field_cplx)
                    {
                        let new_field_complexity = self.x.complexity(&value.x, &cache.x);
                        cache.cplx = cache.cplx - current_field_cplx + new_field_complexity;
                        return ::std::option::Option::Some(Self::UnmutateToken {
                            x: ::std::option::Option::Some(token),
                            cplx: current_cplx,
                            ..Self::UnmutateToken::default()
                        });
                    } else {
                        inner_step_to_remove = ::std::option::Option::Some(orig_step % step.inner.len());
                        recurse = true;
                    }
                }
                AInnerMutationStep::y => {
                    let current_field_cplx = self.y.complexity(&value.y, &cache.y);
                    let max_field_cplx = max_cplx - current_cplx + current_field_cplx;
                    if let ::std::option::Option::Some(token) =
                        self.y
                            .ordered_mutate(&mut value.y, &mut cache.y, &mut step.y, max_field_cplx)
                    {
                        let new_field_complexity = self.y.complexity(&value.y, &cache.y);
                        cache.cplx = cache.cplx - current_field_cplx + new_field_complexity;
                        return ::std::option::Option::Some(Self::UnmutateToken {
                            y: ::std::option::Option::Some(token),
                            cplx: current_cplx,
                            ..Self::UnmutateToken::default()
                        });
                    } else {
                        inner_step_to_remove = ::std::option::Option::Some(orig_step % step.inner.len());
                        recurse = true;
                    }
                }
            }
            if let ::std::option::Option::Some(idx) = inner_step_to_remove {
                step.inner.remove(idx);
            }
            if recurse {
                self.ordered_mutate(value, cache, step, max_cplx)
            } else {
                {
                    ::core::panicking::panic("internal error: entered unreachable code")
                }
            }
        }
        #[coverage(off)] fn random_mutate(
            &mut self,
            value: &mut Self::Value,
            cache: &mut Self::Cache,
            max_cplx: f64,
        ) -> Self::UnmutateToken {
            let current_cplx = self.complexity(value, cache);
            match self.rng.usize(..) % 2 {
                0 => {
                    let current_field_cplx = self.x.complexity(&value.x, &cache.x);
                    let max_field_cplx = max_cplx - current_cplx + current_field_cplx;
                    let token = self.x.random_mutate(&mut value.x, &mut cache.x, max_field_cplx);
                    let new_field_complexity = self.x.complexity(&value.x, &cache.x);
                    cache.cplx = cache.cplx - current_field_cplx + new_field_complexity;
                    return Self::UnmutateToken {
                        x: ::std::option::Option::Some(token),
                        cplx: current_cplx,
                        ..Self::UnmutateToken::default()
                    };
                }
                1 => {
                    let current_field_cplx = self.y.complexity(&value.y, &cache.y);
                    let max_field_cplx = max_cplx - current_cplx + current_field_cplx;
                    let token = self.y.random_mutate(&mut value.y, &mut cache.y, max_field_cplx);
                    let new_field_complexity = self.y.complexity(&value.y, &cache.y);
                    cache.cplx = cache.cplx - current_field_cplx + new_field_complexity;
                    return Self::UnmutateToken {
                        y: ::std::option::Option::Some(token),
                        cplx: current_cplx,
                        ..Self::UnmutateToken::default()
                    };
                }
                _ => ::core::panicking::panic("internal error: entered unreachable code"),
            }
        }
        #[coverage(off)] fn unmutate(&self, value: &mut Self::Value, cache: &mut Self::Cache, t: Self::UnmutateToken) {
            cache.cplx = t.cplx;
            if let ::std::option::Option::Some(subtoken) = t.x {
                self.x.unmutate(&mut value.x, &mut cache.x, subtoken);
            }
            if let ::std::option::Option::Some(subtoken) = t.y {
                self.y.unmutate(&mut value.y, &mut cache.y, subtoken);
            }
        }
    }
}
