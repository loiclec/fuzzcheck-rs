use crate::DefaultMutator;
use fastrand::Rng;
use fuzzcheck_traits::Mutator;

use std::ops::Range;
use std::{iter::repeat, marker::PhantomData};

pub struct VecMutator<T: Clone, M: Mutator<T>> {
    pub rng: Rng,
    pub m: M,
    _phantom: PhantomData<T>,
}
impl<T: Clone, M: Mutator<T>> VecMutator<T, M> {
    pub fn new(mutator: M) -> Self {
        Self {
            rng: Rng::default(),
            m: mutator,
            _phantom: <_>::default(),
        }
    }
}
impl<T: Clone, M: Mutator<T>> Default for VecMutator<T, M>
where
    M: Default,
{
    fn default() -> Self {
        Self {
            rng: Rng::default(),
            m: M::default(),
            _phantom: <_>::default(),
        }
    }
}
impl<T: Clone> DefaultMutator for Vec<T>
where
    T: DefaultMutator,
{
    type Mutator = VecMutator<T, <T as DefaultMutator>::Mutator>;
    fn default_mutator() -> Self::Mutator {
        Self::Mutator::new(T::default_mutator())
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
    Remove(usize, f64),
    RemoveMany(Range<usize>, f64),
    Insert(usize, T, M::Cache),
    InsertMany(usize, Vec<T>, <VecMutator<T, M> as Mutator<Vec<T>>>::Cache),
    Replace(Vec<T>, <VecMutator<T, M> as Mutator<Vec<T>>>::Cache),
    Nothing,
}

impl<T: Clone, M: Mutator<T>> VecMutator<T, M> {
    fn mutate_element(
        &self,
        value: &mut Vec<T>,
        cache: &mut VecMutatorCache<M::Cache>,
        step: &mut MutationStep<M::MutationStep>,
        idx: usize,
        spare_cplx: f64,
    ) -> Option<UnmutateVecToken<T, M>> {
        let el = &mut value[idx];
        let el_cache = &mut cache.inner[idx];
        let el_step = &mut step.inner[idx];

        let old_cplx = self.m.complexity(&el, el_cache);

        if let Some(token) = self.m.ordered_mutate(el, el_cache, el_step, spare_cplx + old_cplx) {
            let new_cplx = self.m.complexity(&el, el_cache);
            cache.sum_cplx += new_cplx - old_cplx;
            Some(UnmutateVecToken::Element(idx, token, old_cplx - new_cplx))
        } else {
            None
        }
    }

    fn insert_element(
        &self,
        value: &mut Vec<T>,
        cache: &mut VecMutatorCache<M::Cache>,
        spare_cplx: f64,
    ) -> Option<UnmutateVecToken<T, M>> {
        let idx = if value.is_empty() {
            0
        } else {
            self.rng.usize(0..value.len())
        };

        let (el, el_cache) = self.m.random_arbitrary(spare_cplx);
        let el_cplx = self.m.complexity(&el, &el_cache);

        value.insert(idx, el);
        cache.inner.insert(idx, el_cache);

        let token = UnmutateVecToken::Remove(idx, el_cplx);

        cache.sum_cplx += el_cplx;

        Some(token)
    }

    fn remove_element(
        &self,
        value: &mut Vec<T>,
        cache: &mut VecMutatorCache<M::Cache>,
    ) -> Option<UnmutateVecToken<T, M>> {
        if value.is_empty() {
            return None;
        }

        let idx = self.rng.usize(0..value.len());

        let el = &value[idx];
        let el_cplx = self.m.complexity(&el, &cache.inner[idx]);

        let removed_el = value.remove(idx);
        let removed_el_cache = cache.inner.remove(idx);

        let token = UnmutateVecToken::Insert(idx, removed_el, removed_el_cache);

        cache.sum_cplx -= el_cplx;

        Some(token)
    }

    fn remove_many_elements(
        &self,
        value: &mut Vec<T>,
        cache: &mut VecMutatorCache<M::Cache>,
    ) -> Option<UnmutateVecToken<T, M>> {
        if value.is_empty() {
            return None;
        }
        let start_idx = if value.len() == 1 {
            0
        } else {
            self.rng.usize(0..value.len() - 1)
        };
        let end_idx = std::cmp::min(value.len(), start_idx + self.rng.usize(1..10));
        let (removed_elements, removed_cache) = {
            let removed_elements: Vec<_> = value.drain(start_idx..end_idx).collect();
            let removed_cache: Vec<_> = cache.inner.drain(start_idx..end_idx).collect();
            (removed_elements, removed_cache)
        };
        let removed_els_cplx = removed_elements
            .iter()
            .zip(removed_cache.iter())
            .fold(0.0, |cplx, (v, c)| self.m.complexity(&v, &c) + cplx);

        let removed_cache = VecMutatorCache {
            inner: removed_cache,
            sum_cplx: removed_els_cplx,
        };

        let token = UnmutateVecToken::InsertMany(start_idx, removed_elements, removed_cache);

        cache.sum_cplx -= removed_els_cplx;

        Some(token)
    }

    fn insert_repeated_elements(
        &self,
        value: &mut Vec<T>,
        cache: &mut VecMutatorCache<M::Cache>,
        spare_cplx: f64,
    ) -> Option<UnmutateVecToken<T, M>> {
        if spare_cplx < 0.01 {
            return None;
        }

        let idx = if value.is_empty() {
            0
        } else {
            self.rng.usize(0..value.len())
        };

        let target_cplx = crate::gen_f64(
            &self.rng,
            0.0..crate::gen_f64(
                &self.rng,
                0.0..crate::gen_f64(&self.rng, 0.0..crate::gen_f64(&self.rng, 0.0..spare_cplx)),
            ),
        );
        let (min_length, max_length) = self.choose_slice_length(target_cplx);
        let min_length = min_length.unwrap_or(0);

        let len = if min_length >= max_length {
            min_length
        } else {
            self.rng.usize(min_length..max_length)
        };
        if len == 0 {
            // TODO: maybe that shouldn't happen under normal circumstances?
            return None;
        }
        // println!("len: {:.2}", len);
        // println!("max_cplx: {:.2}", target_cplx / (len as f64));
        let (el, el_cache) = self.m.random_arbitrary(target_cplx / (len as f64));
        let el_cplx = self.m.complexity(&el, &el_cache);

        insert_many(value, idx, repeat(el).take(len));
        insert_many(&mut cache.inner, idx, repeat(el_cache).take(len));

        let added_cplx = el_cplx * (len as f64);
        cache.sum_cplx += added_cplx;

        let token = UnmutateVecToken::RemoveMany(idx..(idx + len), added_cplx);

        Some(token)
    }

    fn choose_slice_length(&self, target_cplx: f64) -> (Option<usize>, usize) {
        let min_cplx_el = self.m.min_complexity();

        // slight underestimate of the maximum number of elements required to produce an input of max_cplx
        let max_len_most_complex = {
            let overestimated_max_len: f64 = target_cplx / min_cplx_el;
            let max_len = if overestimated_max_len.is_infinite() {
                // min_cplx_el is 0, so the max length is the maximum complexity of the length component of the vector
                crate::cplxity_to_size(target_cplx)
            } else {
                // an underestimate of the true max_length, but not by much
                (overestimated_max_len - overestimated_max_len.log2()) as usize
            };
            if max_len > 10_000 {
                /* TODO */
                // 10_000?
                target_cplx.trunc() as usize
            } else {
                max_len
            }
        };
        let max_cplx_el = self.m.max_complexity();
        // slight underestimate of the minimum number of elements required to produce an input of max_cplx
        // will be inf. if elements can be of infinite complexity
        // or if elements are of max_cplx 0.0
        let min_len_most_complex = target_cplx / max_cplx_el - (target_cplx / max_cplx_el).log2();

        // arbitrary restriction on the length of the generated number, to avoid creating absurdly large vectors
        // of very simple elements, that take up too much memory
        let max_len_most_complex = if max_len_most_complex > 10_000 {
            /* TODO */
            // 10_000?
            target_cplx.trunc() as usize
        } else {
            max_len_most_complex
        };

        if !min_len_most_complex.is_finite() {
            (None, max_len_most_complex)
        } else {
            let min_len_most_complex = min_len_most_complex.trunc() as usize;
            (Some(min_len_most_complex), max_len_most_complex)
        }
    }

    fn new_input_with_length_and_complexity(
        &self,
        target_len: usize,
        target_cplx: f64,
    ) -> (Vec<T>, <Self as Mutator<Vec<T>>>::Cache) {
        // TODO: create a new_input_with_complexity method
        let mut v = Vec::with_capacity(target_len);
        let mut cache = VecMutatorCache {
            inner: Vec::with_capacity(target_len),
            sum_cplx: 0.0,
        };

        let mut remaining_cplx = target_cplx;
        for i in 0..target_len {
            let max_cplx_element = remaining_cplx / ((target_len - i) as f64);
            let min_cplx_el = self.m.min_complexity();
            if min_cplx_el >= max_cplx_element {
                break;
            }
            let cplx_element = crate::gen_f64(&self.rng, min_cplx_el..max_cplx_element);
            let (x, x_cache) = self.m.random_arbitrary(cplx_element);
            let x_cplx = self.m.complexity(&x, &x_cache);
            v.push(x);
            cache.inner.push(x_cache);
            cache.sum_cplx += x_cplx;
            remaining_cplx -= x_cplx;
        }
        (v, cache)
    }
}

impl<T: Clone, M: Mutator<T>> Mutator<Vec<T>> for VecMutator<T, M> {
    type Cache = VecMutatorCache<M::Cache>;
    type MutationStep = MutationStep<M::MutationStep>;
    type ArbitraryStep = bool; // false: check empty vector, true: random
    type UnmutateToken = UnmutateVecToken<T, M>;

    fn cache_from_value(&self, value: &Vec<T>) -> Self::Cache {
        let inner: Vec<_> = value.iter().map(|x| self.m.cache_from_value(&x)).collect();

        let sum_cplx = value
            .iter()
            .zip(inner.iter())
            .fold(0.0, |cplx, (v, cache)| cplx + self.m.complexity(&v, cache));

        VecMutatorCache { inner, sum_cplx }
    }

    fn initial_step_from_value(&self, value: &Vec<T>) -> Self::MutationStep {
        let inner: Vec<_> = value.iter().map(|x| self.m.initial_step_from_value(&x)).collect();
        MutationStep { inner, element_step: 0 }
    }

    fn max_complexity(&self) -> f64 {
        std::f64::INFINITY
    }

    fn min_complexity(&self) -> f64 {
        1.0
    }

    fn complexity(&self, value: &Vec<T>, cache: &Self::Cache) -> f64 {
        1.0 + cache.sum_cplx + crate::size_to_cplxity(value.len() + 1)
    }

    fn ordered_arbitrary(&self, step: &mut Self::ArbitraryStep, max_cplx: f64) -> Option<(Vec<T>, Self::Cache)> {
        if max_cplx < self.min_complexity() {
            return None;
        }
        if !*step || max_cplx <= 1.0 {
            *step = true;
            return Some((<_>::default(), Self::Cache::default()));
        } else {
            return Some(self.random_arbitrary(max_cplx));
        }
    }

    fn random_arbitrary(&self, max_cplx: f64) -> (Vec<T>, Self::Cache) {
        let target_cplx = fastrand::f64() * crate::gen_f64(&self.rng, 0.0..max_cplx);
        let lengths = self.choose_slice_length(target_cplx);

        if lengths.0.is_none() && self.m.max_complexity() < 0.001 {
            // distinguish between the case where elements are of max_cplx 0 and the case where they are of max_cplx inf.
            // in this case, the elements are always of cplx 0, so we can only vary the length of the vector
            // that's not true!!!
            if lengths.1 <= 0 {
                return (<_>::default(), Self::Cache::default());
            }
            assert!(lengths.1 > 0);
            let len = self.rng.usize(0..lengths.1);
            let (el, el_cache) = self.m.random_arbitrary(0.0);
            let v = repeat(el).take(len).collect();
            let cache = Self::Cache {
                inner: repeat(el_cache).take(len).collect(),
                sum_cplx: 0.0,
            };
            return (v, cache);
        }
        let (min_length, max_length) = (lengths.0.unwrap_or(0), lengths.1);

        // choose a length between min_len_most_complex and max_len_most_complex
        let target_len = if min_length >= max_length {
            min_length
        } else {
            self.rng.usize(min_length..max_length)
        };

        self.new_input_with_length_and_complexity(target_len, target_cplx)
    }

    fn ordered_mutate(
        &self,
        value: &mut Vec<T>,
        cache: &mut Self::Cache,
        step: &mut Self::MutationStep,
        max_cplx: f64,
    ) -> Option<Self::UnmutateToken> {
        // Some(self.random_mutate(value, cache, max_cplx))
        if max_cplx < self.min_complexity() {
            return None;
        }
        let spare_cplx = max_cplx - self.complexity(value, cache);

        let token = if value.is_empty() || self.rng.usize(0..20) == 0 {
            // vector mutation
            match self.rng.usize(0..10) {
                0..=3 => self.insert_element(value, cache, spare_cplx),
                4..=7 => self.remove_element(value, cache),
                8 => self.insert_repeated_elements(value, cache, spare_cplx),
                9 => self.remove_many_elements(value, cache),
                _ => unreachable!(),
            }
        } else {
            // element mutation
            let idx = step.element_step % value.len();
            step.element_step += 1;
            self.mutate_element(value, cache, step, idx, spare_cplx)
        };
        if let Some(token) = token {
            Some(token)
        } else {
            Some(self.random_mutate(value, cache, max_cplx))
        }
    }

    fn random_mutate(&self, value: &mut Vec<T>, cache: &mut Self::Cache, max_cplx: f64) -> Self::UnmutateToken {
        let spare_cplx = max_cplx - self.complexity(value, cache);

        if value.is_empty() || self.rng.usize(0..10) == 0 {
            // vector mutation
            match self.rng.usize(0..10) {
                0..=3 => self.insert_element(value, cache, spare_cplx),
                4..=7 => self.remove_element(value, cache),
                8 => self.insert_repeated_elements(value, cache, spare_cplx),
                9 => self.remove_many_elements(value, cache),
                _ => unreachable!(),
            }
            .unwrap_or_else(|| self.random_mutate(value, cache, max_cplx))
        } else {
            // element mutation
            let idx = self.rng.usize(0..value.len());
            let el = &mut value[idx];
            let el_cache = &mut cache.inner[idx];

            let old_el_cplx = self.m.complexity(&el, el_cache);
            let token = self.m.random_mutate(el, el_cache, spare_cplx + old_el_cplx);

            let new_el_cplx = self.m.complexity(&el, el_cache);
            cache.sum_cplx += new_el_cplx - old_el_cplx;
            UnmutateVecToken::Element(idx, token, old_el_cplx - new_el_cplx)
        }
    }

    fn unmutate(&self, value: &mut Vec<T>, cache: &mut Self::Cache, t: Self::UnmutateToken) {
        match t {
            UnmutateVecToken::Element(idx, inner_t, diff_cplx) => {
                let el = &mut value[idx];
                let el_cache = &mut cache.inner[idx];
                self.m.unmutate(el, el_cache, inner_t);
                cache.sum_cplx += diff_cplx;
            }
            UnmutateVecToken::Insert(idx, el, el_cache) => {
                cache.sum_cplx += self.m.complexity(&el, &el_cache);

                value.insert(idx, el);
                cache.inner.insert(idx, el_cache);
            }
            UnmutateVecToken::Remove(idx, el_cplx) => {
                value.remove(idx);
                cache.inner.remove(idx);
                cache.sum_cplx -= el_cplx;
            }
            UnmutateVecToken::Replace(new_value, new_cache) => {
                // M::ValueConversion::replace(value, new_value);
                let _ = std::mem::replace(value, new_value);
                let _ = std::mem::replace(cache, new_cache);
            }
            UnmutateVecToken::InsertMany(idx, v, c) => {
                insert_many(value, idx, v.into_iter());
                insert_many(&mut cache.inner, idx, c.inner.into_iter());
                let added_cplx = c.sum_cplx;
                cache.sum_cplx += added_cplx;
            }
            UnmutateVecToken::RemoveMany(range, cplx) => {
                value.drain(range.clone());
                cache.inner.drain(range);
                cache.sum_cplx -= cplx;
            }
            UnmutateVecToken::Nothing => {}
        }
    }
}

fn insert_many<T>(v: &mut Vec<T>, idx: usize, iter: impl Iterator<Item = T>) {
    let moved_slice = v.drain(idx..).collect::<Vec<T>>().into_iter();
    v.extend(iter);
    v.extend(moved_slice);
}
