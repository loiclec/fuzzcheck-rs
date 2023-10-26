use super::VecMutator;
use crate::mutators::mutations::{Mutation, RevertMutation};
use crate::{Mutator, SubValueProvider};

pub struct Arbitrary;

#[derive(Clone)]
pub struct ArbitraryStep;

pub struct ConcreteArbitrary<T> {
    value: Vec<T>,
    cplx: f64,
}
pub struct RevertArbitrary<T> {
    value: Vec<T>,
}

impl<T, M> RevertMutation<Vec<T>, VecMutator<T, M>> for RevertArbitrary<T>
where
    T: Clone + 'static,
    M: Mutator<T>,
{
    #[coverage(off)]
    fn revert(
        mut self,
        _mutator: &VecMutator<T, M>,
        value: &mut Vec<T>,
        _cache: &mut <VecMutator<T, M> as Mutator<Vec<T>>>::Cache,
    ) {
        std::mem::swap(value, &mut self.value);
    }
}

impl<T, M> Mutation<Vec<T>, VecMutator<T, M>> for Arbitrary
where
    T: Clone + 'static,
    M: Mutator<T>,
{
    type RandomStep = ArbitraryStep;
    type Step = ArbitraryStep;
    type Concrete<'a> = ConcreteArbitrary<T>;
    type Revert = RevertArbitrary<T>;

    #[coverage(off)]
    fn default_random_step(&self, mutator: &VecMutator<T, M>, _value: &Vec<T>) -> Option<Self::RandomStep> {
        if mutator.m.max_complexity() == 0. {
            return None;
        }
        Some(ArbitraryStep)
    }

    #[coverage(off)]
    fn random<'a>(
        mutator: &VecMutator<T, M>,
        _value: &Vec<T>,
        _cache: &<VecMutator<T, M> as Mutator<Vec<T>>>::Cache,
        _random_step: &Self::RandomStep,
        max_cplx: f64,
    ) -> Self::Concrete<'a> {
        let (new_value, new_cplx) = mutator.random_arbitrary(max_cplx);
        ConcreteArbitrary {
            value: new_value,
            cplx: new_cplx,
        }
    }

    #[coverage(off)]
    fn default_step(
        &self,
        mutator: &VecMutator<T, M>,
        value: &Vec<T>,
        _cache: &<VecMutator<T, M> as Mutator<Vec<T>>>::Cache,
    ) -> Option<Self::Step> {
        self.default_random_step(mutator, value)
    }

    #[coverage(off)]
    fn from_step<'a>(
        mutator: &VecMutator<T, M>,
        value: &Vec<T>,
        cache: &<VecMutator<T, M> as Mutator<Vec<T>>>::Cache,
        step: &'a mut Self::Step,
        _subvalue_provider: &dyn SubValueProvider,
        max_cplx: f64,
    ) -> Option<Self::Concrete<'a>> {
        Some(Self::random(mutator, value, cache, step, max_cplx))
    }

    #[coverage(off)]
    fn apply<'a>(
        mut mutation: Self::Concrete<'a>,
        _mutator: &VecMutator<T, M>,
        value: &mut Vec<T>,
        _cache: &mut <VecMutator<T, M> as Mutator<Vec<T>>>::Cache,
        _subvalue_provider: &dyn SubValueProvider,
        _max_cplx: f64,
    ) -> (Self::Revert, f64) {
        std::mem::swap(value, &mut mutation.value);
        (RevertArbitrary { value: mutation.value }, mutation.cplx)
    }
}
