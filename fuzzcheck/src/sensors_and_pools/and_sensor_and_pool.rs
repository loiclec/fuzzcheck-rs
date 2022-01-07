//! Types to combine multiple sensors and pools together
//!
//! If we have two tuples of compatible sensors and pools:
//! * `s1` and `p1`
//! * `s2` and `p2`
//!
//! Then we can combine them into a single sensor and pool as follows:
//! ```
//! use fuzzcheck::sensors_and_pools::{AndSensor, AndPool};
//! # use fuzzcheck::sensors_and_pools::{NoopSensor, UniqueValuesPool};
//! # let (s1, s2) = (NoopSensor, NoopSensor);
//! # let (p1, p2) = (UniqueValuesPool::new("a", 0), UniqueValuesPool::new("b", 0));
//! let s = AndSensor(s1, s2);
//! let p = AndPool::new(p1, p2, 128);
//! // 128 is the ratio of times the first pool is chosen when selecting a test case to mutate.
//! // The implicit denominator is 256. So the first pool is chosen 128 / 256 = 50% of the time.
//! ```
//!
//! At every iteration of the fuzz test, both pools have a chance to provide a test case to mutate.
//! After the test function is run, both sensors will collect data and feed them to their respective pool.
use std::{fmt::Display, marker::PhantomData, path::PathBuf};

use crate::{
    fuzzer::PoolStorageIndex,
    traits::{
        CompatibleWithObservations, CorpusDelta, Observations, Pool, SaveToStatsFolder, Sensor, SensorAndPool, Stats,
    },
    CSVField, ToCSV,
};

use super::CloneObservations;

// use super::compatible_with_iterator_sensor::CompatibleWithIteratorSensor;

pub enum SameObservations {}
pub enum DifferentObservations {}

/// A pool that combines two pools
pub struct AndPool<P1, P2, SensorMarker>
where
    P1: Pool,
    P2: Pool,
{
    pub p1: P1,
    pub p2: P2,

    pub ratio_choose_first: u8,
    rng: fastrand::Rng,
    _phantom: PhantomData<SensorMarker>,
}
impl<P1, P2, SensorMarker> AndPool<P1, P2, SensorMarker>
where
    P1: Pool,
    P2: Pool,
{
    #[no_coverage]
    pub fn new(p1: P1, p2: P2, ratio_choose_first: u8) -> Self {
        Self {
            p1,
            p2,
            ratio_choose_first,
            rng: fastrand::Rng::new(),
            _phantom: PhantomData,
        }
    }
}
impl<P1, P2, SensorMarker> Pool for AndPool<P1, P2, SensorMarker>
where
    P1: Pool,
    P2: Pool,
{
    type Stats = AndPoolStats<P1::Stats, P2::Stats>;

    #[no_coverage]
    fn stats(&self) -> Self::Stats {
        AndPoolStats(self.p1.stats(), self.p2.stats())
    }
    #[no_coverage]
    fn get_random_index(&mut self) -> Option<PoolStorageIndex> {
        if self.rng.u8(..) <= self.ratio_choose_first {
            if let Some(idx) = self.p1.get_random_index() {
                Some(idx)
            } else {
                self.p2.get_random_index()
            }
        } else if let Some(idx) = self.p2.get_random_index() {
            Some(idx)
        } else {
            self.p1.get_random_index()
        }
    }
}
impl<P1, P2, SensorMarker> SaveToStatsFolder for AndPool<P1, P2, SensorMarker>
where
    P1: Pool,
    P2: Pool,
{
    #[no_coverage]
    fn save_to_stats_folder(&self) -> Vec<(PathBuf, Vec<u8>)> {
        let mut x = self.p1.save_to_stats_folder();
        x.extend(self.p2.save_to_stats_folder());
        x
    }
}

/// A sensor that combines two sensors
///
/// This type assumes nothing about the relationship between the two sensors.
/// It is most likely that you are also using two different pools to process
/// each sensor’s observations. Then, you can use an [`AndPool`] to combine these
/// two pools and make them compatible with this `AndSensor`.
pub struct AndSensor<S1, S2>(pub S1, pub S2)
where
    S1: Sensor,
    S2: Sensor;

pub struct Tuple2Observations<A, B>
where
    A: Observations,
    B: Observations,
{
    _phantom: PhantomData<(A, B)>,
}

impl<A, B> Observations for Tuple2Observations<A, B>
where
    A: Observations,
    B: Observations,
{
    type Concrete<'a> = (A::Concrete<'a>, B::Concrete<'a>);
}

impl<S1, S2> Sensor for AndSensor<S1, S2>
where
    S1: Sensor,
    S2: Sensor,
{
    type Observations = Tuple2Observations<S1::Observations, S2::Observations>;

    #[no_coverage]
    fn start_recording(&mut self) {
        self.0.start_recording();
        self.1.start_recording();
    }
    #[no_coverage]
    fn stop_recording(&mut self) {
        self.0.stop_recording();
        self.1.stop_recording();
    }
    #[no_coverage]
    fn get_observations<'a>(&'a mut self) -> <Self::Observations as Observations>::Concrete<'a> {
        (self.0.get_observations(), self.1.get_observations())
    }
}

impl<S1, S2> SaveToStatsFolder for AndSensor<S1, S2>
where
    S1: Sensor,
    S2: Sensor,
{
    #[no_coverage]
    fn save_to_stats_folder(&self) -> Vec<(PathBuf, Vec<u8>)> {
        let mut x = self.0.save_to_stats_folder();
        x.extend(self.1.save_to_stats_folder());
        x
    }
}

/// The statistics of an [AndPool]
#[derive(Clone)]
pub struct AndPoolStats<S1: Display, S2: Display>(pub S1, pub S2);
impl<S1: Display, S2: Display> Display for AndPoolStats<S1, S2> {
    #[no_coverage]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {}", self.0, self.1)
    }
}
impl<S1: Display, S2: Display> Stats for AndPoolStats<S1, S2>
where
    S1: Stats,
    S2: Stats,
{
}

impl<O1, O2, P1, P2> CompatibleWithObservations<Tuple2Observations<O1, O2>> for AndPool<P1, P2, DifferentObservations>
where
    O1: Observations,
    O2: Observations,
    P1: Pool,
    P2: Pool,
    P1: CompatibleWithObservations<O1>,
    P2: CompatibleWithObservations<O2>,
{
    #[no_coverage]
    fn process<'a>(
        &'a mut self,
        input_id: PoolStorageIndex,
        observations: (O1::Concrete<'a>, O2::Concrete<'a>),
        complexity: f64,
    ) -> Vec<CorpusDelta> {
        let AndPool { p1, p2, .. } = self;
        let mut deltas = p1.process(input_id, observations.0, complexity);
        deltas.extend(p2.process(input_id, observations.1, complexity));
        deltas
    }
}

impl<P1, P2, O> CompatibleWithObservations<O> for AndPool<P1, P2, SameObservations>
where
    O: CloneObservations,
    P1: CompatibleWithObservations<O>,
    P2: CompatibleWithObservations<O>,
{
    #[no_coverage]
    fn process<'a>(
        &'a mut self,
        input_id: PoolStorageIndex,
        observations: O::Concrete<'a>,
        complexity: f64,
    ) -> Vec<CorpusDelta> {
        let AndPool { p1, p2, .. } = self;
        let mut deltas = p1.process(input_id, O::clone(&observations), complexity);
        deltas.extend(p2.process(input_id, observations, complexity));
        deltas
    }
}

// impl<P1, P2> CompatibleWithIteratorSensor for AndPool<P1, P2>
// where
//     P1: CompatibleWithIteratorSensor,
//     P2: CompatibleWithIteratorSensor<Observation = P1::Observation>,
// {
//     type Observation = P1::Observation;
//     type ObservationState = (P1::ObservationState, P2::ObservationState);

//     #[no_coverage]
//     fn start_observing(&mut self) -> Self::ObservationState {
//         (self.p1.start_observing(), self.p2.start_observing())
//     }

//     #[inline]
//     #[no_coverage]
//     fn observe(&mut self, observation: &Self::Observation, input_complexity: f64, state: &mut Self::ObservationState) {
//         self.p1.observe(observation, input_complexity, &mut state.0);
//         self.p2.observe(observation, input_complexity, &mut state.1);
//     }

//     #[no_coverage]
//     fn add_if_interesting(
//         &mut self,
//         input_id: PoolStorageIndex,
//         complexity: f64,
//         observation_state: Self::ObservationState,
//         observations: &[Self::Observation],
//     ) -> Vec<CorpusDelta> {
//         let (o1, o2) = observation_state;
//         let mut deltas = vec![];
//         deltas.extend(self.p1.add_if_interesting(input_id, complexity, o1, observations));
//         deltas.extend(self.p2.add_if_interesting(input_id, complexity, o2, observations));
//         deltas
//     }
// }
impl<S1, S2> ToCSV for AndPoolStats<S1, S2>
where
    S1: Display,
    S2: Display,
    S1: ToCSV,
    S2: ToCSV,
{
    #[no_coverage]
    fn csv_headers(&self) -> Vec<CSVField> {
        let mut h = self.0.csv_headers();
        h.extend(self.1.csv_headers());
        h
    }

    #[no_coverage]
    fn to_csv_record(&self) -> Vec<CSVField> {
        let mut h = self.0.to_csv_record();
        h.extend(self.1.to_csv_record());
        h
    }
}

/// Combines two [`SensorAndPool`](crate::traits::SensorAndPool) trait objects into one.
pub struct AndSensorAndPool {
    sap1: Box<dyn SensorAndPool>,
    sap2: Box<dyn SensorAndPool>,
    ratio_choose_first: u8,
    rng: fastrand::Rng,
}
impl AndSensorAndPool {
    #[no_coverage]
    pub fn new(sap1: Box<dyn SensorAndPool>, sap2: Box<dyn SensorAndPool>, ratio_choose_first: u8) -> Self {
        Self {
            sap1,
            sap2,
            ratio_choose_first,
            rng: fastrand::Rng::new(),
        }
    }
}
impl SaveToStatsFolder for AndSensorAndPool {
    #[no_coverage]
    fn save_to_stats_folder(&self) -> Vec<(PathBuf, Vec<u8>)> {
        let mut x = self.sap1.save_to_stats_folder();
        x.extend(self.sap2.save_to_stats_folder());
        x
    }
}
impl SensorAndPool for AndSensorAndPool {
    #[no_coverage]
    fn stats(&self) -> Box<dyn crate::traits::Stats> {
        Box::new(AndPoolStats(self.sap1.stats(), self.sap2.stats()))
    }

    #[no_coverage]
    fn start_recording(&mut self) {
        self.sap1.start_recording();
        self.sap2.start_recording();
    }

    #[no_coverage]
    fn stop_recording(&mut self) {
        self.sap1.stop_recording();
        self.sap2.stop_recording();
    }

    #[no_coverage]
    fn process(&mut self, input_id: PoolStorageIndex, cplx: f64) -> Vec<CorpusDelta> {
        let mut x = self.sap1.process(input_id, cplx);
        x.extend(self.sap2.process(input_id, cplx));
        x
    }

    #[no_coverage]
    fn get_random_index(&mut self) -> Option<PoolStorageIndex> {
        if self.rng.u8(..) <= self.ratio_choose_first {
            if let Some(idx) = self.sap1.get_random_index() {
                Some(idx)
            } else {
                self.sap2.get_random_index()
            }
        } else if let Some(idx) = self.sap2.get_random_index() {
            Some(idx)
        } else {
            self.sap1.get_random_index()
        }
    }
}
