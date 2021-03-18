use crate::DefaultMutator;
use fastrand::Rng;
use fuzzcheck_traits::Mutator;

use std::{
    cmp,
    ops::{Range, RangeInclusive},
};
use std::{iter::repeat, marker::PhantomData};

pub struct VecMutator<T: Clone, M: Mutator<T>> {
    pub rng: Rng,
    pub m: M,
    pub len_range: RangeInclusive<usize>,
    _phantom: PhantomData<T>,
}
impl<T: Clone, M: Mutator<T>> VecMutator<T, M> {
    pub fn new(mutator: M, len_range: RangeInclusive<usize>) -> Self {
        Self {
            rng: Rng::default(),
            m: mutator,
            len_range,
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
            len_range: 0..=10_000,
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
        Self::Mutator::new(T::default_mutator(), 0..=usize::MAX)
    }
}

#[derive(Clone)]
pub struct MutationStep<S> {
    inner: Vec<S>,
    element_step: usize,
}
impl<S> Default for MutationStep<S> {
    fn default() -> Self {
        Self {
            inner: vec![],
            element_step: 0,
        }
    }
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
        if value.len() >= *self.len_range.end() {
            return None;
        }
        let idx = if value.is_empty() {
            0
        } else {
            self.rng.usize(0..value.len())
        };

        let (el, el_cache, el_step) = self.m.random_arbitrary(spare_cplx);
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
        if value.len() <= *self.len_range.start() {
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
        if value.len() <= *self.len_range.start() {
            return None;
        }
        let max_elements_to_remove = cmp::max(value.len() - *self.len_range.start(), 10);

        let start_idx = if value.len() == 1 {
            0
        } else {
            self.rng.usize(0..value.len() - 1)
        };
        let end_idx = cmp::min(value.len(), start_idx + self.rng.usize(1..max_elements_to_remove));
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
        if value.len() >= *self.len_range.end() || spare_cplx < 0.01 {
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
        let len_range = self.choose_slice_length(target_cplx);

        let len = self.rng.usize(len_range);
        if len == 0 {
            // TODO: maybe that shouldn't happen under normal circumstances?
            return None;
        }

        let (el, el_cache, el_step) = self.m.random_arbitrary(target_cplx / (len as f64));
        let el_cplx = self.m.complexity(&el, &el_cache);

        insert_many(value, idx, repeat(el).take(len));
        insert_many(&mut cache.inner, idx, repeat(el_cache).take(len));

        let added_cplx = el_cplx * (len as f64);
        cache.sum_cplx += added_cplx;

        let token = UnmutateVecToken::RemoveMany(idx..(idx + len), added_cplx);

        Some(token)
    }

    /**
    Give an approximation for the range of lengths within which the target complexity can be reached.
    result.0 is the minimum length, result.1 is the maximum length
    */
    fn choose_slice_length(&self, target_cplx: f64) -> RangeInclusive<usize> {
        // The maximum length is the target complexity divided by the minimum complexity of each element
        // But that does not take into account the part of the complexity of the vector that comes from its length.
        // That complexity is given by 1.0 + crate::size_to_compelxity(len)
        fn length_for_elements_of_cplx(target_cplx: f64, cplx: f64) -> usize {
            if cplx == 0.0 {
                // cplx is 0, so the length is the maximum complexity of the length component of the vector
                crate::cplxity_to_size(target_cplx - 1.0)
            } else if !cplx.is_finite() {
                0
            } else {
                (target_cplx / cplx).trunc() as usize
            }
        }

        let min_len = length_for_elements_of_cplx(target_cplx, self.m.max_complexity());
        let max_len = length_for_elements_of_cplx(target_cplx, self.m.min_complexity());

        let min_len = clamp(&self.len_range, min_len);
        let max_len = clamp(&(min_len..=*self.len_range.end()), max_len);

        min_len..=max_len
    }

    fn new_input_with_length_and_complexity(
        &self,
        target_len: usize,
        target_cplx: f64,
    ) -> (
        Vec<T>,
        <Self as Mutator<Vec<T>>>::Cache,
        <Self as Mutator<Vec<T>>>::MutationStep,
    ) {
        // TODO: create a new_input_with_complexity method
        let mut v = Vec::with_capacity(target_len);
        let mut cache = VecMutatorCache {
            inner: Vec::with_capacity(target_len),
            sum_cplx: 0.0,
        };
        let mut step = MutationStep {
            inner: Vec::with_capacity(target_len),
            element_step: 0,
        };

        let mut remaining_cplx = target_cplx;
        for i in 0..target_len {
            let max_cplx_element = remaining_cplx / ((target_len - i) as f64);
            let min_cplx_el = self.m.min_complexity();
            if min_cplx_el >= max_cplx_element {
                break;
            }
            let cplx_element = crate::gen_f64(&self.rng, min_cplx_el..max_cplx_element);
            let (x, x_cache, x_step) = self.m.random_arbitrary(cplx_element);
            let x_cplx = self.m.complexity(&x, &x_cache);
            v.push(x);
            cache.inner.push(x_cache);
            step.inner.push(x_step);
            cache.sum_cplx += x_cplx;
            remaining_cplx -= x_cplx;
        }

        if self.len_range.contains(&v.len()) {
        } else {
            // at this point it should be smaller, not larger than it must be, so we add new elements
            let remaining = target_len - v.len();
            for _ in 0..remaining {
                let (x, x_cache, x_step) = self.m.random_arbitrary(0.0);
                let x_cplx = self.m.complexity(&x, &x_cache);
                v.push(x);
                cache.inner.push(x_cache);
                step.inner.push(x_step);
                cache.sum_cplx += x_cplx;
            }
        }
        (v, cache, step)
    }
}

impl<T: Clone, M: Mutator<T>> Mutator<Vec<T>> for VecMutator<T, M> {
    type Cache = VecMutatorCache<M::Cache>;
    type MutationStep = MutationStep<M::MutationStep>;
    type ArbitraryStep = bool; // false: check empty vector, true: random
    type UnmutateToken = UnmutateVecToken<T, M>;

    fn default_arbitrary_step(&self) -> Self::ArbitraryStep {
        <_>::default()
    }

    fn validate_value(&self, value: &Vec<T>) -> Option<(Self::Cache, Self::MutationStep)> {
        let inner: Vec<_> = value.iter().filter_map(|x| self.m.validate_value(x)).collect();

        if inner.len() < value.len() {
            return None;
        }

        let sum_cplx = value
            .iter()
            .zip(inner.iter().map(|x| x.0))
            .fold(0.0, |cplx, (v, cache)| cplx + self.m.complexity(&v, &cache));

        let cache = VecMutatorCache {
            inner: inner.iter().map(|x| x.0).collect(),
            sum_cplx,
        };
        let step = MutationStep {
            inner: inner.iter().map(|x| x.1).collect(),
            element_step: 0,
        };

        Some((cache, step))
    }

    fn max_complexity(&self) -> f64 {
        let max_len = *self.len_range.end();
        1.0 + (max_len as f64) * self.m.max_complexity() + crate::size_to_cplxity(max_len + 1)
    }

    fn min_complexity(&self) -> f64 {
        1.0
    }

    fn complexity(&self, value: &Vec<T>, cache: &Self::Cache) -> f64 {
        1.0 + cache.sum_cplx + crate::size_to_cplxity(value.len() + 1)
    }

    fn ordered_arbitrary(
        &self,
        step: &mut Self::ArbitraryStep,
        mut max_cplx: f64,
    ) -> Option<(Vec<T>, Self::Cache, Self::MutationStep)> {
        if max_cplx < self.min_complexity() {
            return None;
        }
        let mutator_max_cplx = self.max_complexity();
        if max_cplx > mutator_max_cplx {
            max_cplx = mutator_max_cplx;
        }
        if !*step || max_cplx <= 1.0 {
            *step = true;
            if self.len_range.contains(&0) {
                return Some((<_>::default(), Self::Cache::default(), Self::MutationStep::default()));
            } else {
                return Some(self.random_arbitrary(max_cplx));
            }
        } else {
            return Some(self.random_arbitrary(max_cplx));
        }
    }

    fn random_arbitrary(&self, mut max_cplx: f64) -> (Vec<T>, Self::Cache, Self::MutationStep) {
        let mutator_max_cplx = self.max_complexity();
        if max_cplx > mutator_max_cplx {
            max_cplx = mutator_max_cplx;
        }

        let target_cplx = crate::gen_f64(&self.rng, 1.0..max_cplx);
        let len_range = self.choose_slice_length(target_cplx);
        let target_len = self.rng.usize(len_range);

        self.new_input_with_length_and_complexity(target_len, target_cplx)
    }

    fn ordered_mutate(
        &self,
        value: &mut Vec<T>,
        cache: &mut Self::Cache,
        step: &mut Self::MutationStep,
        mut max_cplx: f64,
    ) -> Option<Self::UnmutateToken> {
        // Some(self.random_mutate(value, cache, max_cplx))
        if max_cplx < self.min_complexity() {
            return None;
        }
        let mutator_max_cplx = self.max_complexity();
        if max_cplx > mutator_max_cplx {
            max_cplx = mutator_max_cplx;
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

    fn random_mutate(&self, value: &mut Vec<T>, cache: &mut Self::Cache, mut max_cplx: f64) -> Self::UnmutateToken {
        let mutator_max_cplx = self.max_complexity();
        if max_cplx > mutator_max_cplx {
            max_cplx = mutator_max_cplx;
        }

        let spare_cplx = max_cplx - self.complexity(value, cache);

        if value.is_empty() || self.rng.usize(0..10) == 0 {
            // vector mutation
            match self.rng.usize(0..10) {
                0..=3 => self.insert_element(value, cache, spare_cplx),
                4..=7 => self.remove_element(value, cache),
                8 => self.insert_repeated_elements(value, cache, spare_cplx),
                9 => self.remove_many_elements(value, cache),
                _ => None,
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

fn clamp(range: &RangeInclusive<usize>, x: usize) -> usize {
    cmp::min(cmp::max(*range.start(), x), *range.end())
}

#[cfg(test)]
mod tests {
    use fuzzcheck_traits::Mutator;

    use crate::U8Mutator;
    use crate::VecMutator;
    #[test]
    fn test_constrained_length_mutator() {
        let range = 0..=10;
        let m = VecMutator::<u8, U8Mutator>::new(U8Mutator::default(), range.clone());
        let mut step = false;
        for _ in 0..100 {
            let (x, _, _) = m.ordered_arbitrary(&mut step, 800.0).unwrap();
            eprintln!("{}", x.len());
            assert!(range.contains(&x.len()), "{}", x.len());
        }
    }
}
