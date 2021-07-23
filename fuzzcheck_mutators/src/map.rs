use std::marker::PhantomData;

use crate::fuzzcheck_traits::Mutator;

pub struct MapMutator<From, To, M, Parse, Map> where From: Clone, To: Clone, M: Mutator<From>, Parse: Fn(&To) -> Option<From>, Map: Fn(&From) -> To {
    pub mutator: M,
    pub parse: Parse,
    pub map: Map,
    _phantom: PhantomData<(From, To)>
}
impl<From, To, M, Parse, Map> MapMutator<From, To, M, Parse, Map> where From: Clone, To: Clone, M: Mutator<From>, Parse: Fn(&To) -> Option<From>, Map: Fn(&From) -> To {
    pub fn new(mutator: M, parse: Parse, map: Map) -> Self {
        Self {
            mutator,
            parse,
            map,
            _phantom: PhantomData
        }
    }
}

pub struct Cache<From, M> where From:Clone, M: Mutator<From> {
    from_value: From,
    from_cache: M::Cache
}

impl<From, To, M, Parse, Map> Mutator<To> for MapMutator<From, To, M, Parse, Map> where From: Clone, To: Clone, M: Mutator<From>, Parse: Fn(&To) -> Option<From>, Map: Fn(&From) -> To {
    type Cache = Cache<From, M>;
    type MutationStep = M::MutationStep;
    type ArbitraryStep = M::ArbitraryStep;
    type UnmutateToken = M::UnmutateToken;

    fn default_arbitrary_step(&self) -> Self::ArbitraryStep {
        self.mutator.default_arbitrary_step()
    }

    fn validate_value(&self, to_value: &To) -> Option<(Self::Cache, Self::MutationStep)> {
        let from_value = (self.parse)(to_value)?;
        let (from_cache, step) = self.mutator.validate_value(&from_value)?;
        Some((
            Cache {
                from_value,
                from_cache,
            },
            step
        ))
    }

    fn max_complexity(&self) -> f64 {
        self.mutator.max_complexity()
    }

    fn min_complexity(&self) -> f64 {
        self.mutator.min_complexity()
    }

    fn complexity(&self, _value: &To, cache: &Self::Cache) -> f64 {
        self.mutator.complexity(&cache.from_value, &cache.from_cache)
    }

    fn ordered_arbitrary(&self, step: &mut Self::ArbitraryStep, max_cplx: f64) -> Option<(To, f64)> {
        let (from_value, cplx) = self.mutator.ordered_arbitrary(step, max_cplx)?;
        let to_value = (self.map)(&from_value);
        Some((to_value, cplx))
    }

    fn random_arbitrary(&self, max_cplx: f64) -> (To, f64) {
        let (from_value, cplx) = self.mutator.random_arbitrary(max_cplx);
        let to_value = (self.map)(&from_value);
        (to_value, cplx)
    }

    fn ordered_mutate(
        &self,
        value: &mut To,
        cache: &mut Self::Cache,
        step: &mut Self::MutationStep,
        max_cplx: f64,
    ) -> Option<(Self::UnmutateToken, f64)> {
        let (token, cplx) = self.mutator.ordered_mutate(&mut cache.from_value, &mut cache.from_cache, step, max_cplx)?;
        *value = (self.map)(&cache.from_value);
        Some((token, cplx))
    }

    fn random_mutate(&self, value: &mut To, cache: &mut Self::Cache, max_cplx: f64) -> (Self::UnmutateToken, f64) {
        let (token, cplx) = self.mutator.random_mutate(&mut cache.from_value, &mut cache.from_cache, max_cplx);
        *value = (self.map)(&cache.from_value);
        (token, cplx)
    }

    fn unmutate(&self, value: &mut To, cache: &mut Self::Cache, t: Self::UnmutateToken) {
        self.mutator.unmutate(&mut cache.from_value, &mut cache.from_cache, t);
        *value = (self.map)(&cache.from_value);
    }
}