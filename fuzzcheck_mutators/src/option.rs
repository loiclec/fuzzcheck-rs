use crate::HasDefaultMutator;
use fuzzcheck_traits::Mutator;

macro_rules! match_all_options {
    ( $main:expr, $( $others:expr ),* ) => {
        {
            if $main.is_some() {
                Some(($main.unwrap() $(, $others.unwrap())*))
            } else {
                None
            }
        }
    };
}

pub struct OptionMutator<M: Mutator> {
    m: M,
}
impl<M: Mutator> OptionMutator<M> {
    pub fn new(m: M) -> Self {
        Self { m }
    }
}
impl<M: Mutator> Default for OptionMutator<M>
where
    M: Default,
{
    fn default() -> Self {
        Self::new(M::default())
    }
}
impl<T> HasDefaultMutator for Option<T>
where
    T: HasDefaultMutator,
{
    type Mutator = OptionMutator<<T as HasDefaultMutator>::Mutator>;
    fn default_mutator() -> Self::Mutator {
        Self::Mutator::default()
    }
}

pub enum OptionMutatorUnmutateToken<Value, Token> {
    UnmutateSome(Token),
    ToSome(Value),
    ToNone,
}
use crate::option::OptionMutatorUnmutateToken::{ToNone, ToSome, UnmutateSome};

#[derive(Debug, Clone)]
pub struct OptionMutatorStep<MS> {
    did_check_none: bool,
    inner_arbitrary: usize,
    inner: Option<MS>,
}

struct OptionMutatorArbitrarySeed {
    check_none: bool,
    inner_seed: usize,
}

impl OptionMutatorArbitrarySeed {
    fn new(seed: usize) -> Self {
        Self {
            check_none: seed == 0,
            inner_seed: seed.saturating_sub(1),
        }
    }
}

impl<M: Mutator> Mutator for OptionMutator<M> {
    type Value = Option<M::Value>;
    type Cache = Option<M::Cache>;
    type MutationStep = OptionMutatorStep<M::MutationStep>;
    type UnmutateToken = OptionMutatorUnmutateToken<M::Value, M::UnmutateToken>;

    fn cache_from_value(&self, value: &Self::Value) -> Self::Cache {
        value.as_ref().map(|inner| self.m.cache_from_value(&inner))
    }

    fn mutation_step_from_value(&self, value: &Self::Value) -> Self::MutationStep {
        OptionMutatorStep {
            did_check_none: value.is_none(),
            inner_arbitrary: 0,
            inner: value.as_ref().map(|inner| self.m.mutation_step_from_value(&inner)),
        }
    }

    fn arbitrary(&mut self, seed: usize, max_cplx: f64) -> (Self::Value, Self::Cache) {
        let seed = OptionMutatorArbitrarySeed::new(seed);
        if seed.check_none {
            (None, None)
        } else {
            let (inner_value, inner_cache) = self.m.arbitrary(seed.inner_seed, max_cplx - 1.0);
            (Some(inner_value), Some(inner_cache))
        }
    }

    fn max_complexity(&self) -> f64 {
        1.0 + self.m.max_complexity()
    }

    fn min_complexity(&self) -> f64 {
        1.0 + self.m.min_complexity()
    }

    fn complexity(&self, value: &Self::Value, cache: &Self::Cache) -> f64 {
        if let Some((inner_value, inner_cache)) = match_all_options!(value.as_ref(), cache.as_ref()) {
            1.0 + self.m.complexity(inner_value, inner_cache)
        } else {
            1.0
        }
    }

    fn mutate(
        &mut self,
        value: &mut Self::Value,
        cache: &mut Self::Cache,
        step: &mut Self::MutationStep,
        max_cplx: f64,
    ) -> Self::UnmutateToken {
        let inner_max_cplx = max_cplx - 1.0;

        if !step.did_check_none {
            let mut old_value = None;
            std::mem::swap(value, &mut old_value);
            step.did_check_none = true;
            ToSome(old_value.unwrap())
        } else if let Some((inner_value, inner_cache, inner_step)) =
            match_all_options!(value.as_mut(), cache.as_mut(), step.inner.as_mut())
        {
            let inner_token = self.m.mutate(inner_value, inner_cache, inner_step, inner_max_cplx);
            UnmutateSome(inner_token)
        } else {
            let (inner_value, inner_cache) = self.m.arbitrary(step.inner_arbitrary, inner_max_cplx);
            *value = Some(inner_value);
            *cache = Some(inner_cache);

            step.inner_arbitrary += 1;

            ToNone
        }
    }

    fn unmutate(&self, value: &mut Self::Value, cache: &mut Self::Cache, t: Self::UnmutateToken) {
        match t {
            UnmutateSome(t) => {
                let inner_value = value.as_mut().unwrap();
                let inner_cache = cache.as_mut().unwrap();
                self.m.unmutate(inner_value, inner_cache, t);
            }
            ToSome(v) => {
                *value = Some(v);
            }
            ToNone => {
                *value = None;
            }
        }
    }
}
