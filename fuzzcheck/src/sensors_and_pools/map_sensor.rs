use crate::{Observations, SaveToStatsFolder, Sensor};
use std::marker::PhantomData;

pub struct MapSensor<S, ToObservations, F>
where
    S: Sensor,
    ToObservations: Observations,
    F: for<'a> Fn(PhantomData<&'a ()>, <S::Observations as Observations>::Concrete<'a>) -> ToObservations::Concrete<'a>,
    Self: 'static,
{
    sensor: S,
    map_f: F,
    _phantom: PhantomData<ToObservations>,
}

impl<S, ToObservations, F> MapSensor<S, ToObservations, F>
where
    S: Sensor,
    ToObservations: Observations,
    F: for<'a> Fn(PhantomData<&'a ()>, <S::Observations as Observations>::Concrete<'a>) -> ToObservations::Concrete<'a>,
    Self: 'static,
{
    pub fn new(sensor: S, map_f: F) -> Self {
        Self {
            sensor,
            map_f,
            _phantom: PhantomData,
        }
    }
}
impl<S, ToObservations, F> SaveToStatsFolder for MapSensor<S, ToObservations, F>
where
    S: Sensor,
    ToObservations: Observations,
    F: for<'a> Fn(PhantomData<&'a ()>, <S::Observations as Observations>::Concrete<'a>) -> ToObservations::Concrete<'a>,
    Self: 'static,
{
    fn save_to_stats_folder(&self) -> Vec<(std::path::PathBuf, Vec<u8>)> {
        self.sensor.save_to_stats_folder()
    }
}
impl<S, ToObservations, F> Sensor for MapSensor<S, ToObservations, F>
where
    S: Sensor,
    ToObservations: Observations,
    F: for<'a> Fn(PhantomData<&'a ()>, <S::Observations as Observations>::Concrete<'a>) -> ToObservations::Concrete<'a>,
    Self: 'static,
{
    type Observations = ToObservations;

    fn start_recording(&mut self) {
        self.sensor.start_recording();
    }

    fn stop_recording(&mut self) {
        self.sensor.stop_recording();
    }

    fn get_observations<'a>(&'a mut self) -> <Self::Observations as Observations>::Concrete<'a> {
        let observations = self.sensor.get_observations();
        (self.map_f)(PhantomData, observations)
    }
}
pub trait WrapperSensor: Sensor {
    type Wrapped: Sensor;
    fn wrapped(&self) -> &Self::Wrapped;
}

impl<S, ToObservations, F> WrapperSensor for MapSensor<S, ToObservations, F>
where
    S: Sensor,
    ToObservations: Observations,
    F: for<'a> Fn(PhantomData<&'a ()>, <S::Observations as Observations>::Concrete<'a>) -> ToObservations::Concrete<'a>,
    Self: 'static,
{
    type Wrapped = S;
    fn wrapped(&self) -> &S {
        &self.sensor
    }
}
