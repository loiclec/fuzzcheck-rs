
use crate::{DefaultMutator, Tuple1, Tuple1Mutator, Enum1PayloadMutator, Enum1PayloadStructure, Either2};

impl<T> Enum1PayloadStructure for Option<T> where T: 'static {
    type T0 = T;
    type TupleKind0 = Tuple1<T>;

    fn get_ref<'a>(&'a self) -> Either2<&'a T, usize> {
        match self {
            Some(x) => { Either2::T0(x) }
            None => { Either2::T1(0) }
        }
    }
    fn get_mut<'a>(&'a mut self) -> Either2<&'a mut T, usize> {
        match self {
            Some(x) => { Either2::T0(x) }
            None => { Either2::T1(0) }
        }
    }
    fn new(t: Either2<Self::T0, usize>) -> Self {
        match t {
            Either2::T0(x) => Some(x),
            Either2::T1(_) => None
        }
    }
}

impl<T> DefaultMutator for Option<T> where T: DefaultMutator + 'static {
    type Mutator = Enum1PayloadMutator<T, Tuple1Mutator<T, <T as DefaultMutator>::Mutator>, crate::Tuple1<T>>;

    fn default_mutator() -> Self::Mutator {
        Self::Mutator::new(
            Tuple1Mutator::new(T::default_mutator())
        )
    }
}