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

#[derive(Default)]
pub struct OptionMutator<M: Mutator> {
    m: M,
    rng: fastrand::Rng
}
impl<M: Mutator> OptionMutator<M> {
    pub fn new(value_mutator: M) -> Self {
        Self { m : value_mutator, rng: fastrand::Rng::new() }
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

pub enum UnmutateToken<Value, Token> {
    UnmutateSome(Token),
    ToSome(Value),
    ToNone,
}
use crate::option::UnmutateToken::{ToNone, ToSome, UnmutateSome};

#[derive(Debug, Clone)]
pub struct MutatorStep<MS, AS> {
    did_check_none: bool,
    inner_arbitrary: AS,
    inner: Option<MS>,
}

#[derive(Clone)]
pub struct ArbitraryStep<T> where T: Default + Clone {
    check_none: bool,
    inner_step: T,
}
impl<T> Default for ArbitraryStep<T> where T: Default + Clone {
    fn default() -> Self {
        Self {
            check_none: true,
            inner_step: <_>::default()
        }
    }
}

impl<M: Mutator> Mutator for OptionMutator<M> {
    type Value = Option<M::Value>;
    type Cache = Option<M::Cache>;
    type MutationStep = MutatorStep<M::MutationStep, M::ArbitraryStep>;
    type ArbitraryStep = ArbitraryStep<M::ArbitraryStep>;
    type UnmutateToken = UnmutateToken<M::Value, M::UnmutateToken>;

    fn cache_from_value(&self, value: &Self::Value) -> Self::Cache {
        value.as_ref().map(|inner| self.m.cache_from_value(&inner))
    }

    fn initial_step_from_value(&self, value: &Self::Value) -> Self::MutationStep {
        MutatorStep {
            did_check_none: value.is_none(),
            inner_arbitrary: <_>::default(),
            inner: value.as_ref().map(|inner| self.m.initial_step_from_value(&inner)),
        }
    }
    fn random_step_from_value(&self, value: &Self::Value) -> Self::MutationStep {
        MutatorStep {
            did_check_none: value.is_none(),
            inner_arbitrary: <_>::default(),
            inner: value.as_ref().map(|inner| self.m.random_step_from_value(&inner)),
        }
    }

    fn ordered_arbitrary(&mut self, step: &mut Self::ArbitraryStep, max_cplx: f64) -> Option<(Self::Value, Self::Cache)> {
        if step.check_none {
            step.check_none = false;
            Some((None, None))
        } else {
            if let Some((inner_value, inner_cache)) = self.m.ordered_arbitrary(&mut step.inner_step, max_cplx - 1.0) {
                Some((Some(inner_value), Some(inner_cache)))
            } else {
                None
            }
        }
    }
    fn random_arbitrary(&mut self, max_cplx: f64) -> (Self::Value, Self::Cache) {
        let max_cplx_some = self.m.max_complexity();
        let odds = if max_cplx_some.is_finite() && max_cplx < 100.0 {
            if max_cplx > 1.0 { max_cplx as usize } else { 2 }
        } else {
            100
        };
        if self.rng.usize(0 .. odds+1) == 0 {
            (None, None)
        } else {
            let (value, cache) = self.m.random_arbitrary(max_cplx);
            (Some(value), Some(cache))
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
    ) -> Option<Self::UnmutateToken> {
        let inner_max_cplx = max_cplx - 1.0;

        if !step.did_check_none {
            let mut old_value = None;
            std::mem::swap(value, &mut old_value);
            step.did_check_none = true;
            Some(ToSome(old_value.unwrap()))
        } else if let Some((inner_value, inner_cache, inner_step)) =
            match_all_options!(value.as_mut(), cache.as_mut(), step.inner.as_mut())
        {
            if let Some(inner_token) = self.m.mutate(inner_value, inner_cache, inner_step, inner_max_cplx) {
                Some(UnmutateSome(inner_token))
            } else {
                None
            }
        } else {
            if let Some((inner_value, inner_cache)) = self.m.ordered_arbitrary(&mut step.inner_arbitrary, inner_max_cplx) {
                *value = Some(inner_value);
                *cache = Some(inner_cache);

                Some(ToNone)
            } else {
                None
            }
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
