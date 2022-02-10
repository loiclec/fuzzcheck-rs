use crate::mutators::mutations::{Mutation, RevertMutation};
use crate::traits::EmptySubValueProvider;
use crate::{DefaultMutator, Mutator};
use std::any::{Any, TypeId};
use std::cmp;
use std::marker::PhantomData;
use std::ops::RangeInclusive;

use self::vec_mutation::{RevertVectorMutation, VectorMutation, VectorMutationRandomStep, VectorMutationStep};

pub mod arbitrary;
pub mod copy_element;
pub mod crossover_replace_element;
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
    #[no_coverage]
    fn default_mutator() -> Self::Mutator {
        VecMutator::new(T::default_mutator(), 0..=usize::MAX, true)
    }
}

#[derive(Clone)]
pub enum VecArbitraryStep {
    InnerMutatorIsUnit { length_step: usize },
    Normal { make_empty: bool },
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
    #[no_coverage]
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
    inherent_complexity: bool,
    _phantom: PhantomData<T>,
}

impl<T, M> VecMutator<T, M>
where
    T: Clone + 'static,
    M: Mutator<T>,
{
    #[no_coverage]
    pub fn new(m: M, len_range: RangeInclusive<usize>, inherent_complexity: bool) -> Self {
        Self {
            m,
            len_range,
            rng: fastrand::Rng::new(),
            mutations: VectorMutation::default(),
            inherent_complexity,
            _phantom: PhantomData,
        }
    }

    #[no_coverage]
    fn complexity_from_inner(&self, cplx: f64, len: usize) -> f64 {
        if self.inherent_complexity {
            1.0 + if len == 0 || self.m.min_complexity() > 0.0 {
                cplx
            } else {
                len as f64 + cplx
            }
        } else {
            cplx
        }
    }
}
impl<T, M> Mutator<Vec<T>> for VecMutator<T, M>
where
    T: Clone + 'static,
    M: Mutator<T>,
{
    #[doc(hidden)]
    type Cache = VecMutatorCache<T, M>;
    #[doc(hidden)]
    type MutationStep = VectorMutationStep<T, M>;
    #[doc(hidden)]
    type ArbitraryStep = VecArbitraryStep;
    #[doc(hidden)]
    type UnmutateToken = RevertVectorMutation<T, M>;
    #[doc(hidden)]
    type LensPath = (usize, Option<M::LensPath>);

    #[doc(hidden)]
    #[no_coverage]
    fn default_arbitrary_step(&self) -> Self::ArbitraryStep {
        if self.m.max_complexity() == 0.0 {
            Self::ArbitraryStep::InnerMutatorIsUnit {
                length_step: *self.len_range.start(),
            }
        } else {
            Self::ArbitraryStep::Normal { make_empty: false }
        }
    }
    #[doc(hidden)]
    #[no_coverage]
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
        println!("sum cplx: {sum_cplx:.2}");

        let random_mutation_step = self.mutations.default_random_step(self, value).unwrap();

        println!("random mutation step done");

        let cache = VecMutatorCache {
            inner: inner_caches,
            sum_cplx,
            random_mutation_step,
        };

        println!("return cache");

        Some(cache)
    }
    #[doc(hidden)]
    #[no_coverage]
    fn default_mutation_step(&self, value: &Vec<T>, cache: &Self::Cache) -> Self::MutationStep {
        self.mutations.default_step(self, value, cache).unwrap()
    }
    #[doc(hidden)]
    #[no_coverage]
    fn global_search_space_complexity(&self) -> f64 {
        if self.m.global_search_space_complexity() == 0.0 {
            super::size_to_cplxity(self.len_range.end() - self.len_range.start() + 1)
        } else {
            self.m.global_search_space_complexity() * ((self.len_range.end() - self.len_range.start()) as f64)
        }
    }
    #[doc(hidden)]
    #[no_coverage]
    fn max_complexity(&self) -> f64 {
        let max_len = *self.len_range.end();
        let cplx = self.complexity_from_inner((max_len as f64) * self.m.max_complexity(), max_len);
        cplx
    }
    #[doc(hidden)]
    #[no_coverage]
    fn min_complexity(&self) -> f64 {
        let min_len = *self.len_range.start();
        let min_sum_cplx = if min_len == 0 {
            0.0
        } else {
            (min_len as f64) * self.m.min_complexity()
        };
        let cplx = self.complexity_from_inner(min_sum_cplx, min_len);
        cplx
    }
    #[doc(hidden)]
    #[no_coverage]
    fn complexity(&self, value: &Vec<T>, cache: &Self::Cache) -> f64 {
        self.complexity_from_inner(cache.sum_cplx, value.len())
    }
    #[doc(hidden)]
    #[no_coverage]
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
                        Some((<_>::default(), self.complexity_from_inner(0.0, 0)))
                    } else {
                        Some(self.random_arbitrary(max_cplx))
                    }
                } else {
                    Some(self.random_arbitrary(max_cplx))
                }
            }
        }
    }
    #[doc(hidden)]
    #[no_coverage]
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

        let (v, inner_cplx) =
            self.new_input_with_length_and_complexity(*self.len_range.start(), target_len, target_cplx);
        let cplx = self.complexity_from_inner(inner_cplx, v.len());
        (v, cplx)
    }
    #[doc(hidden)]
    #[no_coverage]
    fn ordered_mutate(
        &self,
        value: &mut Vec<T>,
        cache: &mut Self::Cache,
        step: &mut Self::MutationStep,
        subvalue_provider: &dyn crate::SubValueProvider,
        max_cplx: f64,
    ) -> Option<(Self::UnmutateToken, f64)> {
        if max_cplx < self.min_complexity() {
            return None;
        }
        let mutation = VectorMutation::from_step(self, value, cache, step, subvalue_provider, max_cplx)?;
        Some(VectorMutation::apply(
            mutation,
            self,
            value,
            cache,
            subvalue_provider,
            max_cplx,
        ))
    }

    #[doc(hidden)]
    #[no_coverage]
    fn random_mutate(&self, value: &mut Vec<T>, cache: &mut Self::Cache, max_cplx: f64) -> (Self::UnmutateToken, f64) {
        let mutation = VectorMutation::random(self, value, cache, &cache.random_mutation_step, max_cplx);
        VectorMutation::apply(mutation, self, value, cache, &EmptySubValueProvider, max_cplx)
    }

    #[doc(hidden)]
    #[no_coverage]
    fn unmutate(&self, value: &mut Vec<T>, cache: &mut Self::Cache, t: Self::UnmutateToken) {
        RevertVectorMutation::revert(t, self, value, cache)
    }

    #[doc(hidden)]
    #[no_coverage]
    fn lens<'a>(&self, value: &'a Vec<T>, cache: &'a Self::Cache, path: &Self::LensPath) -> &'a dyn Any {
        let value = &value[path.0];
        if let Some(subpath) = &path.1 {
            let cache = &cache.inner[path.0];
            self.m.lens(value, cache, subpath)
        } else {
            value
        }
    }
    #[doc(hidden)]
    #[no_coverage]
    fn all_paths(
        &self,
        value: &Vec<T>,
        cache: &Self::Cache,
        register_path: &mut dyn FnMut(TypeId, Self::LensPath, f64),
    ) {
        println!("vec all paths, value is empty: {}", value.is_empty());
        if !value.is_empty() {
            println!("iterating over elements");
            let typeid = TypeId::of::<T>();
            for idx in 0..value.len() {
                let cplx = self.m.complexity(&value[idx], &cache.inner[idx]);
                register_path(typeid, (idx, None), cplx);
            }
            for (idx, (el, el_cache)) in value.iter().zip(cache.inner.iter()).enumerate() {
                self.m.all_paths(
                    el,
                    el_cache,
                    #[no_coverage]
                    &mut |typeid, subpath, cplx| register_path(typeid, (idx, Some(subpath)), cplx),
                );
            }
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
    fn new_input_with_length_and_complexity(
        &self,
        min_len: usize,
        target_len: usize,
        target_cplx: f64,
    ) -> (Vec<T>, f64) {
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
        if v.len() < min_len {
            // at this point it is smaller than it must be, so we add new, minimal, elements
            let remaining = min_len - v.len();
            for _ in 0..remaining {
                let (x, x_cplx) = self.m.random_arbitrary(0.0);
                v.push(x);
                sum_cplx += x_cplx;
            }
        }
        self.rng.shuffle(&mut v);
        // let cplx = self.complexity_from_inner(sum_cplx, v.len());
        (v, sum_cplx)
    }
}
#[no_coverage]
fn clamp(range: &RangeInclusive<usize>, x: usize) -> usize {
    cmp::min(cmp::max(*range.start(), x), *range.end())
}
