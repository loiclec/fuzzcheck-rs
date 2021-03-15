use fastrand::Rng;
use fuzzcheck_traits::Mutator;

use std::marker::PhantomData;

pub struct FixedLenVecMutator<T: Clone, M: Mutator<T>> {
    pub rng: Rng,
    mutators: Vec<M>,
    min_complexity: f64,
    max_complexity: f64,
    _phantom: PhantomData<T>,
}
impl<T: Clone, M: Mutator<T> + Clone> FixedLenVecMutator<T, M> {
    pub fn new_with_repeated_mutator(mutator: M, len: usize) -> Self {
        Self::new(std::iter::repeat(mutator).take(len).collect())
    }
}

impl<T: Clone, M: Mutator<T>> FixedLenVecMutator<T, M> {
    pub fn new(mutators: Vec<M>) -> Self {
        let max_complexity =
            crate::size_to_cplxity(mutators.len() + 1) + mutators.iter().fold(0.0, |cplx, m| cplx + m.max_complexity());
        let min_complexity = crate::size_to_cplxity(mutators.len() + 1);
        Self {
            rng: Rng::default(),
            mutators,
            min_complexity,
            max_complexity,
            _phantom: <_>::default(),
        }
    }
}

#[derive(Clone)]
pub struct MutationStep<S> {
    inner: Vec<S>,
    element_step: usize,
}

#[derive(Clone)]
pub struct VecMutatorCache<C> {
    inner: Vec<C>,
    sum_cplx: f64,
}
impl<C> Default for VecMutatorCache<C> {
    fn default() -> Self {
        Self {
            inner: Vec::new(),
            sum_cplx: 0.0,
        }
    }
}

pub enum UnmutateVecToken<T: Clone, M: Mutator<T>> {
    Element(usize, M::UnmutateToken, f64),
}

impl<T: Clone, M: Mutator<T>> FixedLenVecMutator<T, M> {
    fn len(&self) -> usize {
        self.mutators.len()
    }
    fn mutate_element(
        &self,
        value: &mut Vec<T>,
        cache: &mut VecMutatorCache<M::Cache>,
        step: &mut MutationStep<M::MutationStep>,
        idx: usize,
        spare_cplx: f64,
    ) -> Option<UnmutateVecToken<T, M>> {
        let mutator = &self.mutators[idx];
        let el = &mut value[idx];
        let el_cache = &mut cache.inner[idx];
        let el_step = &mut step.inner[idx];

        let old_cplx = mutator.complexity(&el, el_cache);

        if let Some(token) = mutator.ordered_mutate(el, el_cache, el_step, spare_cplx + old_cplx) {
            let new_cplx = mutator.complexity(&el, el_cache);
            cache.sum_cplx += new_cplx - old_cplx;
            Some(UnmutateVecToken::Element(idx, token, old_cplx - new_cplx))
        } else {
            None
        }
    }

    fn new_input_with_complexity(&self, target_cplx: f64) -> (Vec<T>, <Self as Mutator<Vec<T>>>::Cache) {
        let mut v = Vec::with_capacity(self.len());
        let mut cache = VecMutatorCache {
            inner: Vec::with_capacity(self.len()),
            sum_cplx: 0.0,
        };

        let mut remaining_cplx = target_cplx;
        for (i, mutator) in self.mutators.iter().enumerate() {
            let mut max_cplx_element = remaining_cplx / ((self.len() - i) as f64);
            let min_cplx_el = mutator.min_complexity();
            if min_cplx_el >= max_cplx_element {
                max_cplx_element = min_cplx_el;
            }
            let cplx_element = crate::gen_f64(&self.rng, min_cplx_el..max_cplx_element);
            let (x, x_cache) = mutator.random_arbitrary(cplx_element);
            let x_cplx = mutator.complexity(&x, &x_cache);
            v.push(x);
            cache.inner.push(x_cache);
            cache.sum_cplx += x_cplx;
            remaining_cplx -= x_cplx;
        }
        (v, cache)
    }
}

impl<T: Clone, M: Mutator<T>> Mutator<Vec<T>> for FixedLenVecMutator<T, M> {
    type Cache = VecMutatorCache<M::Cache>;
    type MutationStep = MutationStep<M::MutationStep>;
    type ArbitraryStep = ();
    type UnmutateToken = UnmutateVecToken<T, M>;

    fn cache_from_value(&self, value: &Vec<T>) -> Self::Cache {
        let inner: Vec<_> = value
            .iter()
            .zip(self.mutators.iter())
            .map(|(x, mutator)| mutator.cache_from_value(&x))
            .collect();

        let sum_cplx = value
            .iter()
            .zip(self.mutators.iter())
            .zip(inner.iter())
            .fold(0.0, |cplx, ((v, mutator), cache)| cplx + mutator.complexity(&v, cache));

        VecMutatorCache { inner, sum_cplx }
    }

    fn initial_step_from_value(&self, value: &Vec<T>) -> Self::MutationStep {
        let inner: Vec<_> = value
            .iter()
            .zip(self.mutators.iter())
            .map(|(x, m)| m.initial_step_from_value(&x))
            .collect();
        MutationStep { inner, element_step: 0 }
    }

    fn max_complexity(&self) -> f64 {
        self.max_complexity
    }

    fn min_complexity(&self) -> f64 {
        self.min_complexity
    }

    fn complexity(&self, _value: &Vec<T>, cache: &Self::Cache) -> f64 {
        self.min_complexity + cache.sum_cplx
    }

    fn ordered_arbitrary(&self, _step: &mut Self::ArbitraryStep, mut max_cplx: f64) -> Option<(Vec<T>, Self::Cache)> {
        if max_cplx < self.min_complexity() {
            return None;
        }
        let mutator_max_cplx = self.max_complexity();
        if max_cplx > mutator_max_cplx {
            max_cplx = mutator_max_cplx;
        }
        return Some(self.random_arbitrary(max_cplx));
    }

    fn random_arbitrary(&self, mut max_cplx: f64) -> (Vec<T>, Self::Cache) {
        let mutator_max_cplx = self.max_complexity();
        if max_cplx > mutator_max_cplx {
            max_cplx = mutator_max_cplx;
        }

        let target_cplx = crate::gen_f64(&self.rng, 1.0..max_cplx);

        self.new_input_with_complexity(target_cplx)
    }

    fn ordered_mutate(
        &self,
        value: &mut Vec<T>,
        cache: &mut Self::Cache,
        step: &mut Self::MutationStep,
        mut max_cplx: f64,
    ) -> Option<Self::UnmutateToken> {
        if max_cplx < self.min_complexity() {
            return None;
        }
        let mutator_max_cplx = self.max_complexity();
        if max_cplx > mutator_max_cplx {
            max_cplx = mutator_max_cplx;
        }

        let spare_cplx = max_cplx - self.complexity(value, cache);

        let idx = step.element_step % value.len();
        step.element_step += 1;
        self.mutate_element(value, cache, step, idx, spare_cplx)
            .or_else(|| Some(self.random_mutate(value, cache, max_cplx)))
    }

    fn random_mutate(&self, value: &mut Vec<T>, cache: &mut Self::Cache, mut max_cplx: f64) -> Self::UnmutateToken {
        let mutator_max_cplx = self.max_complexity();
        if max_cplx > mutator_max_cplx {
            max_cplx = mutator_max_cplx;
        }

        let spare_cplx = max_cplx - self.complexity(value, cache);

        let idx = self.rng.usize(0..value.len());
        let el = &mut value[idx];
        let el_cache = &mut cache.inner[idx];

        let old_el_cplx = self.mutators[idx].complexity(&el, el_cache);
        let token = self.mutators[idx].random_mutate(el, el_cache, spare_cplx + old_el_cplx);

        let new_el_cplx = self.mutators[idx].complexity(&el, el_cache);
        cache.sum_cplx += new_el_cplx - old_el_cplx;
        UnmutateVecToken::Element(idx, token, old_el_cplx - new_el_cplx)
    }

    fn unmutate(&self, value: &mut Vec<T>, cache: &mut Self::Cache, t: Self::UnmutateToken) {
        match t {
            UnmutateVecToken::Element(idx, inner_t, diff_cplx) => {
                let el = &mut value[idx];
                let el_cache = &mut cache.inner[idx];
                self.mutators[idx].unmutate(el, el_cache, inner_t);
                cache.sum_cplx += diff_cplx;
            }
        }
    }
}
#[cfg(test)]
mod tests {
    use fuzzcheck_traits::Mutator;

    use super::FixedLenVecMutator;
    use crate::U8Mutator;
    #[test]
    fn test_constrained_length_mutator() {
        let m = FixedLenVecMutator::<u8, U8Mutator>::new_with_repeated_mutator(U8Mutator::default(), 3);
        for _ in 0..100 {
            let (x, _) = m.ordered_arbitrary(&mut (), 800.0).unwrap();
            eprintln!("{:?}", x);
        }
    }
}
