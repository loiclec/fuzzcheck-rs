use crate::Mutator;

/** Wrap a mutator and prioritise the generation of a few given values.
```
use fuzzcheck::DefaultMutator;
use fuzzcheck::mutators::dictionary::DictionaryMutator;

let m = usize::default_mutator();
let m = DictionaryMutator::new(m, [256, 65_536, 1_000_000]);
// m will first generate the values given to the DictionaryMutator constructor
// and will then use usize’s default mutator
```
*/
pub struct DictionaryMutator<T: Clone, M: Mutator<T>> {
    m: M,
    dictionary: Vec<(T, f64)>,
    rng: fastrand::Rng,
}
impl<T: Clone, M: Mutator<T>> DictionaryMutator<T, M> {
    #[no_coverage]
    pub fn new(value_mutator: M, dictionary: impl IntoIterator<Item = T>) -> Self {
        let dictionary = dictionary
            .into_iter()
            .filter_map(
                #[no_coverage]
                |v| {
                    if let Some(cache) = value_mutator.validate_value(&v) {
                        let complexity = value_mutator.complexity(&v, &cache);
                        Some((v, complexity))
                    } else {
                        None
                    }
                },
            )
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

impl<T: Clone + 'static, M: Mutator<T>> Mutator<T> for DictionaryMutator<T, M> {
    #[doc(hidden)]
    type Cache = M::Cache;
    #[doc(hidden)]
    type MutationStep = self::MutationStep<M::MutationStep>;
    #[doc(hidden)]
    type ArbitraryStep = self::ArbitraryStep<M::ArbitraryStep>;
    #[doc(hidden)]
    type UnmutateToken = UnmutateToken<T, M>;

    #[doc(hidden)]
    #[no_coverage]
    fn default_arbitrary_step(&self) -> Self::ArbitraryStep {
        <_>::default()
    }

    #[doc(hidden)]
    #[no_coverage]
    fn validate_value(&self, value: &T) -> Option<Self::Cache> {
        self.m.validate_value(value)
    }
    #[doc(hidden)]
    #[no_coverage]
    fn default_mutation_step(&self, value: &T, cache: &Self::Cache) -> Self::MutationStep {
        Self::MutationStep::new(self.m.default_mutation_step(value, cache))
    }

    #[doc(hidden)]
    #[no_coverage]
    fn max_complexity(&self) -> f64 {
        self.m.max_complexity()
    }

    #[doc(hidden)]
    #[no_coverage]
    fn min_complexity(&self) -> f64 {
        self.m.min_complexity()
    }

    #[doc(hidden)]
    #[no_coverage]
    fn complexity(&self, value: &T, cache: &Self::Cache) -> f64 {
        self.m.complexity(value, cache)
    }

    #[doc(hidden)]
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

    #[doc(hidden)]
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

    #[doc(hidden)]
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

    #[doc(hidden)]
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

    #[doc(hidden)]
    #[no_coverage]
    fn unmutate(&self, value: &mut T, cache: &mut Self::Cache, t: Self::UnmutateToken) {
        match t {
            UnmutateToken::Replace(new_value) => {
                let _ = std::mem::replace(value, new_value);
            }
            UnmutateToken::Unmutate(t) => self.m.unmutate(value, cache, t),
        }
    }

    #[doc(hidden)]
    type LensPath = M::LensPath;

    #[doc(hidden)]
    #[no_coverage]
    fn lens<'a>(&self, value: &'a T, cache: &'a Self::Cache, path: &Self::LensPath) -> &'a dyn std::any::Any {
        self.m.lens(value, cache, path)
    }

    #[doc(hidden)]
    #[no_coverage]
    fn all_paths(&self, value: &T, cache: &Self::Cache, register_path: &mut dyn FnMut(std::any::TypeId, Self::LensPath))
    {
        self.m.all_paths(value, cache, register_path)
    }

    #[doc(hidden)]
    #[no_coverage]
    fn crossover_mutate(
        &self,
        value: &mut T,
        cache: &mut Self::Cache,
        subvalue_provider: &dyn crate::SubValueProvider,
        max_cplx: f64,
    ) -> (Self::UnmutateToken, f64) {
        let (t, cplx) = self.m.crossover_mutate(value, cache, subvalue_provider, max_cplx);
        (self::UnmutateToken::Unmutate(t), cplx)
    }
}
