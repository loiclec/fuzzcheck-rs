use std::marker::PhantomData;

extern crate fuzzcheck;
use fuzzcheck::Mutator;

extern crate fastrand;
use fastrand::Rng;

pub trait TupleMap {
    type A;
    type B;
    type V: Clone;

    fn get_a(v: &Self::V) -> &Self::A;
    fn get_b(v: &Self::V) -> &Self::B;

    fn get_a_mut(v: &mut Self::V) -> &mut Self::A;
    fn get_b_mut(v: &mut Self::V) -> &mut Self::B;

    fn new(a: Self::A, b: Self::B) -> Self::V;
}

impl<A, B> TupleMap for (A, B)
where
    A: Clone,
    B: Clone,
{
    type A = A;
    type B = B;
    type V = Self;

    fn get_a(v: &(A, B)) -> &A {
        &v.0
    }
    fn get_b(v: &(A, B)) -> &B {
        &v.1
    }
    fn get_a_mut(v: &mut (A, B)) -> &mut A {
        &mut v.0
    }
    fn get_b_mut(v: &mut (A, B)) -> &mut B {
        &mut v.1
    }
    fn new(a: A, b: B) -> (A, B) {
        (a, b)
    }
}

pub struct Tuple2Mutator<Map, A, B>
where
    A: Mutator,
    B: Mutator,
    Map: TupleMap,
{
    a: A,
    b: B,
    rng: Rng,
    phantom: PhantomData<Map>,
}
impl<Map: TupleMap, A: Mutator, B: Mutator> Tuple2Mutator<Map, A, B> {
    pub fn new(a: A, b: B) -> Self {
        Self {
            a,
            b,
            rng: Rng::new(),
            phantom: PhantomData,
        }
    }
}
impl<Map: TupleMap, A: Mutator, B: Mutator> Default for Tuple2Mutator<Map, A, B>
where
    A: Default,
    B: Default,
{
    fn default() -> Self {
        Self::new(A::default(), B::default())
    }
}

#[derive(Clone)]
pub struct Tuple2MutatorStep<A, B> {
    a_step: A,
    b_step: B,
    pick_step: usize,
}

pub struct UnmutateTuple2Token<A, B> {
    a: Option<A>,
    b: Option<B>,
}

impl<A: Mutator, B: Mutator, Map: TupleMap<A = A::Value, B = B::Value>> Mutator for Tuple2Mutator<Map, A, B> {
    type Value = Map::V;
    type Cache = (A::Cache, B::Cache);
    type MutationStep = Tuple2MutatorStep<A::MutationStep, B::MutationStep>;
    type UnmutateToken = UnmutateTuple2Token<A::UnmutateToken, B::UnmutateToken>;

    fn max_complexity(&self) -> f64 {
        self.a.max_complexity() + self.b.max_complexity()
    }

    fn min_complexity(&self) -> f64 {
        self.a.min_complexity() + self.b.min_complexity()
    }

    fn complexity(&self, value: &Self::Value, cache: &Self::Cache) -> f64 {
        self.a.complexity(Map::get_a(&value), &cache.0) + self.b.complexity(Map::get_b(&value), &cache.1)
    }

    fn cache_from_value(&self, value: &Self::Value) -> Self::Cache {
        let a_cache = self.a.cache_from_value(Map::get_a(&value));
        let b_cache = self.b.cache_from_value(Map::get_b(&value));

        (a_cache, b_cache)
    }
    fn mutation_step_from_value(&self, value: &Self::Value) -> Self::MutationStep {
        let a_step = self.a.mutation_step_from_value(Map::get_a(&value));
        let b_step = self.b.mutation_step_from_value(Map::get_b(&value));

        Tuple2MutatorStep {
            a_step,
            b_step,
            pick_step: 0,
        }
    }

    fn arbitrary(&mut self, seed: usize, max_cplx: f64) -> (Self::Value, Self::Cache) {
        let cplx = if seed < 10 {
            // first 10 vary in cplx from max_cplx to max_cplx / 10
            max_cplx / (10.0 - seed as f64)
        } else {
            max_cplx * crate::gen_f64(&self.rng, 0.0 .. 1.0)
        };
        let cplx_a = crate::gen_f64(&self.rng, 0.0 .. cplx);
        let (a_value, a_cache) = self.a.arbitrary(self.rng.usize(..), cplx_a);
        let (b_value, b_cache) = self.b.arbitrary(self.rng.usize(..), max_cplx - cplx_a);
        let value = Map::new(a_value, b_value);
        let cache = (a_cache, b_cache);
        (value, cache)
    }

    fn mutate(
        &mut self,
        value: &mut Self::Value,
        cache: &mut Self::Cache,
        step: &mut Self::MutationStep,
        max_cplx: f64,
    ) -> Self::UnmutateToken {
        let cplx_a = self.a.complexity(Map::get_a(&value), &cache.0);
        let cplx_b = self.b.complexity(Map::get_b(&value), &cache.1);

        step.pick_step += 1;
        if step.pick_step % 10 == 0 {
            // mutate both once every ten times
            let cplx = self.complexity(value, cache);
            let remaining_cplx = max_cplx - cplx;
            // allocate remaining complexity randomly to a and b
            // only expands complexity, does not reduce it
            let remaining_a_cplx = remaining_cplx * crate::gen_f64(&self.rng, 0.0 .. 1.0);
            let max_a_cplx = cplx_a + remaining_a_cplx;

            let token_a = self
                .a
                .mutate(Map::get_a_mut(value), &mut cache.0, &mut step.a_step, max_a_cplx);
            let new_a_cplx = self.a.complexity(Map::get_a(&value), &cache.0);

            let max_b_cplx = max_cplx - new_a_cplx;
            let token_b = self
                .b
                .mutate(Map::get_b_mut(value), &mut cache.1, &mut step.b_step, max_b_cplx);
            UnmutateTuple2Token {
                a: Some(token_a),
                b: Some(token_b),
            }
        } else if step.pick_step % 2 == 0 {
            // TODO: frequency depending on complexity or customizable
            let max_a_cplx = max_cplx - cplx_b;
            let token = self
                .a
                .mutate(Map::get_a_mut(value), &mut cache.0, &mut step.a_step, max_a_cplx);
            UnmutateTuple2Token {
                a: Some(token),
                b: None,
            }
        } else {
            // mutate b ~half the time
            let max_b_cplx = max_cplx - cplx_a;
            let token = self
                .b
                .mutate(Map::get_b_mut(value), &mut cache.1, &mut step.b_step, max_b_cplx);
            UnmutateTuple2Token {
                a: None,
                b: Some(token),
            }
        }
    }

    fn unmutate(&self, value: &mut Self::Value, cache: &mut Self::Cache, t: Self::UnmutateToken) {
        if let Some(ua) = t.a {
            self.a.unmutate(Map::get_a_mut(value), &mut cache.0, ua)
        }
        if let Some(ub) = t.b {
            self.b.unmutate(Map::get_b_mut(value), &mut cache.1, ub)
        }
    }
}
