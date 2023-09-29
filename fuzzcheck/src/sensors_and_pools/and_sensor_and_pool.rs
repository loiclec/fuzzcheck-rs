//! Types to combine multiple sensors and pools together
//!
//! If we have two tuples of compatible sensors and pools:
//! * `s1` and `p1`
//! * `s2` and `p2`
//!
//! Then we can combine them into a single sensor and pool as follows:
//! ```
//! use fuzzcheck::sensors_and_pools::{AndSensor, AndPool, DifferentObservations};
//! use fuzzcheck::PoolExt;
//! # use fuzzcheck::sensors_and_pools::{NoopSensor, UniqueValuesPool};
//! # let (s1, s2) = (NoopSensor, NoopSensor);
//! # let (p1, p2) = (UniqueValuesPool::<u8>::new("a", 0), UniqueValuesPool::<bool>::new("b", 0));
//! let s = AndSensor(s1, s2);
//! let p = p1.and(p2, Some(2.0), DifferentObservations);
//! // 1.0 overrides the weight of `p2`, which influences how often the fuzzer will choose a test case
//! // from that pool. By default, all pools have a weight of 1.0. Therefore, in this case, asssuming
//! // `p1` has a weight of 1.0, then test cases will chosen from `p2` as often as `p1`. We can keep
//! // `p2`s original weight using:
//! # let (p1, p2) = (UniqueValuesPool::<u8>::new("a", 0), UniqueValuesPool::<bool>::new("b", 0));
//! let p = p1.and(p2, None, DifferentObservations);
//! // Note that the weight of `p` is the weight of `p1` plus the weight of `p2`.
//! ```
//! At every iteration of the fuzz test, both pools have a chance to provide a test case to mutate.
//! After the test function is run, both sensors will collect data and feed them to their respective pool.
//!
//! It is also possible to use two pools processing the observations of a single sensor. This is done
//! as follows:
//! ```
//! use fuzzcheck::sensors_and_pools::{AndSensor, AndPool, SameObservations};
//! use fuzzcheck::PoolExt;
//! # use fuzzcheck::sensors_and_pools::{NoopSensor, UniqueValuesPool};
//! # let s = NoopSensor;
//! # let (p1, p2) = (UniqueValuesPool::<u8>::new("a", 0), UniqueValuesPool::<u8>::new("b", 0));
//! let p = p1.and(p2, Some(2.0), SameObservations);
//! // if both `p1` and `p2` are compatible with the observations from sensor `s`,
//! // then (s, p) is a valid combination of sensor and pool
//! ```
use std::fmt::Display;
use std::marker::PhantomData;
use std::path::PathBuf;

use crate::traits::{CompatibleWithObservations, CorpusDelta, Pool, SaveToStatsFolder, Sensor, SensorAndPool, Stats};
use crate::{CSVField, PoolStorageIndex, ToCSV};
/// Marker type used by [`AndPool`] to signal that all sub-pools are compatible with the same observations.
pub struct SameObservations;

/// Marker type used by [`AndPool`] to signal that each sub-pool works is compatible with different observations.
pub struct DifferentObservations;

/// A pool that combines two pools
///
/// A convenient way to create an `AndPool` is to use [`p1.and(p2, ..)`](crate::PoolExt::and), but you
/// are free to use [`AndPool::new`](AndPool::new) as well.
///
/// If the two pools act on the same observations , then the `ObservationsMarker` generic type
/// parameter should be [`SameObservations`]. However, if they act on different observations,
/// then this type parameter should be [`DifferentObservations`]. This will influence what
/// observations the `AndPool` is [compatible with](crate::CompatibleWithObservations).
///
/// If both `P1` and `P2` are [`CompatibleWithObservations<O>`], then
/// `AndPool<P1, P2, SameObservations>` will be `CompatibleWithObservations<O>` as well.
///
/// If `P1` is [`CompatibleWithObservations<O1>`] and `P2` is [`CompatibleWithObservations<O2>`], then
/// `AndPool<P1, P2, DifferentObservations>` will be `CompatibleWithObservations<(O1, O2)>`.
///
/// When the `AndPool` is [asked to provide a test case](crate::Pool::get_random_index), it will
/// choose between `p1` and `p2` randomly based on their weights, given by `self.p1_weight` and `self.p2_weight`,
/// and based on how recently `p1` or `p2` made some progress. Pools that make progress will be prefered
/// over pools that do not.
pub struct AndPool<P1, P2, ObservationsMarker>
where
    P1: Pool,
    P2: Pool,
{
    pub p1: P1,
    pub p2: P2,

    pub p1_weight: f64,
    pub p2_weight: f64,

    p1_number_times_chosen_since_last_progress: usize,
    p2_number_times_chosen_since_last_progress: usize,

    rng: fastrand::Rng,
    _phantom: PhantomData<ObservationsMarker>,
}
impl<P1, P2, ObservationsMarker> AndPool<P1, P2, ObservationsMarker>
where
    P1: Pool,
    P2: Pool,
{
    #[coverage(off)]
    pub fn new(p1: P1, p2: P2, p1_weight: f64, p2_weight: f64) -> Self {
        Self {
            p1,
            p2,
            p1_weight,
            p2_weight,
            p1_number_times_chosen_since_last_progress: 1,
            p2_number_times_chosen_since_last_progress: 1,
            rng: fastrand::Rng::new(),
            _phantom: PhantomData,
        }
    }
}
impl<P1, P2, ObservationsMarker> AndPool<P1, P2, ObservationsMarker>
where
    P1: Pool,
    P2: Pool,
{
    fn p1_weight(&self) -> f64 {
        self.p1_weight / self.p1_number_times_chosen_since_last_progress as f64
    }
    fn p2_weight(&self) -> f64 {
        self.p2_weight / self.p2_number_times_chosen_since_last_progress as f64
    }
}
impl<P1, P2, ObservationsMarker> Pool for AndPool<P1, P2, ObservationsMarker>
where
    P1: Pool,
    P2: Pool,
{
    type Stats = AndPoolStats<P1::Stats, P2::Stats>;

    #[coverage(off)]
    fn stats(&self) -> Self::Stats {
        AndPoolStats(self.p1.stats(), self.p2.stats())
    }
    #[coverage(off)]
    fn get_random_index(&mut self) -> Option<PoolStorageIndex> {
        let choice = self.rng.f64() * self.weight();
        if choice <= self.p1_weight() {
            if let Some(idx) = self.p1.get_random_index() {
                self.p1_number_times_chosen_since_last_progress += 1;
                Some(idx)
            } else {
                self.p2_number_times_chosen_since_last_progress += 1;
                self.p2.get_random_index()
            }
        } else if let Some(idx) = self.p2.get_random_index() {
            self.p2_number_times_chosen_since_last_progress += 1;
            Some(idx)
        } else {
            self.p1_number_times_chosen_since_last_progress += 1;
            self.p1.get_random_index()
        }
    }

    fn weight(&self) -> f64 {
        self.p1_weight() + self.p2_weight()
    }
}

impl<P1, P2, ObservationsMarker> SaveToStatsFolder for AndPool<P1, P2, ObservationsMarker>
where
    P1: Pool,
    P2: Pool,
{
    #[coverage(off)]
    fn save_to_stats_folder(&self) -> Vec<(PathBuf, Vec<u8>)> {
        let mut x = self.p1.save_to_stats_folder();
        x.extend(self.p2.save_to_stats_folder());
        x
    }
}

/// A sensor that combines two sensors
///
/// The [`observations`](crate::Sensor::Observations) from this sensor are the combination
/// of the observations of both `S1` and `S2`.
/// So `AndSensor<S1, S2>` implements `Sensor<Observations = (S1::Observations, S2::Observations)>`.
///
/// To create a pool that is compatible with an `AndSensor`, use an [`AndPool`] with the [`DifferentObservations`]
/// marker type.
pub struct AndSensor<S1, S2>(pub S1, pub S2)
where
    S1: Sensor,
    S2: Sensor;

impl<S1, S2> Sensor for AndSensor<S1, S2>
where
    S1: Sensor,
    S2: Sensor,
{
    type Observations = (S1::Observations, S2::Observations);

    #[coverage(off)]
    fn start_recording(&mut self) {
        self.0.start_recording();
        self.1.start_recording();
    }
    #[coverage(off)]
    fn stop_recording(&mut self) {
        self.0.stop_recording();
        self.1.stop_recording();
    }
    #[coverage(off)]
    fn get_observations(&mut self) -> Self::Observations {
        (self.0.get_observations(), self.1.get_observations())
    }
}

impl<S1, S2> SaveToStatsFolder for AndSensor<S1, S2>
where
    S1: Sensor,
    S2: Sensor,
{
    #[coverage(off)]
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
    #[coverage(off)]
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

impl<O1, O2, P1, P2> CompatibleWithObservations<(O1, O2)> for AndPool<P1, P2, DifferentObservations>
where
    P1: Pool,
    P2: Pool,
    P1: CompatibleWithObservations<O1>,
    P2: CompatibleWithObservations<O2>,
{
    #[coverage(off)]
    fn process(&mut self, input_id: PoolStorageIndex, observations: &(O1, O2), complexity: f64) -> Vec<CorpusDelta> {
        let AndPool {
            p1,
            p2,
            p1_number_times_chosen_since_last_progress,
            p2_number_times_chosen_since_last_progress,
            ..
        } = self;
        let deltas_1 = p1.process(input_id, &observations.0, complexity);
        if !deltas_1.is_empty() {
            *p1_number_times_chosen_since_last_progress = 1;
        }
        let deltas_2 = p2.process(input_id, &observations.1, complexity);
        if !deltas_2.is_empty() {
            *p2_number_times_chosen_since_last_progress = 1;
        }
        let mut deltas = deltas_1;
        deltas.extend(deltas_2);
        deltas
    }
}

impl<P1, P2, O> CompatibleWithObservations<O> for AndPool<P1, P2, SameObservations>
where
    P1: CompatibleWithObservations<O>,
    P2: CompatibleWithObservations<O>,
{
    #[coverage(off)]
    fn process(&mut self, input_id: PoolStorageIndex, observations: &O, complexity: f64) -> Vec<CorpusDelta> {
        let AndPool {
            p1,
            p2,
            p1_number_times_chosen_since_last_progress,
            p2_number_times_chosen_since_last_progress,
            ..
        } = self;
        let deltas_1 = p1.process(input_id, observations, complexity);
        if !deltas_1.is_empty() {
            *p1_number_times_chosen_since_last_progress = 1;
        }
        let deltas_2 = p2.process(input_id, observations, complexity);
        if !deltas_2.is_empty() {
            *p2_number_times_chosen_since_last_progress = 1;
        }
        let mut deltas = deltas_1;
        deltas.extend(deltas_2);
        deltas
    }
}

impl<S1, S2> ToCSV for AndPoolStats<S1, S2>
where
    S1: Display,
    S2: Display,
    S1: ToCSV,
    S2: ToCSV,
{
    #[coverage(off)]
    fn csv_headers(&self) -> Vec<CSVField> {
        let mut h = self.0.csv_headers();
        h.extend(self.1.csv_headers());
        h
    }

    #[coverage(off)]
    fn to_csv_record(&self) -> Vec<CSVField> {
        let mut h = self.0.to_csv_record();
        h.extend(self.1.to_csv_record());
        h
    }
}

/// Combines two [`SensorAndPool`](crate::SensorAndPool) trait objects into one.
///
/// You probably won't need to use this type directly because the
/// [`SensorAndPool`](crate::SensorAndPool) trait is mainly used by fuzzcheck itself
/// and not its users. Instead, it is more likely that you are working with types implementing
/// [`Sensor`](crate::Sensor) and [`Pool`](crate::Pool). If that is the case, then you will
/// want to look at [`AndSensor`] and [`AndPool`] (as well as the convenience method to
/// create an `AndPool`: [`p1.and(p2, ..)`](crate::PoolExt::and)).
pub struct AndSensorAndPool {
    sap1: Box<dyn SensorAndPool>,
    sap2: Box<dyn SensorAndPool>,
    sap1_weight: f64,
    sap2_weight: f64,
    sap1_number_times_chosen_since_last_progress: usize,
    sap2_number_times_chosen_since_last_progress: usize,
    rng: fastrand::Rng,
}
impl AndSensorAndPool {
    #[coverage(off)]
    pub fn new(sap1: Box<dyn SensorAndPool>, sap2: Box<dyn SensorAndPool>, sap1_weight: f64, sap2_weight: f64) -> Self {
        Self {
            sap1,
            sap2,
            sap1_weight,
            sap2_weight,
            sap1_number_times_chosen_since_last_progress: 1,
            sap2_number_times_chosen_since_last_progress: 1,
            rng: fastrand::Rng::new(),
        }
    }
}
impl SaveToStatsFolder for AndSensorAndPool {
    #[coverage(off)]
    fn save_to_stats_folder(&self) -> Vec<(PathBuf, Vec<u8>)> {
        let mut x = self.sap1.save_to_stats_folder();
        x.extend(self.sap2.save_to_stats_folder());
        x
    }
}
impl SensorAndPool for AndSensorAndPool {
    #[coverage(off)]
    fn stats(&self) -> Box<dyn crate::traits::Stats> {
        Box::new(AndPoolStats(self.sap1.stats(), self.sap2.stats()))
    }

    #[coverage(off)]
    fn start_recording(&mut self) {
        self.sap1.start_recording();
        self.sap2.start_recording();
    }

    #[coverage(off)]
    fn stop_recording(&mut self) {
        self.sap1.stop_recording();
        self.sap2.stop_recording();
    }

    #[coverage(off)]
    fn process(&mut self, input_id: PoolStorageIndex, cplx: f64) -> Vec<CorpusDelta> {
        let AndSensorAndPool {
            sap1,
            sap2,
            sap1_number_times_chosen_since_last_progress,
            sap2_number_times_chosen_since_last_progress,
            ..
        } = self;
        let deltas_1 = sap1.process(input_id, cplx);
        if !deltas_1.is_empty() {
            *sap1_number_times_chosen_since_last_progress = 1;
        }
        let deltas_2 = sap2.process(input_id, cplx);
        if !deltas_2.is_empty() {
            *sap2_number_times_chosen_since_last_progress = 1;
        }
        let mut deltas = deltas_1;
        deltas.extend(deltas_2);
        deltas
    }

    #[coverage(off)]
    fn get_random_index(&mut self) -> Option<PoolStorageIndex> {
        let sum_weight = self.sap1_weight + self.sap2_weight;
        if self.rng.f64() <= sum_weight {
            if let Some(idx) = self.sap1.get_random_index() {
                self.sap1_number_times_chosen_since_last_progress += 1;
                Some(idx)
            } else {
                self.sap2_number_times_chosen_since_last_progress += 1;
                self.sap2.get_random_index()
            }
        } else if let Some(idx) = self.sap2.get_random_index() {
            self.sap2_number_times_chosen_since_last_progress += 1;
            Some(idx)
        } else {
            self.sap1_number_times_chosen_since_last_progress += 1;
            self.sap1.get_random_index()
        }
    }
}
