//! Mutators that can handle recursive types.
//!
//! There are two main mutators:
//! 1. [`RecursiveMutator`] is the top-level mutator for the recursive type
//! 2. [`RecurToMutator`] is the mutator used at points of recursion. It is essentially a weak reference to [`RecursiveMutator`]
//!
//! In practice, you will want to use the [`make_mutator!`](crate::make_mutator) procedural macro to create recursive mutators.
//! For example:
//! ```
//! # #![feature(coverage_attribute)]
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

use std::any::Any;
use std::fmt::Debug;
use std::rc::{Rc, Weak};

use crate::Mutator;

/// The ArbitraryStep that is used for recursive mutators
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RecursingArbitraryStep<AS> {
    Default,
    Initialized(AS),
}
impl<AS> Default for RecursingArbitraryStep<AS> {
    #[coverage(off)]
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
# #![feature(coverage_attribute)]
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
    rng: fastrand::Rng,
}
impl<M> RecursiveMutator<M> {
    /// Create a new `RecursiveMutator` using a weak reference to itself.
    #[coverage(off)]
    pub fn new(data_fn: impl FnOnce(&Weak<M>) -> M) -> Self {
        Self {
            mutator: Rc::new_cyclic(data_fn),
            rng: fastrand::Rng::new(),
        }
    }
}

/// A mutator that defers to a weak reference of a
/// [`RecursiveMutator`](crate::mutators::recursive::RecursiveMutator)
pub struct RecurToMutator<M> {
    reference: Weak<M>,
}
impl<M> From<&Weak<M>> for RecurToMutator<M> {
    #[coverage(off)]
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
    #[coverage(off)]
    fn initialize(&self) {}

    #[doc(hidden)]
    #[coverage(off)]
    fn default_arbitrary_step(&self) -> Self::ArbitraryStep {
        RecursingArbitraryStep::Default
    }

    #[doc(hidden)]
    #[coverage(off)]
    fn is_valid(&self, value: &T) -> bool {
        self.reference.upgrade().unwrap().is_valid(value)
    }

    #[doc(hidden)]
    #[coverage(off)]
    fn validate_value(&self, value: &T) -> Option<Self::Cache> {
        self.reference.upgrade().unwrap().validate_value(value)
    }

    #[doc(hidden)]
    #[coverage(off)]
    fn default_mutation_step(&self, value: &T, cache: &Self::Cache) -> Self::MutationStep {
        self.reference.upgrade().unwrap().default_mutation_step(value, cache)
    }

    #[doc(hidden)]
    #[coverage(off)]
    fn global_search_space_complexity(&self) -> f64 {
        std::f64::INFINITY
    }

    #[doc(hidden)]
    #[coverage(off)]
    fn max_complexity(&self) -> f64 {
        // can potentially recur infinitely
        std::f64::INFINITY
    }

    #[doc(hidden)]
    #[coverage(off)]
    fn min_complexity(&self) -> f64 {
        // this will crash if called before the RecurToMutator is connected
        // to the RecursiveMutator
        self.reference.upgrade().unwrap().min_complexity()
    }

    #[doc(hidden)]
    #[coverage(off)]
    fn complexity(&self, value: &T, cache: &Self::Cache) -> f64 {
        self.reference.upgrade().unwrap().complexity(value, cache)
    }

    #[doc(hidden)]
    #[coverage(off)]
    fn ordered_arbitrary(&self, step: &mut Self::ArbitraryStep, max_cplx: f64) -> Option<(T, f64)> {
        match step {
            RecursingArbitraryStep::Default => {
                let inner_step = self.reference.upgrade().unwrap().default_arbitrary_step();
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
    #[coverage(off)]
    fn random_arbitrary(&self, max_cplx: f64) -> (T, f64) {
        self.reference.upgrade().unwrap().random_arbitrary(max_cplx)
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
        self.reference
            .upgrade()
            .unwrap()
            .ordered_mutate(value, cache, step, subvalue_provider, max_cplx)
    }

    #[doc(hidden)]
    #[coverage(off)]
    fn random_mutate(&self, value: &mut T, cache: &mut Self::Cache, max_cplx: f64) -> (Self::UnmutateToken, f64) {
        self.reference.upgrade().unwrap().random_mutate(value, cache, max_cplx)
    }

    #[doc(hidden)]
    #[coverage(off)]
    fn unmutate(&self, value: &mut T, cache: &mut Self::Cache, t: Self::UnmutateToken) {
        self.reference.upgrade().unwrap().unmutate(value, cache, t)
    }

    #[doc(hidden)]
    #[coverage(off)]
    fn visit_subvalues<'a>(&self, value: &'a T, cache: &'a Self::Cache, visit: &mut dyn FnMut(&'a dyn Any, f64)) {
        self.reference.upgrade().unwrap().visit_subvalues(value, cache, visit)
    }
}

#[derive(Clone)]
pub struct RecursiveMutatorCache<T, C> {
    inner: C,
    _cloned_self: Box<(T, C)>,
    sub_self_values: Vec<(*const T, f64)>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RecursiveMutatorMutationStep<MS> {
    mutation_step: MS,
    idx_sub_self_values: usize,
}

pub enum RecursiveMutatorUnmutateToken<T, UnmutateToken> {
    Replace(T),
    Token(UnmutateToken),
}

impl<M, T: Clone + 'static> Mutator<T> for RecursiveMutator<M>
where
    M: Mutator<T>,
{
    #[doc(hidden)]
    type Cache = RecursiveMutatorCache<T, M::Cache>;
    #[doc(hidden)]
    type MutationStep = RecursiveMutatorMutationStep<M::MutationStep>;
    #[doc(hidden)]
    type ArbitraryStep = M::ArbitraryStep;
    #[doc(hidden)]
    type UnmutateToken = RecursiveMutatorUnmutateToken<T, M::UnmutateToken>;

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
        self.mutator.is_valid(value)
    }

    #[doc(hidden)]
    #[coverage(off)]
    fn validate_value(&self, value: &T) -> Option<Self::Cache> {
        if let Some(cache) = self.mutator.validate_value(value) {
            let cloned_self = Box::new((value.clone(), cache.clone()));
            let mut sub_self_values = vec![];

            let mut visit_subvalues = #[coverage(off)]
            |subvalue: &dyn Any, cplx: f64| {
                if let Some(sub_self_value) = subvalue.downcast_ref::<T>()
                && let Some(subcache) = self.mutator.validate_value(sub_self_value)
                {
                    let subcplx = self.mutator.complexity(sub_self_value, &subcache);
                    assert_eq!(cplx, subcplx);
                    sub_self_values.push((sub_self_value as *const _, subcplx));
                }
            };

            self.mutator
                .visit_subvalues(&cloned_self.0, &cloned_self.1, &mut visit_subvalues);
            Some(RecursiveMutatorCache {
                inner: cache,
                _cloned_self: cloned_self,
                sub_self_values,
            })
        } else {
            None
        }
    }
    #[doc(hidden)]
    #[coverage(off)]
    fn default_mutation_step(&self, value: &T, cache: &Self::Cache) -> Self::MutationStep {
        let mutation_step = self.mutator.default_mutation_step(value, &cache.inner);

        RecursiveMutatorMutationStep {
            mutation_step,
            idx_sub_self_values: 0,
        }
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
        self.mutator.complexity(value, &cache.inner)
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
        if step.idx_sub_self_values < cache.sub_self_values.len() {
            let (subself, cplx) = cache.sub_self_values[step.idx_sub_self_values];
            let subself = unsafe { subself.as_ref() }.unwrap();
            let mut tmp = subself.clone();
            step.idx_sub_self_values += 1;
            std::mem::swap(value, &mut tmp);
            Some((RecursiveMutatorUnmutateToken::Replace(tmp), cplx))
        } else {
            if let Some((token, cplx)) = self.mutator.ordered_mutate(
                value,
                &mut cache.inner,
                &mut step.mutation_step,
                subvalue_provider,
                max_cplx,
            ) {
                Some((RecursiveMutatorUnmutateToken::Token(token), cplx))
            } else {
                None
            }
        }
    }

    #[doc(hidden)]
    #[coverage(off)]
    fn random_mutate(&self, value: &mut T, cache: &mut Self::Cache, max_cplx: f64) -> (Self::UnmutateToken, f64) {
        if !cache.sub_self_values.is_empty() && self.rng.usize(..100) == 0 {
            let idx = self.rng.usize(..cache.sub_self_values.len());
            let (subself, cplx) = cache.sub_self_values[idx];
            let subself = unsafe { subself.as_ref() }.unwrap();
            let mut tmp = subself.clone();
            std::mem::swap(value, &mut tmp);
            (RecursiveMutatorUnmutateToken::Replace(tmp), cplx)
        } else {
            let (token, cplx) = self.mutator.random_mutate(value, &mut cache.inner, max_cplx);
            let token = RecursiveMutatorUnmutateToken::Token(token);
            (token, cplx)
        }
    }

    #[doc(hidden)]
    #[coverage(off)]
    fn unmutate(&self, value: &mut T, cache: &mut Self::Cache, t: Self::UnmutateToken) {
        match t {
            RecursiveMutatorUnmutateToken::Replace(x) => {
                let _ = std::mem::replace(value, x);
            }
            RecursiveMutatorUnmutateToken::Token(t) => self.mutator.unmutate(value, &mut cache.inner, t),
        }
    }

    #[doc(hidden)]
    #[coverage(off)]
    fn visit_subvalues<'a>(&self, value: &'a T, cache: &'a Self::Cache, visit: &mut dyn FnMut(&'a dyn Any, f64)) {
        self.mutator.visit_subvalues(value, &cache.inner, visit)
    }
}
