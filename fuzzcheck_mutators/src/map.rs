use std::marker::PhantomData;

use crate::fuzzcheck_traits::Mutator;

pub struct MapMutator<T, U, Mut, Parse, Map>
where
    T: Clone,
    U: Clone,
    Mut: Mutator<T>,
    Parse: Fn(&U) -> Option<T>,
    Map: Fn(T) -> U,
{
    mutator: Mut,
    map: Map,
    parse: Parse,
    _phantom: PhantomData<(T, U)>,
}
impl<T, U, Mut, Parse, Map> MapMutator<T, U, Mut, Parse, Map>
where
    T: Clone,
    U: Clone,
    Mut: Mutator<T>,
    Parse: Fn(&U) -> Option<T>,
    Map: Fn(T) -> U,
{
    pub fn new(mutator: Mut, map: Map, parse: Parse) -> Self {
        Self {
            mutator,
            map,
            parse,
            _phantom: PhantomData,
        }
    }
}

pub struct Cache<T, C> {
    value: T,
    cache: C,
}

pub struct UnmutateToken<V> {
    value: V,
}

impl<T, U, Mut, Parse, Map> Mutator<U> for MapMutator<T, U, Mut, Parse, Map>
where
    T: Clone,
    U: Clone,
    Mut: Mutator<T>,
    Parse: Fn(&U) -> Option<T>,
    Map: Fn(T) -> U,
{
    type Cache = Cache<T, Mut::Cache>;
    type MutationStep = Mut::MutationStep;
    type ArbitraryStep = Mut::ArbitraryStep;
    type UnmutateToken = UnmutateToken<U>;

    fn default_arbitrary_step(&self) -> Self::ArbitraryStep {
        self.mutator.default_arbitrary_step()
    }

    fn validate_value(&self, value: &U) -> Option<(Self::Cache, Self::MutationStep)> {
        if let Some(value) = (self.parse)(value) {
            if let Some((cache, step)) = self.mutator.validate_value(&value) {
                Some((Cache { value, cache }, step))
            } else {
                None
            }
        } else {
            None
        }
    }

    fn max_complexity(&self) -> f64 {
        self.mutator.max_complexity()
    }

    fn min_complexity(&self) -> f64 {
        self.mutator.min_complexity()
    }

    fn complexity(&self, _value: &U, cache: &Self::Cache) -> f64 {
        self.mutator.complexity(&cache.value, &cache.cache)
    }

    fn ordered_arbitrary(&self, step: &mut Self::ArbitraryStep, max_cplx: f64) -> Option<(U, f64)> {
        self.mutator
            .ordered_arbitrary(step, max_cplx)
            .map(|(value, cplx)| ((self.map)(value), cplx))
    }

    fn random_arbitrary(&self, max_cplx: f64) -> (U, f64) {
        let (v, c) = self.mutator.random_arbitrary(max_cplx);
        let value = (self.map)(v);
        (value, c)
    }

    fn ordered_mutate(
        &self,
        value: &mut U,
        cache: &Self::Cache,
        step: &mut Self::MutationStep,
        max_cplx: f64,
    ) -> Option<(Self::UnmutateToken, f64)> {
        let mut inner_value = cache.value.clone();
        if let Some((_, cplx)) = self
            .mutator
            .ordered_mutate(&mut inner_value, &cache.cache, step, max_cplx)
        {
            let old_value = std::mem::replace(value, (self.map)(inner_value));
            Some((UnmutateToken { value: old_value }, cplx))
        } else {
            None
        }
    }

    fn random_mutate(&self, value: &mut U, cache: &Self::Cache, max_cplx: f64) -> (Self::UnmutateToken, f64) {
        let mut inner_value = cache.value.clone();
        let (_, cplx) = self.mutator.random_mutate(&mut inner_value, &cache.cache, max_cplx);
        let old_value = std::mem::replace(value, (self.map)(inner_value));
        (UnmutateToken { value: old_value }, cplx)
    }

    fn unmutate(&self, value: &mut U, t: Self::UnmutateToken) {
        *value = t.value;
    }
}
