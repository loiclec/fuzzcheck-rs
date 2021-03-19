use std::marker::PhantomData;

use fuzzcheck_traits::Mutator;

struct MapMutator<T, U, Mut, Parse, Map>
where
    T: Clone,
    U: Clone,
    Mut: Mutator<T>,
    Parse: Fn(&U) -> Option<T>,
    Map: Fn(&T) -> U,
{
    mutator: Mut,
    map: Map,
    parse: Parse,
    _phantom: PhantomData<(T, U)>,
}

#[derive(Clone)]
struct Cache<T, C> {
    value: T,
    cache: C,
}

struct UnmutateToken<V, T> {
    value: V,
    inner: T,
}

impl<T, U, Mut, Parse, Map> Mutator<U> for MapMutator<T, U, Mut, Parse, Map>
where
    T: Clone,
    U: Clone,
    Mut: Mutator<T>,
    Parse: Fn(&U) -> Option<T>,
    Map: Fn(&T) -> U,
{
    type Cache = Cache<T, Mut::Cache>;
    type MutationStep = Mut::MutationStep;
    type ArbitraryStep = Mut::ArbitraryStep;
    type UnmutateToken = UnmutateToken<U, Mut::UnmutateToken>;

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

    fn ordered_arbitrary(
        &self,
        step: &mut Self::ArbitraryStep,
        max_cplx: f64,
    ) -> Option<(U, Self::Cache, Self::MutationStep)> {
        self.mutator
            .ordered_arbitrary(step, max_cplx)
            .map(|(value, cache, step)| {
                let cache = Cache { value, cache };
                ((self.map)(&cache.value), cache, step)
            })
    }

    fn random_arbitrary(&self, max_cplx: f64) -> (U, Self::Cache, Self::MutationStep) {
        let (v, c, s) = self.mutator.random_arbitrary(max_cplx);
        let cache = Cache { value: v, cache: c };
        let value = (self.map)(&cache.value);
        (value, cache, s)
    }

    fn ordered_mutate(
        &self,
        value: &mut U,
        cache: &mut Self::Cache,
        step: &mut Self::MutationStep,
        max_cplx: f64,
    ) -> Option<Self::UnmutateToken> {
        if let Some(t) = self
            .mutator
            .ordered_mutate(&mut cache.value, &mut cache.cache, step, max_cplx)
        {
            let old_value = std::mem::replace(value, (self.map)(&cache.value));
            Some(UnmutateToken {
                value: old_value,
                inner: t,
            })
        } else {
            None
        }
    }

    fn random_mutate(&self, value: &mut U, cache: &mut Self::Cache, max_cplx: f64) -> Self::UnmutateToken {
        let t = self.mutator.random_mutate(&mut cache.value, &mut cache.cache, max_cplx);
        *value = (self.map)(&cache.value);
        let old_value = std::mem::replace(value, (self.map)(&cache.value));
        UnmutateToken {
            value: old_value,
            inner: t,
        }
    }

    fn unmutate(&self, value: &mut U, cache: &mut Self::Cache, t: Self::UnmutateToken) {
        *value = t.value;
        self.mutator.unmutate(&mut cache.value, &mut cache.cache, t.inner);
    }
}
