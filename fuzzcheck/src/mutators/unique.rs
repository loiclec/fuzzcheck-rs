use std::any::Any;
use std::cell::{Cell, RefCell};
use std::hash::Hash;
use std::marker::PhantomData;
use std::rc::Rc;

use crate::bloom_filter::BloomFilter;
use crate::Mutator;

const SIZE_BLOOM: usize = 10_000_000;
const FALSE_POSITIVE_RATE: f64 = 0.000_001;

/// Experimental mutator which tries to prevent a value from being tested more
/// than once (using a bloom filter).
///
/// **Important:** this mutator cannot be used as a submutator.
pub struct UniqueMutator<T, TH, Focus, M>
where
    T: Clone + 'static,
    TH: Hash,
    M: Mutator<T>,
    Focus: Fn(&T) -> &TH,
{
    mutator: M,
    uniques: Rc<RefCell<BloomFilter<TH>>>,
    nbr_inserted: Cell<usize>,
    focus: Focus,
    _phantom: PhantomData<T>,
}

impl<T, TH, Focus, M> UniqueMutator<T, TH, Focus, M>
where
    T: Clone + 'static,
    TH: Hash,
    M: Mutator<T>,
    Focus: Fn(&T) -> &TH,
{
    /// Create a new `UniqueMutator` by wrapping another mutator.
    ///
    /// The `Focus` closure points to the hashable part of the value. This should almost always
    /// be the identity function. But there is an exception when we don't care about some part
    /// of the generated value. For example, a grammar-based mutator implements `Mutator<(AST, String)>`,
    /// but it is very likely that the test function will only operate on the String and not the AST.
    /// In that case, the `Focus` closure is `|x| &x.1`.
    #[coverage(off)]
    pub fn new(mutator: M, focus: Focus) -> Self {
        Self {
            mutator,
            uniques: Rc::new(RefCell::new(BloomFilter::new(SIZE_BLOOM, FALSE_POSITIVE_RATE))),
            nbr_inserted: <_>::default(),
            focus,
            _phantom: <_>::default(),
        }
    }
}
impl<T, TH, Focus, M> UniqueMutator<T, TH, Focus, M>
where
    T: Clone + 'static,
    TH: Hash,
    M: Mutator<T>,
    Focus: Fn(&T) -> &TH,
    Self: 'static,
{
    #[coverage(off)]
    fn clear_if_needed(&self) {
        if self.nbr_inserted.get() >= SIZE_BLOOM {
            let bitmap = &mut self.uniques.borrow_mut().bitmap;
            bitmap.clear();
            self.nbr_inserted.set(0);
        }
    }
}
impl<T, TH, Focus, M> Mutator<T> for UniqueMutator<T, TH, Focus, M>
where
    T: Clone + 'static,
    TH: Hash,
    M: Mutator<T>,
    Focus: Fn(&T) -> &TH,
    Self: 'static,
{
    type Cache = M::Cache;
    type MutationStep = M::MutationStep;
    type ArbitraryStep = M::ArbitraryStep;
    type UnmutateToken = M::UnmutateToken;

    #[coverage(off)]
    fn initialize(&self) {
        self.mutator.initialize();
    }

    #[coverage(off)]
    fn default_arbitrary_step(&self) -> Self::ArbitraryStep {
        self.mutator.default_arbitrary_step()
    }

    #[coverage(off)]
    fn is_valid(&self, value: &T) -> bool {
        self.mutator.is_valid(value)
    }

    #[coverage(off)]
    fn validate_value(&self, value: &T) -> Option<Self::Cache> {
        self.mutator.validate_value(value)
    }
    #[coverage(off)]
    fn default_mutation_step(&self, value: &T, cache: &Self::Cache) -> Self::MutationStep {
        self.mutator.default_mutation_step(value, cache)
    }

    #[doc(hidden)]
    #[coverage(off)]
    fn global_search_space_complexity(&self) -> f64 {
        self.mutator.global_search_space_complexity()
    }

    #[coverage(off)]
    fn max_complexity(&self) -> f64 {
        self.mutator.max_complexity()
    }

    #[coverage(off)]
    fn min_complexity(&self) -> f64 {
        self.mutator.min_complexity()
    }

    #[coverage(off)]
    fn complexity(&self, value: &T, cache: &Self::Cache) -> f64 {
        self.mutator.complexity(value, cache)
    }

    #[coverage(off)]
    fn ordered_arbitrary(&self, step: &mut Self::ArbitraryStep, max_cplx: f64) -> Option<(T, f64)> {
        self.clear_if_needed();
        loop {
            if let Some((v, cplx)) = self.mutator.ordered_arbitrary(step, max_cplx) {
                let mut uniques = self.uniques.borrow_mut();
                let focused = (self.focus)(&v);
                if uniques.contains(focused) {
                    drop(uniques);
                } else {
                    uniques.insert(focused);
                    let prev_inserted = self.nbr_inserted.get();
                    self.nbr_inserted.set(prev_inserted + 1);
                    return Some((v, cplx));
                }
            } else {
                return None;
            }
        }
    }

    #[coverage(off)]
    fn random_arbitrary(&self, _max_cplx: f64) -> (T, f64) {
        panic!()
    }

    #[coverage(off)]
    fn ordered_mutate(
        &self,
        value: &mut T,
        cache: &mut Self::Cache,
        step: &mut Self::MutationStep,
        subvalue_provider: &dyn crate::SubValueProvider,
        max_cplx: f64,
    ) -> Option<(Self::UnmutateToken, f64)> {
        self.clear_if_needed();
        loop {
            if let Some((t, cplx)) = self
                .mutator
                .ordered_mutate(value, cache, step, subvalue_provider, max_cplx)
            {
                let mut uniques = self.uniques.borrow_mut();
                let focused = (self.focus)(value);
                if uniques.contains(focused) || cplx >= max_cplx {
                    drop(uniques);
                    self.unmutate(value, cache, t);
                } else {
                    uniques.insert(focused);
                    let prev_inserted = self.nbr_inserted.get();
                    self.nbr_inserted.set(prev_inserted + 1);
                    return Some((t, cplx));
                }
            } else {
                return None;
            }
        }
    }

    #[coverage(off)]
    fn random_mutate(&self, value: &mut T, cache: &mut Self::Cache, max_cplx: f64) -> (Self::UnmutateToken, f64) {
        self.clear_if_needed();
        for _ in 0..20 {
            let (t, cplx) = self.mutator.random_mutate(value, cache, max_cplx);
            let mut uniques = self.uniques.borrow_mut();
            let focused = (self.focus)(value);
            if uniques.contains(focused) || cplx >= max_cplx {
                drop(uniques);
                self.unmutate(value, cache, t);
            } else {
                uniques.insert(focused);
                let prev_inserted = self.nbr_inserted.get();
                self.nbr_inserted.set(prev_inserted + 1);
                return (t, cplx);
            }
        }
        self.mutator.random_mutate(value, cache, max_cplx)
    }

    #[coverage(off)]
    fn unmutate(&self, value: &mut T, cache: &mut Self::Cache, t: Self::UnmutateToken) {
        self.mutator.unmutate(value, cache, t)
    }

    #[coverage(off)]
    fn visit_subvalues<'a>(&self, value: &'a T, cache: &'a Self::Cache, visit: &mut dyn FnMut(&'a dyn Any, f64)) {
        self.mutator.visit_subvalues(value, cache, visit)
    }
}
