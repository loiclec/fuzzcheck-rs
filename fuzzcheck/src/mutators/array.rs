use std::any::Any;
use std::cell::Cell;
use std::marker::PhantomData;

use fastrand::Rng;

use crate::{DefaultMutator, Mutator};

/// A mutator for fixed-size arrays `[T; N]`.
pub struct ArrayMutator<M, T, const N: usize>
where
    T: Clone + 'static,
    M: Mutator<T>,
{
    mutator: M,
    initialized: Cell<bool>,
    min_complexity: Cell<f64>,
    max_complexity: Cell<f64>,
    pub rng: Rng,
    _phantom: PhantomData<T>,
}

impl<M, T, const N: usize> ArrayMutator<M, T, N>
where
    T: Clone + 'static,
    M: Mutator<T>,
{
    #[coverage(off)]
    pub fn new(mutator: M) -> Self {
        Self {
            mutator,
            initialized: Cell::new(false),
            min_complexity: Cell::new(std::f64::INFINITY),
            max_complexity: Cell::default(),
            rng: Rng::default(),
            _phantom: PhantomData,
        }
    }
}

impl<M, T, const N: usize> DefaultMutator for [T; N]
where
    T: 'static + DefaultMutator<Mutator = M> + Clone,
    M: Mutator<T>,
{
    type Mutator = ArrayMutator<M, T, N>;

    #[coverage(off)]
    fn default_mutator() -> Self::Mutator {
        Self::Mutator::new(T::default_mutator())
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MutationStep<S> {
    inner: Vec<S>,
    element_step: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ArrayMutatorCache<C> {
    inner: Vec<C>,
    sum_cplx: f64,
}
impl<C> Default for ArrayMutatorCache<C> {
    #[coverage(off)]
    fn default() -> Self {
        Self {
            inner: Vec::new(),
            sum_cplx: 0.0,
        }
    }
}

pub enum UnmutateArrayToken<M: Mutator<T>, T: Clone + 'static, const N: usize> {
    Element(usize, M::UnmutateToken),
    Elements(Vec<(usize, M::UnmutateToken)>),
    Replace([T; N]),
}

impl<M: Mutator<T>, T: Clone + 'static, const N: usize> ArrayMutator<M, T, N> {
    #[coverage(off)]
    fn len(&self) -> usize {
        N
    }
    #[coverage(off)]
    fn mutate_elements(
        &self,
        value: &mut [T; N],
        cache: &mut ArrayMutatorCache<M::Cache>,
        idcs: &[usize],
        current_cplx: f64,
        max_cplx: f64,
    ) -> (UnmutateArrayToken<M, T, N>, f64) {
        let mut cplx = current_cplx;
        let mut tokens = vec![];
        for &idx in idcs {
            let spare_cplx = max_cplx - cplx;
            let mutator = &self.mutator;
            let el = &mut value[idx];
            let el_cache = &mut cache.inner[idx];

            let old_cplx = mutator.complexity(el, el_cache);

            let (token, new_cplx) = mutator.random_mutate(el, el_cache, spare_cplx + old_cplx);
            tokens.push((idx, token));
            cplx = cplx - old_cplx + new_cplx;
        }
        (UnmutateArrayToken::Elements(tokens), cplx)
    }
    #[coverage(off)]
    fn mutate_element(
        &self,
        value: &mut [T; N],
        cache: &mut ArrayMutatorCache<M::Cache>,
        step: &mut MutationStep<M::MutationStep>,
        subvalue_provider: &dyn crate::SubValueProvider,
        idx: usize,
        current_cplx: f64,
        spare_cplx: f64,
    ) -> Option<(UnmutateArrayToken<M, T, N>, f64)> {
        let mutator = &self.mutator;
        let el = &mut value[idx];
        let el_cache = &mut cache.inner[idx];
        let el_step = &mut step.inner[idx];

        let old_cplx = mutator.complexity(el, el_cache);

        if let Some((token, new_cplx)) =
            mutator.ordered_mutate(el, el_cache, el_step, subvalue_provider, spare_cplx + old_cplx)
        {
            Some((
                UnmutateArrayToken::Element(idx, token),
                current_cplx - old_cplx + new_cplx,
            ))
        } else {
            None
        }
    }

    #[coverage(off)]
    fn new_input_with_complexity(&self, target_cplx: f64) -> ([T; N], f64) {
        let mut v = Vec::with_capacity(self.len());
        let mut sum_cplx = 0.0;
        let mut remaining_cplx = target_cplx;
        let mut remaining_min_complexity = self.min_complexity();
        for i in 0..N {
            let mut max_cplx_element = (remaining_cplx / ((self.len() - i) as f64)) - remaining_min_complexity;
            let min_cplx_el = self.mutator.min_complexity();
            if min_cplx_el >= max_cplx_element {
                max_cplx_element = min_cplx_el;
            }
            let (x, x_cplx) = self.mutator.random_arbitrary(max_cplx_element);
            v.push(x);
            sum_cplx += x_cplx;
            remaining_cplx -= x_cplx;
            remaining_min_complexity -= self.mutator.min_complexity();
        }
        (v.try_into().ok().unwrap(), sum_cplx)
    }
}

impl<M: Mutator<T>, T: Clone + 'static, const N: usize> Mutator<[T; N]> for ArrayMutator<M, T, N> {
    #[doc(hidden)]
    type Cache = ArrayMutatorCache<M::Cache>;
    #[doc(hidden)]
    type MutationStep = MutationStep<M::MutationStep>;
    #[doc(hidden)]
    type ArbitraryStep = ();
    #[doc(hidden)]
    type UnmutateToken = UnmutateArrayToken<M, T, N>;

    #[doc(hidden)]
    #[coverage(off)]
    fn initialize(&self) {
        self.mutator.initialize();

        let max_complexity = self.mutator.max_complexity() * N as f64;
        let min_complexity = self.mutator.min_complexity() * N as f64;
        self.min_complexity.set(min_complexity);
        self.max_complexity.set(max_complexity);
        self.initialized.set(true);
    }

    #[doc(hidden)]
    #[coverage(off)]
    fn default_arbitrary_step(&self) -> Self::ArbitraryStep {}

    #[doc(hidden)]
    #[coverage(off)]
    fn is_valid(&self, value: &[T; N]) -> bool {
        for v in value.iter() {
            if !self.mutator.is_valid(v) {
                return false;
            }
        }
        true
    }

    #[doc(hidden)]
    #[coverage(off)]
    fn validate_value(&self, value: &[T; N]) -> Option<Self::Cache> {
        let inner_caches: Vec<_> = value
            .iter()
            .map(
                #[coverage(off)]
                |x| self.mutator.validate_value(x),
            )
            .collect::<Option<_>>()?;

        let sum_cplx = value.iter().zip(inner_caches.iter()).fold(
            0.0,
            #[coverage(off)]
            |cplx, (v, cache)| cplx + self.mutator.complexity(v, cache),
        );

        let cache = ArrayMutatorCache {
            inner: inner_caches,
            sum_cplx,
        };

        Some(cache)
    }

    #[doc(hidden)]
    #[coverage(off)]
    fn default_mutation_step(&self, value: &[T; N], cache: &Self::Cache) -> Self::MutationStep {
        let inner = value
            .iter()
            .zip(cache.inner.iter())
            .map(
                #[coverage(off)]
                |(v, c)| self.mutator.default_mutation_step(v, c),
            )
            .collect::<Vec<_>>();
        MutationStep { inner, element_step: 0 }
    }

    #[doc(hidden)]
    #[coverage(off)]
    fn global_search_space_complexity(&self) -> f64 {
        self.mutator.global_search_space_complexity() * (N as f64)
    }

    #[doc(hidden)]
    #[coverage(off)]
    fn max_complexity(&self) -> f64 {
        self.max_complexity.get()
    }

    #[doc(hidden)]
    #[coverage(off)]
    fn min_complexity(&self) -> f64 {
        self.min_complexity.get()
    }

    #[doc(hidden)]
    #[coverage(off)]
    fn complexity(&self, _value: &[T; N], cache: &Self::Cache) -> f64 {
        cache.sum_cplx
    }

    #[doc(hidden)]
    #[coverage(off)]
    fn ordered_arbitrary(&self, _step: &mut Self::ArbitraryStep, max_cplx: f64) -> Option<([T; N], f64)> {
        if max_cplx < self.min_complexity() {
            return None;
        }
        Some(self.random_arbitrary(max_cplx))
    }

    #[doc(hidden)]
    #[coverage(off)]
    fn random_arbitrary(&self, max_cplx: f64) -> ([T; N], f64) {
        let target_cplx = crate::mutators::gen_f64(&self.rng, 1.0..max_cplx);
        self.new_input_with_complexity(target_cplx)
    }

    #[doc(hidden)]
    #[coverage(off)]
    fn ordered_mutate(
        &self,
        value: &mut [T; N],
        cache: &mut Self::Cache,
        step: &mut Self::MutationStep,
        subvalue_provider: &dyn crate::SubValueProvider,
        max_cplx: f64,
    ) -> Option<(Self::UnmutateToken, f64)> {
        if max_cplx < self.min_complexity() {
            return None;
        }
        if value.is_empty() || self.rng.usize(0..100) == 0 {
            let (mut v, cplx) = self.random_arbitrary(max_cplx);
            std::mem::swap(value, &mut v);
            return Some((UnmutateArrayToken::Replace(v), cplx));
        }
        let current_cplx = self.complexity(value, cache);
        let spare_cplx = max_cplx - current_cplx;
        if value.len() > 1 && self.rng.usize(..20) == 0 {
            let mut idcs = (0..value.len()).collect::<Vec<_>>();
            self.rng.shuffle(&mut idcs);
            let count = self.rng.usize(2..=value.len());
            let idcs = &idcs[..count];
            Some(self.mutate_elements(value, cache, idcs, current_cplx, max_cplx))
        } else {
            let idx = step.element_step % value.len();
            step.element_step += 1;
            self.mutate_element(value, cache, step, subvalue_provider, idx, current_cplx, spare_cplx)
                .or_else(
                    #[coverage(off)]
                    || Some(self.random_mutate(value, cache, max_cplx)),
                )
        }
    }

    #[doc(hidden)]
    #[coverage(off)]
    fn random_mutate(&self, value: &mut [T; N], cache: &mut Self::Cache, max_cplx: f64) -> (Self::UnmutateToken, f64) {
        if value.is_empty() || self.rng.usize(0..100) == 0 {
            let (mut v, cplx) = self.random_arbitrary(max_cplx);
            std::mem::swap(value, &mut v);
            return (UnmutateArrayToken::Replace(v), cplx);
        }
        let current_cplx = self.complexity(value, cache);
        if value.len() > 1 && self.rng.usize(..20) == 0 {
            let mut idcs = (0..value.len()).collect::<Vec<_>>();
            self.rng.shuffle(&mut idcs);
            let count = self.rng.usize(2..=value.len());
            let idcs = &idcs[..count];
            return self.mutate_elements(value, cache, idcs, current_cplx, max_cplx);
        }
        let spare_cplx = max_cplx - current_cplx;

        let idx = self.rng.usize(0..value.len());
        let el = &mut value[idx];
        let el_cache = &mut cache.inner[idx];

        let old_el_cplx = self.mutator.complexity(el, el_cache);
        let (token, new_el_cplx) = self.mutator.random_mutate(el, el_cache, spare_cplx + old_el_cplx);

        (
            UnmutateArrayToken::Element(idx, token),
            current_cplx - old_el_cplx + new_el_cplx,
        )
    }

    #[doc(hidden)]
    #[coverage(off)]
    fn unmutate(&self, value: &mut [T; N], cache: &mut Self::Cache, t: Self::UnmutateToken) {
        match t {
            UnmutateArrayToken::Element(idx, inner_t) => {
                let el = &mut value[idx];
                self.mutator.unmutate(el, &mut cache.inner[idx], inner_t);
            }
            UnmutateArrayToken::Elements(tokens) => {
                for (idx, token) in tokens {
                    let el = &mut value[idx];
                    self.mutator.unmutate(el, &mut cache.inner[idx], token);
                }
            }
            UnmutateArrayToken::Replace(new_value) => {
                let _ = std::mem::replace(value, new_value);
            }
        }
    }

    #[doc(hidden)]
    #[coverage(off)]
    fn visit_subvalues<'a>(&self, value: &'a [T; N], cache: &'a Self::Cache, visit: &mut dyn FnMut(&'a dyn Any, f64)) {
        if !value.is_empty() {
            for idx in 0..value.len() {
                let cplx = self.mutator.complexity(&value[idx], &cache.inner[idx]);
                visit(&value[idx], cplx);
            }
            for (el, el_cache) in value.iter().zip(cache.inner.iter()) {
                self.mutator.visit_subvalues(el, el_cache, visit);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::ArrayMutator;
    use crate::mutators::integer::U8Mutator;
    use crate::Mutator;
    #[test]
    #[coverage(off)]
    fn test_array_mutator() {
        let m = ArrayMutator::<U8Mutator, u8, 32>::new(U8Mutator::default());
        m.initialize();
        for _ in 0..100 {
            let (x, _) = m.ordered_arbitrary(&mut (), 800.0).unwrap();
            eprintln!("{:?}", x);
        }
    }
}
