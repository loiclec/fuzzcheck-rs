use fuzzcheck_mutators::DefaultMutator;
// use fuzzcheck_mutators::TupleStructure;
// #[derive(Clone, DefaultMutator)]
// pub struct X(bool);

#[derive(Clone, DefaultMutator)]
pub struct Y {
    x: bool,
}

#[cfg(test)]
mod test {
    use super::*;
    use fuzzcheck_mutators::fuzzcheck_traits::Mutator;
    #[test]
    fn test_compile() {
        // let _m = X::default_mutator();
        let m = Y::default_mutator();

        let (y, _) = m.random_arbitrary(10.0);
        assert!(false, "{}", y.x);
    }
}

// pub struct YMutator<M0>
// where
//     M0: fuzzcheck_mutators::fuzzcheck_traits::Mutator<bool>,
// {
//     pub mutator: fuzzcheck_mutators::TupleMutatorWrapper<
//         Y,
//         fuzzcheck_mutators::Tuple1Mutator<bool, M0>,
//         fuzzcheck_mutators::Tuple1<bool>,
//     >,
// }
// pub struct YMutatorCache<M0>
// where
//     M0: fuzzcheck_mutators::fuzzcheck_traits::Mutator<bool>,
// {
//     inner: <fuzzcheck_mutators::TupleMutatorWrapper<
//         Y,
//         fuzzcheck_mutators::Tuple1Mutator<bool, M0>,
//         fuzzcheck_mutators::Tuple1<bool>,
//     > as fuzzcheck_mutators::fuzzcheck_traits::Mutator<Y>>::Cache,
// }
// impl<M0> YMutatorCache<M0>
// where
//     M0: fuzzcheck_mutators::fuzzcheck_traits::Mutator<bool>,
// {
//     fn new(
//         inner: <fuzzcheck_mutators::TupleMutatorWrapper<
//             Y,
//             fuzzcheck_mutators::Tuple1Mutator<bool, M0>,
//             fuzzcheck_mutators::Tuple1<bool>,
//         > as fuzzcheck_mutators::fuzzcheck_traits::Mutator<Y>>::Cache,
//     ) -> Self {
//         Self { inner: inner }
//     }
// }
// impl<M0> ::std::clone::Clone for YMutatorCache<M0>
// where
//     M0: fuzzcheck_mutators::fuzzcheck_traits::Mutator<bool>,
// {
//     fn clone(&self) -> Self {
//         Self {
//             inner: self.inner.clone(),
//         }
//     }
// }
// pub struct YMutatorMutationStep<M0>
// where
//     M0: fuzzcheck_mutators::fuzzcheck_traits::Mutator<bool>,
// {
//     inner: <fuzzcheck_mutators::TupleMutatorWrapper<
//         Y,
//         fuzzcheck_mutators::Tuple1Mutator<bool, M0>,
//         fuzzcheck_mutators::Tuple1<bool>,
//     > as fuzzcheck_mutators::fuzzcheck_traits::Mutator<Y>>::MutationStep,
// }
// impl<M0> YMutatorMutationStep<M0>
// where
//     M0: fuzzcheck_mutators::fuzzcheck_traits::Mutator<bool>,
// {
//     fn new(
//         inner: <fuzzcheck_mutators::TupleMutatorWrapper<
//             Y,
//             fuzzcheck_mutators::Tuple1Mutator<bool, M0>,
//             fuzzcheck_mutators::Tuple1<bool>,
//         > as fuzzcheck_mutators::fuzzcheck_traits::Mutator<Y>>::MutationStep,
//     ) -> Self {
//         Self { inner: inner }
//     }
// }
// impl<M0> ::std::clone::Clone for YMutatorMutationStep<M0>
// where
//     M0: fuzzcheck_mutators::fuzzcheck_traits::Mutator<bool>,
// {
//     fn clone(&self) -> Self {
//         Self {
//             inner: self.inner.clone(),
//         }
//     }
// }
// pub struct YMutatorArbitraryStep<M0>
// where
//     M0: fuzzcheck_mutators::fuzzcheck_traits::Mutator<bool>,
// {
//     inner: <fuzzcheck_mutators::TupleMutatorWrapper<
//         Y,
//         fuzzcheck_mutators::Tuple1Mutator<bool, M0>,
//         fuzzcheck_mutators::Tuple1<bool>,
//     > as fuzzcheck_mutators::fuzzcheck_traits::Mutator<Y>>::ArbitraryStep,
// }
// impl<M0> YMutatorArbitraryStep<M0>
// where
//     M0: fuzzcheck_mutators::fuzzcheck_traits::Mutator<bool>,
// {
//     fn new(
//         inner: <fuzzcheck_mutators::TupleMutatorWrapper<
//             Y,
//             fuzzcheck_mutators::Tuple1Mutator<bool, M0>,
//             fuzzcheck_mutators::Tuple1<bool>,
//         > as fuzzcheck_mutators::fuzzcheck_traits::Mutator<Y>>::ArbitraryStep,
//     ) -> Self {
//         Self { inner: inner }
//     }
// }
// impl<M0> ::std::clone::Clone for YMutatorArbitraryStep<M0>
// where
//     M0: fuzzcheck_mutators::fuzzcheck_traits::Mutator<bool>,
// {
//     fn clone(&self) -> Self {
//         Self {
//             inner: self.inner.clone(),
//         }
//     }
// }
// impl<M0> ::std::default::Default for YMutatorArbitraryStep<M0>
// where
//     M0: fuzzcheck_mutators::fuzzcheck_traits::Mutator<bool>,
// {
//     fn default() -> Self {
//         Self { inner: <_>::default() }
//     }
// }
// pub struct YMutatorUnmutateToken<M0>
// where
//     M0: fuzzcheck_mutators::fuzzcheck_traits::Mutator<bool>,
// {
//     inner: <fuzzcheck_mutators::TupleMutatorWrapper<
//         Y,
//         fuzzcheck_mutators::Tuple1Mutator<bool, M0>,
//         fuzzcheck_mutators::Tuple1<bool>,
//     > as fuzzcheck_mutators::fuzzcheck_traits::Mutator<Y>>::UnmutateToken,
// }
// impl<M0> YMutatorUnmutateToken<M0>
// where
//     M0: fuzzcheck_mutators::fuzzcheck_traits::Mutator<bool>,
// {
//     fn new(
//         inner: <fuzzcheck_mutators::TupleMutatorWrapper<
//             Y,
//             fuzzcheck_mutators::Tuple1Mutator<bool, M0>,
//             fuzzcheck_mutators::Tuple1<bool>,
//         > as fuzzcheck_mutators::fuzzcheck_traits::Mutator<Y>>::UnmutateToken,
//     ) -> Self {
//         Self { inner: inner }
//     }
// }
// impl<M0> YMutator<M0>
// where
//     M0: fuzzcheck_mutators::fuzzcheck_traits::Mutator<bool>,
// {
//     pub fn new(mutator_x: M0) -> Self {
//         Self {
//             mutator: fuzzcheck_mutators::TupleMutatorWrapper::new(fuzzcheck_mutators::Tuple1Mutator::new(mutator_x)),
//         }
//     }
// }
// impl<M0> ::std::default::Default for YMutator<M0>
// where
//     M0: fuzzcheck_mutators::fuzzcheck_traits::Mutator<bool>,
//     fuzzcheck_mutators::TupleMutatorWrapper<
//         Y,
//         fuzzcheck_mutators::Tuple1Mutator<bool, M0>,
//         fuzzcheck_mutators::Tuple1<bool>,
//     >: ::std::default::Default,
// {
//     fn default() -> Self {
//         Self {
//             mutator: <_>::default(),
//         }
//     }
// }
// impl<M0> fuzzcheck_mutators::fuzzcheck_traits::Mutator<Y> for YMutator<M0>
// where
//     M0: fuzzcheck_mutators::fuzzcheck_traits::Mutator<bool>,
// {
//     type Cache = YMutatorCache<M0>;
//     type ArbitraryStep = YMutatorArbitraryStep<M0>;
//     type MutationStep = YMutatorMutationStep<M0>;
//     type UnmutateToken = YMutatorUnmutateToken<M0>;
//     fn cache_from_value(&self, value: &Y) -> Self::Cache {
//         Self::Cache::new(self.mutator.cache_from_value(value))
//     }
//     fn initial_step_from_value(&self, value: &Y) -> Self::MutationStep {
//         Self::MutationStep::new(self.mutator.initial_step_from_value(value))
//     }
//     fn max_complexity(&self) -> f64 {
//         self.mutator.max_complexity()
//     }
//     fn min_complexity(&self) -> f64 {
//         self.mutator.min_complexity()
//     }
//     fn complexity(&self, value: &Y, cache: &Self::Cache) -> f64 {
//         self.mutator.complexity(value, &cache.inner)
//     }
//     fn ordered_arbitrary(&self, step: &mut Self::ArbitraryStep, max_cplx: f64) -> Option<(Y, Self::Cache)> {
//         if let ::std::option::Option::Some((value, cache)) = self.mutator.ordered_arbitrary(&mut step.inner, max_cplx) {
//             ::std::option::Option::Some((value, Self::Cache::new(cache)))
//         } else {
//             ::std::option::Option::None
//         }
//     }
//     fn random_arbitrary(&self, max_cplx: f64) -> (Y, Self::Cache) {
//         let (value, cache) = self.mutator.random_arbitrary(max_cplx);
//         (value, Self::Cache::new(cache))
//     }
//     fn ordered_mutate(
//         &self,
//         value: &mut Y,
//         cache: &mut Self::Cache,
//         step: &mut Self::MutationStep,
//         max_cplx: f64,
//     ) -> Option<Self::UnmutateToken> {
//         if let ::std::option::Option::Some(t) =
//             self.mutator
//                 .ordered_mutate(value, &mut cache.inner, &mut step.inner, max_cplx)
//         {
//             ::std::option::Option::Some(Self::UnmutateToken::new(t))
//         } else {
//             ::std::option::Option::None
//         }
//     }
//     fn random_mutate(&self, value: &mut Y, cache: &mut Self::Cache, max_cplx: f64) -> Self::UnmutateToken {
//         Self::UnmutateToken::new(self.mutator.random_mutate(value, &mut cache.inner, max_cplx))
//     }
//     fn unmutate(&self, value: &mut Y, cache: &mut Self::Cache, t: Self::UnmutateToken) {
//         self.mutator.unmutate(value, &mut cache.inner, t.inner)
//     }
// }
// impl fuzzcheck_mutators::DefaultMutator for Y {
//     type Mutator = YMutator<<bool as fuzzcheck_mutators::DefaultMutator>::Mutator>;
//     fn default_mutator() -> Self::Mutator {
//         Self::Mutator::new(<bool>::default_mutator())
//     }
// }
