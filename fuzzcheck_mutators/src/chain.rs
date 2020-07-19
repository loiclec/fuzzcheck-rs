
use fuzzcheck_traits::Mutator;

#[derive(Default)]
pub struct ChainingMutator<M: Mutator> {
    m: M,
    rng: fastrand::Rng
}
impl<M: Mutator> ChainingMutator<M> {
    pub fn new(value_mutator: M) -> Self {
        Self { m : value_mutator, rng: fastrand::Rng::new() }
    }
}

impl<M: Mutator> Mutator for ChainingMutator<M> {
    type Value = M::Value;
    type Cache = M::Cache;
    type MutationStep = M::MutationStep;
    type UnmutateToken = Vec<M::UnmutateToken>;

    fn cache_from_value(&self, value: &Self::Value) -> Self::Cache {
        self.m.cache_from_value(value)
    }

    fn initial_step_from_value(&self, value: &Self::Value) -> Self::MutationStep { 
        self.m.initial_step_from_value(value)
    }

    fn random_step_from_value(&self, value: &Self::Value) -> Self::MutationStep { 
        self.m.random_step_from_value(value)
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
        let mut tokens = Vec::new();
        let r = self.rng.usize(..150);
        if r < 15 {
            let mut s = self.m.random_step_from_value(value);
            tokens.push(self.m.mutate(value, cache, &mut s, max_cplx));

            if r < 3 {
                let mut s = self.m.random_step_from_value(value);
                tokens.push(self.m.mutate(value, cache, &mut s, max_cplx));
            
                if r == 0 {
                    let mut s = self.m.random_step_from_value(value);
                    tokens.push(self.m.mutate(value, cache, &mut s, max_cplx));
                }
            }
        } else {
            tokens.push(self.m.mutate(value, cache, step, max_cplx));
        }
        tokens
    }

    fn unmutate(&self, value: &mut Self::Value, cache: &mut Self::Cache, t: Self::UnmutateToken) {
        for token in t.into_iter().rev() {
            self.m.unmutate(value, cache, token)
        }
    }
}
