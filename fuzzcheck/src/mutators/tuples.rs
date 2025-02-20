//! Mutators for tuple-like types
//!
//! This module contains the following traits and types:
//! - [`RefTypes`] is a trait which essentially holds the types of a destructured tuple or structure.
//!
//! - `TupleN` is a marker type which implements [`RefTypes`] for tuples and structures of N elements.
//!
//!    In this module, `Tuple0` to `Tuple10` are defined.
//!
//! - [`TupleStructure`] is a trait that can actually perform the destructuring for tuples and structures.
//!   For example, the code below shows how to implement `TupleStructure<Tuple2<A, B>>` for a struct `S`.
//!   ```
//!   use fuzzcheck::mutators::tuples::*;
//!   struct S<A, B> {
//!       x: A,
//!       y: B
//!   }
//!   impl<A: 'static, B: 'static> TupleStructure<Tuple2<A, B>> for S<A, B> {
//!       fn get_ref<'a>(&'a self) -> <Tuple2<A, B> as RefTypes>::Ref<'a> { // Ref is (&'a A, &'a B)
//!           (&self.x, &self.y)
//!       }
//!       fn get_mut<'a>(&'a mut self) -> <Tuple2<A, B> as RefTypes>::Mut<'a> { // Mut is (&'a mut A, &'a mut B)
//!           (&mut self.x, &mut self.y)
//!       }
//!       fn new(t: <Tuple2<A, B> as RefTypes>::Owned) -> Self { // Owned is (A, B)
//!           S { x: t.0, y: t.1 }
//!       }
//!   }
//!   let mut s = S { x: true, y: true };
//!   let (x, y) = s.get_ref(); // : (&bool, &bool)
//!   let (x, y) = s.get_mut(); // : (&mut bool, &mut bool)
//!   let s = S::new((true, false));
//!   ```
//!   
//! - [`TupleMutator`] is a trait that is exactly the same as [`Mutator`] except that it works on
//!  the destructured form of types implementing [`TupleStructure`] instead.
//!
//! - [`TupleMutatorWrapper`] creates a [`Mutator`] from a [`TupleMutator`]
//!
//! - `TupleNMutator` is a [`TupleMutator`] for types that implememt `TupleStructure<TupleN<..>>`.
//!   
//!   In this module, `Tuple1Mutator` to `Tuple10Mutator` are defined.
//!
//! ### It seems convoluted, why does all of this exist?”
//!
//! To make the the [`#[derive(DefaultMutator)]`](derive@crate::DefaultMutator) procedural macro much simpler.
//!
//! First, it allows me to reuse a limited number of [`TupleMutator`](TupleMutator) implementations,
//! paired with [`TupleMutatorWrapper`], to create mutators for any struct that implements `TupleStructure`. This makes the
//! derive macro easier to write because now its job is mostly to implement `TupleStructure` for the struct, which is easy to do.
//!
//! Second, it also allows me to reuse the same tuple mutators to mutate the content of enum variants. For example,
//! ```
//! enum S {
//!     A { x: u8, y: bool },
//!     B(u8, bool),
//!     C,
//!     D
//! }
//! ```
//! Here, the enum `S` is essentially a sum type of `(u8, bool)`, `(u8, bool)`, `()`, `()`.
//! So I'd like to reuse the mutators I already have for `(u8, bool)` and `()` to mutate `S`. If `TupleMutator` didn't
//! exist, then I would have to defer to a `Mutator<(u8, bool)>`. But that wouldn't be efficient, because when the enum
//! is destructured through `match`, we get access to `(&u8, &bool)` or `(&mut u8, &mut bool)`, which cannot be handled
//! by a `Mutator<(u8, bool)>`:
//! ```
//! # enum S {
//! #     A { x: u8, y: bool },
//! #     B(u8, bool),
//! #     C,
//! #     D { }
//! # }
//! let mut s = S::A { x: 7, y: true };
//! match &mut s {
//!     S::A { x, y } => {
//!         // here we have access to (x, y): (&mut u8, &mut bool)
//!         // but a Mutator<(u8, bool)> would ask for a &mut (u8, bool)
//!         // there is no efficient way to convert between the two.
//!         // By contrast, if I have a `Tuple2Mutator<U8Mutator, BoolMutator>`
//!         // then I can write:
//!         // mutator.random_mutate((x, y), ...)
//!     }
//!     _ => {}
//! }
//! ```
//! None of it is *strictly* necessary since I could always write a brand new mutator for each type from scratch instead
//! of trying to reuse mutators. But it would be a much larger amount of work, would probably increase compile times, and
//! it would be more difficult to refactor and keep the implementations correct.
use std::any::Any;
use std::marker::PhantomData;

use crate::Mutator;

/// A trait which essentially holds the types of a destructured tuple or structure.
///
/// Read the [module documentation](crate::mutators::tuples) for more information about it.
pub trait RefTypes {
    type Owned;
    type Ref<'a>: Copy;
    type Mut<'a>;
    fn get_ref_from_mut<'a>(v: &'a Self::Mut<'a>) -> Self::Ref<'a>;
}

/// Trait for types that have the same shape as tuples, such as tuples and structs.
///
/// For example, the tuple `(A, B)` implements `TupleStructure<Tuple2<A, B>>` since it is
/// a 2-tuple with fields of type `A` and `B`. The struct `S { a: A, b: B }`
/// also implements `TupleStructure<Tuple2<A, B>>`.
///
/// We can then write generic functions over both `(A, B)` and `S` using this trait.
///
/// * [`self.get_ref()`](TupleStructure::get_ref) returns immutable references to each of their fields (e.g. `(&A, &B)`)
/// * [`self.get_mut()`](TupleStructure::get_mut) returns mutable references to each of their fields (e.g. `(&mut A, &mut B)`)
/// * [`Self::new(..)`](TupleStructure::new) creates a new `Self` from a list of its fields (e.g. `Self::new((a, b))`)
pub trait TupleStructure<TupleKind: RefTypes> {
    fn get_ref(&self) -> TupleKind::Ref<'_>;
    fn get_mut(&mut self) -> TupleKind::Mut<'_>;
    fn new(t: TupleKind::Owned) -> Self;
}

/// A trait equivalent in every way to [`Mutator`] except that it operates
/// on the destructured form of types implementing [`TupleStructure`].
///
/// Defer to the documentation of [`Mutator`] to understand the purpose of each method.
pub trait TupleMutator<T, TupleKind>: Sized + 'static
where
    TupleKind: RefTypes,
    T: TupleStructure<TupleKind>,
{
    type Cache: Clone;
    type MutationStep: Clone;
    type ArbitraryStep: Clone;
    type UnmutateToken;

    fn initialize(&self);

    fn default_arbitrary_step(&self) -> Self::ArbitraryStep;

    fn complexity<'a>(&self, value: TupleKind::Ref<'a>, cache: &'a Self::Cache) -> f64;

    fn is_valid<'a>(&self, value: TupleKind::Ref<'a>) -> bool;

    fn validate_value<'a>(&self, value: TupleKind::Ref<'a>) -> Option<Self::Cache>;

    fn default_mutation_step<'a>(&self, value: TupleKind::Ref<'a>, cache: &'a Self::Cache) -> Self::MutationStep;

    fn global_search_space_complexity(&self) -> f64;

    fn max_complexity(&self) -> f64;

    fn min_complexity(&self) -> f64;

    fn ordered_arbitrary(&self, step: &mut Self::ArbitraryStep, max_cplx: f64) -> Option<(T, f64)>;

    fn random_arbitrary(&self, max_cplx: f64) -> (T, f64);

    fn ordered_mutate<'a>(
        &self,
        value: TupleKind::Mut<'a>,
        cache: &'a mut Self::Cache,
        step: &'a mut Self::MutationStep,
        subvalue_provider: &dyn crate::SubValueProvider,
        max_cplx: f64,
    ) -> Option<(Self::UnmutateToken, f64)>;

    fn random_mutate<'a>(
        &self,
        value: TupleKind::Mut<'a>,
        cache: &'a mut Self::Cache,
        max_cplx: f64,
    ) -> (Self::UnmutateToken, f64);

    fn unmutate<'a>(&self, value: TupleKind::Mut<'a>, cache: &'a mut Self::Cache, t: Self::UnmutateToken);

    fn visit_subvalues<'a>(
        &self,
        value: TupleKind::Ref<'a>,
        cache: &'a Self::Cache,
        visit: &mut dyn FnMut(&'a dyn Any, f64),
    );
}

/// A wrapper that transforms a [`TupleMutator`] into a [`Mutator`] of values [with a tuple structure](TupleStructure).
pub struct TupleMutatorWrapper<M, TupleKind>
where
    TupleKind: RefTypes,
{
    pub mutator: M,
    _phantom: PhantomData<TupleKind>,
}
impl<M, TupleKind> TupleMutatorWrapper<M, TupleKind>
where
    TupleKind: RefTypes,
{
    #[coverage(off)]
    pub fn new(mutator: M) -> Self {
        Self {
            mutator,
            _phantom: PhantomData,
        }
    }
}
impl<M, TupleKind> Default for TupleMutatorWrapper<M, TupleKind>
where
    TupleKind: RefTypes,
    M: Default,
{
    #[coverage(off)]
    fn default() -> Self {
        Self {
            mutator: <_>::default(),
            _phantom: PhantomData,
        }
    }
}

impl<T, M, TupleKind> Mutator<T> for TupleMutatorWrapper<M, TupleKind>
where
    T: Clone + 'static,
    TupleKind: RefTypes + 'static,
    T: TupleStructure<TupleKind>,
    M: TupleMutator<T, TupleKind>,
{
    #[doc(hidden)]
    type Cache = M::Cache;
    #[doc(hidden)]
    type MutationStep = M::MutationStep;
    #[doc(hidden)]
    type ArbitraryStep = M::ArbitraryStep;
    #[doc(hidden)]
    type UnmutateToken = M::UnmutateToken;

    #[doc(hidden)]
    #[coverage(off)]
    fn initialize(&self) {
        self.mutator.initialize();
    }

    #[doc(hidden)]
    #[coverage(off)]
    fn default_arbitrary_step(&self) -> Self::ArbitraryStep {
        self.mutator.default_arbitrary_step()
    }

    #[doc(hidden)]
    #[coverage(off)]
    fn is_valid(&self, value: &T) -> bool {
        self.mutator.is_valid(value.get_ref())
    }

    #[doc(hidden)]
    #[coverage(off)]
    fn validate_value(&self, value: &T) -> Option<Self::Cache> {
        self.mutator.validate_value(value.get_ref())
    }
    #[doc(hidden)]
    #[coverage(off)]
    fn default_mutation_step(&self, value: &T, cache: &Self::Cache) -> Self::MutationStep {
        self.mutator.default_mutation_step(value.get_ref(), cache)
    }

    #[doc(hidden)]
    #[coverage(off)]
    fn global_search_space_complexity(&self) -> f64 {
        self.mutator.global_search_space_complexity()
    }

    #[doc(hidden)]
    #[coverage(off)]
    fn max_complexity(&self) -> f64 {
        self.mutator.max_complexity()
    }

    #[doc(hidden)]
    #[coverage(off)]
    fn min_complexity(&self) -> f64 {
        self.mutator.min_complexity()
    }

    #[doc(hidden)]
    #[coverage(off)]
    fn complexity(&self, value: &T, cache: &Self::Cache) -> f64 {
        self.mutator.complexity(value.get_ref(), cache)
    }

    #[doc(hidden)]
    #[coverage(off)]
    fn ordered_arbitrary(&self, step: &mut Self::ArbitraryStep, max_cplx: f64) -> Option<(T, f64)> {
        self.mutator.ordered_arbitrary(step, max_cplx)
    }

    #[doc(hidden)]
    #[coverage(off)]
    fn random_arbitrary(&self, max_cplx: f64) -> (T, f64) {
        self.mutator.random_arbitrary(max_cplx)
    }

    #[doc(hidden)]
    #[coverage(off)]
    fn ordered_mutate(
        &self,
        value: &mut T,
        cache: &mut Self::Cache,
        step: &mut Self::MutationStep,
        subvalue_provider: &dyn crate::SubValueProvider,
        max_cplx: f64,
    ) -> Option<(Self::UnmutateToken, f64)> {
        self.mutator
            .ordered_mutate(value.get_mut(), cache, step, subvalue_provider, max_cplx)
    }

    #[doc(hidden)]
    #[coverage(off)]
    fn random_mutate(&self, value: &mut T, cache: &mut Self::Cache, max_cplx: f64) -> (Self::UnmutateToken, f64) {
        self.mutator.random_mutate(value.get_mut(), cache, max_cplx)
    }

    #[doc(hidden)]
    #[coverage(off)]
    fn unmutate(&self, value: &mut T, cache: &mut Self::Cache, t: Self::UnmutateToken) {
        self.mutator.unmutate(value.get_mut(), cache, t)
    }

    #[doc(hidden)]
    #[coverage(off)]
    fn visit_subvalues<'a>(&self, value: &'a T, cache: &'a Self::Cache, visit: &mut dyn FnMut(&'a dyn Any, f64)) {
        self.mutator.visit_subvalues(value.get_ref(), cache, visit)
    }
}

pub use tuple0::{Tuple0, Tuple0Mutator};
mod tuple0 {

    use std::any::Any;

    use super::TupleMutator;
    use crate::mutators::tuples::{RefTypes, TupleStructure};

    /// A marker type implementing [`RefTypes`] indicating that a type is equivalent to the unit type `()`
    pub struct Tuple0;
    impl RefTypes for Tuple0 {
        type Owned = ();
        type Ref<'a> = ();
        type Mut<'a> = ();
        #[coverage(off)]
        fn get_ref_from_mut<'a>(_v: &'a Self::Mut<'a>) -> Self::Ref<'a> {}
    }
    impl TupleStructure<Tuple0> for () {
        #[coverage(off)]
        fn get_ref(&self) -> <Tuple0 as RefTypes>::Ref<'_> {}
        #[coverage(off)]
        fn get_mut(&mut self) -> <Tuple0 as RefTypes>::Mut<'_> {}
        #[coverage(off)]
        fn new(_t: <Tuple0 as RefTypes>::Owned) -> Self {}
    }
    /// A `TupleMutator` for types equivalent to the unit type `()`
    pub struct Tuple0Mutator;
    impl TupleMutator<(), Tuple0> for Tuple0Mutator {
        #[doc(hidden)]
        type Cache = ();
        #[doc(hidden)]
        type MutationStep = bool;
        #[doc(hidden)]
        type ArbitraryStep = bool;
        #[doc(hidden)]
        type UnmutateToken = ();

        #[doc(hidden)]
        #[coverage(off)]
        fn initialize(&self) {}

        #[doc(hidden)]
        #[coverage(off)]
        fn default_arbitrary_step(&self) -> Self::ArbitraryStep {
            false
        }

        #[doc(hidden)]
        #[coverage(off)]
        fn complexity(&self, _value: (), _cache: &Self::Cache) -> f64 {
            0.0
        }

        #[doc(hidden)]
        #[coverage(off)]
        fn is_valid<'a>(&self, _value: <Tuple0 as RefTypes>::Ref<'a>) -> bool {
            true
        }

        #[doc(hidden)]
        #[coverage(off)]
        fn validate_value(&self, _value: ()) -> Option<Self::Cache> {
            Some(())
        }

        #[doc(hidden)]
        #[coverage(off)]
        fn default_mutation_step<'a>(&self, _value: (), _cache: &'a Self::Cache) -> Self::MutationStep {
            false
        }

        #[doc(hidden)]
        #[coverage(off)]
        fn global_search_space_complexity(&self) -> f64 {
            0.0
        }

        #[doc(hidden)]
        #[coverage(off)]
        fn max_complexity(&self) -> f64 {
            0.0
        }

        #[doc(hidden)]
        #[coverage(off)]
        fn min_complexity(&self) -> f64 {
            0.0
        }

        #[doc(hidden)]
        #[coverage(off)]
        fn ordered_arbitrary(&self, step: &mut Self::ArbitraryStep, _max_cplx: f64) -> Option<((), f64)> {
            if !*step {
                *step = true;
                Some(((), 0.0))
            } else {
                None
            }
        }

        #[doc(hidden)]
        #[coverage(off)]
        fn random_arbitrary(&self, _max_cplx: f64) -> ((), f64) {
            ((), 0.0)
        }

        #[doc(hidden)]
        #[coverage(off)]
        fn ordered_mutate(
            &self,
            _value: (),
            _cache: &mut Self::Cache,
            step: &mut Self::MutationStep,
            _subvalue_provider: &dyn crate::SubValueProvider,
            _max_cplx: f64,
        ) -> Option<(Self::UnmutateToken, f64)> {
            if !*step {
                *step = true;
                Some(((), 0.0))
            } else {
                None
            }
        }

        #[doc(hidden)]
        #[coverage(off)]
        fn random_mutate(&self, _value: (), _cache: &mut Self::Cache, _max_cplx: f64) -> (Self::UnmutateToken, f64) {
            ((), 0.0)
        }

        #[doc(hidden)]
        #[coverage(off)]
        fn unmutate(&self, _value: (), _cache: &mut Self::Cache, _t: Self::UnmutateToken) {}

        #[doc(hidden)]
        #[coverage(off)]
        fn visit_subvalues<'a>(&self, _value: (), _cache: &'a Self::Cache, _visit: &mut dyn FnMut(&'a dyn Any, f64)) {}
    }
}

pub use tuple1::{Tuple1, Tuple1Mutator};
mod tuple1 {
    use std::any::Any;

    use super::{TupleMutator, TupleMutatorWrapper};
    use crate::mutators::tuples::RefTypes;
    use crate::mutators::{CrossoverStep, CROSSOVER_RATE};

    #[doc = "A marker type implementing [`RefTypes`](crate::mutators::tuples::RefTypes) indicating that a type has the [structure](crate::mutators::tuples::TupleStructure) of a 1-tuple."]
    pub struct Tuple1<T0: 'static> {
        _phantom: ::std::marker::PhantomData<(T0,)>,
    }
    impl<T0: 'static> RefTypes for Tuple1<T0> {
        type Owned = (T0,);
        type Ref<'a> = (&'a T0,);
        type Mut<'a> = (&'a mut T0,);
        #[coverage(off)]
        fn get_ref_from_mut<'a>(v: &'a Self::Mut<'a>) -> Self::Ref<'a> {
            (v.0,)
        }
    }
    impl<T0: 'static> crate::mutators::tuples::TupleStructure<Tuple1<T0>> for (T0,) {
        #[coverage(off)]
        fn get_ref<'a>(&'a self) -> (&'a T0,) {
            (&self.0,)
        }
        #[coverage(off)]
        fn get_mut<'a>(&'a mut self) -> (&'a mut T0,) {
            (&mut self.0,)
        }
        #[coverage(off)]
        fn new(t: (T0,)) -> Self {
            t
        }
    }
    #[derive(Clone)]
    pub struct MutationStep<T, MS> {
        crossover_step: CrossoverStep<T>,
        inner: MS,
    }
    pub enum UnmutateTuple1Token<T, U> {
        Replace(T),
        Inner(U),
    }

    #[doc = " A `TupleMutator` for types that have a 1-tuple structure"]
    #[derive(::std::default::Default)]
    pub struct Tuple1Mutator<M0> {
        mutator_0: M0,
        rng: fastrand::Rng,
    }
    impl<M0> Tuple1Mutator<M0> {
        #[coverage(off)]
        pub fn new(mutator_0: M0) -> Self {
            Self {
                mutator_0,
                rng: fastrand::Rng::new(),
            }
        }
    }

    impl<T, T0, M0> TupleMutator<T, Tuple1<T0>> for Tuple1Mutator<M0>
    where
        T: ::std::clone::Clone + 'static,
        T0: ::std::clone::Clone + 'static,
        M0: crate::Mutator<T0>,
        T: crate::mutators::tuples::TupleStructure<Tuple1<T0>>,
    {
        #[doc(hidden)]
        type Cache = <M0 as crate::Mutator<T0>>::Cache;
        #[doc(hidden)]
        type MutationStep = MutationStep<T0, <M0 as crate::Mutator<T0>>::MutationStep>;
        #[doc(hidden)]
        type ArbitraryStep = <M0 as crate::Mutator<T0>>::ArbitraryStep;
        #[doc(hidden)]
        type UnmutateToken = UnmutateTuple1Token<T0, <M0 as crate::Mutator<T0>>::UnmutateToken>;

        #[doc(hidden)]
        #[coverage(off)]
        fn initialize(&self) {
            self.mutator_0.initialize();
        }

        #[doc(hidden)]
        #[coverage(off)]
        fn default_arbitrary_step(&self) -> Self::ArbitraryStep {
            self.mutator_0.default_arbitrary_step()
        }

        #[doc(hidden)]
        #[coverage(off)]
        fn complexity<'a>(&self, value: <Tuple1<T0> as RefTypes>::Ref<'a>, cache: &'a Self::Cache) -> f64 {
            self.mutator_0.complexity(value.0, cache)
        }

        #[doc(hidden)]
        #[coverage(off)]
        fn is_valid<'a>(&self, value: <Tuple1<T0> as RefTypes>::Ref<'a>) -> bool {
            self.mutator_0.is_valid(value.0)
        }

        #[doc(hidden)]
        #[coverage(off)]
        fn validate_value<'a>(&self, value: <Tuple1<T0> as RefTypes>::Ref<'a>) -> Option<Self::Cache> {
            self.mutator_0.validate_value(value.0)
        }

        #[doc(hidden)]
        #[coverage(off)]
        fn default_mutation_step<'a>(
            &self,
            value: <Tuple1<T0> as RefTypes>::Ref<'a>,
            cache: &'a Self::Cache,
        ) -> Self::MutationStep {
            MutationStep {
                crossover_step: CrossoverStep::default(),
                inner: self.mutator_0.default_mutation_step(value.0, cache),
            }
        }

        #[doc(hidden)]
        #[coverage(off)]
        fn global_search_space_complexity(&self) -> f64 {
            self.mutator_0.global_search_space_complexity()
        }
        #[doc(hidden)]
        #[coverage(off)]
        fn max_complexity(&self) -> f64 {
            self.mutator_0.max_complexity()
        }
        #[doc(hidden)]
        #[coverage(off)]
        fn min_complexity(&self) -> f64 {
            self.mutator_0.min_complexity()
        }
        #[doc(hidden)]
        #[coverage(off)]
        fn ordered_arbitrary(&self, step: &mut Self::ArbitraryStep, max_cplx: f64) -> Option<(T, f64)> {
            self.mutator_0.ordered_arbitrary(step, max_cplx).map(
                #[coverage(off)]
                |(value, cplx)| (T::new((value,)), cplx),
            )
        }
        #[doc(hidden)]
        #[coverage(off)]
        fn random_arbitrary(&self, max_cplx: f64) -> (T, f64) {
            let (value, cplx) = self.mutator_0.random_arbitrary(max_cplx);
            (T::new((value,)), cplx)
        }
        #[doc(hidden)]
        #[coverage(off)]
        fn ordered_mutate<'a>(
            &self,
            value: <Tuple1<T0> as RefTypes>::Mut<'a>,
            cache: &'a mut Self::Cache,
            step: &'a mut Self::MutationStep,
            subvalue_provider: &dyn crate::SubValueProvider,
            max_cplx: f64,
        ) -> Option<(Self::UnmutateToken, f64)> {
            if self.rng.u8(..CROSSOVER_RATE) == 0
                && let Some((subvalue, subcplx)) = step.crossover_step.get_next_subvalue(subvalue_provider, max_cplx)
                && self.mutator_0.is_valid(subvalue)
            {
                let mut replacer = subvalue.clone();
                std::mem::swap(value.0, &mut replacer);
                return Some((UnmutateTuple1Token::Replace(replacer), subcplx));
            }
            if let Some((token, cplx)) =
                self.mutator_0
                    .ordered_mutate(value.0, cache, &mut step.inner, subvalue_provider, max_cplx)
            {
                Some((UnmutateTuple1Token::Inner(token), cplx))
            } else {
                None
            }
        }
        #[doc(hidden)]
        #[coverage(off)]
        fn random_mutate<'a>(
            &self,
            value: <Tuple1<T0> as RefTypes>::Mut<'a>,
            cache: &'a mut Self::Cache,
            max_cplx: f64,
        ) -> (Self::UnmutateToken, f64) {
            let (token, cplx) = self.mutator_0.random_mutate(value.0, cache, max_cplx);
            (UnmutateTuple1Token::Inner(token), cplx)
        }

        #[doc(hidden)]
        #[coverage(off)]
        fn unmutate<'a>(
            &'a self,
            value: <Tuple1<T0> as RefTypes>::Mut<'a>,
            cache: &'a mut Self::Cache,
            t: Self::UnmutateToken,
        ) {
            match t {
                UnmutateTuple1Token::Replace(x) => {
                    *value.0 = x;
                }
                UnmutateTuple1Token::Inner(t) => self.mutator_0.unmutate(value.0, cache, t),
            }
        }

        #[doc(hidden)]
        #[coverage(off)]
        fn visit_subvalues<'a>(
            &self,
            value: <Tuple1<T0> as RefTypes>::Ref<'a>,
            cache: &'a Self::Cache,
            visit: &mut dyn FnMut(&'a dyn Any, f64),
        ) {
            let cplx = self.mutator_0.complexity(value.0, cache);
            visit(value.0, cplx);
            self.mutator_0.visit_subvalues(value.0, cache, visit);
        }
    }
    impl<T0> crate::mutators::DefaultMutator for (T0,)
    where
        T0: crate::mutators::DefaultMutator + 'static,
    {
        type Mutator = TupleMutatorWrapper<Tuple1Mutator<<T0 as crate::mutators::DefaultMutator>::Mutator>, Tuple1<T0>>;
        #[coverage(off)]
        fn default_mutator() -> Self::Mutator {
            Self::Mutator::new(Tuple1Mutator::new(
                <T0 as crate::mutators::DefaultMutator>::default_mutator(),
            ))
        }
    }
}
pub use tuple2::{Tuple2, Tuple2Mutator};
mod tuple2 {
    extern crate self as fuzzcheck;
    fuzzcheck_mutators_derive::make_basic_tuple_mutator!(2);
}
pub use tuple3::{Tuple3, Tuple3Mutator};
mod tuple3 {
    extern crate self as fuzzcheck;
    fuzzcheck_mutators_derive::make_basic_tuple_mutator!(3);
}
pub use tuple4::{Tuple4, Tuple4Mutator};
mod tuple4 {
    extern crate self as fuzzcheck;
    fuzzcheck_mutators_derive::make_basic_tuple_mutator!(4);
}
pub use tuple5::{Tuple5, Tuple5Mutator};
mod tuple5 {
    extern crate self as fuzzcheck;
    fuzzcheck_mutators_derive::make_basic_tuple_mutator!(5);
}
pub use tuple6::{Tuple6, Tuple6Mutator};
mod tuple6 {
    extern crate self as fuzzcheck;
    fuzzcheck_mutators_derive::make_basic_tuple_mutator!(6);
}
pub use tuple7::{Tuple7, Tuple7Mutator};
mod tuple7 {
    extern crate self as fuzzcheck;
    fuzzcheck_mutators_derive::make_basic_tuple_mutator!(7);
}
pub use tuple8::{Tuple8, Tuple8Mutator};
mod tuple8 {
    extern crate self as fuzzcheck;
    fuzzcheck_mutators_derive::make_basic_tuple_mutator!(8);
}
pub use tuple9::{Tuple9, Tuple9Mutator};
mod tuple9 {
    extern crate self as fuzzcheck;
    fuzzcheck_mutators_derive::make_basic_tuple_mutator!(9);
}
pub use tuple10::{Tuple10, Tuple10Mutator};
mod tuple10 {
    extern crate self as fuzzcheck;
    fuzzcheck_mutators_derive::make_basic_tuple_mutator!(10);
}
pub use tuple11::{Tuple11, Tuple11Mutator};
mod tuple11 {
    extern crate self as fuzzcheck;
    fuzzcheck_mutators_derive::make_basic_tuple_mutator!(11);
}
pub use tuple12::{Tuple12, Tuple12Mutator};
mod tuple12 {
    extern crate self as fuzzcheck;
    fuzzcheck_mutators_derive::make_basic_tuple_mutator!(12);
}
pub use tuple13::{Tuple13, Tuple13Mutator};
mod tuple13 {
    extern crate self as fuzzcheck;
    fuzzcheck_mutators_derive::make_basic_tuple_mutator!(13);
}
pub use tuple14::{Tuple14, Tuple14Mutator};
mod tuple14 {
    extern crate self as fuzzcheck;
    fuzzcheck_mutators_derive::make_basic_tuple_mutator!(14);
}
pub use tuple15::{Tuple15, Tuple15Mutator};
mod tuple15 {
    extern crate self as fuzzcheck;
    fuzzcheck_mutators_derive::make_basic_tuple_mutator!(15);
}
pub use tuple16::{Tuple16, Tuple16Mutator};
mod tuple16 {
    extern crate self as fuzzcheck;
    fuzzcheck_mutators_derive::make_basic_tuple_mutator!(16);
}
