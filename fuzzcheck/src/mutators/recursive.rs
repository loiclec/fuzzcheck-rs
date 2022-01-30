//! Mutators that can handle recursive types.
//!
//! There are two main mutators:
//! 1. [`RecursiveMutator`] is the top-level mutator for the recursive type
//! 2. [`RecurToMutator`] is the mutator used at points of recursion. It is essentially a weak reference to [`RecursiveMutator`]
//!
//! In practice, you will want to use the [`make_mutator!`](crate::make_mutator) procedural macro to create recursive mutators.
//! For example:
//! ```
//! # #![feature(no_coverage)]
//! use fuzzcheck::mutators::{option::OptionMutator, boxed::BoxMutator};
//! use fuzzcheck::mutators::recursive::{RecursiveMutator, RecurToMutator};
//! use fuzzcheck::DefaultMutator;
//! use fuzzcheck::make_mutator;
//!
//! #[derive(Clone)]
//! struct S {
//!     content: bool,
//!     next: Option<Box<S>> // the type recurses here
//! }
//!
//! make_mutator! {
//!     name: SMutator,
//!     recursive: true, // this is important
//!     default: false,
//!     type: struct S {
//!         content: bool,
//!         // We need to specify a concrete sub-mutator for this field to avoid creating an infinite type.
//!         // We use the standard Option and Box mutators, but replace what would be SMutator<M0, M1> by
//!         // RecurToMutator<SMutator<M0>>, which indicates that this is a point of recursion
//!         // and the mutator should be a weak reference to a RecursiveMutator
//!         // The M0 part refers to the mutator for the `content: bool` field.
//!         #[field_mutator(OptionMutator<Box<S>, BoxMutator<RecurToMutator<SMutator<M0>>>>)]
//!         next: Option<Box<S>>
//!     }
//! }
//! # fn main() {
//!
//! let s_mutator = RecursiveMutator::new(|mutator| {
//!     SMutator::new(
//!         /*content_mutator:*/ bool::default_mutator(),
//!         /*next_mutator:*/ OptionMutator::new(BoxMutator::new(RecurToMutator::from(mutator)))
//!     )
//! });
//! // s_mutator impl Mutator<S>
//! # }
//! ```

use crate::Mutator;
use std::{
    any::Any,
    fmt::Debug,
    rc::{Rc, Weak},
};

/// The ArbitraryStep that is used for recursive mutators
#[derive(Clone, Debug, PartialEq)]
pub enum RecursingArbitraryStep<AS> {
    Default,
    Initialized(AS),
}
impl<AS> Default for RecursingArbitraryStep<AS> {
    #[no_coverage]
    fn default() -> Self {
        Self::Default
    }
}

/**
A wrapper that allows a mutator to call itself recursively.

For example, it is used to provide mutators for types such as:
```
struct S {
    content: bool,
    // to mutate this field, a mutator must be able to recursively call itself
    next: Option<Box<S>>
}
```
`RecursiveMutator` is only the top-level type. It must be used in conjuction
with [`RecurToMutator`](crate::mutators::recursive::RecurToMutator) at points of recursion.
For example:
```
# #![feature(no_coverage)]
use fuzzcheck::DefaultMutator;
use fuzzcheck::mutators::{option::OptionMutator, boxed::BoxMutator};
use fuzzcheck::mutators::recursive::{RecursiveMutator, RecurToMutator};

# use fuzzcheck::make_mutator;
# #[derive(Clone)]
# struct S {
#     content: bool,
#     next: Option<Box<S>>
# }
# make_mutator! {
#     name: SMutator,
#     recursive: true,
#     default: false,
#     type: struct S {
#         content: bool,
#         #[field_mutator(OptionMutator<Box<S>, BoxMutator<RecurToMutator<SMutator<M0>>>>)]
#         next: Option<Box<S>>
#     }
# }
let s_mutator = RecursiveMutator::new(|mutator| {
    SMutator::new(
        /*content_mutator:*/ bool::default_mutator(),
        /*next_mutator:*/ OptionMutator::new(BoxMutator::new(RecurToMutator::from(mutator)))
    )
});
```
*/
pub struct RecursiveMutator<M> {
    pub mutator: Rc<M>,
}
impl<M> RecursiveMutator<M> {
    /// Create a new `RecursiveMutator` using a weak reference to itself.
    #[no_coverage]
    pub fn new(data_fn: impl FnOnce(&Weak<M>) -> M) -> Self {
        Self {
            mutator: Rc::new_cyclic(data_fn),
        }
    }
}

/// A mutator that defers to a weak reference of a
/// [`RecursiveMutator`](crate::mutators::recursive::RecursiveMutator)
pub struct RecurToMutator<M> {
    reference: Weak<M>,
}
impl<M> From<&Weak<M>> for RecurToMutator<M> {
    #[no_coverage]
    fn from(reference: &Weak<M>) -> Self {
        Self {
            reference: reference.clone(),
        }
    }
}

impl<T, M> Mutator<T> for RecurToMutator<M>
where
    M: Mutator<T>,
    T: Clone + 'static,
{
    #[doc(hidden)]
    type Cache = <M as Mutator<T>>::Cache;
    #[doc(hidden)]
    type MutationStep = <M as Mutator<T>>::MutationStep;
    #[doc(hidden)]
    type ArbitraryStep = RecursingArbitraryStep<<M as Mutator<T>>::ArbitraryStep>;
    #[doc(hidden)]
    type UnmutateToken = <M as Mutator<T>>::UnmutateToken;

    #[doc(hidden)]
    #[no_coverage]
    fn default_arbitrary_step(&self) -> Self::ArbitraryStep {
        RecursingArbitraryStep::Default
    }

    #[doc(hidden)]
    #[no_coverage]
    fn validate_value(&self, value: &T) -> Option<Self::Cache> {
        self.reference.upgrade().unwrap().validate_value(value)
    }

    #[doc(hidden)]
    #[no_coverage]
    fn default_mutation_step(&self, value: &T, cache: &Self::Cache) -> Self::MutationStep {
        self.reference.upgrade().unwrap().default_mutation_step(value, cache)
    }

    #[doc(hidden)]
    #[no_coverage]
    fn max_complexity(&self) -> f64 {
        std::f64::INFINITY
    }

    #[doc(hidden)]
    #[no_coverage]
    fn min_complexity(&self) -> f64 {
        // should be the min complexity of the mutator
        if let Some(m) = self.reference.upgrade() {
            m.as_ref().min_complexity()
        } else {
            1.0 // not right, but easy hack for now
        }
    }

    #[doc(hidden)]
    #[no_coverage]
    fn complexity(&self, value: &T, cache: &Self::Cache) -> f64 {
        self.reference.upgrade().unwrap().complexity(value, cache)
    }

    #[doc(hidden)]
    #[no_coverage]
    fn ordered_arbitrary(&self, step: &mut Self::ArbitraryStep, max_cplx: f64) -> Option<(T, f64)> {
        match step {
            RecursingArbitraryStep::Default => {
                let mutator = self.reference.upgrade().unwrap();
                let inner_step = mutator.default_arbitrary_step();
                *step = RecursingArbitraryStep::Initialized(inner_step);
                self.ordered_arbitrary(step, max_cplx)
            }
            RecursingArbitraryStep::Initialized(inner_step) => self
                .reference
                .upgrade()
                .unwrap()
                .ordered_arbitrary(inner_step, max_cplx),
        }
    }

    #[doc(hidden)]
    #[no_coverage]
    fn random_arbitrary(&self, max_cplx: f64) -> (T, f64) {
        self.reference.upgrade().unwrap().random_arbitrary(max_cplx)
    }

    #[doc(hidden)]
    #[no_coverage]
    fn ordered_mutate(
        &self,
        value: &mut T,
        cache: &mut Self::Cache,
        step: &mut Self::MutationStep,
        max_cplx: f64,
    ) -> Option<(Self::UnmutateToken, f64)> {
        self.reference
            .upgrade()
            .unwrap()
            .ordered_mutate(value, cache, step, max_cplx)
    }

    #[doc(hidden)]
    #[no_coverage]
    fn random_mutate(&self, value: &mut T, cache: &mut Self::Cache, max_cplx: f64) -> (Self::UnmutateToken, f64) {
        self.reference.upgrade().unwrap().random_mutate(value, cache, max_cplx)
    }

    #[doc(hidden)]
    #[no_coverage]
    fn unmutate(&self, value: &mut T, cache: &mut Self::Cache, t: Self::UnmutateToken) {
        self.reference.upgrade().unwrap().unmutate(value, cache, t)
    }

    type LensPath = <M as Mutator<T>>::LensPath;

    fn lens<'a>(&self, value: &'a T, cache: &'a Self::Cache, path: &Self::LensPath) -> &'a dyn Any {
        self.reference.upgrade().unwrap().lens(value, cache, path)
    }

    #[doc(hidden)]
    #[no_coverage]
    fn all_paths(
        &self,
        value: &T,
        cache: &Self::Cache,
    ) -> std::collections::HashMap<std::any::TypeId, Vec<Self::LensPath>> {
        self.reference.upgrade().unwrap().all_paths(value, cache)
    }

    fn crossover_mutate(
        &self,
        value: &mut T,
        cache: &mut Self::Cache,
        subvalue_provider: &dyn crate::SubValueProvider,
        max_cplx: f64,
    ) -> (Self::UnmutateToken, f64) {
        todo!()
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct RecursiveMutatorMutationStep<MS> {
    mutation_step: MS,
}

pub enum RecursiveMutatorUnmutateToken<T, UnmutateToken> {
    Replace(T),
    Token(UnmutateToken),
}

impl<M, T: Clone + 'static> Mutator<T> for RecursiveMutator<M>
where
    M: Mutator<T>,
{
    type Cache = M::Cache;
    type MutationStep = RecursiveMutatorMutationStep<M::MutationStep>;
    type ArbitraryStep = M::ArbitraryStep;
    type UnmutateToken = RecursiveMutatorUnmutateToken<T, M::UnmutateToken>;

    #[doc(hidden)]
    #[no_coverage]
    fn default_arbitrary_step(&self) -> Self::ArbitraryStep {
        self.mutator.default_arbitrary_step()
    }

    #[doc(hidden)]
    #[no_coverage]
    fn validate_value(&self, value: &T) -> Option<Self::Cache> {
        self.mutator.validate_value(value)
    }
    #[doc(hidden)]
    #[no_coverage]
    fn default_mutation_step(&self, value: &T, cache: &Self::Cache) -> Self::MutationStep {
        let mutation_step = self.mutator.default_mutation_step(value, cache);

        RecursiveMutatorMutationStep { mutation_step }
    }

    #[doc(hidden)]
    #[no_coverage]
    fn max_complexity(&self) -> f64 {
        self.mutator.max_complexity()
    }

    #[doc(hidden)]
    #[no_coverage]
    fn min_complexity(&self) -> f64 {
        self.mutator.min_complexity()
    }

    #[doc(hidden)]
    #[no_coverage]
    fn complexity(&self, value: &T, cache: &Self::Cache) -> f64 {
        self.mutator.complexity(value, cache)
    }

    #[doc(hidden)]
    #[no_coverage]
    fn ordered_arbitrary(&self, step: &mut Self::ArbitraryStep, max_cplx: f64) -> Option<(T, f64)> {
        self.mutator.ordered_arbitrary(step, max_cplx)
    }

    #[doc(hidden)]
    #[no_coverage]
    fn random_arbitrary(&self, max_cplx: f64) -> (T, f64) {
        self.mutator.random_arbitrary(max_cplx)
    }

    #[doc(hidden)]
    #[no_coverage]
    fn ordered_mutate(
        &self,
        value: &mut T,
        cache: &mut Self::Cache,
        step: &mut Self::MutationStep,
        max_cplx: f64,
    ) -> Option<(Self::UnmutateToken, f64)> {
        if let Some((token, cplx)) = self
            .mutator
            .ordered_mutate(value, cache, &mut step.mutation_step, max_cplx)
        {
            Some((RecursiveMutatorUnmutateToken::Token(token), cplx))
        } else {
            None
        }
    }

    #[doc(hidden)]
    #[no_coverage]
    fn random_mutate(&self, value: &mut T, cache: &mut Self::Cache, max_cplx: f64) -> (Self::UnmutateToken, f64) {
        let (token, cplx) = self.mutator.random_mutate(value, cache, max_cplx);
        let token = RecursiveMutatorUnmutateToken::Token(token);
        (token, cplx)
    }

    #[doc(hidden)]
    #[no_coverage]
    fn unmutate(&self, value: &mut T, cache: &mut Self::Cache, t: Self::UnmutateToken) {
        match t {
            RecursiveMutatorUnmutateToken::Replace(x) => {
                let _ = std::mem::replace(value, x);
            }
            RecursiveMutatorUnmutateToken::Token(t) => self.mutator.unmutate(value, cache, t),
        }
    }

    type LensPath = <M as Mutator<T>>::LensPath;

    fn lens<'a>(&self, value: &'a T, cache: &'a Self::Cache, path: &Self::LensPath) -> &'a dyn Any {
        self.mutator.lens(value, cache, path)
    }

    #[doc(hidden)]
    #[no_coverage]
    fn all_paths(
        &self,
        value: &T,
        cache: &Self::Cache,
    ) -> std::collections::HashMap<std::any::TypeId, Vec<Self::LensPath>> {
        self.mutator.all_paths(value, cache)
    }

    fn crossover_mutate(
        &self,
        value: &mut T,
        cache: &mut Self::Cache,
        subvalue_provider: &dyn crate::SubValueProvider,
        max_cplx: f64,
    ) -> (Self::UnmutateToken, f64) {
        todo!()
    }
}
