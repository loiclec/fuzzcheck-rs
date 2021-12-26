use crate::mutators::mutations::{Mutation, RevertMutation};
use crate::{DefaultMutator, Mutator};
use std::cmp;
use std::marker::PhantomData;
use std::ops::RangeInclusive;

use self::vec_mutation::{RevertVectorMutation, VectorMutation, VectorMutationRandomStep, VectorMutationStep};

pub mod arbitrary;
pub mod insert_element;
pub mod insert_many_elements;
pub mod mutate_element;
pub mod only_choose_length;
pub mod remove;
pub mod remove_and_insert_element;
pub mod swap_elements;
pub mod vec_mutation;

impl<T> DefaultMutator for Vec<T>
where
    T: DefaultMutator + 'static,
{
    type Mutator = VecMutator<T, T::Mutator>;
    fn default_mutator() -> Self::Mutator {
        VecMutator::new(T::default_mutator(), 0..=usize::MAX)
    }
}

#[derive(Clone)]
pub enum VecArbitraryStep {
    InnerMutatorIsUnit { length_step: usize },
    Normal { make_empty: bool },
}

#[doc(hidden)]
#[derive(Clone)]
pub struct RecursingPartIndex<RPI> {
    inner: Vec<RPI>,
    indices: Vec<usize>,
}

pub struct VecMutatorCache<T, M>
where
    T: 'static + Clone,
    M: Mutator<T>,
{
    pub inner: Vec<<M as Mutator<T>>::Cache>,
    pub sum_cplx: f64,
    pub random_mutation_step: VectorMutationRandomStep<T, M>,
}
impl<T, M> Clone for VecMutatorCache<T, M>
where
    T: 'static + Clone,
    M: Mutator<T>,
{
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
            sum_cplx: self.sum_cplx,
            random_mutation_step: self.random_mutation_step.clone(),
        }
    }
}

pub struct VecMutator<T, M>
where
    T: Clone + 'static,
    M: Mutator<T>,
{
    m: M,
    len_range: RangeInclusive<usize>,
    rng: fastrand::Rng,
    mutations: VectorMutation,
    _phantom: PhantomData<T>,
}

impl<T, M> VecMutator<T, M>
where
    T: Clone + 'static,
    M: Mutator<T>,
{
    pub fn new(m: M, len_range: RangeInclusive<usize>) -> Self {
        Self {
            m,
            len_range,
            rng: fastrand::Rng::new(),
            mutations: VectorMutation::default(),
            _phantom: PhantomData,
        }
    }

    #[no_coverage]
    fn complexity_from_inner(&self, cplx: f64, len: usize) -> f64 {
        1.0 + if cplx <= 0.0 { len as f64 } else { cplx }
    }
}
impl<T, M> Mutator<Vec<T>> for VecMutator<T, M>
where
    T: Clone + 'static,
    M: Mutator<T>,
{
    type Cache = VecMutatorCache<T, M>;
    type MutationStep = VectorMutationStep<T, M>;
    type ArbitraryStep = VecArbitraryStep;
    type UnmutateToken = RevertVectorMutation<T, M>;

    fn default_arbitrary_step(&self) -> Self::ArbitraryStep {
        if self.m.max_complexity() == 0.0 {
            Self::ArbitraryStep::InnerMutatorIsUnit {
                length_step: *self.len_range.start(),
            }
        } else {
            Self::ArbitraryStep::Normal { make_empty: false }
        }
    }

    fn validate_value(&self, value: &Vec<T>) -> Option<Self::Cache> {
        let inner_caches: Vec<_> = value
            .iter()
            .map(
                #[no_coverage]
                |x| self.m.validate_value(x),
            )
            .collect::<Option<_>>()?;

        let cplxs = value
            .iter()
            .zip(inner_caches.iter())
            .map(
                #[no_coverage]
                |(v, c)| self.m.complexity(v, c),
            )
            .collect::<Vec<_>>();

        let sum_cplx = cplxs.iter().fold(
            0.0,
            #[no_coverage]
            |sum_cplx, c| sum_cplx + c,
        );

        let random_mutation_step = self.mutations.default_random_step(self, value).unwrap();

        let cache = VecMutatorCache {
            inner: inner_caches,
            sum_cplx,
            random_mutation_step,
        };
        Some(cache)
    }

    fn default_mutation_step(&self, value: &Vec<T>, cache: &Self::Cache) -> Self::MutationStep {
        self.mutations.default_step(self, value, cache).unwrap()
    }

    fn max_complexity(&self) -> f64 {
        let max_len = *self.len_range.end();
        self.complexity_from_inner((max_len as f64) * self.m.max_complexity(), max_len.saturating_add(1))
    }

    fn min_complexity(&self) -> f64 {
        let min_len = *self.len_range.start();
        if min_len == 0 {
            1.0
        } else {
            self.complexity_from_inner((min_len as f64) * self.m.min_complexity(), min_len)
        }
    }

    fn complexity(&self, value: &Vec<T>, cache: &Self::Cache) -> f64 {
        self.complexity_from_inner(cache.sum_cplx, value.len())
    }

    fn ordered_arbitrary(&self, step: &mut Self::ArbitraryStep, max_cplx: f64) -> Option<(Vec<T>, f64)> {
        if max_cplx < self.min_complexity() {
            return None;
        }
        match step {
            VecArbitraryStep::InnerMutatorIsUnit { length_step } => {
                if self.len_range.contains(length_step) && (*length_step as f64) < max_cplx {
                    let mut result = Vec::with_capacity(*length_step);
                    for _ in 0..*length_step {
                        let (e, c) = self.m.random_arbitrary(1.0);
                        assert!(c == 0.0);
                        result.push(e);
                    }
                    let cplx = self.complexity_from_inner(0.0, *length_step);
                    *length_step += 1;
                    Some((result, cplx))
                } else {
                    None
                }
            }
            VecArbitraryStep::Normal { make_empty } => {
                if !*make_empty || max_cplx <= 1.0 {
                    *make_empty = true;
                    if self.len_range.contains(&0) {
                        Some((<_>::default(), 1.0))
                    } else {
                        Some(self.random_arbitrary(max_cplx))
                    }
                } else {
                    Some(self.random_arbitrary(max_cplx))
                }
            }
        }
    }

    fn random_arbitrary(&self, max_cplx: f64) -> (Vec<T>, f64) {
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

    fn ordered_mutate(
        &self,
        value: &mut Vec<T>,
        cache: &mut Self::Cache,
        step: &mut Self::MutationStep,
        max_cplx: f64,
    ) -> Option<(Self::UnmutateToken, f64)> {
        if max_cplx < self.min_complexity() {
            return None;
        }
        let mutation = VectorMutation::from_step(self, value, cache, step, max_cplx)?;
        Some(VectorMutation::apply(mutation, self, value, cache, max_cplx))
    }

    fn random_mutate(&self, value: &mut Vec<T>, cache: &mut Self::Cache, max_cplx: f64) -> (Self::UnmutateToken, f64) {
        let mutation = VectorMutation::random(self, value, cache, &cache.random_mutation_step, max_cplx);
        VectorMutation::apply(mutation, self, value, cache, max_cplx)
    }

    fn unmutate(&self, value: &mut Vec<T>, cache: &mut Self::Cache, t: Self::UnmutateToken) {
        RevertVectorMutation::revert(t, self, value, cache)
    }
    #[doc(hidden)]
    type RecursingPartIndex = RecursingPartIndex<M::RecursingPartIndex>;
    #[doc(hidden)]
    #[no_coverage]
    fn default_recursing_part_index(&self, value: &Vec<T>, cache: &Self::Cache) -> Self::RecursingPartIndex {
        RecursingPartIndex {
            inner: value
                .iter()
                .zip(cache.inner.iter())
                .map(|(v, c)| self.m.default_recursing_part_index(v, c))
                .collect(),
            indices: (0..value.len()).collect(),
        }
    }
    #[doc(hidden)]
    #[no_coverage]
    fn recursing_part<'a, V, N>(
        &self,
        parent: &N,
        value: &'a Vec<T>,
        index: &mut Self::RecursingPartIndex,
    ) -> Option<&'a V>
    where
        V: Clone + 'static,
        N: Mutator<V>,
    {
        assert_eq!(index.inner.len(), index.indices.len());
        if index.inner.is_empty() {
            return None;
        }
        let choice = self.rng.usize(..index.inner.len());
        let subindex = &mut index.inner[choice];
        let value_index = index.indices[choice];
        let v = &value[value_index];
        let result = self.m.recursing_part(parent, v, subindex);
        if result.is_none() {
            index.inner.remove(choice);
            index.indices.remove(choice);
            self.recursing_part::<V, N>(parent, value, index)
        } else {
            result
        }
    }
}

impl<T, M> VecMutator<T, M>
where
    T: Clone + 'static,
    M: Mutator<T>,
{
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
                assert!(target_cplx.is_finite());
                assert!(target_cplx >= 0.0);
                // cplx is 0, so the length is the maximum complexity of the length component of the vector
                target_cplx.round() as usize
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
#[no_coverage]
fn clamp(range: &RangeInclusive<usize>, x: usize) -> usize {
    cmp::min(cmp::max(*range.start(), x), *range.end())
}
