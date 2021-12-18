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

trait SensorExt: Sensor {
    fn map<ToObservations, F>(self, map_f: F) -> MapSensor<Self, ToObservations, F>
    where
        Self: Sized,
        F: for<'a> Fn(
            PhantomData<&'a ()>,
            <Self::Observations as Observations>::Concrete<'a>,
        ) -> <ToObservations as Observations>::Concrete<'a>,
        ToObservations: Observations,
    {
        MapSensor {
            sensor: self,
            map_f,
            _phantom: PhantomData,
        }
    }
}
impl<T> SensorExt for T where T: Sensor {}
