use crate::mutators::operations::vector::VecM;
use crate::mutators::operations::{MutateOperation, RevertMutation};
use crate::Mutator;

#[derive(Clone)]
pub struct RemoveElement {
    idx: usize,
}

pub struct RevertByInsertingElement<T> {
    pub element: T,
    pub idx: usize,
}
impl<T, M> RevertMutation<Vec<T>, VecM<T, M>> for RevertByInsertingElement<T>
where
    T: Clone + 'static,
    M: Mutator<T>,
{
    fn revert(self, _mutator: &VecM<T, M>, value: &mut Vec<T>, _cache: &mut <VecM<T, M> as Mutator<Vec<T>>>::Cache) {
        value.insert(self.idx, self.element);
    }
}

impl<T, M> MutateOperation<Vec<T>, VecM<T, M>> for RemoveElement
where
    T: Clone + 'static,
    M: Mutator<T>,
{
    type Revert = RevertByInsertingElement<T>;

    fn from_cache(
        mutator: &VecM<T, M>,
        value: &Vec<T>,
        _cache: &<VecM<T, M> as Mutator<Vec<T>>>::Cache,
        _cplx: f64,
    ) -> Option<Self> {
        if *mutator.len_range.start() >= value.len() {
            None
        } else {
            Some(RemoveElement { idx: 0 })
        }
    }

    fn apply(
        &mut self,
        _mutator: &VecM<T, M>,
        value: &mut Vec<T>,
        _cache: &mut <VecM<T, M> as Mutator<Vec<T>>>::Cache,
        max_cplx: f64,
    ) -> Option<(Self::Revert, f64)> {
        // TODO: max_cplx handling
        if self.idx >= value.len() {
            return None;
        }
        let removed_element = value.remove(self.idx);

        let result = RevertByInsertingElement {
            element: removed_element,
            idx: self.idx,
        };
        self.idx += 1;
        Some((result, 0.0))
    }
}
