use crate::{Mutator, SubValueProvider};

// maybe the mutator should be a generic type parameter of the MutateOperation trait, maybe the T and C should be too
// but the step and revert should not
pub trait Mutation<Value, M>: Sized
where
    Value: Clone + 'static,
    M: Mutator<Value>,
{
    type RandomStep: Clone;
    type Step: Clone;
    type Concrete<'a>
    where
        M: 'a,
        Value: 'a;
    type Revert: RevertMutation<Value, M>;

    fn default_random_step(&self, mutator: &M, value: &Value) -> Option<Self::RandomStep>;

    fn random<'a>(
        mutator: &M,
        value: &Value,
        cache: &M::Cache,
        random_step: &Self::RandomStep,
        max_cplx: f64,
    ) -> Self::Concrete<'a>;

    fn default_step(&self, mutator: &M, value: &Value, cache: &M::Cache) -> Option<Self::Step>;
    fn from_step<'a>(
        mutator: &M,
        value: &Value,
        cache: &M::Cache,
        step: &'a mut Self::Step,
        subvalue_provider: &dyn SubValueProvider,
        max_cplx: f64,
    ) -> Option<Self::Concrete<'a>>;

    fn apply<'a>(
        mutation: Self::Concrete<'a>,
        mutator: &M,
        value: &mut Value,
        cache: &mut M::Cache,
        subvalue_provider: &dyn SubValueProvider,
        max_cplx: f64,
    ) -> (Self::Revert, f64);
}
pub trait RevertMutation<Value, M>
where
    Value: Clone + 'static,
    M: Mutator<Value>,
{
    fn revert(self, mutator: &M, value: &mut Value, cache: &mut M::Cache);
}

pub struct NoMutation;
impl<Value, M> RevertMutation<Value, M> for NoMutation
where
    Value: Clone + 'static,
    M: Mutator<Value>,
{
    #[coverage(off)]
    fn revert(self, _mutator: &M, _value: &mut Value, _cache: &mut M::Cache) {}
}
impl<Value, M> Mutation<Value, M> for NoMutation
where
    Value: Clone + 'static,
    M: Mutator<Value>,
{
    type RandomStep = ();
    type Step = ();
    type Concrete<'a> = ();

    type Revert = NoMutation;

    #[coverage(off)]
    fn default_random_step(&self, _mutator: &M, _value: &Value) -> Option<Self::RandomStep> {
        None
    }

    #[coverage(off)]
    fn random<'a>(
        _mutator: &M,
        _value: &Value,
        _cache: &M::Cache,
        _random_step: &Self::RandomStep,
        _max_cplx: f64,
    ) -> Self::Concrete<'a> {
    }

    #[coverage(off)]
    fn default_step(&self, _mutator: &M, _value: &Value, _cache: &M::Cache) -> Option<Self::Step> {
        None
    }

    #[coverage(off)]
    fn from_step<'a>(
        _mutator: &M,
        _value: &Value,
        _cache: &M::Cache,
        _step: &'a mut Self::Step,
        _subvalue_provider: &dyn SubValueProvider,
        _max_cplx: f64,
    ) -> Option<Self::Concrete<'a>> {
        None
    }

    #[coverage(off)]
    fn apply<'a>(
        _mutation: Self::Concrete<'a>,
        mutator: &M,
        value: &mut Value,
        cache: &mut M::Cache,
        _subvalue_provider: &dyn SubValueProvider,
        _max_cplx: f64,
    ) -> (Self::Revert, f64) {
        (NoMutation, mutator.complexity(value, cache))
    }
}
