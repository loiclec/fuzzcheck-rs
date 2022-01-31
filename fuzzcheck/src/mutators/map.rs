use std::marker::PhantomData;

use crate::Mutator;

pub struct MapMutator<From, To, M, Parse, Map>
where
    From: Clone,
    To: Clone,
    M: Mutator<From>,
    Parse: Fn(&To) -> Option<From>,
    Map: Fn(&From) -> To,
{
    pub mutator: M,
    pub parse: Parse,
    pub map: Map,
    _phantom: PhantomData<(From, To)>,
}
impl<From, To, M, Parse, Map> MapMutator<From, To, M, Parse, Map>
where
    From: Clone,
    To: Clone,
    M: Mutator<From>,
    Parse: Fn(&To) -> Option<From>,
    Map: Fn(&From) -> To,
{
    #[no_coverage]
    pub fn new(mutator: M, parse: Parse, map: Map) -> Self {
        Self {
            mutator,
            parse,
            map,
            _phantom: PhantomData,
        }
    }
}

pub struct Cache<From, M>
where
    From: Clone,
    M: Mutator<From>,
{
    from_value: From,
    from_cache: M::Cache,
}
impl<From, M> Clone for Cache<From, M>
where
    From: Clone,
    M: Mutator<From>,
{
    fn clone(&self) -> Self {
        Self {
            from_value: self.from_value.clone(),
            from_cache: self.from_cache.clone(),
        }
    }
}

impl<From, To, M, Parse, Map> Mutator<To> for MapMutator<From, To, M, Parse, Map>
where
    From: Clone,
    To: Clone,
    M: Mutator<From>,
    Parse: Fn(&To) -> Option<From>,
    Map: Fn(&From) -> To,
    Self: 'static,
{
    #[doc(hidden)]
    type Cache = Cache<From, M>;
    #[doc(hidden)]
    type MutationStep = M::MutationStep;
    #[doc(hidden)]
    type ArbitraryStep = M::ArbitraryStep;
    #[doc(hidden)]
    type UnmutateToken = M::UnmutateToken;

    #[doc(hidden)]
    #[no_coverage]
    fn default_arbitrary_step(&self) -> Self::ArbitraryStep {
        self.mutator.default_arbitrary_step()
    }

    #[doc(hidden)]
    #[no_coverage]
    fn validate_value(&self, to_value: &To) -> Option<Self::Cache> {
        let from_value = (self.parse)(to_value)?;
        let from_cache = self.mutator.validate_value(&from_value)?;
        Some(Cache { from_value, from_cache })
    }
    #[doc(hidden)]
    #[no_coverage]
    fn default_mutation_step(&self, _value: &To, cache: &Self::Cache) -> Self::MutationStep {
        self.mutator.default_mutation_step(&cache.from_value, &cache.from_cache)
    }

    #[doc(hidden)]
    #[no_coverage]
    fn max_complexity(&self) -> f64 {
        self.mutator.max_complexity()
    }

    #[doc(hidden)]
    #[no_coverage]
    fn min_complexity(&self) -> f64 {
        self.mutator.min_complexity()
    }

    #[doc(hidden)]
    #[no_coverage]
    fn complexity(&self, _value: &To, cache: &Self::Cache) -> f64 {
        self.mutator.complexity(&cache.from_value, &cache.from_cache)
    }

    #[doc(hidden)]
    #[no_coverage]
    fn ordered_arbitrary(&self, step: &mut Self::ArbitraryStep, max_cplx: f64) -> Option<(To, f64)> {
        let (from_value, cplx) = self.mutator.ordered_arbitrary(step, max_cplx)?;
        let to_value = (self.map)(&from_value);
        Some((to_value, cplx))
    }

    #[doc(hidden)]
    #[no_coverage]
    fn random_arbitrary(&self, max_cplx: f64) -> (To, f64) {
        let (from_value, cplx) = self.mutator.random_arbitrary(max_cplx);
        let to_value = (self.map)(&from_value);
        (to_value, cplx)
    }

    #[doc(hidden)]
    #[no_coverage]
    fn ordered_mutate(
        &self,
        value: &mut To,
        cache: &mut Self::Cache,
        step: &mut Self::MutationStep,
        max_cplx: f64,
    ) -> Option<(Self::UnmutateToken, f64)> {
        let (token, cplx) =
            self.mutator
                .ordered_mutate(&mut cache.from_value, &mut cache.from_cache, step, max_cplx)?;
        *value = (self.map)(&cache.from_value);
        Some((token, cplx))
    }

    #[doc(hidden)]
    #[no_coverage]
    fn random_mutate(&self, value: &mut To, cache: &mut Self::Cache, max_cplx: f64) -> (Self::UnmutateToken, f64) {
        let (token, cplx) = self
            .mutator
            .random_mutate(&mut cache.from_value, &mut cache.from_cache, max_cplx);
        *value = (self.map)(&cache.from_value);
        (token, cplx)
    }

    #[doc(hidden)]
    #[no_coverage]
    fn unmutate(&self, value: &mut To, cache: &mut Self::Cache, t: Self::UnmutateToken) {
        self.mutator.unmutate(&mut cache.from_value, &mut cache.from_cache, t);
        *value = (self.map)(&cache.from_value);
    }

    type LensPath = M::LensPath;

    fn lens<'a>(&self, _value: &'a To, cache: &'a Self::Cache, path: &Self::LensPath) -> &'a dyn std::any::Any {
        self.mutator.lens(&cache.from_value, &cache.from_cache, path)
    }

    fn all_paths(
        &self,
        _value: &To,
        cache: &Self::Cache,
    ) -> std::collections::HashMap<std::any::TypeId, Vec<Self::LensPath>> {
        self.mutator.all_paths(&cache.from_value, &cache.from_cache)
    }

    fn crossover_mutate(
        &self,
        value: &mut To,
        cache: &mut Self::Cache,
        subvalue_provider: &dyn crate::SubValueProvider,
        max_cplx: f64,
    ) -> (Self::UnmutateToken, f64) {
        let (token, cplx) = self.mutator.crossover_mutate(
            &mut cache.from_value,
            &mut cache.from_cache,
            subvalue_provider,
            max_cplx,
        );
        *value = (self.map)(&cache.from_value);
        (token, cplx)
    }
}
