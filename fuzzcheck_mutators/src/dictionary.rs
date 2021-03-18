use fuzzcheck_traits::Mutator;

pub struct DictionaryMutator<T: Clone, M: Mutator<T>> {
    m: M,
    dictionary: Vec<(T, <M as Mutator<T>>::Cache, <M as Mutator<T>>::MutationStep)>,
    rng: fastrand::Rng,
}
impl<T: Clone, M: Mutator<T>> DictionaryMutator<T, M> {
    pub fn new(value_mutator: M, dictionary: impl Iterator<Item = T>) -> Self {
        let dictionary = dictionary
            .filter_map(|v| {
                if let Some((cache, step)) = value_mutator.validate_value(&v) {
                    Some((v, cache, step))
                } else {
                    None
                }
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
pub struct MutationStep<T> {
    idx: usize,
    wrapped: T,
}
impl<T> MutationStep<T> {
    fn new(wrapped: T) -> Self {
        Self { idx: 0, wrapped }
    }
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

    fn default_arbitrary_step(&self) -> Self::ArbitraryStep {
        <_>::default()
    }

    fn validate_value(&self, value: &T) -> Option<(Self::Cache, Self::MutationStep)> {
        if let Some((cache, step)) = self.m.validate_value(value) {
            Some((cache, Self::MutationStep::new(step)))
        } else {
            None
        }
    }

    fn ordered_arbitrary(
        &self,
        step: &mut Self::ArbitraryStep,
        max_cplx: f64,
    ) -> Option<(T, Self::Cache, Self::MutationStep)> {
        match step {
            ArbitraryStep::Dictionary(inner_step) => {
                if *inner_step < self.dictionary.len() {
                    let (v, c, s) = self.dictionary[*inner_step].clone();
                    *inner_step += 1;
                    Some((v, c, Self::MutationStep::new(s)))
                } else {
                    let inner_step = self.m.default_arbitrary_step();
                    *step = self::ArbitraryStep::Wrapped(inner_step);
                    self.ordered_arbitrary(step, max_cplx)
                }
            }
            ArbitraryStep::Wrapped(inner_step) => self
                .m
                .ordered_arbitrary(inner_step, max_cplx)
                .map(|(v, c, s)| (v.into(), c, MutationStep::new(s))),
        }
    }

    fn random_arbitrary(&self, max_cplx: f64) -> (T, Self::Cache, Self::MutationStep) {
        let (v, c, s) = if !self.dictionary.is_empty() && self.rng.usize(..20) == 0 {
            let idx = self.rng.usize(..self.dictionary.len());
            self.dictionary[idx].clone()
        } else {
            self.m.random_arbitrary(max_cplx)
        };
        (v, c, MutationStep::new(s))
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
        if step.idx < self.dictionary.len() {
            let (new_value, new_cache, _new_step) = self.dictionary[step.idx].clone();
            step.idx += 1;
            let old_value = std::mem::replace(value, new_value);
            let old_cache = std::mem::replace(cache, new_cache);

            Some(UnmutateToken::Replace(old_value, old_cache))
        } else {
            self.m
                .ordered_mutate(value, cache, &mut step.wrapped, max_cplx)
                .map(self::UnmutateToken::Unmutate)
        }
    }

    fn random_mutate(&self, value: &mut T, cache: &mut Self::Cache, max_cplx: f64) -> Self::UnmutateToken {
        if !self.dictionary.is_empty() && self.rng.usize(..20) == 0 {
            let idx = self.rng.usize(..self.dictionary.len());
            let (new_value, new_cache, _new_step) = self.dictionary[idx].clone();

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
