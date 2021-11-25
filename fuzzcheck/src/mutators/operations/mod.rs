use crate::Mutator;

// pub mod vector;
pub mod vector2;

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

    fn default_random_step(mutator: &M, value: &Value) -> Option<Self::RandomStep>;

    fn random<'a>(
        mutator: &M,
        value: &Value,
        cache: &M::Cache,
        random_step: &Self::RandomStep,
        max_cplx: f64,
    ) -> Self::Concrete<'a>;

    fn default_step(mutator: &M, value: &Value, cache: &M::Cache) -> Option<Self::Step>;
    fn from_step<'a>(
        mutator: &M,
        value: &Value,
        cache: &M::Cache,
        step: &'a mut Self::Step,
        max_cplx: f64,
    ) -> Option<Self::Concrete<'a>>;

    fn apply<'a>(
        mutation: Self::Concrete<'a>,
        mutator: &M,
        value: &mut Value,
        cache: &mut M::Cache,
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
