use crate::traits::MutatorWrapper;

pub struct Wrapper<T>(pub T);
impl<T> MutatorWrapper for Wrapper<T> {
    type Wrapped = T;
    #[no_coverage]
    fn wrapped_mutator(&self) -> &Self::Wrapped {
        &self.0
    }
}
