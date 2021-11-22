use crate::Mutator;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Either<L, R> {
    Left(L),
    Right(R),
}
impl<T, M1, M2> Mutator<T> for Either<M1, M2>
where
    T: Clone,
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
    #[no_coverage]
    fn default_arbitrary_step(&self) -> Self::ArbitraryStep {
        match self {
            Either::Left(m) => Either::Left(m.default_arbitrary_step()),
            Either::Right(m) => Either::Right(m.default_arbitrary_step()),
        }
    }

    #[doc(hidden)]
    #[no_coverage]
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
    #[no_coverage]
    fn default_mutation_step(&self, value: &T, cache: &Self::Cache) -> Self::MutationStep {
        match (self, cache) {
            (Either::Left(m), Either::Left(c)) => Either::Left(m.default_mutation_step(value, c)),
            (Either::Right(m), Either::Right(c)) => Either::Right(m.default_mutation_step(value, c)),
            _ => panic!(),
        }
    }

    #[doc(hidden)]
    #[no_coverage]
    fn max_complexity(&self) -> f64 {
        match self {
            Either::Left(m) => m.max_complexity(),
            Either::Right(m) => m.max_complexity(),
        }
    }

    #[doc(hidden)]
    #[no_coverage]
    fn min_complexity(&self) -> f64 {
        match self {
            Either::Left(m) => m.min_complexity(),
            Either::Right(m) => m.min_complexity(),
        }
    }

    #[doc(hidden)]
    #[no_coverage]
    fn complexity(&self, value: &T, cache: &Self::Cache) -> f64 {
        match (self, cache) {
            (Either::Left(m), Either::Left(c)) => m.complexity(value, c),
            (Either::Right(m), Either::Right(c)) => m.complexity(value, c),
            _ => panic!(),
        }
    }

    #[doc(hidden)]
    #[no_coverage]
    fn ordered_arbitrary(&self, step: &mut Self::ArbitraryStep, max_cplx: f64) -> Option<(T, f64)> {
        match (self, step) {
            (Either::Left(m), Either::Left(s)) => m.ordered_arbitrary(s, max_cplx),
            (Either::Right(m), Either::Right(s)) => m.ordered_arbitrary(s, max_cplx),
            _ => panic!(),
        }
    }

    #[doc(hidden)]
    #[no_coverage]
    fn random_arbitrary(&self, max_cplx: f64) -> (T, f64) {
        match self {
            Either::Left(m) => m.random_arbitrary(max_cplx),
            Either::Right(m) => m.random_arbitrary(max_cplx),
        }
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
        match (self, cache, step) {
            (Either::Left(m), Either::Left(c), Either::Left(s)) => {
                let (t, cplx) = m.ordered_mutate(value, c, s, max_cplx)?;
                Some((Either::Left(t), cplx))
            }
            (Either::Right(m), Either::Right(c), Either::Right(s)) => {
                let (t, cplx) = m.ordered_mutate(value, c, s, max_cplx)?;
                Some((Either::Right(t), cplx))
            }
            _ => panic!(),
        }
    }

    #[doc(hidden)]
    #[no_coverage]
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
            _ => panic!(),
        }
    }

    #[doc(hidden)]
    #[no_coverage]
    fn unmutate(&self, value: &mut T, cache: &mut Self::Cache, t: Self::UnmutateToken) {
        match (self, cache, t) {
            (Either::Left(m), Either::Left(c), Either::Left(t)) => {
                m.unmutate(value, c, t);
            }
            (Either::Right(m), Either::Right(c), Either::Right(t)) => {
                m.unmutate(value, c, t);
            }
            _ => panic!(),
        }
    }

    #[doc(hidden)]
    type RecursingPartIndex = Either<M1::RecursingPartIndex, M2::RecursingPartIndex>;
    #[doc(hidden)]
    #[no_coverage]
    fn default_recursing_part_index(&self, value: &T, cache: &Self::Cache) -> Self::RecursingPartIndex {
        match (self, cache) {
            (Either::Left(m), Either::Left(c)) => Either::Left(m.default_recursing_part_index(value, c)),
            (Either::Right(m), Either::Right(c)) => Either::Right(m.default_recursing_part_index(value, c)),
            _ => unreachable!(),
        }
    }
    #[doc(hidden)]
    #[no_coverage]
    fn recursing_part<'a, V, N>(&self, parent: &N, value: &'a T, index: &mut Self::RecursingPartIndex) -> Option<&'a V>
    where
        V: Clone + 'static,
        N: Mutator<V>,
    {
        match (self, index) {
            (Either::Left(m), Either::Left(i)) => m.recursing_part::<V, N>(parent, value, i),
            (Either::Right(m), Either::Right(i)) => m.recursing_part::<V, N>(parent, value, i),
            _ => panic!(),
        }
    }
}
