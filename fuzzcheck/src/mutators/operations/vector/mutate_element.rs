use crate::mutators::operations::vector::VecM;
use crate::mutators::operations::{MutateOperation, RevertMutation};
use crate::Mutator;

#[derive(Clone)]
pub struct MutateElement<S> {
    idx: usize,
    inner_steps: Vec<S>,
}

pub struct RevertByUnmutatingElement<UT> {
    pub idx: usize,
    pub unmutate_token: UT,
}

impl<T, M> RevertMutation<Vec<T>, VecM<T, M>> for RevertByUnmutatingElement<M::UnmutateToken>
where
    T: Clone + 'static,
    M: Mutator<T>,
{
    fn revert(self, mutator: &VecM<T, M>, value: &mut Vec<T>, cache: &mut <VecM<T, M> as Mutator<Vec<T>>>::Cache) {
        mutator
            .m
            .unmutate(&mut value[self.idx], &mut cache.inner[self.idx], self.unmutate_token);
    }
}

impl<T, M> MutateOperation<Vec<T>, VecM<T, M>> for MutateElement<M::MutationStep>
where
    T: Clone + 'static,
    M: Mutator<T>,
{
    type Revert = RevertByUnmutatingElement<M::UnmutateToken>;

    fn from_cache(
        mutator: &VecM<T, M>,
        value: &Vec<T>,
        cache: &<VecM<T, M> as Mutator<Vec<T>>>::Cache,
        _cplx: f64,
    ) -> Option<Self> {
        if value.is_empty() {
            None
        } else {
            let inner_steps = {
                for (v, c) in value.iter().zip(cache.inner.iter()) {
                    todo!()
                }
                todo!();
            };
            Some(MutateElement { idx: 0, inner_steps })
        }
    }

    fn apply(
        &mut self,
        _mutator: &VecM<T, M>,
        value: &mut Vec<T>,
        _cache: &mut <VecM<T, M> as Mutator<Vec<T>>>::Cache,
        max_cplx: f64,
    ) -> Option<(Self::Revert, f64)> {
        todo!()
    }
}
