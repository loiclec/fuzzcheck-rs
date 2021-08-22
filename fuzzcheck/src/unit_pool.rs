use crate::{sensor_and_pool::Pool, FuzzedInput, Mutator};

pub struct UnitPool<T, M>
where
    T: Clone,
    M: Mutator<T>,
{
    input: FuzzedInput<T, M>,
    dead_end: bool,
}
impl<T, M> UnitPool<T, M>
where
    T: Clone,
    M: Mutator<T>,
{
    #[no_coverage]
    pub(crate) fn new(input: FuzzedInput<T, M>) -> Self {
        Self { input, dead_end: false }
    }
}

impl<T, M> Pool for UnitPool<T, M>
where
    T: Clone,
    M: Mutator<T>,
{
    type TestCase = FuzzedInput<T, M>;
    type Index = ();
    #[no_coverage]
    fn len(&self) -> usize {
        1
    }
    #[no_coverage]
    fn get_random_index(&self) -> Option<Self::Index> {
        if self.dead_end {
            None
        } else {
            Some(())
        }
    }
    #[no_coverage]
    fn get(&self, _idx: Self::Index) -> &Self::TestCase {
        &self.input
    }
    #[no_coverage]
    fn get_mut(&mut self, _idx: Self::Index) -> &mut Self::TestCase {
        &mut self.input
    }
    #[no_coverage]
    fn retrieve_after_processing(&mut self, _idx: Self::Index, _generation: usize) -> Option<&mut Self::TestCase> {
        Some(&mut self.input)
    }
    #[no_coverage]
    fn mark_test_case_as_dead_end(&mut self, _idx: Self::Index) {
        self.dead_end = true
    }
}
