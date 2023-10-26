use std::any::Any;

use crate::Mutator;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Either<L, R> {
    Left(L),
    Right(R),
}
impl<T, M1, M2> Mutator<T> for Either<M1, M2>
where
    T: Clone + 'static,
    M1: Mutator<T>,
    M2: Mutator<T>,
{
    #[doc(hidden)]
    type Cache = Either<M1::Cache, M2::Cache>;
    #[doc(hidden)]
    type MutationStep = Either<M1::MutationStep, M2::MutationStep>;
    #[doc(hidden)]
    type ArbitraryStep = Either<M1::ArbitraryStep, M2::ArbitraryStep>;
    #[doc(hidden)]
    type UnmutateToken = Either<M1::UnmutateToken, M2::UnmutateToken>;

    #[doc(hidden)]
    #[inline]
    #[coverage(off)]
    fn initialize(&self) {
        match self {
            Either::Left(m) => m.initialize(),
            Either::Right(m) => m.initialize(),
        }
    }

    #[doc(hidden)]
    #[inline]
    #[coverage(off)]
    fn default_arbitrary_step(&self) -> Self::ArbitraryStep {
        match self {
            Either::Left(m) => Either::Left(m.default_arbitrary_step()),
            Either::Right(m) => Either::Right(m.default_arbitrary_step()),
        }
    }

    #[doc(hidden)]
    #[inline]
    #[coverage(off)]
    fn is_valid(&self, value: &T) -> bool {
        match self {
            Either::Left(m) => m.is_valid(value),
            Either::Right(m) => m.is_valid(value),
        }
    }

    #[doc(hidden)]
    #[inline]
    #[coverage(off)]
    fn validate_value(&self, value: &T) -> Option<Self::Cache> {
        match self {
            Either::Left(m) => {
                let c = m.validate_value(value)?;
                Some(Either::Left(c))
            }
            Either::Right(m) => {
                let c = m.validate_value(value)?;
                Some(Either::Right(c))
            }
        }
    }

    #[doc(hidden)]
    #[inline]
    #[coverage(off)]
    fn default_mutation_step(&self, value: &T, cache: &Self::Cache) -> Self::MutationStep {
        match (self, cache) {
            (Either::Left(m), Either::Left(c)) => Either::Left(m.default_mutation_step(value, c)),
            (Either::Right(m), Either::Right(c)) => Either::Right(m.default_mutation_step(value, c)),
            _ => unreachable!(),
        }
    }

    #[doc(hidden)]
    #[coverage(off)]
    fn global_search_space_complexity(&self) -> f64 {
        match self {
            Either::Left(m) => m.global_search_space_complexity(),
            Either::Right(m) => m.global_search_space_complexity(),
        }
    }

    #[doc(hidden)]
    #[inline]
    #[coverage(off)]
    fn max_complexity(&self) -> f64 {
        match self {
            Either::Left(m) => m.max_complexity(),
            Either::Right(m) => m.max_complexity(),
        }
    }

    #[doc(hidden)]
    #[inline]
    #[coverage(off)]
    fn min_complexity(&self) -> f64 {
        match self {
            Either::Left(m) => m.min_complexity(),
            Either::Right(m) => m.min_complexity(),
        }
    }

    #[doc(hidden)]
    #[inline]
    #[coverage(off)]
    fn complexity(&self, value: &T, cache: &Self::Cache) -> f64 {
        match (self, cache) {
            (Either::Left(m), Either::Left(c)) => m.complexity(value, c),
            (Either::Right(m), Either::Right(c)) => m.complexity(value, c),
            _ => unreachable!(),
        }
    }

    #[doc(hidden)]
    #[inline]
    #[coverage(off)]
    fn ordered_arbitrary(&self, step: &mut Self::ArbitraryStep, max_cplx: f64) -> Option<(T, f64)> {
        match (self, step) {
            (Either::Left(m), Either::Left(s)) => m.ordered_arbitrary(s, max_cplx),
            (Either::Right(m), Either::Right(s)) => m.ordered_arbitrary(s, max_cplx),
            _ => unreachable!(),
        }
    }

    #[doc(hidden)]
    #[inline]
    #[coverage(off)]
    fn random_arbitrary(&self, max_cplx: f64) -> (T, f64) {
        match self {
            Either::Left(m) => m.random_arbitrary(max_cplx),
            Either::Right(m) => m.random_arbitrary(max_cplx),
        }
    }

    #[doc(hidden)]
    #[inline]
    #[coverage(off)]
    fn ordered_mutate(
        &self,
        value: &mut T,
        cache: &mut Self::Cache,
        step: &mut Self::MutationStep,
        subvalue_provider: &dyn crate::SubValueProvider,
        max_cplx: f64,
    ) -> Option<(Self::UnmutateToken, f64)> {
        match (self, cache, step) {
            (Either::Left(m), Either::Left(c), Either::Left(s)) => {
                let (t, cplx) = m.ordered_mutate(value, c, s, subvalue_provider, max_cplx)?;
                Some((Either::Left(t), cplx))
            }
            (Either::Right(m), Either::Right(c), Either::Right(s)) => {
                let (t, cplx) = m.ordered_mutate(value, c, s, subvalue_provider, max_cplx)?;
                Some((Either::Right(t), cplx))
            }
            _ => unreachable!(),
        }
    }

    #[doc(hidden)]
    #[inline]
    #[coverage(off)]
    fn random_mutate(&self, value: &mut T, cache: &mut Self::Cache, max_cplx: f64) -> (Self::UnmutateToken, f64) {
        match (self, cache) {
            (Either::Left(m), Either::Left(c)) => {
                let (t, cplx) = m.random_mutate(value, c, max_cplx);
                (Either::Left(t), cplx)
            }
            (Either::Right(m), Either::Right(c)) => {
                let (t, cplx) = m.random_mutate(value, c, max_cplx);
                (Either::Right(t), cplx)
            }
            _ => unreachable!(),
        }
    }

    #[doc(hidden)]
    #[inline]
    #[coverage(off)]
    fn unmutate(&self, value: &mut T, cache: &mut Self::Cache, t: Self::UnmutateToken) {
        match (self, cache, t) {
            (Either::Left(m), Either::Left(c), Either::Left(t)) => {
                m.unmutate(value, c, t);
            }
            (Either::Right(m), Either::Right(c), Either::Right(t)) => {
                m.unmutate(value, c, t);
            }
            _ => unreachable!(),
        }
    }

    #[doc(hidden)]
    #[inline]
    #[coverage(off)]
    fn visit_subvalues<'a>(&self, value: &'a T, cache: &'a Self::Cache, visit: &mut dyn FnMut(&'a dyn Any, f64)) {
        match (self, cache) {
            (Either::Left(m), Either::Left(cache)) => {
                m.visit_subvalues(value, cache, visit);
            }
            (Either::Right(m), Either::Right(cache)) => {
                m.visit_subvalues(value, cache, visit);
            }
            _ => unreachable!(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Either3<A, B, C> {
    A(A),
    B(B),
    C(C),
}
impl<T, A, B, C> Mutator<T> for Either3<A, B, C>
where
    T: Clone + 'static,
    A: Mutator<T>,
    B: Mutator<T>,
    C: Mutator<T>,
{
    #[doc(hidden)]
    type Cache = Either3<A::Cache, B::Cache, C::Cache>;
    #[doc(hidden)]
    type MutationStep = Either3<A::MutationStep, B::MutationStep, C::MutationStep>;
    #[doc(hidden)]
    type ArbitraryStep = Either3<A::ArbitraryStep, B::ArbitraryStep, C::ArbitraryStep>;
    #[doc(hidden)]
    type UnmutateToken = Either3<A::UnmutateToken, B::UnmutateToken, C::UnmutateToken>;

    #[doc(hidden)]
    #[inline]
    #[coverage(off)]
    fn initialize(&self) {
        match self {
            Either3::A(m) => m.initialize(),
            Either3::B(m) => m.initialize(),
            Either3::C(m) => m.initialize(),
        }
    }

    #[doc(hidden)]
    #[inline]
    #[coverage(off)]
    fn default_arbitrary_step(&self) -> Self::ArbitraryStep {
        match self {
            Either3::A(m) => Either3::A(m.default_arbitrary_step()),
            Either3::B(m) => Either3::B(m.default_arbitrary_step()),
            Either3::C(m) => Either3::C(m.default_arbitrary_step()),
        }
    }

    #[doc(hidden)]
    #[inline]
    #[coverage(off)]
    fn is_valid(&self, value: &T) -> bool {
        match self {
            Either3::A(m) => m.is_valid(value),
            Either3::B(m) => m.is_valid(value),
            Either3::C(m) => m.is_valid(value),
        }
    }

    #[doc(hidden)]
    #[inline]
    #[coverage(off)]
    fn validate_value(&self, value: &T) -> Option<Self::Cache> {
        match self {
            Either3::A(m) => {
                let c = m.validate_value(value)?;
                Some(Either3::A(c))
            }
            Either3::B(m) => {
                let c = m.validate_value(value)?;
                Some(Either3::B(c))
            }
            Either3::C(m) => {
                let c = m.validate_value(value)?;
                Some(Either3::C(c))
            }
        }
    }

    #[doc(hidden)]
    #[inline]
    #[coverage(off)]
    fn default_mutation_step(&self, value: &T, cache: &Self::Cache) -> Self::MutationStep {
        match (self, cache) {
            (Either3::A(m), Either3::A(c)) => Either3::A(m.default_mutation_step(value, c)),
            (Either3::B(m), Either3::B(c)) => Either3::B(m.default_mutation_step(value, c)),
            (Either3::C(m), Either3::C(c)) => Either3::C(m.default_mutation_step(value, c)),
            _ => unreachable!(),
        }
    }

    #[doc(hidden)]
    #[coverage(off)]
    fn global_search_space_complexity(&self) -> f64 {
        match self {
            Either3::A(m) => m.global_search_space_complexity(),
            Either3::B(m) => m.global_search_space_complexity(),
            Either3::C(m) => m.global_search_space_complexity(),
        }
    }

    #[doc(hidden)]
    #[inline]
    #[coverage(off)]
    fn max_complexity(&self) -> f64 {
        match self {
            Either3::A(m) => m.max_complexity(),
            Either3::B(m) => m.max_complexity(),
            Either3::C(m) => m.max_complexity(),
        }
    }

    #[doc(hidden)]
    #[inline]
    #[coverage(off)]
    fn min_complexity(&self) -> f64 {
        match self {
            Either3::A(m) => m.min_complexity(),
            Either3::B(m) => m.min_complexity(),
            Either3::C(m) => m.min_complexity(),
        }
    }

    #[doc(hidden)]
    #[inline]
    #[coverage(off)]
    fn complexity(&self, value: &T, cache: &Self::Cache) -> f64 {
        match (self, cache) {
            (Either3::A(m), Either3::A(c)) => m.complexity(value, c),
            (Either3::B(m), Either3::B(c)) => m.complexity(value, c),
            (Either3::C(m), Either3::C(c)) => m.complexity(value, c),
            _ => unreachable!(),
        }
    }

    #[doc(hidden)]
    #[inline]
    #[coverage(off)]
    fn ordered_arbitrary(&self, step: &mut Self::ArbitraryStep, max_cplx: f64) -> Option<(T, f64)> {
        match (self, step) {
            (Either3::A(m), Either3::A(s)) => m.ordered_arbitrary(s, max_cplx),
            (Either3::B(m), Either3::B(s)) => m.ordered_arbitrary(s, max_cplx),
            (Either3::C(m), Either3::C(s)) => m.ordered_arbitrary(s, max_cplx),
            _ => unreachable!(),
        }
    }

    #[doc(hidden)]
    #[inline]
    #[coverage(off)]
    fn random_arbitrary(&self, max_cplx: f64) -> (T, f64) {
        match self {
            Either3::A(m) => m.random_arbitrary(max_cplx),
            Either3::B(m) => m.random_arbitrary(max_cplx),
            Either3::C(m) => m.random_arbitrary(max_cplx),
        }
    }

    #[doc(hidden)]
    #[inline]
    #[coverage(off)]
    fn ordered_mutate(
        &self,
        value: &mut T,
        cache: &mut Self::Cache,
        step: &mut Self::MutationStep,
        subvalue_provider: &dyn crate::SubValueProvider,
        max_cplx: f64,
    ) -> Option<(Self::UnmutateToken, f64)> {
        match (self, cache, step) {
            (Either3::A(m), Either3::A(c), Either3::A(s)) => {
                let (t, cplx) = m.ordered_mutate(value, c, s, subvalue_provider, max_cplx)?;
                Some((Either3::A(t), cplx))
            }
            (Either3::B(m), Either3::B(c), Either3::B(s)) => {
                let (t, cplx) = m.ordered_mutate(value, c, s, subvalue_provider, max_cplx)?;
                Some((Either3::B(t), cplx))
            }
            (Either3::C(m), Either3::C(c), Either3::C(s)) => {
                let (t, cplx) = m.ordered_mutate(value, c, s, subvalue_provider, max_cplx)?;
                Some((Either3::C(t), cplx))
            }
            _ => unreachable!(),
        }
    }

    #[doc(hidden)]
    #[inline]
    #[coverage(off)]
    fn random_mutate(&self, value: &mut T, cache: &mut Self::Cache, max_cplx: f64) -> (Self::UnmutateToken, f64) {
        match (self, cache) {
            (Either3::A(m), Either3::A(c)) => {
                let (t, cplx) = m.random_mutate(value, c, max_cplx);
                (Either3::A(t), cplx)
            }
            (Either3::B(m), Either3::B(c)) => {
                let (t, cplx) = m.random_mutate(value, c, max_cplx);
                (Either3::B(t), cplx)
            }
            (Either3::C(m), Either3::C(c)) => {
                let (t, cplx) = m.random_mutate(value, c, max_cplx);
                (Either3::C(t), cplx)
            }
            _ => unreachable!(),
        }
    }

    #[doc(hidden)]
    #[inline]
    #[coverage(off)]
    fn unmutate(&self, value: &mut T, cache: &mut Self::Cache, t: Self::UnmutateToken) {
        match (self, cache, t) {
            (Either3::A(m), Either3::A(c), Either3::A(t)) => {
                m.unmutate(value, c, t);
            }
            (Either3::B(m), Either3::B(c), Either3::B(t)) => {
                m.unmutate(value, c, t);
            }
            (Either3::C(m), Either3::C(c), Either3::C(t)) => {
                m.unmutate(value, c, t);
            }
            _ => unreachable!(),
        }
    }

    #[doc(hidden)]
    #[inline]
    #[coverage(off)]
    fn visit_subvalues<'a>(&self, value: &'a T, cache: &'a Self::Cache, visit: &mut dyn FnMut(&'a dyn Any, f64)) {
        match (self, cache) {
            (Either3::A(m), Either3::A(cache)) => {
                m.visit_subvalues(value, cache, visit);
            }
            (Either3::B(m), Either3::B(cache)) => {
                m.visit_subvalues(value, cache, visit);
            }
            (Either3::C(m), Either3::C(cache)) => {
                m.visit_subvalues(value, cache, visit);
            }
            _ => unreachable!(),
        }
    }
}
