
use fuzzcheck_traits::Mutator;

pub struct DictionaryMutator<M: Mutator> {
    m: M,
    dictionary: Vec<(<M as Mutator>::Value, <M as Mutator>::Cache)>,
    rng: fastrand::Rng
}
impl<M: Mutator> DictionaryMutator<M> {
    pub fn new(value_mutator: M, dictionary: impl Iterator<Item = <M as Mutator>::Value>) -> Self {
        let dictionary = dictionary.map(|v| {
            let cache = value_mutator.cache_from_value(&v);
            (v, cache)
        }).collect();
        Self { 
            m: value_mutator, 
            dictionary,
            rng: fastrand::Rng::new()
        }
    }
}

#[derive(Clone)]
pub struct MutationStep<S> {
    counter: usize,
    step: S
}

pub enum UnmutateToken<M: Mutator> {
    Replace(M::Value, M::Cache),
    Unmutate(M::UnmutateToken),
}

impl<M: Mutator> Mutator for DictionaryMutator<M> {
    type Value = M::Value;
    type Cache = M::Cache;
    type MutationStep = MutationStep<M::MutationStep>;
    type UnmutateToken = UnmutateToken<M>;

    fn cache_from_value(&self, value: &Self::Value) -> Self::Cache {
        self.m.cache_from_value(value)
    }

    fn initial_step_from_value(&self, value: &Self::Value) -> Self::MutationStep { 
        MutationStep {
            counter: 0,
            step: self.m.initial_step_from_value(value)
        }
    }

    fn random_step_from_value(&self, value: &Self::Value) -> Self::MutationStep { 
        MutationStep {
            counter: self.rng.usize(..),
            step: self.m.random_step_from_value(value)
        }
    }

    fn arbitrary(&mut self, seed: usize, max_cplx: f64) -> (Self::Value, Self::Cache) {
        self.m.arbitrary(seed, max_cplx)
    }

    fn max_complexity(&self) -> f64 {
        self.m.max_complexity()
    }

    fn min_complexity(&self) -> f64 {
        self.m.min_complexity()
    }

    fn complexity(&self, value: &Self::Value, cache: &Self::Cache) -> f64 {
        self.m.complexity(value, cache)
    }

    fn mutate(
        &mut self,
        value: &mut Self::Value,
        cache: &mut Self::Cache,
        step: &mut Self::MutationStep,
        max_cplx: f64,
    ) -> Self::UnmutateToken {
        
        let token = if step.counter < self.dictionary.len() || self.rng.usize(..250) == 0 {
            let (new_value, new_cache) = self.dictionary[step.counter % self.dictionary.len()].clone();
            
            let old_value = std::mem::replace(value, new_value);
            let old_cache = std::mem::replace(cache, new_cache);
           
            UnmutateToken::Replace(old_value, old_cache)
        } else {
            UnmutateToken::Unmutate(self.m.mutate(value, cache, &mut step.step, max_cplx))
        };

        step.counter += 1;
        token
    }

    fn unmutate(&self, value: &mut Self::Value, cache: &mut Self::Cache, t: Self::UnmutateToken) {
        match t {
            UnmutateToken::Replace(new_value, new_cache) => {
                let _ = std::mem::replace(value, new_value);
                let _ = std::mem::replace(cache, new_cache);
            }
            UnmutateToken::Unmutate(t) => {
                self.m.unmutate(value, cache, t)
            }
        }
    }
}
