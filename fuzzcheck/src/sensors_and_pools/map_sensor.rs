use std::marker::PhantomData;

use crate::{SaveToStatsFolder, Sensor};

/// The result of [`sensor.map(..)`](crate::SensorExt::map)
pub struct MapSensor<S, ToObservations, F>
where
    S: Sensor,
    F: Fn(S::Observations) -> ToObservations,
{
    sensor: S,
    map_f: F,
    _phantom: PhantomData<ToObservations>,
}

impl<S, ToObservations, F> MapSensor<S, ToObservations, F>
where
    S: Sensor,
    F: Fn(S::Observations) -> ToObservations,
{
    #[coverage(off)]
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
    F: Fn(S::Observations) -> ToObservations,
{
    #[coverage(off)]
    fn save_to_stats_folder(&self) -> Vec<(std::path::PathBuf, Vec<u8>)> {
        self.sensor.save_to_stats_folder()
    }
}
impl<S, ToObservations, F> Sensor for MapSensor<S, ToObservations, F>
where
    S: Sensor,
    F: Fn(S::Observations) -> ToObservations,
    Self: 'static,
{
    type Observations = ToObservations;

    #[coverage(off)]
    fn start_recording(&mut self) {
        self.sensor.start_recording();
    }

    #[coverage(off)]
    fn stop_recording(&mut self) {
        self.sensor.stop_recording();
    }

    #[coverage(off)]
    fn get_observations(&mut self) -> Self::Observations {
        let observations = self.sensor.get_observations();
        (self.map_f)(observations)
    }
}
pub trait WrapperSensor: Sensor {
    type Wrapped: Sensor;
    fn wrapped(&self) -> &Self::Wrapped;
}

impl<S, ToObservations, F> WrapperSensor for MapSensor<S, ToObservations, F>
where
    S: Sensor,
    F: Fn(S::Observations) -> ToObservations,
    Self: 'static,
{
    type Wrapped = S;
    #[coverage(off)]
    fn wrapped(&self) -> &S {
        &self.sensor
    }
}
