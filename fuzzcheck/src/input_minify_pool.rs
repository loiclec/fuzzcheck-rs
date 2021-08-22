use std::marker::PhantomData;

use crate::{
    mutators::either::Either,
    noop_sensor::NoopSensor,
    sensor_and_pool::{CorpusDelta, EmptyStats, Pool, SensorAndPool},
    unit_pool::UnitPool,
    FuzzedInput, Mutator,
};

pub struct InputMinifySensorAndPool<T, M>
where
    T: Clone,
    M: Mutator<T>,
{
    _phantom: PhantomData<(T, M)>,
}

impl<T, M> SensorAndPool for InputMinifySensorAndPool<T, M>
where
    T: Clone,
    M: Mutator<T>,
{
    type Sensor = NoopSensor;
    type Pool = UnitPool<T, M>;
    type TestCase = FuzzedInput<T, M>;
    type Event = ();
    type Stats = EmptyStats;
    #[no_coverage]
    fn process(
        _sensor: &mut Self::Sensor,
        _pool: &mut Self::Pool,
        _stats: &mut Self::Stats,
        _get_input_ref: Either<<Self::Pool as Pool>::Index, &Self::TestCase>,
        _clone_input: &impl Fn(&Self::TestCase) -> Self::TestCase,
        _complexity: f64,
        _event_handler: impl FnMut(
            CorpusDelta<&Self::TestCase, <Self::Pool as Pool>::Index>,
            &Self::Stats,
        ) -> Result<(), std::io::Error>,
    ) -> Result<(), std::io::Error> {
        Ok(())
    }
    #[no_coverage]
    fn minify(
        _sensor: &mut Self::Sensor,
        _pool: &mut Self::Pool,
        _stats: &mut Self::Stats,
        _target_len: usize,
        _event_handler: impl FnMut(CorpusDelta<&Self::TestCase, <Self::Pool as Pool>::Index>, &Self::Stats),
    ) {
    }
    #[no_coverage]
    fn get_corpus_delta_from_event<'a>(
        _pool: &'a Self::Pool,
        _event: Self::Event,
    ) -> CorpusDelta<&'a Self::TestCase, <Self::Pool as Pool>::Index> {
        CorpusDelta::default()
    }
}
