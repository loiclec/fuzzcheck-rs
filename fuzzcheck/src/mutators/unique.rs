use crate::{bloom_filter::BloomFilter, Mutator};
use std::{
    cell::{Cell, RefCell},
    hash::Hash,
    marker::PhantomData,
    rc::Rc,
};

const SIZE_BLOOM: usize = 10_000_000;
const FALSE_POSITIVE_RATE: f64 = 0.000_001;

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
    rng: fastrand::Rng,
    _phantom: PhantomData<T>,
}

impl<T, TH, Focus, M> UniqueMutator<T, TH, Focus, M>
where
    T: Clone + 'static,
    TH: Hash,
    M: Mutator<T>,
    Focus: Fn(&T) -> &TH,
{
    #[no_coverage]
    pub fn new(mutator: M, focus: Focus) -> Self {
        Self {
            mutator,
            uniques: Rc::new(RefCell::new(BloomFilter::new(SIZE_BLOOM, FALSE_POSITIVE_RATE))),
            nbr_inserted: <_>::default(),
            focus,
            rng: <_>::default(),
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
    #[no_coverage]
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
    type LensPath = M::LensPath;
    #[no_coverage]
    fn default_arbitrary_step(&self) -> Self::ArbitraryStep {
        self.mutator.default_arbitrary_step()
    }
    #[no_coverage]
    fn validate_value(&self, value: &T) -> Option<Self::Cache> {
        self.mutator.validate_value(value)
    }
    #[no_coverage]
    fn default_mutation_step(&self, value: &T, cache: &Self::Cache) -> Self::MutationStep {
        self.mutator.default_mutation_step(value, cache)
    }
    #[no_coverage]
    fn max_complexity(&self) -> f64 {
        self.mutator.max_complexity()
    }

    #[no_coverage]
    fn min_complexity(&self) -> f64 {
        self.mutator.min_complexity()
    }

    #[no_coverage]
    fn complexity(&self, value: &T, cache: &Self::Cache) -> f64 {
        self.mutator.complexity(value, cache)
    }

    #[no_coverage]
    fn ordered_arbitrary(&self, step: &mut Self::ArbitraryStep, max_cplx: f64) -> Option<(T, f64)> {
        self.clear_if_needed();
        loop {
            if let Some((v, cplx)) = self.mutator.ordered_arbitrary(step, max_cplx) {
                let mut uniques = self.uniques.borrow_mut();
                let focused = (self.focus)(&v);
                if uniques.contains(&focused) {
                    drop(uniques);
                } else {
                    uniques.insert(&focused);
                    let prev_inserted = self.nbr_inserted.get();
                    self.nbr_inserted.set(prev_inserted + 1);
                    return Some((v, cplx));
                }
            } else {
                return None;
            }
        }
    }

    #[no_coverage]
    fn random_arbitrary(&self, _max_cplx: f64) -> (T, f64) {
        panic!()
    }

    #[no_coverage]
    fn ordered_mutate(
        &self,
        value: &mut T,
        cache: &mut Self::Cache,
        step: &mut Self::MutationStep,
        max_cplx: f64,
    ) -> Option<(Self::UnmutateToken, f64)> {
        self.clear_if_needed();
        loop {
            if let Some((t, cplx)) = self.mutator.ordered_mutate(value, cache, step, max_cplx) {
                let mut uniques = self.uniques.borrow_mut();
                let focused = (self.focus)(value);
                if uniques.contains(&focused) || cplx >= max_cplx {
                    drop(uniques);
                    self.unmutate(value, cache, t);
                } else {
                    uniques.insert(&focused);
                    let prev_inserted = self.nbr_inserted.get();
                    self.nbr_inserted.set(prev_inserted + 1);
                    return Some((t, cplx));
                }
            } else {
                return None;
            }
        }
    }

    #[no_coverage]
    fn random_mutate(&self, value: &mut T, cache: &mut Self::Cache, max_cplx: f64) -> (Self::UnmutateToken, f64) {
        self.clear_if_needed();
        for _ in 0..20 {
            let (t, cplx) = self.mutator.random_mutate(value, cache, max_cplx);
            let mut uniques = self.uniques.borrow_mut();
            let focused = (self.focus)(value);
            if uniques.contains(&focused) || cplx >= max_cplx {
                drop(uniques);
                self.unmutate(value, cache, t);
            } else {
                uniques.insert(&focused);
                let prev_inserted = self.nbr_inserted.get();
                self.nbr_inserted.set(prev_inserted + 1);
                return (t, cplx);
            }
        }
        self.mutator.random_mutate(value, cache, max_cplx)
    }

    #[no_coverage]
    fn unmutate(&self, value: &mut T, cache: &mut Self::Cache, t: Self::UnmutateToken) {
        self.mutator.unmutate(value, cache, t)
    }

    #[no_coverage]
    fn lens<'a>(&self, value: &'a T, cache: &'a Self::Cache, path: &Self::LensPath) -> &'a dyn std::any::Any {
        self.mutator.lens(value, cache, path)
    }

    #[no_coverage]
    fn all_paths(
        &self,
        value: &T,
        cache: &Self::Cache,
        register_path: &mut dyn FnMut(std::any::TypeId, Self::LensPath),
    ) {
        self.mutator.all_paths(value, cache, register_path)
    }

    #[no_coverage]
    fn crossover_mutate(
        &self,
        value: &mut T,
        cache: &mut Self::Cache,
        subvalue_provider: &dyn crate::SubValueProvider,
        max_cplx: f64,
    ) -> (Self::UnmutateToken, f64) {
        self.clear_if_needed();
        for _ in 0..20 {
            let (t, cplx) = self.mutator.crossover_mutate(value, cache, subvalue_provider, max_cplx);
            let mut uniques = self.uniques.borrow_mut();
            let focused = (self.focus)(value);
            if uniques.contains(&focused) || cplx >= max_cplx {
                drop(uniques);
                self.unmutate(value, cache, t);
            } else {
                uniques.insert(&focused);
                let prev_inserted = self.nbr_inserted.get();
                self.nbr_inserted.set(prev_inserted + 1);
                return (t, cplx);
            }
        }
        self.random_mutate(value, cache, max_cplx)
    }
}
