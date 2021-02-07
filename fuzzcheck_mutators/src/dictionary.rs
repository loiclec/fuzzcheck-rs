use fuzzcheck_traits::Mutator;

pub struct DictionaryMutator<T: Clone, M: Mutator<T>> {
    m: M,
    dictionary: Vec<(T, <M as Mutator<T>>::Cache)>,
    rng: fastrand::Rng,
}
impl<T: Clone, M: Mutator<T>> DictionaryMutator<T, M> {
    pub fn new(value_mutator: M, dictionary: impl Iterator<Item = T>) -> Self {
        let dictionary = dictionary
            .map(|v| {
                let cache = value_mutator.cache_from_value(&v);
                (v, cache)
            })
            .collect();
        Self {
            m: value_mutator,
            dictionary,
            rng: fastrand::Rng::new(),
        }
    }
}

#[derive(Clone)]
pub enum MutationStep<T> {
    Dictionary(usize),
    Wrapped(T),
}

pub enum UnmutateToken<T: Clone, M: Mutator<T>> {
    Replace(T, M::Cache),
    Unmutate(M::UnmutateToken),
}
#[derive(Clone)]
pub enum ArbitraryStep<T> {
    Dictionary(usize),
    Wrapped(T),
}
impl<T> Default for ArbitraryStep<T> {
    fn default() -> Self {
        Self::Dictionary(0)
    }
}

impl<T: Clone, M: Mutator<T>> Mutator<T> for DictionaryMutator<T, M> {
    type Cache = M::Cache;
    type MutationStep = self::MutationStep<M::MutationStep>;
    type ArbitraryStep = self::ArbitraryStep<M::ArbitraryStep>;
    type UnmutateToken = UnmutateToken<T, M>;

    fn cache_from_value(&self, value: &T) -> Self::Cache {
        self.m.cache_from_value(value)
    }

    fn initial_step_from_value(&self, value: &T) -> Self::MutationStep {
        if self.dictionary.is_empty() {
            self::MutationStep::Wrapped(self.m.initial_step_from_value(value))
        } else {
            self::MutationStep::Dictionary(0)
        }
    }

    fn ordered_arbitrary(&self, step: &mut Self::ArbitraryStep, max_cplx: f64) -> Option<(T, Self::Cache)> {
        match step {
            ArbitraryStep::Dictionary(inner_step) => {
                if *inner_step < self.dictionary.len() {
                    let (v, c) = self.dictionary[*inner_step].clone();
                    *inner_step += 1;
                    Some((v, c))
                } else {
                    let inner_step = <_>::default();
                    *step = self::ArbitraryStep::Wrapped(inner_step);
                    self.ordered_arbitrary(step, max_cplx)
                }
            }
            ArbitraryStep::Wrapped(inner_step) => self
                .m
                .ordered_arbitrary(inner_step, max_cplx)
                .map(|(v, c)| (v.into(), c)),
        }
    }

    fn random_arbitrary(&self, max_cplx: f64) -> (T, Self::Cache) {
        if !self.dictionary.is_empty() && self.rng.usize(..20) == 0 {
            let idx = self.rng.usize(..self.dictionary.len());
            self.dictionary[idx].clone()
        } else {
            let (v, c) = self.m.random_arbitrary(max_cplx);
            (v, c)
        }
    }

    fn max_complexity(&self) -> f64 {
        self.m.max_complexity()
    }

    fn min_complexity(&self) -> f64 {
        self.m.min_complexity()
    }

    fn complexity(&self, value: &T, cache: &Self::Cache) -> f64 {
        self.m.complexity(value, cache)
    }

    fn ordered_mutate(
        &self,
        value: &mut T,
        cache: &mut Self::Cache,
        step: &mut Self::MutationStep,
        max_cplx: f64,
    ) -> Option<Self::UnmutateToken> {
        match step {
            MutationStep::Dictionary(idx) => {
                if *idx < self.dictionary.len() {
                    let (new_value, new_cache) = self.dictionary[*idx].clone();
                    *idx = 1;
                    let old_value = std::mem::replace(value, new_value);
                    let old_cache = std::mem::replace(cache, new_cache);

                    Some(UnmutateToken::Replace(old_value, old_cache))
                } else {
                    *step = self::MutationStep::Wrapped(self.m.initial_step_from_value(&value));
                    self.ordered_mutate(value, cache, step, max_cplx)
                }
            }
            MutationStep::Wrapped(inner_step) => self
                .m
                .ordered_mutate(value, cache, inner_step, max_cplx)
                .map(self::UnmutateToken::Unmutate),
        }
    }

    fn random_mutate(&self, value: &mut T, cache: &mut Self::Cache, max_cplx: f64) -> Self::UnmutateToken {
        if !self.dictionary.is_empty() && self.rng.usize(..20) == 0 {
            let idx = self.rng.usize(..self.dictionary.len());
            let (new_value, new_cache) = self.dictionary[idx].clone();

            let old_value = std::mem::replace(value, new_value);
            let old_cache = std::mem::replace(cache, new_cache);

            UnmutateToken::Replace(old_value, old_cache)
        } else {
            self::UnmutateToken::Unmutate(self.m.random_mutate(value, cache, max_cplx))
        }
    }

    fn unmutate(&self, value: &mut T, cache: &mut Self::Cache, t: Self::UnmutateToken) {
        match t {
            UnmutateToken::Replace(new_value, new_cache) => {
                let _ = std::mem::replace(value, new_value);
                let _ = std::mem::replace(cache, new_cache);
            }
            UnmutateToken::Unmutate(t) => self.m.unmutate(value, cache, t),
        }
    }
}
