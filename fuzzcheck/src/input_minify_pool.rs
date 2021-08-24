use crate::{
    mutators::either::Either,
    sensor_and_pool::{CompatibleWithSensor, CorpusDelta, Sensor},
    unit_pool::UnitPool,
    Mutator,
};

impl<T, M, S: Sensor> CompatibleWithSensor<S> for UnitPool<T, M>
where
    T: Clone,
    M: Mutator<T>,
{
    #[no_coverage]
    fn process(
        &mut self,
        _sensor: &mut S,
        _get_input_ref: Either<Self::Index, &Self::TestCase>,
        _clone_input: &impl Fn(&Self::TestCase) -> Self::TestCase,
        _complexity: f64,
        _event_handler: impl FnMut(CorpusDelta<&Self::TestCase, Self::Index>, Self::Stats) -> Result<(), std::io::Error>,
    ) -> Result<(), std::io::Error> {
        Ok(())
    }
    #[no_coverage]
    fn minify(
        &mut self,
        _sensor: &mut S,
        _target_len: usize,
        _event_handler: impl FnMut(CorpusDelta<&Self::TestCase, Self::Index>, Self::Stats) -> Result<(), std::io::Error>,
    ) -> Result<(), std::io::Error> {
        Ok(())
    }
}
