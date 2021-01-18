use fuzzcheck_traits::Mutator;
use std::marker::PhantomData;

fuzzcheck_mutators_derive::make_basic_tuple_mutator!(2);

pub struct Tuple2Mutator<T0, T1, MT0, MT1>
where
    T0: Clone,
    T1: Clone,
    MT0: Mutator<T0>,
    MT1: Mutator<T1>,
{
    pub a: MT0,
    pub b: MT1,
    rng: fastrand::Rng,
    _phantom: PhantomData<(T0, T1)>,
}
impl<T0, T1, MT0, MT1> Tuple2Mutator<T0, T1, MT0, MT1>
where
    T0: Clone,
    T1: Clone,
    MT0: Mutator<T0>,
    MT1: Mutator<T1>,
{
    pub fn new(m0: MT0, m1: MT1) -> Self {
        Self {
            a: m0,
            b: m1,
            rng: <_>::default(),
            _phantom: PhantomData,
        }
    }

    pub fn replacing_mutator_a<MA2>(self, mutator: MA2) -> Tuple2Mutator<T0, T1, MA2, MT1>
    where
        MA2: Mutator<T0>,
    {
        Tuple2Mutator {
            a: mutator,
            b: self.b,
            rng: self.rng,
            _phantom: self._phantom,
        }
    }
    pub fn replacing_mutator_b<MB2>(self, mutator: MB2) -> Tuple2Mutator<T0, T1, MT0, MB2>
    where
        MB2: Mutator<T1>,
    {
        Tuple2Mutator {
            a: self.a,
            b: mutator,
            rng: self.rng,
            _phantom: self._phantom,
        }
    }
}

#[derive(Clone)]
pub struct Cache<A, B> {
    pub a: A,
    pub b: B,
    pub cplx: f64,
}
#[derive(Clone)]
pub enum InnerMutationStep {
    A,
    B,
}
#[derive(Clone)]
pub struct MutationStep<A, B> {
    pub a: A,
    pub b: B,
    pub step: usize,
    pub inner: Vec<InnerMutationStep>,
}

#[derive(Default, Clone)]
pub struct ArbitraryStep<A, B> {
    a: A,
    b: B,
}

pub struct UnmutateToken<A, B> {
    pub a: Option<A>,
    pub b: Option<B>,
    pub cplx: f64,
}
impl<A, B> Default for UnmutateToken<A, B> {
    fn default() -> Self {
        Self {
            a: None,
            b: None,
            cplx: <_>::default(),
        }
    }
}

impl<T, A, B, MA, MB> Mutator<T> for Tuple2Mutator<A, B, MA, MB>
where
    T: Clone,
    A: Clone,
    B: Clone,
    MA: Mutator<A>,
    MB: Mutator<B>,
    T: Tuple2Structure<T0 = A, T1 = B>,
{
    type Cache = Cache<<MA as Mutator<A>>::Cache, <MB as Mutator<B>>::Cache>;
    type MutationStep = MutationStep<<MA as Mutator<A>>::MutationStep, <MB as Mutator<B>>::MutationStep>;
    type ArbitraryStep = ArbitraryStep<<MA as Mutator<A>>::ArbitraryStep, <MB as Mutator<B>>::ArbitraryStep>;
    type UnmutateToken = UnmutateToken<<MA as Mutator<A>>::UnmutateToken, <MB as Mutator<B>>::UnmutateToken>;

    fn max_complexity(&self) -> f64 {
        self.a.max_complexity() + self.b.max_complexity()
    }
    fn min_complexity(&self) -> f64 {
        self.a.min_complexity() + self.b.min_complexity()
    }
    fn complexity(&self, _value: &T, cache: &Self::Cache) -> f64 {
        cache.cplx
    }
    fn cache_from_value(&self, value: &T) -> Self::Cache {
        let a = self.a.cache_from_value(value.get_0());
        let b = self.b.cache_from_value(value.get_1());
        let cplx = self.a.complexity(value.get_0(), &a) + self.b.complexity(value.get_1(), &b);
        Self::Cache { a, b, cplx }
    }
    fn initial_step_from_value(&self, value: &T) -> Self::MutationStep {
        let a = self.a.initial_step_from_value(value.get_0());
        let b = self.b.initial_step_from_value(value.get_1());
        let step = 0;
        Self::MutationStep {
            a,
            b,
            inner: vec![InnerMutationStep::A, InnerMutationStep::B],
            step,
        }
    }
    fn ordered_arbitrary(&mut self, step: &mut Self::ArbitraryStep, max_cplx: f64) -> Option<(T, Self::Cache)> {
        Option::Some(self.random_arbitrary(max_cplx))
    }
    fn random_arbitrary(&mut self, max_cplx: f64) -> (T, Self::Cache) {
        let mut a_value: Option<_> = Option::None;
        let mut a_cache: Option<_> = Option::None;
        let mut b_value: Option<_> = Option::None;
        let mut b_cache: Option<_> = Option::None;
        let mut indices = (0..2).collect::<Vec<_>>();
        fastrand::shuffle(&mut indices);

        let mut cplx = f64::default();
        for idx in indices.iter() {
            match idx {
                0 => {
                    let (value, cache) = self.a.random_arbitrary(max_cplx - cplx);
                    cplx += self.a.complexity(&value, &cache);
                    a_value = Option::Some(value);
                    a_cache = Option::Some(cache);
                }
                1 => {
                    let (value, cache) = self.b.random_arbitrary(max_cplx - cplx);
                    cplx += self.b.complexity(&value, &cache);
                    b_value = Option::Some(value);
                    b_cache = Option::Some(cache);
                }
                _ => unreachable!(),
            }
        }
        (
            T::new((a_value.unwrap(), b_value.unwrap())),
            Self::Cache {
                a: a_cache.unwrap(),
                b: b_cache.unwrap(),
                cplx,
            },
        )
    }

    fn ordered_mutate(
        &mut self,
        value: &mut T,
        cache: &mut Self::Cache,
        step: &mut Self::MutationStep,
        max_cplx: f64,
    ) -> Option<Self::UnmutateToken> {
        if step.inner.is_empty() {
            return Option::None;
        }
        let orig_step = step.step;
        step.step += 1;
        let current_cplx = self.complexity(value, cache);
        let inner_step_to_remove: usize;

        match step.inner[orig_step % step.inner.len()] {
            InnerMutationStep::A => {
                let current_field_cplx = self.a.complexity(value.get_0(), &cache.a);
                let max_field_cplx = max_cplx - current_cplx + current_field_cplx;
                if let Option::Some(token) =
                    self.a
                        .ordered_mutate(value.get_0_mut(), &mut cache.a, &mut step.a, max_field_cplx)
                {
                    let new_field_complexity = self.a.complexity(value.get_0(), &cache.a);
                    cache.cplx = cache.cplx - current_field_cplx + new_field_complexity;
                    return Option::Some(Self::UnmutateToken {
                        a: Option::Some(token),
                        cplx: current_cplx,
                        ..Self::UnmutateToken::default()
                    });
                } else {
                    inner_step_to_remove = orig_step % step.inner.len();
                }
            }
            InnerMutationStep::B => {
                let current_field_cplx = self.b.complexity(value.get_1(), &cache.b);
                let max_field_cplx = max_cplx - current_cplx + current_field_cplx;
                if let Option::Some(token) =
                    self.b
                        .ordered_mutate(value.get_1_mut(), &mut cache.b, &mut step.b, max_field_cplx)
                {
                    let new_field_complexity = self.b.complexity(value.get_1(), &cache.b);
                    cache.cplx = cache.cplx - current_field_cplx + new_field_complexity;
                    return Option::Some(Self::UnmutateToken {
                        b: Option::Some(token),
                        cplx: current_cplx,
                        ..Self::UnmutateToken::default()
                    });
                } else {
                    inner_step_to_remove = orig_step % step.inner.len();
                }
            }
        }
        step.inner.remove(inner_step_to_remove);
        <Self as Mutator<T>>::ordered_mutate(self, value, cache, step, max_cplx)
    }
    fn random_mutate(&mut self, value: &mut T, cache: &mut Self::Cache, max_cplx: f64) -> Self::UnmutateToken {
        let current_cplx = <Self as Mutator<T>>::complexity(self, value, cache);
        match self.rng.usize(..) % 2 {
            0 => {
                let current_field_cplx = self.a.complexity(value.get_0(), &cache.a);
                let max_field_cplx = max_cplx - current_cplx + current_field_cplx;
                let token = self.a.random_mutate(value.get_0_mut(), &mut cache.a, max_field_cplx);
                let new_field_complexity = self.a.complexity(value.get_0(), &cache.a);
                cache.cplx = cache.cplx - current_field_cplx + new_field_complexity;
                return Self::UnmutateToken {
                    a: Option::Some(token),
                    cplx: current_cplx,
                    ..Self::UnmutateToken::default()
                };
            }
            1 => {
                let current_field_cplx = self.b.complexity(value.get_1(), &cache.b);
                let max_field_cplx = max_cplx - current_cplx + current_field_cplx;
                let token = self.b.random_mutate(value.get_1_mut(), &mut cache.b, max_field_cplx);
                let new_field_complexity = self.b.complexity(value.get_1(), &cache.b);
                cache.cplx = cache.cplx - current_field_cplx + new_field_complexity;
                return Self::UnmutateToken {
                    b: Option::Some(token),
                    cplx: current_cplx,
                    ..Self::UnmutateToken::default()
                };
            }
            _ => unreachable!(),
        }
    }
    fn unmutate(&self, value: &mut T, cache: &mut Self::Cache, t: Self::UnmutateToken) {
        cache.cplx = t.cplx;
        if let Option::Some(subtoken) = t.a {
            self.a.unmutate(value.get_0_mut(), &mut cache.a, subtoken);
        }
        if let Option::Some(subtoken) = t.b {
            self.b.unmutate(value.get_1_mut(), &mut cache.b, subtoken);
        }
    }
}
