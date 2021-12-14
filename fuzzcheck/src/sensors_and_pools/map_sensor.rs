use crate::{traits::SaveToStatsFolder, Sensor};
use std::{iter, marker::PhantomData};

pub struct MapSensor<To, S, Map>
where
    S: Sensor,
    for<'a> S::Observations<'a>: IntoIterator,
    Map: Copy + for<'a> Fn(<S::Observations<'a> as IntoIterator>::Item) -> To,
    Self: Sized + 'static,
{
    sensor: S,
    map: Map,
    _phantom: PhantomData<To>,
}
impl<To, S, Map> SaveToStatsFolder for MapSensor<To, S, Map>
where
    S: Sensor,
    for<'a> S::Observations<'a>: IntoIterator,
    Map: Copy + for<'a> Fn(<S::Observations<'a> as IntoIterator>::Item) -> To,
    Self: Sized + 'static,
{
    fn save_to_stats_folder(&self) -> Vec<(std::path::PathBuf, Vec<u8>)> {
        self.sensor.save_to_stats_folder()
    }
}

impl<To, S, Map> Sensor for MapSensor<To, S, Map>
where
    S: Sensor,
    for<'a> S::Observations<'a>: IntoIterator,
    Map: Copy + for<'a> Fn(<S::Observations<'a> as IntoIterator>::Item) -> To,
    Self: Sized + 'static,
{
    type Observations<'a>
    where
        Self: 'a,
    = iter::Map<<S::Observations<'a> as IntoIterator>::IntoIter, Map>;

    fn start_recording(&mut self) {
        self.sensor.start_recording();
    }

    fn stop_recording(&mut self) {
        self.sensor.stop_recording();
    }

    fn get_observations<'a>(&'a mut self) -> Self::Observations<'a> {
        self.sensor.get_observations().into_iter().map(self.map)
    }
}

pub trait IteratorSensor: Sensor
where
    for<'a> Self::Observations<'a>: IntoIterator,
    Self: Sized + 'static,
{
    fn sensor_map<F, To>(self, map_f: F) -> MapSensor<To, Self, F>
    where
        F: Copy + for<'a> Fn(<Self::Observations<'a> as IntoIterator>::Item) -> To,
    {
        MapSensor {
            sensor: self,
            map: map_f,
            _phantom: PhantomData,
        }
    }
}

impl<S> IteratorSensor for S
where
    S: Sensor,
    for<'a> S::Observations<'a>: IntoIterator,
    S: Sized + 'static,
{
}

fn foo() {
    // let sensor = CodeCoverageSensor::observing_only_files_from_current_dir();
    // sensor.sensor_map(|x| *x);

    // let mut iter = <&[(usize, u64)] as IntoIterator>::into_iter(&[]);

    // let x = iter.map(|y| y);

    // for a in x {}
    // for a in x {}

    // for (i, c) in iter {}
}
