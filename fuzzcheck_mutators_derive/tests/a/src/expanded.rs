#![feature(prelude_import)]
#![feature(move_ref_pattern)]
#[prelude_import]
use std::prelude::v1::*;
#[macro_use]
extern crate std;
extern crate fuzzcheck_mutators;
use fuzzcheck_mutators::fuzzcheck_derive_mutator;
pub enum X {
    A(u8),
    B(u16),
    C,
    D(bool),
}
#[automatically_derived]
#[allow(unused_qualifications)]
impl ::core::clone::Clone for X {
    #[inline]
    fn clone(&self) -> X {
        match (&*self,) {
            (&X::A(ref __self_0),) => X::A(::core::clone::Clone::clone(&(*__self_0))),
            (&X::B(ref __self_0),) => X::B(::core::clone::Clone::clone(&(*__self_0))),
            (&X::C,) => X::C,
            (&X::D(ref __self_0),) => X::D(::core::clone::Clone::clone(&(*__self_0))),
        }
    }
}
pub struct XMutator<A_0Type, B_0Type, D_0Type>
where
    u8: ::core::clone::Clone,
    A_0Type: fuzzcheck_mutators::fuzzcheck_traits::Mutator<Value = u8>,
    u16: ::core::clone::Clone,
    B_0Type: fuzzcheck_mutators::fuzzcheck_traits::Mutator<Value = u16>,
    bool: ::core::clone::Clone,
    D_0Type: fuzzcheck_mutators::fuzzcheck_traits::Mutator<Value = bool>,
    X: ::core::clone::Clone,
{
    A_0: A_0Type,
    B_0: B_0Type,
    D_0: D_0Type,
    pub rng: fuzzcheck_mutators::fastrand::Rng,
}
pub enum XInnerMutatorCache<A_0Type, B_0Type, D_0Type> {
    A { A_0: A_0Type },
    B { B_0: B_0Type },
    D { D_0: D_0Type },
}
#[automatically_derived]
#[allow(unused_qualifications)]
impl<
        A_0Type: ::core::clone::Clone,
        B_0Type: ::core::clone::Clone,
        D_0Type: ::core::clone::Clone,
    > ::core::clone::Clone for XInnerMutatorCache<A_0Type, B_0Type, D_0Type>
{
    #[inline]
    fn clone(&self) -> XInnerMutatorCache<A_0Type, B_0Type, D_0Type> {
        match (&*self,) {
            (&XInnerMutatorCache::A { A_0: ref __self_0 },) => XInnerMutatorCache::A {
                A_0: ::core::clone::Clone::clone(&(*__self_0)),
            },
            (&XInnerMutatorCache::B { B_0: ref __self_0 },) => XInnerMutatorCache::B {
                B_0: ::core::clone::Clone::clone(&(*__self_0)),
            },
            (&XInnerMutatorCache::D { D_0: ref __self_0 },) => XInnerMutatorCache::D {
                D_0: ::core::clone::Clone::clone(&(*__self_0)),
            },
        }
    }
}
pub struct XMutatorCache<A_0Type, B_0Type, D_0Type> {
    inner: Option<XInnerMutatorCache<A_0Type, B_0Type, D_0Type>>,
    cplx: f64,
}
#[automatically_derived]
#[allow(unused_qualifications)]
impl<
        A_0Type: ::core::clone::Clone,
        B_0Type: ::core::clone::Clone,
        D_0Type: ::core::clone::Clone,
    > ::core::clone::Clone for XMutatorCache<A_0Type, B_0Type, D_0Type>
{
    #[inline]
    fn clone(&self) -> XMutatorCache<A_0Type, B_0Type, D_0Type> {
        match *self {
            XMutatorCache {
                inner: ref __self_0_0,
                cplx: ref __self_0_1,
            } => XMutatorCache {
                inner: ::core::clone::Clone::clone(&(*__self_0_0)),
                cplx: ::core::clone::Clone::clone(&(*__self_0_1)),
            },
        }
    }
}
pub enum XInnerArbitraryStep<A_0Type, B_0Type, D_0Type> {
    A { A_0: A_0Type },
    B { B_0: B_0Type },
    D { D_0: D_0Type },
}
#[automatically_derived]
#[allow(unused_qualifications)]
impl<
        A_0Type: ::core::clone::Clone,
        B_0Type: ::core::clone::Clone,
        D_0Type: ::core::clone::Clone,
    > ::core::clone::Clone for XInnerArbitraryStep<A_0Type, B_0Type, D_0Type>
{
    #[inline]
    fn clone(&self) -> XInnerArbitraryStep<A_0Type, B_0Type, D_0Type> {
        match (&*self,) {
            (&XInnerArbitraryStep::A { A_0: ref __self_0 },) => XInnerArbitraryStep::A {
                A_0: ::core::clone::Clone::clone(&(*__self_0)),
            },
            (&XInnerArbitraryStep::B { B_0: ref __self_0 },) => XInnerArbitraryStep::B {
                B_0: ::core::clone::Clone::clone(&(*__self_0)),
            },
            (&XInnerArbitraryStep::D { D_0: ref __self_0 },) => XInnerArbitraryStep::D {
                D_0: ::core::clone::Clone::clone(&(*__self_0)),
            },
        }
    }
}
pub struct XArbitraryStep<A_0Type, B_0Type, D_0Type> {
    inner: Vec<XInnerArbitraryStep<A_0Type, B_0Type, D_0Type>>,
    step: usize,
}
#[automatically_derived]
#[allow(unused_qualifications)]
impl<
        A_0Type: ::core::clone::Clone,
        B_0Type: ::core::clone::Clone,
        D_0Type: ::core::clone::Clone,
    > ::core::clone::Clone for XArbitraryStep<A_0Type, B_0Type, D_0Type>
{
    #[inline]
    fn clone(&self) -> XArbitraryStep<A_0Type, B_0Type, D_0Type> {
        match *self {
            XArbitraryStep {
                inner: ref __self_0_0,
                step: ref __self_0_1,
            } => XArbitraryStep {
                inner: ::core::clone::Clone::clone(&(*__self_0_0)),
                step: ::core::clone::Clone::clone(&(*__self_0_1)),
            },
        }
    }
}
impl<A_0Type, B_0Type, D_0Type> ::core::default::Default
    for XArbitraryStep<A_0Type, B_0Type, D_0Type>
where
    A_0Type: ::core::default::Default,
    B_0Type: ::core::default::Default,
    D_0Type: ::core::default::Default,
{
    fn default() -> Self {
        Self {
            inner: <[_]>::into_vec(box [
                XInnerArbitraryStep::A {
                    A_0: <_>::default(),
                },
                XInnerArbitraryStep::B {
                    B_0: <_>::default(),
                },
                XInnerArbitraryStep::D {
                    D_0: <_>::default(),
                },
            ]),
            step: <_>::default(),
        }
    }
}
pub enum XAInnerMutationStep<A_0Type> {
    A_0(A_0Type),
}
#[automatically_derived]
#[allow(unused_qualifications)]
impl<A_0Type: ::core::clone::Clone> ::core::clone::Clone for XAInnerMutationStep<A_0Type> {
    #[inline]
    fn clone(&self) -> XAInnerMutationStep<A_0Type> {
        match (&*self,) {
            (&XAInnerMutationStep::A_0(ref __self_0),) => {
                XAInnerMutationStep::A_0(::core::clone::Clone::clone(&(*__self_0)))
            }
        }
    }
}
pub enum XBInnerMutationStep<B_0Type> {
    B_0(B_0Type),
}
#[automatically_derived]
#[allow(unused_qualifications)]
impl<B_0Type: ::core::clone::Clone> ::core::clone::Clone for XBInnerMutationStep<B_0Type> {
    #[inline]
    fn clone(&self) -> XBInnerMutationStep<B_0Type> {
        match (&*self,) {
            (&XBInnerMutationStep::B_0(ref __self_0),) => {
                XBInnerMutationStep::B_0(::core::clone::Clone::clone(&(*__self_0)))
            }
        }
    }
}
pub enum XDInnerMutationStep<D_0Type> {
    D_0(D_0Type),
}
#[automatically_derived]
#[allow(unused_qualifications)]
impl<D_0Type: ::core::clone::Clone> ::core::clone::Clone for XDInnerMutationStep<D_0Type> {
    #[inline]
    fn clone(&self) -> XDInnerMutationStep<D_0Type> {
        match (&*self,) {
            (&XDInnerMutationStep::D_0(ref __self_0),) => {
                XDInnerMutationStep::D_0(::core::clone::Clone::clone(&(*__self_0)))
            }
        }
    }
}
pub enum XInnerMutationStep<A_0Type, B_0Type, D_0Type> {
    A(Vec<XAInnerMutationStep<A_0Type>>),
    B(Vec<XBInnerMutationStep<B_0Type>>),
    D(Vec<XDInnerMutationStep<D_0Type>>),
}
#[automatically_derived]
#[allow(unused_qualifications)]
impl<
        A_0Type: ::core::clone::Clone,
        B_0Type: ::core::clone::Clone,
        D_0Type: ::core::clone::Clone,
    > ::core::clone::Clone for XInnerMutationStep<A_0Type, B_0Type, D_0Type>
{
    #[inline]
    fn clone(&self) -> XInnerMutationStep<A_0Type, B_0Type, D_0Type> {
        match (&*self,) {
            (&XInnerMutationStep::A(ref __self_0),) => {
                XInnerMutationStep::A(::core::clone::Clone::clone(&(*__self_0)))
            }
            (&XInnerMutationStep::B(ref __self_0),) => {
                XInnerMutationStep::B(::core::clone::Clone::clone(&(*__self_0)))
            }
            (&XInnerMutationStep::D(ref __self_0),) => {
                XInnerMutationStep::D(::core::clone::Clone::clone(&(*__self_0)))
            }
        }
    }
}
pub struct XMutationStep<A_0Type, B_0Type, D_0Type, ArbitraryStep> {
    inner: Option<XInnerMutationStep<A_0Type, B_0Type, D_0Type>>,
    step: usize,
    arbitrary_step: Option<ArbitraryStep>,
}
#[automatically_derived]
#[allow(unused_qualifications)]
impl<
        A_0Type: ::core::clone::Clone,
        B_0Type: ::core::clone::Clone,
        D_0Type: ::core::clone::Clone,
        ArbitraryStep: ::core::clone::Clone,
    > ::core::clone::Clone for XMutationStep<A_0Type, B_0Type, D_0Type, ArbitraryStep>
{
    #[inline]
    fn clone(&self) -> XMutationStep<A_0Type, B_0Type, D_0Type, ArbitraryStep> {
        match *self {
            XMutationStep {
                inner: ref __self_0_0,
                step: ref __self_0_1,
                arbitrary_step: ref __self_0_2,
            } => XMutationStep {
                inner: ::core::clone::Clone::clone(&(*__self_0_0)),
                step: ::core::clone::Clone::clone(&(*__self_0_1)),
                arbitrary_step: ::core::clone::Clone::clone(&(*__self_0_2)),
            },
        }
    }
}
pub enum XInnerUnmutateToken<A_0Type, B_0Type, D_0Type, ___Value, ___Cache> {
    A { A_0: ::std::option::Option<A_0Type> },
    B { B_0: ::std::option::Option<B_0Type> },
    D { D_0: ::std::option::Option<D_0Type> },
    ___Replace(___Value, ___Cache),
}
pub struct XUnmutateToken<A_0Type, B_0Type, D_0Type, ___Value, ___Cache> {
    inner: XInnerUnmutateToken<A_0Type, B_0Type, D_0Type, ___Value, ___Cache>,
    cplx: f64,
}
#[allow(non_shorthand_field_patterns)]
impl<A_0Type, B_0Type, D_0Type> fuzzcheck_mutators::fuzzcheck_traits::Mutator
    for XMutator<A_0Type, B_0Type, D_0Type>
where
    u8: ::core::clone::Clone,
    A_0Type: fuzzcheck_mutators::fuzzcheck_traits::Mutator<Value = u8>,
    u16: ::core::clone::Clone,
    B_0Type: fuzzcheck_mutators::fuzzcheck_traits::Mutator<Value = u16>,
    bool: ::core::clone::Clone,
    D_0Type: fuzzcheck_mutators::fuzzcheck_traits::Mutator<Value = bool>,
    X: ::core::clone::Clone,
{
    type Value = X;
    type Cache = XMutatorCache<
        <A_0Type as fuzzcheck_mutators::fuzzcheck_traits::Mutator>::Cache,
        <B_0Type as fuzzcheck_mutators::fuzzcheck_traits::Mutator>::Cache,
        <D_0Type as fuzzcheck_mutators::fuzzcheck_traits::Mutator>::Cache,
    >;
    type ArbitraryStep = XArbitraryStep<
        <A_0Type as fuzzcheck_mutators::fuzzcheck_traits::Mutator>::ArbitraryStep,
        <B_0Type as fuzzcheck_mutators::fuzzcheck_traits::Mutator>::ArbitraryStep,
        <D_0Type as fuzzcheck_mutators::fuzzcheck_traits::Mutator>::ArbitraryStep,
    >;
    type MutationStep = XMutationStep<
        <A_0Type as fuzzcheck_mutators::fuzzcheck_traits::Mutator>::MutationStep,
        <B_0Type as fuzzcheck_mutators::fuzzcheck_traits::Mutator>::MutationStep,
        <D_0Type as fuzzcheck_mutators::fuzzcheck_traits::Mutator>::MutationStep,
        Self::ArbitraryStep,
    >;
    type UnmutateToken = XUnmutateToken<
        <A_0Type as fuzzcheck_mutators::fuzzcheck_traits::Mutator>::UnmutateToken,
        <B_0Type as fuzzcheck_mutators::fuzzcheck_traits::Mutator>::UnmutateToken,
        <D_0Type as fuzzcheck_mutators::fuzzcheck_traits::Mutator>::UnmutateToken,
        Self::Value,
        Self::Cache,
    >;
    fn max_complexity(&self) -> f64 {
        2f64 + [
            self.A_0.max_complexity(),
            self.B_0.max_complexity(),
            self.D_0.max_complexity(),
        ]
        .iter()
        .max_by(|x, y| x.partial_cmp(y).unwrap_or(core::cmp::Ordering::Equal))
        .unwrap()
    }
    fn min_complexity(&self) -> f64 {
        2f64 + [
            self.A_0.min_complexity(),
            self.B_0.min_complexity(),
            self.D_0.min_complexity(),
        ]
        .iter()
        .min_by(|x, y| x.partial_cmp(y).unwrap_or(core::cmp::Ordering::Equal))
        .unwrap()
    }
    fn complexity(&self, value: &Self::Value, cache: &Self::Cache) -> f64 {
        cache.cplx
    }
    fn cache_from_value(&self, value: &Self::Value) -> Self::Cache {
        match value {
            X::A(_0) => {
                let mut cplx = 2f64;
                let inner__0 = self.A_0.cache_from_value(&_0);
                cplx += self.A_0.complexity(&_0, &inner__0);
                let inner = Some(XInnerMutatorCache::A { A_0: inner__0 });
                XMutatorCache { inner, cplx }
            }
            X::B(_0) => {
                let mut cplx = 2f64;
                let inner__0 = self.B_0.cache_from_value(&_0);
                cplx += self.B_0.complexity(&_0, &inner__0);
                let inner = Some(XInnerMutatorCache::B { B_0: inner__0 });
                XMutatorCache { inner, cplx }
            }
            X::D(_0) => {
                let mut cplx = 2f64;
                let inner__0 = self.D_0.cache_from_value(&_0);
                cplx += self.D_0.complexity(&_0, &inner__0);
                let inner = Some(XInnerMutatorCache::D { D_0: inner__0 });
                XMutatorCache { inner, cplx }
            }
            _ => XMutatorCache {
                inner: None,
                cplx: 2f64,
            },
        }
    }
    fn initial_step_from_value(&self, value: &Self::Value) -> Self::MutationStep {
        match value {
            X::A(_0) => {
                let inner = Some(XInnerMutationStep::A(<[_]>::into_vec(box [
                    XAInnerMutationStep::A_0(self.A_0.initial_step_from_value(&_0)),
                ])));
                let step = 0;
                XMutationStep {
                    inner,
                    step,
                    arbitrary_step: None,
                }
            }
            X::B(_0) => {
                let inner = Some(XInnerMutationStep::B(<[_]>::into_vec(box [
                    XBInnerMutationStep::B_0(self.B_0.initial_step_from_value(&_0)),
                ])));
                let step = 0;
                XMutationStep {
                    inner,
                    step,
                    arbitrary_step: None,
                }
            }
            X::D(_0) => {
                let inner = Some(XInnerMutationStep::D(<[_]>::into_vec(box [
                    XDInnerMutationStep::D_0(self.D_0.initial_step_from_value(&_0)),
                ])));
                let step = 0;
                XMutationStep {
                    inner,
                    step,
                    arbitrary_step: None,
                }
            }
            _ => XMutationStep {
                inner: None,
                step: 0,
                arbitrary_step: Some(<_>::default()),
            },
        }
    }
    fn ordered_arbitrary(
        &mut self,
        step: &mut Self::ArbitraryStep,
        max_cplx: f64,
    ) -> Option<(Self::Value, Self::Cache)> {
        if step.inner.is_empty() {
            return None;
        }
        let orig_step = step.step;
        let mut inner_step_to_remove: Option<usize> = None;
        let mut recurse = false;
        step.step += 1;
        if orig_step < 1 {
            match orig_step {
                0 => {
                    let value = X::C;
                    let cache = XMutatorCache {
                        inner: None,
                        cplx: 2f64,
                    };
                    return Some((value, cache));
                }
                _ => ::std::rt::begin_panic("internal error: entered unreachable code"),
            }
        }
        let inner_len = step.inner.len();
        match &mut step.inner[orig_step % inner_len] {
            XInnerArbitraryStep::A { A_0: A_0 } => {
                if let Some((inner_value, inner_cache)) = self.A_0.ordered_arbitrary(A_0, max_cplx)
                {
                    let cplx = 2f64 + self.A_0.complexity(&inner_value, &inner_cache);
                    let value = X::A(inner_value);
                    let cache = XMutatorCache {
                        inner: Some(XInnerMutatorCache::A { A_0: inner_cache }),
                        cplx,
                    };
                    return Some((value, cache));
                } else {
                    step.step -= 1;
                    inner_step_to_remove = Some(step.step);
                    recurse = true;
                }
            }
            XInnerArbitraryStep::B { B_0: B_0 } => {
                if let Some((inner_value, inner_cache)) = self.B_0.ordered_arbitrary(B_0, max_cplx)
                {
                    let cplx = 2f64 + self.B_0.complexity(&inner_value, &inner_cache);
                    let value = X::B(inner_value);
                    let cache = XMutatorCache {
                        inner: Some(XInnerMutatorCache::B { B_0: inner_cache }),
                        cplx,
                    };
                    return Some((value, cache));
                } else {
                    step.step -= 1;
                    inner_step_to_remove = Some(step.step);
                    recurse = true;
                }
            }
            XInnerArbitraryStep::D { D_0: D_0 } => {
                if let Some((inner_value, inner_cache)) = self.D_0.ordered_arbitrary(D_0, max_cplx)
                {
                    let cplx = 2f64 + self.D_0.complexity(&inner_value, &inner_cache);
                    let value = X::D(inner_value);
                    let cache = XMutatorCache {
                        inner: Some(XInnerMutatorCache::D { D_0: inner_cache }),
                        cplx,
                    };
                    return Some((value, cache));
                } else {
                    step.step -= 1;
                    inner_step_to_remove = Some(step.step);
                    recurse = true;
                }
            }
        }
        #[allow(unreachable_code)]
        {
            if let Some(idx) = inner_step_to_remove {
                step.inner.remove(idx);
            }
            if recurse {
                self.ordered_arbitrary(step, max_cplx)
            } else {
                None
            }
        }
    }
    fn random_arbitrary(&mut self, max_cplx: f64) -> (Self::Value, Self::Cache) {
        let step = self.rng.usize(..);
        let max_cplx = max_cplx - 2f64;
        match step % 4 {
            0 => {
                let (A_0_value, A_0_cache) = self.A_0.random_arbitrary(max_cplx);
                let cplx = 2f64 + self.A_0.complexity(&A_0_value, &A_0_cache);
                let value = X::A(A_0_value);
                let cache = XMutatorCache {
                    inner: Some(XInnerMutatorCache::A { A_0: A_0_cache }),
                    cplx,
                };
                (value, cache)
            }
            1 => {
                let (B_0_value, B_0_cache) = self.B_0.random_arbitrary(max_cplx);
                let cplx = 2f64 + self.B_0.complexity(&B_0_value, &B_0_cache);
                let value = X::B(B_0_value);
                let cache = XMutatorCache {
                    inner: Some(XInnerMutatorCache::B { B_0: B_0_cache }),
                    cplx,
                };
                (value, cache)
            }
            2 => {
                let value = X::C;
                let cache = XMutatorCache {
                    inner: None,
                    cplx: 2f64,
                };
                (value, cache)
            }
            3 => {
                let (D_0_value, D_0_cache) = self.D_0.random_arbitrary(max_cplx);
                let cplx = 2f64 + self.D_0.complexity(&D_0_value, &D_0_cache);
                let value = X::D(D_0_value);
                let cache = XMutatorCache {
                    inner: Some(XInnerMutatorCache::D { D_0: D_0_cache }),
                    cplx,
                };
                (value, cache)
            }
            _ => ::std::rt::begin_panic("internal error: entered unreachable code"),
        }
    }
    fn ordered_mutate(
        &mut self,
        mut value: &mut Self::Value,
        mut cache: &mut Self::Cache,
        step: &mut Self::MutationStep,
        max_cplx: f64,
    ) -> Option<Self::UnmutateToken> {
        if let Some(ar_step) = &mut step.arbitrary_step {
            if let Some((v, c)) = self.ordered_arbitrary(ar_step, max_cplx) {
                let old_value = std::mem::replace(value, v);
                let old_cache = std::mem::replace(cache, c);
                return Some(XUnmutateToken {
                    inner: XInnerUnmutateToken::___Replace(old_value, old_cache),
                    cplx: f64::default(),
                });
            } else {
                step.arbitrary_step = None;
                return None;
            }
        }
        let mut recurse = false;
        match (&mut value, &mut cache, &mut step.inner) {
            (
                X::A(_0_value),
                XMutatorCache {
                    inner: Some(XInnerMutatorCache::A { A_0: A_0_cache }),
                    cplx,
                },
                Some(XInnerMutationStep::A(steps)),
            ) => {
                if steps.is_empty() {
                    step.arbitrary_step = Some(<_>::default());
                    recurse = true;
                } else {
                    let orig_step = step.step % steps.len();
                    let mut step_to_remove: Option<usize> = None;
                    step.step += 1;
                    match &mut steps[orig_step] {
                        XAInnerMutationStep::A_0(inner_step) => {
                            let old_field_cplx = self.A_0.complexity(&_0_value, &A_0_cache);
                            let max_cplx = max_cplx - 2f64 - old_field_cplx;
                            if let Some(field_token) = self
                                .A_0
                                .ordered_mutate(_0_value, A_0_cache, inner_step, max_cplx)
                            {
                                let new_field_cplx = self.A_0.complexity(&_0_value, &A_0_cache);
                                *cplx += new_field_cplx - old_field_cplx;
                                return Some(XUnmutateToken {
                                    inner: XInnerUnmutateToken::A {
                                        A_0: Some(field_token),
                                    },
                                    cplx: old_field_cplx,
                                });
                            } else {
                                step_to_remove = Some(orig_step);
                            }
                        }
                    }
                    if let Some(idx) = step_to_remove {
                        steps.remove(idx);
                        recurse = true;
                    }
                }
            }
            (
                X::B(_0_value),
                XMutatorCache {
                    inner: Some(XInnerMutatorCache::B { B_0: B_0_cache }),
                    cplx,
                },
                Some(XInnerMutationStep::B(steps)),
            ) => {
                if steps.is_empty() {
                    step.arbitrary_step = Some(<_>::default());
                    recurse = true;
                } else {
                    let orig_step = step.step % steps.len();
                    let mut step_to_remove: Option<usize> = None;
                    step.step += 1;
                    match &mut steps[orig_step] {
                        XBInnerMutationStep::B_0(inner_step) => {
                            let old_field_cplx = self.B_0.complexity(&_0_value, &B_0_cache);
                            let max_cplx = max_cplx - 2f64 - old_field_cplx;
                            if let Some(field_token) = self
                                .B_0
                                .ordered_mutate(_0_value, B_0_cache, inner_step, max_cplx)
                            {
                                let new_field_cplx = self.B_0.complexity(&_0_value, &B_0_cache);
                                *cplx += new_field_cplx - old_field_cplx;
                                return Some(XUnmutateToken {
                                    inner: XInnerUnmutateToken::B {
                                        B_0: Some(field_token),
                                    },
                                    cplx: old_field_cplx,
                                });
                            } else {
                                step_to_remove = Some(orig_step);
                            }
                        }
                    }
                    if let Some(idx) = step_to_remove {
                        steps.remove(idx);
                        recurse = true;
                    }
                }
            }
            (
                X::D(_0_value),
                XMutatorCache {
                    inner: Some(XInnerMutatorCache::D { D_0: D_0_cache }),
                    cplx,
                },
                Some(XInnerMutationStep::D(steps)),
            ) => {
                if steps.is_empty() {
                    step.arbitrary_step = Some(<_>::default());
                    recurse = true;
                } else {
                    let orig_step = step.step % steps.len();
                    let mut step_to_remove: Option<usize> = None;
                    step.step += 1;
                    match &mut steps[orig_step] {
                        XDInnerMutationStep::D_0(inner_step) => {
                            let old_field_cplx = self.D_0.complexity(&_0_value, &D_0_cache);
                            let max_cplx = max_cplx - 2f64 - old_field_cplx;
                            if let Some(field_token) = self
                                .D_0
                                .ordered_mutate(_0_value, D_0_cache, inner_step, max_cplx)
                            {
                                let new_field_cplx = self.D_0.complexity(&_0_value, &D_0_cache);
                                *cplx += new_field_cplx - old_field_cplx;
                                return Some(XUnmutateToken {
                                    inner: XInnerUnmutateToken::D {
                                        D_0: Some(field_token),
                                    },
                                    cplx: old_field_cplx,
                                });
                            } else {
                                step_to_remove = Some(orig_step);
                            }
                        }
                    }
                    if let Some(idx) = step_to_remove {
                        steps.remove(idx);
                        recurse = true;
                    }
                }
            }
            (value, cache, _) => {
                if let Some(ar_step) = &mut step.arbitrary_step {
                    if let Some((v, c)) = self.ordered_arbitrary(ar_step, max_cplx) {
                        let old_value = std::mem::replace(*value, v);
                        let old_cache = std::mem::replace(*cache, c);
                        return Some(XUnmutateToken {
                            inner: XInnerUnmutateToken::___Replace(old_value, old_cache),
                            cplx: f64::default(),
                        });
                    } else {
                        return None;
                    }
                } else {
                    {
                        {
                            ::std::rt::begin_panic("internal error: entered unreachable code")
                        }
                    }
                }
            }
        }
        if recurse {
            self.ordered_mutate(value, cache, step, max_cplx)
        } else {
            None
        }
    }
    fn random_mutate(
        &mut self,
        mut value: &mut Self::Value,
        mut cache: &mut Self::Cache,
        max_cplx: f64,
    ) -> Self::UnmutateToken {
        let use_arbitrary = self.rng.f64() < 2f64 / self.complexity(&value, &cache);
        if use_arbitrary {
            let (v, c) = self.random_arbitrary(max_cplx - 2f64);
            let old_value = std::mem::replace(value, v);
            let old_cache = std::mem::replace(cache, c);
            return XUnmutateToken {
                inner: XInnerUnmutateToken::___Replace(old_value, old_cache),
                cplx: f64::default(),
            };
        } else {
            match (&mut value, &mut cache) {
                (
                    X::A(_0_value),
                    XMutatorCache {
                        inner: Some(XInnerMutatorCache::A { A_0: A_0_cache }),
                        cplx,
                    },
                ) => match self.rng.usize(..) % 1 {
                    0 => {
                        let old_field_cplx = self.A_0.complexity(&_0_value, &A_0_cache);
                        let max_cplx = max_cplx - 2f64 - old_field_cplx;
                        let field_token = self.A_0.random_mutate(_0_value, A_0_cache, max_cplx);
                        let new_field_cplx = self.A_0.complexity(&_0_value, &A_0_cache);
                        *cplx += new_field_cplx - old_field_cplx;
                        return XUnmutateToken {
                            inner: XInnerUnmutateToken::A {
                                A_0: Some(field_token),
                            },
                            cplx: old_field_cplx,
                        };
                    }
                    _ => ::std::rt::begin_panic("internal error: entered unreachable code"),
                },
                (
                    X::B(_0_value),
                    XMutatorCache {
                        inner: Some(XInnerMutatorCache::B { B_0: B_0_cache }),
                        cplx,
                    },
                ) => match self.rng.usize(..) % 1 {
                    0 => {
                        let old_field_cplx = self.B_0.complexity(&_0_value, &B_0_cache);
                        let max_cplx = max_cplx - 2f64 - old_field_cplx;
                        let field_token = self.B_0.random_mutate(_0_value, B_0_cache, max_cplx);
                        let new_field_cplx = self.B_0.complexity(&_0_value, &B_0_cache);
                        *cplx += new_field_cplx - old_field_cplx;
                        return XUnmutateToken {
                            inner: XInnerUnmutateToken::B {
                                B_0: Some(field_token),
                            },
                            cplx: old_field_cplx,
                        };
                    }
                    _ => ::std::rt::begin_panic("internal error: entered unreachable code"),
                },
                (value, cache) => {
                    let (v, c) = self.random_arbitrary(max_cplx - 2f64);
                    let old_value = std::mem::replace(*value, v);
                    let old_cache = std::mem::replace(*cache, c);
                    return XUnmutateToken {
                        inner: XInnerUnmutateToken::___Replace(old_value, old_cache),
                        cplx: f64::default(),
                    };
                }
                (
                    X::D(_0_value),
                    XMutatorCache {
                        inner: Some(XInnerMutatorCache::D { D_0: D_0_cache }),
                        cplx,
                    },
                ) => match self.rng.usize(..) % 1 {
                    0 => {
                        let old_field_cplx = self.D_0.complexity(&_0_value, &D_0_cache);
                        let max_cplx = max_cplx - 2f64 - old_field_cplx;
                        let field_token = self.D_0.random_mutate(_0_value, D_0_cache, max_cplx);
                        let new_field_cplx = self.D_0.complexity(&_0_value, &D_0_cache);
                        *cplx += new_field_cplx - old_field_cplx;
                        return XUnmutateToken {
                            inner: XInnerUnmutateToken::D {
                                D_0: Some(field_token),
                            },
                            cplx: old_field_cplx,
                        };
                    }
                    _ => ::std::rt::begin_panic("internal error: entered unreachable code"),
                },
            }
        }
    }
    fn unmutate(&self, value: &mut Self::Value, cache: &mut Self::Cache, t: Self::UnmutateToken) {
        match (t, value, cache) {
            (
                XUnmutateToken {
                    inner: XInnerUnmutateToken::A { A_0: A_0_token },
                    cplx: cplx_token,
                },
                X::A(_0_value),
                XMutatorCache {
                    inner: Some(XInnerMutatorCache::A { A_0: A_0_cache }),
                    cplx,
                },
            ) => {
                if let Some(t) = A_0_token {
                    self.A_0.unmutate(_0_value, A_0_cache, t)
                }
                *cplx = cplx_token
            }
            (
                XUnmutateToken {
                    inner: XInnerUnmutateToken::B { B_0: B_0_token },
                    cplx: cplx_token,
                },
                X::B(_0_value),
                XMutatorCache {
                    inner: Some(XInnerMutatorCache::B { B_0: B_0_cache }),
                    cplx,
                },
            ) => {
                if let Some(t) = B_0_token {
                    self.B_0.unmutate(_0_value, B_0_cache, t)
                }
                *cplx = cplx_token
            }
            (
                XUnmutateToken {
                    inner: XInnerUnmutateToken::D { D_0: D_0_token },
                    cplx: cplx_token,
                },
                X::D(_0_value),
                XMutatorCache {
                    inner: Some(XInnerMutatorCache::D { D_0: D_0_cache }),
                    cplx,
                },
            ) => {
                if let Some(t) = D_0_token {
                    self.D_0.unmutate(_0_value, D_0_cache, t)
                }
                *cplx = cplx_token
            }
            (
                XUnmutateToken {
                    inner: XInnerUnmutateToken::___Replace(v, c),
                    cplx: _,
                },
                value,
                cache,
            ) => {
                let _ = std::mem::replace(value, v);
                let _ = std::mem::replace(cache, c);
            }
            _ => ::std::rt::begin_panic("internal error: entered unreachable code"),
        }
    }
}
