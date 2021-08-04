use crate::mutators::vose_alias::VoseAlias;
use crate::DefaultMutator;
use crate::Mutator;
use fastrand::Rng;

use std::{
    cmp,
    ops::{Range, RangeInclusive},
};
use std::{iter::repeat, marker::PhantomData};

pub struct VecMutator<T: Clone, M: Mutator<T>> {
    pub rng: Rng,
    pub m: M,
    pub len_range: RangeInclusive<usize>,
    pub dictionary: Vec<Vec<T>>,
    _phantom: PhantomData<T>,
}
impl<T: Clone, M: Mutator<T>> VecMutator<T, M> {
    #[no_coverage]
    pub fn new(mutator: M, len_range: RangeInclusive<usize>) -> Self {
        Self {
            rng: Rng::default(),
            m: mutator,
            len_range,
            dictionary: vec![],
            _phantom: <_>::default(),
        }
    }
    #[no_coverage]
    pub fn new_with_dict(mutator: M, len_range: RangeInclusive<usize>, dict: Vec<Vec<T>>) -> Self {
        Self {
            rng: Rng::default(),
            m: mutator,
            len_range,
            dictionary: dict,
            _phantom: <_>::default(),
        }
    }
}
impl<T: Clone, M: Mutator<T>> Default for VecMutator<T, M>
where
    M: Default,
{
    #[no_coverage]
    fn default() -> Self {
        Self {
            rng: Rng::default(),
            m: M::default(),
            len_range: 0..=10_000,
            dictionary: vec![],
            _phantom: <_>::default(),
        }
    }
}
impl<T: Clone> DefaultMutator for Vec<T>
where
    T: DefaultMutator,
{
    type Mutator = VecMutator<T, <T as DefaultMutator>::Mutator>;
    #[no_coverage]
    fn default_mutator() -> Self::Mutator {
        Self::Mutator::new(T::default_mutator(), 0..=usize::MAX)
    }
}

pub struct MutationStep<S> {
    inner: Vec<S>,
    alias: Option<VoseAlias>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct VecMutatorCache<C> {
    inner: Vec<C>,
    sum_cplx: f64,
    alias: Option<VoseAlias>,
}

pub enum UnmutateVecToken<T: Clone, M: Mutator<T>> {
    Element(usize, M::UnmutateToken),
    Remove(usize),
    RemoveMany(Range<usize>),
    Insert(usize, T),
    InsertMany(usize, Vec<T>),
    Replace(Vec<T>),
    Nothing,
}

impl<T: Clone, M: Mutator<T>> VecMutator<T, M> {
    #[no_coverage]
    fn complexity_from_inner(&self, cplx: f64, len: usize) -> f64 {
        1.0 + cplx + crate::mutators::size_to_cplxity(len.saturating_add(1))
    }
    #[no_coverage]
    fn mutate_element(
        &self,
        value: &mut Vec<T>,
        cache: &mut VecMutatorCache<M::Cache>,
        step: &mut MutationStep<M::MutationStep>,
        idx: usize,
        spare_cplx: f64,
    ) -> Option<(UnmutateVecToken<T, M>, f64)> {
        let el = &mut value[idx];
        let el_cache = &mut cache.inner[idx];
        let el_step = &mut step.inner[idx];

        let old_cplx = self.m.complexity(el, el_cache);

        if let Some((token, new_cplx)) = self.m.ordered_mutate(el, el_cache, el_step, spare_cplx + old_cplx) {
            Some((
                UnmutateVecToken::Element(idx, token),
                self.complexity_from_inner(cache.sum_cplx - old_cplx + new_cplx, value.len()),
            ))
        } else {
            None
        }
    }

    #[no_coverage]
    fn insert_element(
        &self,
        value: &mut Vec<T>,
        cache: &VecMutatorCache<M::Cache>,
        spare_cplx: f64,
    ) -> Option<(UnmutateVecToken<T, M>, f64)> {
        if value.len() >= *self.len_range.end() {
            return None;
        }
        let idx = if value.is_empty() {
            0
        } else {
            self.rng.usize(0..=value.len())
        };
        let (el, el_cplx) = self.m.random_arbitrary(spare_cplx);
        value.insert(idx, el);
        let token = UnmutateVecToken::Remove(idx);
        Some((token, self.complexity_from_inner(cache.sum_cplx + el_cplx, value.len())))
    }

    #[no_coverage]
    fn remove_element(
        &self,
        value: &mut Vec<T>,
        cache: &VecMutatorCache<M::Cache>,
    ) -> Option<(UnmutateVecToken<T, M>, f64)> {
        if value.len() <= *self.len_range.start() {
            return None;
        }
        let idx = self.rng.usize(0..value.len());
        let removed_el = value.remove(idx);
        let removed_el_cplx = self.m.complexity(&removed_el, &cache.inner[idx]);
        let token = UnmutateVecToken::Insert(idx, removed_el);
        Some((
            token,
            self.complexity_from_inner(cache.sum_cplx - removed_el_cplx, value.len()),
        ))
    }

    #[no_coverage]
    fn remove_many_elements(
        &self,
        value: &mut Vec<T>,
        cache: &VecMutatorCache<M::Cache>,
    ) -> Option<(UnmutateVecToken<T, M>, f64)> {
        if value.len() <= *self.len_range.start() {
            return None;
        }
        let max_elements_to_remove = cmp::min(value.len() - *self.len_range.start(), 10);
        let start_idx = if value.len() == 1 {
            0
        } else {
            self.rng.usize(0..value.len() - 1)
        };
        let nbr_elements_to_remove = self.rng.usize(1..=max_elements_to_remove);
        let end_idx = cmp::min(value.len(), start_idx + nbr_elements_to_remove);

        let removed_cplx = value[start_idx..end_idx]
            .iter()
            .zip(cache.inner[start_idx..end_idx].iter())
            .fold(0.0, |cplx, (v, c)| cplx + self.m.complexity(v, c));
        let removed_elements = value.drain(start_idx..end_idx).collect();

        Some((
            UnmutateVecToken::InsertMany(start_idx, removed_elements),
            self.complexity_from_inner(cache.sum_cplx - removed_cplx, value.len()),
        ))
    }

    #[no_coverage]
    fn use_dictionary(
        &self,
        value: &mut Vec<T>,
        cache: &VecMutatorCache<M::Cache>,
        spare_cplx: f64,
    ) -> Option<(UnmutateVecToken<T, M>, f64)> {
        if value.len() >= *self.len_range.end() || spare_cplx < 0.01 {
            return None;
        }

        let idx = if value.is_empty() {
            0
        } else {
            self.rng.usize(0..value.len())
        };
        let mut indices = (0..self.dictionary.len()).collect::<Vec<_>>();
        self.rng.shuffle(&mut indices);

        for dic_idx in indices {
            let x = &self.dictionary[dic_idx];
            if let Some((el_cache, _)) = self.validate_value(x) {
                let added_complexity = self.complexity(x, &el_cache);
                if added_complexity < spare_cplx {
                    insert_many(value, idx, x.iter().cloned());
                    let token = UnmutateVecToken::RemoveMany(idx..(idx + x.len()));
                    return Some((
                        token,
                        self.complexity_from_inner(cache.sum_cplx + added_complexity, value.len()),
                    ));
                } else {
                    continue;
                }
            } else {
                continue;
            }
        }

        None
    }

    #[no_coverage]
    fn insert_repeated_elements(
        &self,
        value: &mut Vec<T>,
        cache: &VecMutatorCache<M::Cache>,
        spare_cplx: f64,
    ) -> Option<(UnmutateVecToken<T, M>, f64)> {
        if value.len() >= *self.len_range.end() || spare_cplx < 0.01 {
            return None;
        }

        let idx = if value.is_empty() {
            0
        } else {
            self.rng.usize(0..=value.len())
        };

        let target_cplx = crate::mutators::gen_f64(
            &self.rng,
            0.0..crate::mutators::gen_f64(
                &self.rng,
                0.0..crate::mutators::gen_f64(&self.rng, 0.0..crate::mutators::gen_f64(&self.rng, 0.0..spare_cplx)),
            ),
        );
        let len_range = self.choose_slice_length(spare_cplx);

        let len = self.rng.usize(len_range);
        if len == 0 || !self.len_range.contains(&(value.len() + len)) {
            return None;
        }

        let (el, el_cplx) = self.m.random_arbitrary(target_cplx / (len as f64));
        insert_many(value, idx, repeat(el).take(len));

        let token = UnmutateVecToken::RemoveMany(idx..(idx + len));

        Some((
            token,
            self.complexity_from_inner(cache.sum_cplx + (len as f64) * el_cplx, value.len()),
        ))
    }

    /**
    Give an approximation for the range of lengths within which the target complexity can be reached.
    */
    #[no_coverage]
    fn choose_slice_length(&self, target_cplx: f64) -> RangeInclusive<usize> {
        // The maximum length is the target complexity divided by the minimum complexity of each element
        // But that does not take into account the part of the complexity of the vector that comes from its length.
        // That complexity is given by 1.0 + crate::size_to_compelxity(len)
        #[no_coverage]
        fn length_for_elements_of_cplx(target_cplx: f64, cplx: f64) -> usize {
            if cplx == 0.0 {
                // cplx is 0, so the length is the maximum complexity of the length component of the vector
                crate::mutators::cplxity_to_size(target_cplx - 1.0)
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

    #[no_coverage]
    fn new_input_with_length_and_complexity(&self, target_len: usize, target_cplx: f64) -> (Vec<T>, f64) {
        let mut v = Vec::with_capacity(target_len);
        let mut sum_cplx = 0.0;

        let mut remaining_cplx = target_cplx;
        for i in 0..target_len {
            let max_cplx_element = remaining_cplx / ((target_len - i) as f64);
            let min_cplx_el = self.m.min_complexity();

            if min_cplx_el >= max_cplx_element {
                break;
            }
            let (x, x_cplx) = self.m.random_arbitrary(max_cplx_element);
            sum_cplx += x_cplx;
            v.push(x);
            remaining_cplx -= x_cplx;
        }
        if self.len_range.contains(&v.len()) {
        } else {
            // at this point it is smaller than it must be, so we add new, minimal, elements
            let remaining = self.len_range.start() - v.len();
            for _ in 0..remaining {
                let (x, x_cplx) = self.m.random_arbitrary(0.0);
                v.push(x);
                sum_cplx += x_cplx;
            }
        }
        self.rng.shuffle(&mut v);
        let cplx = self.complexity_from_inner(sum_cplx, v.len());
        (v, cplx)
    }
}

impl<T: Clone, M: Mutator<T>> Mutator<Vec<T>> for VecMutator<T, M> {
    type Cache = VecMutatorCache<M::Cache>;
    type MutationStep = MutationStep<M::MutationStep>;
    type ArbitraryStep = bool; // false: check empty vector, true: random
    type UnmutateToken = UnmutateVecToken<T, M>;

    #[no_coverage]
    fn default_arbitrary_step(&self) -> Self::ArbitraryStep {
        <_>::default()
    }

    #[no_coverage]
    fn validate_value(&self, value: &Vec<T>) -> Option<(Self::Cache, Self::MutationStep)> {
        let inner: Vec<_> = value.iter().filter_map(|x| self.m.validate_value(x)).collect();

        if inner.len() < value.len() {
            return None;
        }

        let mut inner_caches = Vec::with_capacity(inner.len());
        let mut inner_steps = Vec::with_capacity(inner.len());
        for (cache, step) in inner.into_iter() {
            inner_caches.push(cache);
            inner_steps.push(step);
        }

        let cplxs = value
            .iter()
            .zip(inner_caches.iter())
            .map(|(v, c)| self.m.complexity(v, c))
            .collect::<Vec<_>>();

        let sum_cplx = cplxs.iter().fold(0.0, |sum_cplx, c| sum_cplx + c);

        let alias = if !inner_caches.is_empty() {
            let mut probabilities = cplxs.into_iter().map(|c| 10. + c).collect::<Vec<_>>();
            let sum_prob = probabilities.iter().sum::<f64>();
            probabilities.iter_mut().for_each(|c| *c /= sum_prob);
            Some(VoseAlias::new(probabilities))
        } else {
            None
        };

        let cache = VecMutatorCache {
            inner: inner_caches,
            sum_cplx,
            alias: alias.clone(),
        };
        let step = MutationStep {
            inner: inner_steps,
            alias,
        };

        Some((cache, step))
    }

    #[no_coverage]
    fn max_complexity(&self) -> f64 {
        let max_len = *self.len_range.end();
        self.complexity_from_inner((max_len as f64) * self.m.max_complexity(), max_len.saturating_add(1))
    }

    #[no_coverage]
    fn min_complexity(&self) -> f64 {
        let min_len = *self.len_range.start();
        self.complexity_from_inner((min_len as f64) * self.m.min_complexity(), min_len)
    }

    #[no_coverage]
    fn complexity(&self, value: &Vec<T>, cache: &Self::Cache) -> f64 {
        self.complexity_from_inner(cache.sum_cplx, value.len())
    }

    #[no_coverage]
    fn ordered_arbitrary(&self, step: &mut Self::ArbitraryStep, mut max_cplx: f64) -> Option<(Vec<T>, f64)> {
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
                Some((<_>::default(), 1.0))
            } else {
                Some(self.random_arbitrary(max_cplx))
            }
        } else {
            Some(self.random_arbitrary(max_cplx))
        }
    }

    #[no_coverage]
    fn random_arbitrary(&self, mut max_cplx: f64) -> (Vec<T>, f64) {
        let mutator_max_cplx = self.max_complexity();
        if max_cplx > mutator_max_cplx {
            max_cplx = mutator_max_cplx;
        }
        let min_cplx = self.min_complexity();
        if max_cplx <= min_cplx || self.rng.u8(..) == 0 {
            // return the least complex value possible
            let mut v = Vec::with_capacity(*self.len_range.start());
            let mut inner_cplx = 0.0;
            for _ in 0..*self.len_range.start() {
                let (el, el_cplx) = self.m.random_arbitrary(0.0);
                v.push(el);
                inner_cplx += el_cplx;
            }
            let cplx = self.complexity_from_inner(inner_cplx, v.len());
            return (v, cplx);
        }
        let target_cplx = crate::mutators::gen_f64(&self.rng, min_cplx..max_cplx);
        let len_range = self.choose_slice_length(target_cplx);
        let upperbound_max_len = std::cmp::min(*len_range.end(), (max_cplx / self.m.min_complexity()).ceil() as usize);
        let target_len = self.rng.usize(0..=upperbound_max_len);

        self.new_input_with_length_and_complexity(target_len, target_cplx)
    }

    #[no_coverage]
    fn ordered_mutate(
        &self,
        value: &mut Vec<T>,
        cache: &mut Self::Cache,
        step: &mut Self::MutationStep,
        mut max_cplx: f64,
    ) -> Option<(Self::UnmutateToken, f64)> {
        if max_cplx < self.min_complexity() {
            return None;
        }
        if self.rng.usize(0..100) == 0 {
            let (mut v, cplx) = self.random_arbitrary(max_cplx);
            std::mem::swap(value, &mut v);
            return Some((UnmutateVecToken::Replace(v), cplx));
        }
        let mutator_max_cplx = self.max_complexity();
        if max_cplx > mutator_max_cplx {
            max_cplx = mutator_max_cplx;
        }
        let current_cplx = self.complexity(value, cache);
        let spare_cplx = max_cplx - current_cplx;

        let token = if value.is_empty() || step.alias.is_none() || self.rng.usize(0..10) == 0 {
            // vector mutation
            match self.rng.usize(0..if value.is_empty() { 5 } else { 15 }) {
                0..=3 => self.insert_element(value, cache, spare_cplx),
                4 => self.insert_repeated_elements(value, cache, spare_cplx),
                5..=8 => self.remove_element(value, cache),
                9 => self.remove_many_elements(value, cache),
                10..=14 => self.use_dictionary(value, cache, spare_cplx),
                _ => unreachable!(),
            }
        } else {
            // we know value is not empty, therefore the alias is Some
            if let Some(alias) = step.alias.as_ref() {
                let idx = alias.sample();
                if let Some(x) = self.mutate_element(value, cache, step, idx, spare_cplx) {
                    Some(x)
                } else {
                    let mut prob = step.alias.as_ref().unwrap().original_probabilities.clone();
                    prob[idx] = 0.0;
                    let sum = prob.iter().sum::<f64>();
                    if sum == 0.0 {
                        step.alias = None;
                    } else {
                        prob.iter_mut().for_each(|c| *c /= sum);
                        step.alias = Some(VoseAlias::new(prob));
                    }

                    None
                }
            } else {
                None
            }
        };
        if let Some(token) = token {
            Some(token)
        } else {
            Some(self.random_mutate(value, cache, max_cplx))
        }
    }

    #[no_coverage]
    fn random_mutate(
        &self,
        value: &mut Vec<T>,
        cache: &mut Self::Cache,
        mut max_cplx: f64,
    ) -> (Self::UnmutateToken, f64) {
        let mutator_max_cplx = self.max_complexity();
        if max_cplx > mutator_max_cplx {
            max_cplx = mutator_max_cplx;
        }
        if self.rng.usize(0..100) == 0 {
            let (mut v, cplx) = self.random_arbitrary(max_cplx);
            std::mem::swap(value, &mut v);
            return (UnmutateVecToken::Replace(v), cplx);
        }
        let current_cplx = self.complexity(value, cache);
        let spare_cplx = max_cplx - current_cplx;

        if value.is_empty() || self.rng.usize(0..10) == 0 {
            // vector mutation
            match self.rng.usize(0..if value.is_empty() { 5 } else { 15 }) {
                0..=3 => self.insert_element(value, cache, spare_cplx),
                4 => self.insert_repeated_elements(value, cache, spare_cplx),
                5..=8 => self.remove_element(value, cache),
                9 => self.remove_many_elements(value, cache),
                10..=14 => self.use_dictionary(value, cache, spare_cplx),
                _ => None,
            }
            .unwrap_or_else(|| self.random_mutate(value, cache, max_cplx))
        } else {
            // element mutation
            // we know value is not empty, therefore the alias is Some
            let alias = cache.alias.as_ref().unwrap();
            // element mutation
            let idx = alias.sample();
            // let idx = self.rng.usize(0..value.len());
            let el = &mut value[idx];
            let el_cache = &mut cache.inner[idx];

            let old_el_cplx = self.m.complexity(el, el_cache);
            let (token, new_el_cplx) = self.m.random_mutate(el, el_cache, spare_cplx + old_el_cplx);

            (
                UnmutateVecToken::Element(idx, token),
                self.complexity_from_inner(cache.sum_cplx - old_el_cplx + new_el_cplx, value.len()),
            )
        }
    }

    #[no_coverage]
    fn unmutate(&self, value: &mut Vec<T>, cache: &mut Self::Cache, t: Self::UnmutateToken) {
        match t {
            UnmutateVecToken::Element(idx, inner_t) => {
                let el = &mut value[idx];
                self.m.unmutate(el, &mut cache.inner[idx], inner_t);
            }
            UnmutateVecToken::Insert(idx, el) => {
                value.insert(idx, el);
            }
            UnmutateVecToken::Remove(idx) => {
                value.remove(idx);
            }
            UnmutateVecToken::Replace(new_value) => {
                let _ = std::mem::replace(value, new_value);
            }
            UnmutateVecToken::InsertMany(idx, v) => {
                insert_many(value, idx, v.into_iter());
            }
            UnmutateVecToken::RemoveMany(range) => {
                value.drain(range);
            }
            UnmutateVecToken::Nothing => {}
        }
    }
}

#[no_coverage]
pub fn insert_many<T>(v: &mut Vec<T>, idx: usize, iter: impl Iterator<Item = T>) {
    let moved_slice = v.drain(idx..).collect::<Vec<T>>().into_iter();
    v.extend(iter);
    v.extend(moved_slice);
}

#[no_coverage]
fn clamp(range: &RangeInclusive<usize>, x: usize) -> usize {
    cmp::min(cmp::max(*range.start(), x), *range.end())
}

#[cfg(test)]
mod tests {
    use std::iter::repeat;

    use crate::Mutator;

    use crate::mutators::integer::U8Mutator;
    use crate::mutators::vector::VecMutator;
    #[test]
    #[no_coverage]
    fn test_constrained_length_mutator_ordered_arbitrary() {
        let range = 0..=10;
        let m = VecMutator::<u8, U8Mutator>::new(U8Mutator::default(), range.clone());
        let mut step = false;

        let mut lengths: Vec<_> = repeat(0).take(11).collect();
        let mut cplxs: Vec<_> = repeat(0).take(81).collect();

        for _ in 0..100000 {
            let (x, cplx) = m.ordered_arbitrary(&mut step, 800.0).unwrap();
            lengths[x.len()] += 1;
            cplxs[(cplx / 10.0) as usize] += 1;
            // eprintln!("{:?}", x);
            assert!(range.contains(&x.len()), "{}", x.len());
        }
        println!("{:?}", lengths);
        println!("{:?}", cplxs);
    }
    #[test]
    #[no_coverage]
    fn test_constrained_length_mutator_ordered_mutate() {
        let range = 0..=10;
        let m = VecMutator::<u8, U8Mutator>::new(U8Mutator::default(), range.clone());
        let mut step = false;

        let mut lengths: Vec<_> = repeat(0).take(11).collect();
        let mut cplxs: Vec<_> = repeat(0).take(81).collect();

        for _ in 0..100000 {
            let (x, cplx) = m.ordered_arbitrary(&mut step, 800.0).unwrap();
            lengths[x.len()] += 1;
            cplxs[(cplx / 10.0) as usize] += 1;
            // eprintln!("{:?}", x);
            assert!(range.contains(&x.len()), "{}", x.len());
        }
        println!("{:?}", lengths);
        println!("{:?}", cplxs);
    }
}
// // ========== WeightedIndex ===========
// /// Generate a random f64 within the given range
// /// The start and end of the range must be finite
// /// This is a very naive implementation
//
// #[no_coverage] fn gen_f64(rng: &fastrand::Rng, range: Range<f64>) -> f64 {
//     range.start + rng.f64() * (range.end - range.start)
// }
// /**
//  * A distribution using weighted sampling to pick a discretely selected item.
//  *
//  * An alternative implementation of the same type by the `rand` crate.
//  */
// #[derive(Debug, Clone)]
// pub struct WeightedIndex<'a> {
//     pub cumulative_weights: &'a Vec<f64>,
// }

// impl<'a> WeightedIndex<'a> {
//     #[no_coverage]pub fn sample(&self, rng: &fastrand::Rng) -> usize {
//         assert!(!self.cumulative_weights.is_empty());
//         if self.cumulative_weights.len() == 1 {
//             return 0;
//         }

//         let range = *self.cumulative_weights.first().unwrap()..*self.cumulative_weights.last().unwrap();
//         let chosen_weight = gen_f64(rng, range);
//         // Find the first item which has a weight *higher* than the chosen weight.
//         self.cumulative_weights
//             .binary_search_by(|w| {
//                 if *w <= chosen_weight {
//                     Ordering::Less
//                 } else {
//                     Ordering::Greater
//                 }
//             })
//             .unwrap_err()
//     }
// }
