use crate::Mutator;
use std::{
    cell::{Cell, RefCell},
    collections::{hash_map::DefaultHasher, HashMap, HashSet},
    hash::{Hash, Hasher},
    marker::PhantomData,
    rc::Rc,
};

pub struct UniqueMutator<T, M>
where
    T: Clone + Hash + 'static,
    M: Mutator<T>,
{
    mutator: M,
    uniques: Rc<RefCell<HashMap<u64, u64>>>,
    max_repetition: Cell<u64>,
    rng: fastrand::Rng,
    _phantom: PhantomData<T>,
}

impl<T, M> UniqueMutator<T, M>
where
    T: Clone + Hash + 'static,
    M: Mutator<T>,
{
    pub fn new(mutator: M) -> Self {
        Self {
            mutator,
            uniques: <_>::default(),
            max_repetition: <_>::default(),
            rng: <_>::default(),
            _phantom: <_>::default(),
        }
    }
}

impl<T, M> Mutator<T> for UniqueMutator<T, M>
where
    T: Clone + Hash + 'static,
    M: Mutator<T>,
{
    type Cache = M::Cache;
    type MutationStep = M::MutationStep;
    type ArbitraryStep = M::ArbitraryStep;
    type UnmutateToken = M::UnmutateToken;
    type LensPath = M::LensPath;
    fn default_arbitrary_step(&self) -> Self::ArbitraryStep {
        self.mutator.default_arbitrary_step()
    }
    fn validate_value(&self, value: &T) -> Option<Self::Cache> {
        self.mutator.validate_value(value)
    }
    fn default_mutation_step(&self, value: &T, cache: &Self::Cache) -> Self::MutationStep {
        self.mutator.default_mutation_step(value, cache)
    }
    fn max_complexity(&self) -> f64 {
        self.mutator.max_complexity()
    }

    fn min_complexity(&self) -> f64 {
        self.mutator.min_complexity()
    }

    fn complexity(&self, value: &T, cache: &Self::Cache) -> f64 {
        self.mutator.complexity(value, cache)
    }

    fn ordered_arbitrary(&self, step: &mut Self::ArbitraryStep, max_cplx: f64) -> Option<(T, f64)> {
        if self.rng.usize(..10000) == 0 {
            let uniques = self.uniques.borrow_mut();
            println!(
                "oa: nbr uniques: {}, max_rep: {}",
                uniques.len(),
                self.max_repetition.get()
            );
        }
        if let Some((v, cplx)) = self.mutator.ordered_arbitrary(step, max_cplx) {
            let mut hasher = DefaultHasher::default();
            v.hash(&mut hasher);
            let h = hasher.finish();
            let mut uniques = self.uniques.borrow_mut();
            if uniques.contains_key(&h) {
                let rep = uniques.get_mut(&h).unwrap();
                *rep += 1;
                let max_rep = self.max_repetition.get();
                if *rep > max_rep {
                    self.max_repetition.set(*rep);
                }
                drop(uniques);
                // self.ordered_arbitrary(step, max_cplx)
                Some((v, cplx))
            } else {
                uniques.insert(h, 0);
                Some((v, cplx))
            }
        } else {
            None
        }
    }

    fn random_arbitrary(&self, max_cplx: f64) -> (T, f64) {
        if self.rng.usize(..10000) == 0 {
            let uniques = self.uniques.borrow_mut();
            println!(
                "ra: nbr uniques: {}, max_rep: {}",
                uniques.len(),
                self.max_repetition.get()
            );
        }
        let (v, cplx) = self.mutator.random_arbitrary(max_cplx);
        let mut hasher = DefaultHasher::default();
        v.hash(&mut hasher);
        let h = hasher.finish();
        let mut uniques = self.uniques.borrow_mut();
        if uniques.contains_key(&h) {
            let rep = uniques.get_mut(&h).unwrap();
            *rep += 1;
            let max_rep = self.max_repetition.get();
            if *rep > max_rep {
                self.max_repetition.set(*rep);
            }
            drop(uniques);
            // self.random_arbitrary(max_cplx)
            (v, cplx)
        } else {
            uniques.insert(h, 0);
            (v, cplx)
        }
    }

    fn ordered_mutate(
        &self,
        value: &mut T,
        cache: &mut Self::Cache,
        step: &mut Self::MutationStep,
        max_cplx: f64,
    ) -> Option<(Self::UnmutateToken, f64)> {
        if self.rng.usize(..10000) == 0 {
            let uniques = self.uniques.borrow_mut();
            println!(
                "om: nbr uniques: {}, max_rep: {}",
                uniques.len(),
                self.max_repetition.get()
            );
        }
        if let Some((t, cplx)) = self.mutator.ordered_mutate(value, cache, step, max_cplx) {
            let mut hasher = DefaultHasher::default();
            value.hash(&mut hasher);
            let h = hasher.finish();
            let mut uniques = self.uniques.borrow_mut();
            if uniques.contains_key(&h) {
                let rep = uniques.get_mut(&h).unwrap();
                *rep += 1;
                let max_rep = self.max_repetition.get();
                if *rep > max_rep {
                    self.max_repetition.set(*rep);
                }
                drop(uniques);
                self.unmutate(value, cache, t);
                self.ordered_mutate(value, cache, step, max_cplx)
                // Some((t, cplx))
            } else {
                uniques.insert(h, 0);
                let new_cache = self.validate_value(value).unwrap();
                let new_cplx = self.complexity(value, &new_cache);
                assert!(cplx == new_cplx, "{cplx:.2} {new_cplx:.2}");
                // assert!(cplx < max_cplx);
                Some((t, cplx))
            }
        } else {
            None
        }
    }

    fn random_mutate(&self, value: &mut T, cache: &mut Self::Cache, max_cplx: f64) -> (Self::UnmutateToken, f64) {
        if self.rng.usize(..10000) == 0 {
            let uniques = self.uniques.borrow_mut();
            println!("rm: nbr uniques: {}", uniques.len());
        }
        let (t, cplx) = self.mutator.random_mutate(value, cache, max_cplx);
        let mut hasher = DefaultHasher::default();
        value.hash(&mut hasher);
        let h = hasher.finish();
        let mut uniques = self.uniques.borrow_mut();
        if uniques.contains_key(&h) {
            let rep = uniques.get_mut(&h).unwrap();
            *rep += 1;
            let max_rep = self.max_repetition.get();
            if *rep > max_rep {
                self.max_repetition.set(*rep);
            }
            drop(uniques);
            // self.unmutate(value, cache, t);
            // self.random_mutate(value, cache, max_cplx)
            (t, cplx)
        } else {
            uniques.insert(h, 0);
            (t, cplx)
        }
    }

    fn unmutate(&self, value: &mut T, cache: &mut Self::Cache, t: Self::UnmutateToken) {
        self.mutator.unmutate(value, cache, t)
    }

    fn lens<'a>(&self, value: &'a T, cache: &'a Self::Cache, path: &Self::LensPath) -> &'a dyn std::any::Any {
        self.mutator.lens(value, cache, path)
    }

    fn all_paths(
        &self,
        value: &T,
        cache: &Self::Cache,
        register_path: &mut dyn FnMut(std::any::TypeId, Self::LensPath),
    ) {
        self.mutator.all_paths(value, cache, register_path)
    }

    fn crossover_mutate(
        &self,
        value: &mut T,
        cache: &mut Self::Cache,
        subvalue_provider: &dyn crate::SubValueProvider,
        max_cplx: f64,
    ) -> (Self::UnmutateToken, f64) {
        if self.rng.usize(..10000) == 0 {
            let uniques = self.uniques.borrow_mut();
            println!(
                "CM: nbr uniques: {}, max_rep: {}",
                uniques.len(),
                self.max_repetition.get()
            );
        }
        let (t, cplx) = self.mutator.crossover_mutate(value, cache, subvalue_provider, max_cplx);
        let mut hasher = DefaultHasher::default();
        value.hash(&mut hasher);
        let h = hasher.finish();
        let mut uniques = self.uniques.borrow_mut();
        if uniques.contains_key(&h) {
            let rep = uniques.get_mut(&h).unwrap();
            *rep += 1;
            let max_rep = self.max_repetition.get();
            if *rep > max_rep {
                self.max_repetition.set(*rep);
            }
            drop(uniques);
            self.unmutate(value, cache, t);
            self.random_mutate(value, cache, max_cplx)
            // self.crossover_mutate(value, cache, subvalue_provider, max_cplx)
            // (t, cplx)
        } else {
            uniques.insert(h, 0);
            (t, cplx)
        }
    }
}
