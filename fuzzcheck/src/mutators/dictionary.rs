use crate::Mutator;

pub struct DictionaryMutator<T: Clone, M: Mutator<T>> {
    m: M,
    dictionary: Vec<(T, f64)>,
    rng: fastrand::Rng,
}
impl<T: Clone, M: Mutator<T>> DictionaryMutator<T, M> {
    #[no_coverage]
    pub fn new(value_mutator: M, dictionary: impl Iterator<Item = T>) -> Self {
        let dictionary = dictionary
            .filter_map(|v| {
                if let Some((cache, _)) = value_mutator.validate_value(&v) {
                    let complexity = value_mutator.complexity(&v, &cache);
                    Some((v, complexity))
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
    #[no_coverage]
    fn new(wrapped: T) -> Self {
        Self { idx: 0, wrapped }
    }
}

pub enum UnmutateToken<T: Clone, M: Mutator<T>> {
    Replace(T),
    Unmutate(M::UnmutateToken),
}

#[derive(Clone)]
pub enum ArbitraryStep<T> {
    Dictionary(usize),
    Wrapped(T),
}
impl<T> Default for ArbitraryStep<T> {
    #[no_coverage]
    fn default() -> Self {
        Self::Dictionary(0)
    }
}

impl<T: Clone, M: Mutator<T>> Mutator<T> for DictionaryMutator<T, M> {
    type Cache = M::Cache;
    type MutationStep = self::MutationStep<M::MutationStep>;
    type ArbitraryStep = self::ArbitraryStep<M::ArbitraryStep>;
    type UnmutateToken = UnmutateToken<T, M>;

    #[no_coverage]
    fn default_arbitrary_step(&self) -> Self::ArbitraryStep {
        <_>::default()
    }

    #[no_coverage]
    fn validate_value(&self, value: &T) -> Option<(Self::Cache, Self::MutationStep)> {
        if let Some((cache, step)) = self.m.validate_value(value) {
            Some((cache, Self::MutationStep::new(step)))
        } else {
            None
        }
    }

    #[no_coverage]
    fn ordered_arbitrary(&self, step: &mut Self::ArbitraryStep, max_cplx: f64) -> Option<(T, f64)> {
        match step {
            ArbitraryStep::Dictionary(inner_step) => {
                if *inner_step < self.dictionary.len() {
                    let (v, c) = self.dictionary[*inner_step].clone();
                    *inner_step += 1;
                    Some((v, c))
                } else {
                    let inner_step = self.m.default_arbitrary_step();
                    *step = self::ArbitraryStep::Wrapped(inner_step);
                    self.ordered_arbitrary(step, max_cplx)
                }
            }
            ArbitraryStep::Wrapped(inner_step) => self.m.ordered_arbitrary(inner_step, max_cplx).map(
                #[no_coverage]
                |(v, c)| (v, c),
            ),
        }
    }

    #[no_coverage]
    fn random_arbitrary(&self, max_cplx: f64) -> (T, f64) {
        let (v, c) = if !self.dictionary.is_empty() && self.rng.usize(..20) == 0 {
            let idx = self.rng.usize(..self.dictionary.len());
            self.dictionary[idx].clone()
        } else {
            self.m.random_arbitrary(max_cplx)
        };
        (v, c)
    }

    #[no_coverage]
    fn max_complexity(&self) -> f64 {
        self.m.max_complexity()
    }

    #[no_coverage]
    fn min_complexity(&self) -> f64 {
        self.m.min_complexity()
    }

    #[no_coverage]
    fn complexity(&self, value: &T, cache: &Self::Cache) -> f64 {
        self.m.complexity(value, cache)
    }

    #[no_coverage]
    fn ordered_mutate(
        &self,
        value: &mut T,
        cache: &mut Self::Cache,
        step: &mut Self::MutationStep,
        max_cplx: f64,
    ) -> Option<(Self::UnmutateToken, f64)> {
        if step.idx < self.dictionary.len() {
            let (new_value, new_value_cplx) = self.dictionary[step.idx].clone();
            step.idx += 1;
            let old_value = std::mem::replace(value, new_value);

            Some((UnmutateToken::Replace(old_value), new_value_cplx))
        } else {
            self.m.ordered_mutate(value, cache, &mut step.wrapped, max_cplx).map(
                #[no_coverage]
                |(t, c)| (self::UnmutateToken::Unmutate(t), c),
            )
        }
    }

    #[no_coverage]
    fn random_mutate(&self, value: &mut T, cache: &mut Self::Cache, max_cplx: f64) -> (Self::UnmutateToken, f64) {
        if !self.dictionary.is_empty() && self.rng.usize(..20) == 0 {
            let idx = self.rng.usize(..self.dictionary.len());
            let (new_value, new_value_cplx) = self.dictionary[idx].clone();

            let old_value = std::mem::replace(value, new_value);

            (UnmutateToken::Replace(old_value), new_value_cplx)
        } else {
            let (t, cplx) = self.m.random_mutate(value, cache, max_cplx);
            (self::UnmutateToken::Unmutate(t), cplx)
        }
    }

    #[no_coverage]
    fn unmutate(&self, value: &mut T, cache: &mut Self::Cache, t: Self::UnmutateToken) {
        match t {
            UnmutateToken::Replace(new_value) => {
                let _ = std::mem::replace(value, new_value);
            }
            UnmutateToken::Unmutate(t) => self.m.unmutate(value, cache, t),
        }
    }
}
