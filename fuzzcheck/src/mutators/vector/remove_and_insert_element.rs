use super::VecMutator;
use crate::mutators::mutations::{Mutation, RevertMutation};
use crate::{Mutator, SubValueProvider};

pub struct RemoveAndInsertElement;

#[derive(Clone)]
pub struct RemoveAndInsertElementRandomStep;

pub struct ConcreteRemoveAndInsertElement<T> {
    remove_idx: usize,
    insert_idx: usize,
    inserted_el: T,
    new_cplx: f64,
}
pub struct RevertRemoveAndInsertElement<T> {
    pub remove_at_idx: usize,
    pub insert_at_idx: usize,
    pub insert_el: T,
}

impl<T, M> RevertMutation<Vec<T>, VecMutator<T, M>> for RevertRemoveAndInsertElement<T>
where
    T: Clone + 'static,
    M: Mutator<T>,
{
    #[coverage(off)]
    fn revert(
        self,
        _mutator: &VecMutator<T, M>,
        value: &mut Vec<T>,
        _cache: &mut <VecMutator<T, M> as Mutator<Vec<T>>>::Cache,
    ) {
        let _ = value.remove(self.remove_at_idx);
        value.insert(self.insert_at_idx, self.insert_el);
    }
}

impl<T, M> Mutation<Vec<T>, VecMutator<T, M>> for RemoveAndInsertElement
where
    T: Clone + 'static,
    M: Mutator<T>,
{
    type RandomStep = RemoveAndInsertElementRandomStep;
    type Step = RemoveAndInsertElementRandomStep;
    type Concrete<'a> = ConcreteRemoveAndInsertElement<T>;
    type Revert = RevertRemoveAndInsertElement<T>;
    #[coverage(off)]
    fn default_random_step(&self, mutator: &VecMutator<T, M>, value: &Vec<T>) -> Option<Self::RandomStep> {
        if mutator.m.max_complexity() == 0. {
            return None;
        }
        if value.len() <= 1 {
            // we'd remove an element and then insert another one at the same index
            // it's best to just mutate that element instead
            return None;
        }
        Some(RemoveAndInsertElementRandomStep)
    }
    #[coverage(off)]
    fn random<'a>(
        mutator: &VecMutator<T, M>,
        value: &Vec<T>,
        cache: &<VecMutator<T, M> as Mutator<Vec<T>>>::Cache,
        _random_step: &Self::RandomStep,
        max_cplx: f64,
    ) -> Self::Concrete<'a> {
        let old_cplx = mutator.complexity(value, cache);

        let remove_idx = mutator.rng.usize(0..value.len());
        // let removed_el = value.remove(removal_idx);
        let removed_el_cplx = mutator.m.complexity(&value[remove_idx], &cache.inner[remove_idx]);

        let choice_insertion = mutator.rng.usize(..value.len() - 1);
        let insert_idx = if choice_insertion < remove_idx {
            choice_insertion
        } else {
            choice_insertion + 1
        };

        let spare_cplx = max_cplx - old_cplx + removed_el_cplx;

        let (inserted_el, inserted_el_cplx) = mutator.m.random_arbitrary(spare_cplx);

        let new_cplx = old_cplx - removed_el_cplx + inserted_el_cplx;

        ConcreteRemoveAndInsertElement {
            remove_idx,
            insert_idx,
            inserted_el,
            new_cplx,
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
        mutation: Self::Concrete<'a>,
        _mutator: &VecMutator<T, M>,
        value: &mut Vec<T>,
        _cache: &mut <VecMutator<T, M> as Mutator<Vec<T>>>::Cache,
        _subvalue_provider: &dyn SubValueProvider,
        _max_cplx: f64,
    ) -> (Self::Revert, f64) {
        let removed_el = value.remove(mutation.remove_idx);
        value.insert(mutation.insert_idx, mutation.inserted_el);
        (
            RevertRemoveAndInsertElement {
                remove_at_idx: mutation.insert_idx,
                insert_at_idx: mutation.remove_idx,
                insert_el: removed_el,
            },
            mutation.new_cplx,
        )
    }
}
