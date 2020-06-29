use std::marker::PhantomData;

use fuzzcheck_traits::Mutator;

use fastrand::Rng;

macro_rules! match_all_eithers {
    ( $main:expr, $( $others:expr ),* ) => {
        {
            match $main {
                Either::Left(inner_main) => {
                    Either::Left((inner_main $(, $others.unwrap_left() )*))
                },
                Either::Right(inner_main) => {
                    Either::Right((inner_main $(, $others.unwrap_right() )*))
                },
            }
        }
    };
}

#[derive(Clone, Copy)]
pub enum Either<A, B> {
    Left(A),
    Right(B),
}
impl<A, B> Either<A, B> {
    fn unwrap_left(self) -> A {
        match self {
            Either::Left(a) => a,
            Either::Right(_) => panic!(),
        }
    }
    fn unwrap_right(self) -> B {
        match self {
            Either::Left(_) => panic!(),
            Either::Right(b) => b,
        }
    }
    fn as_ref(&self) -> Either<&A, &B> {
        match self {
            Either::Left(a) => Either::Left(a),
            Either::Right(b) => Either::Right(b),
        }
    }
    fn as_mut(&mut self) -> Either<&mut A, &mut B> {
        match self {
            Either::Left(a) => Either::Left(a),
            Either::Right(b) => Either::Right(b),
        }
    }
}

pub trait EitherMap {
    type A;
    type B;
    type V: Clone;

    fn left(a: Self::A) -> Self::V;
    fn right(b: Self::B) -> Self::V;

    fn get_either(v: &Self::V) -> Either<&Self::A, &Self::B>;
    fn get_either_mut(v: &mut Self::V) -> Either<&mut Self::A, &mut Self::B>;
}

pub struct EitherMutator<Map, A, B>
where
    A: Mutator,
    B: Mutator,
    Map: EitherMap,
{
    a: A,
    b: B,
    rng: Rng,
    phantom: PhantomData<Map>,
}
impl<Map: EitherMap, A: Mutator, B: Mutator> EitherMutator<Map, A, B> {
    pub fn new(a: A, b: B) -> Self {
        Self {
            a,
            b,
            rng: Rng::new(),
            phantom: PhantomData,
        }
    }
}
impl<Map: EitherMap, A: Mutator, B: Mutator> Default for EitherMutator<Map, A, B>
where
    A: Default,
    B: Default,
{
    fn default() -> Self {
        Self::new(A::default(), B::default())
    }
}

#[derive(Clone)]
pub struct EitherMutatorStep<A, B> {
    inner: Either<A, B>,
    pick_step: usize,
}

pub enum UnmutateEitherToken<V, C, TokenA, TokenB> {
    Restore(V, C),
    UnmutateLeft(TokenA),
    UnmutateRight(TokenB),
}

impl<A: Mutator, B: Mutator, Map: EitherMap<A = A::Value, B = B::Value>> Mutator for EitherMutator<Map, A, B> {
    type Value = Map::V;
    type Cache = Either<A::Cache, B::Cache>;
    type MutationStep = EitherMutatorStep<A::MutationStep, B::MutationStep>;
    type UnmutateToken = UnmutateEitherToken<Self::Value, Self::Cache, A::UnmutateToken, B::UnmutateToken>;

    fn max_complexity(&self) -> f64 {
        1.0 + f64::max(self.a.max_complexity(), self.b.max_complexity())
    }

    fn min_complexity(&self) -> f64 {
        1.0 + f64::min(self.a.min_complexity(), self.b.min_complexity())
    }

    fn complexity(&self, value: &Self::Value, cache: &Self::Cache) -> f64 {
        1.0 + match match_all_eithers!(Map::get_either(value), cache.as_ref()) {
            Either::Left((value, cache)) => self.a.complexity(value, cache),
            Either::Right((value, cache)) => self.b.complexity(value, cache),
        }
    }

    fn cache_from_value(&self, value: &Self::Value) -> Self::Cache {
        match Map::get_either(value) {
            Either::Left(inner_value) => {
                let inner_cache = self.a.cache_from_value(inner_value);
                Either::Left(inner_cache)
            }
            Either::Right(inner_value) => {
                let inner_cache = self.b.cache_from_value(inner_value);
                Either::Right(inner_cache)
            }
        }
    }
    fn mutation_step_from_value(&self, value: &Self::Value) -> Self::MutationStep {
        match Map::get_either(value) {
            Either::Left(inner_value) => {
                let inner_step = self.a.mutation_step_from_value(inner_value);
                EitherMutatorStep {
                    inner: Either::Left(inner_step),
                    pick_step: 0,
                }
            }
            Either::Right(inner_value) => {
                let inner_step = self.b.mutation_step_from_value(inner_value);
                EitherMutatorStep {
                    inner: Either::Right(inner_step),
                    pick_step: 0,
                }
            }
        }
    }

    fn arbitrary(&mut self, seed: usize, max_cplx: f64) -> (Self::Value, Self::Cache) {
        let pick_left = seed % 2 == 0;
        let seed = seed / 2;
        if pick_left {
            let (inner_value, inner_cache) = self.a.arbitrary(seed, max_cplx - 1.0);
            (Map::left(inner_value), Either::Left(inner_cache))
        } else {
            let (inner_value, inner_cache) = self.b.arbitrary(seed, max_cplx - 1.0);
            (Map::right(inner_value), Either::Right(inner_cache))
        }
    }

    fn mutate(
        &mut self,
        value: &mut Self::Value,
        cache: &mut Self::Cache,
        step: &mut Self::MutationStep,
        max_cplx: f64,
    ) -> Self::UnmutateToken {
        let max_cplx = max_cplx - 1.0;
        step.pick_step += 1;

        if step.pick_step % 100 == 0 {
            // switch to a different branch once every 100 times
            match Map::get_either(value) {
                Either::Left(_) => {
                    let (tmp_inner_value, tmp_inner_cache) = self.b.arbitrary(self.rng.usize(..), max_cplx);
                    let mut tmp_value = Map::right(tmp_inner_value);
                    let mut tmp_cache = Either::Right(tmp_inner_cache);

                    std::mem::swap(&mut tmp_cache, cache);
                    std::mem::swap(&mut tmp_value, value);

                    UnmutateEitherToken::Restore(tmp_value, tmp_cache)
                }
                Either::Right(_) => {
                    let (tmp_inner_value, tmp_inner_cache) = self.a.arbitrary(self.rng.usize(..), max_cplx);
                    let mut tmp_value = Map::left(tmp_inner_value);
                    let mut tmp_cache = Either::Left(tmp_inner_cache);

                    std::mem::swap(&mut tmp_cache, cache);
                    std::mem::swap(&mut tmp_value, value);

                    UnmutateEitherToken::Restore(tmp_value, tmp_cache)
                }
            }
        } else {
            match match_all_eithers!(Map::get_either_mut(value), cache.as_mut(), step.inner.as_mut()) {
                Either::Left((inner_value, inner_cache, inner_step)) => {
                    let inner_token = self.a.mutate(inner_value, inner_cache, inner_step, max_cplx);
                    UnmutateEitherToken::UnmutateLeft(inner_token)
                }
                Either::Right((inner_value, inner_cache, inner_step)) => {
                    let inner_token = self.b.mutate(inner_value, inner_cache, inner_step, max_cplx);
                    UnmutateEitherToken::UnmutateRight(inner_token)
                }
            }
        }
    }

    fn unmutate(&self, value: &mut Self::Value, cache: &mut Self::Cache, t: Self::UnmutateToken) {
        match t {
            UnmutateEitherToken::Restore(v, c) => {
                *value = v;
                *cache = c;
            }
            UnmutateEitherToken::UnmutateLeft(left_token) => {
                let left_value = Map::get_either_mut(value).unwrap_left();
                let left_cache = cache.as_mut().unwrap_left();
                self.a.unmutate(left_value, left_cache, left_token);
            }
            UnmutateEitherToken::UnmutateRight(right_token) => {
                let right_value = Map::get_either_mut(value).unwrap_right();
                let right_cache = cache.as_mut().unwrap_right();
                self.b.unmutate(right_value, right_cache, right_token);
            }
        }
    }
}
