/*!
Types implementing the [Sensor](crate::Sensor) and [Pool](crate::Pool) traits.
*/

mod allocations_sensor;
mod and_sensor_and_pool;
mod map_sensor;
mod maximise_each_counter_pool;
mod maximise_observation_pool;
mod most_n_diverse_pool;
mod noop_sensor;
mod simplest_to_activate_counter_pool;
mod static_value_sensor;
mod test_failure_pool;
mod unique_values_pool;
mod unit_pool;

#[doc(inline)]
pub use allocations_sensor::{AllocationSensor, CountingAllocator};
#[doc(inline)]
pub use and_sensor_and_pool::{AndPool, AndSensor, AndSensorAndPool, DifferentObservations, SameObservations};
#[doc(inline)]
pub use map_sensor::MapSensor;
#[doc(inline)]
pub use map_sensor::WrapperSensor;
#[doc(inline)]
pub use maximise_each_counter_pool::MaximiseEachCounterPool;
#[doc(inline)]
pub use maximise_observation_pool::MaximiseObservationPool;
#[doc(inline)]
pub use most_n_diverse_pool::MostNDiversePool;
#[doc(inline)]
pub use noop_sensor::NoopSensor;
#[doc(inline)]
pub use simplest_to_activate_counter_pool::SimplestToActivateCounterPool;
#[doc(inline)]
pub use static_value_sensor::StaticValueSensor;
#[doc(inline)]
pub use test_failure_pool::TestFailure;
#[doc(inline)]
pub use test_failure_pool::TestFailurePool;
#[doc(inline)]
pub use test_failure_pool::TestFailureSensor;
pub(crate) use test_failure_pool::TEST_FAILURE;
#[doc(inline)]
pub use unique_values_pool::UniqueValuesPool;
#[doc(inline)]
pub use unit_pool::UnitPool;

#[doc(inline)]
pub use crate::code_coverage_sensor::CodeCoverageSensor;
use crate::{Pool, Sensor};

/// A trait for convenience methods automatically implemented for all types that conform to Pool.
pub trait PoolExt: Pool + Sized {
    /// Create an [`AndPool`](crate::sensors_and_pools::AndPool) from both `Self` and `P`.
    ///
    /// ## Arguments
    /// - `p` is the other pool to combine with `self`
    /// - `override_weight` determines the relative chance of selecting `p` when the resulting [`AndPool`](crate::sensors_and_pools::AndPool)
    /// is asked to provide a test case. If `None`, [`p.weight()`](crate::Pool::weight) will be used. The weight of `self` is always `self.weight()`.
    /// - `_sensor_marker` tells whether `self` and `p` operate on the same observations or not. If they do, pass [`SameObservations`](crate::sensors_and_pools::SameObservations).
    /// Otherwise, pass [`DifferentObservations`](`crate::sensors_and_pools::DifferentObservations`). See the documentation of [`AndPool`](crate::sensors_and_pools::AndPool) for more details.
    fn and<P, SM>(self, p: P, override_weight: Option<f64>, _sensor_marker: SM) -> AndPool<Self, P, SM>
    where
        P: Pool,
    {
        let self_weight = self.weight();
        let p_weight = p.weight();
        AndPool::<_, _, SM>::new(self, p, self_weight, override_weight.unwrap_or(p_weight))
    }
}

impl<P> PoolExt for P where P: Pool {}

/// A trait for convenience methods automatically implemented
/// for all types that conform to [`Sensor`].
pub trait SensorExt: Sensor {
    /// Maps the observations of the sensor using the given closure.
    ///
    /// For example, if a sensor has observations of type `Vec<u64>`, then we
    /// can create a sensor with observations `(Vec<u64>, u64)`, where the
    /// second element of the tuple is the sum of all observations:
    /// ```
    /// use fuzzcheck::SensorExt;
    /// # use fuzzcheck::sensors_and_pools::StaticValueSensor;
    /// # static mut COUNTERS: [u64; 2] = [0; 2];
    /// # // inside the fuzz test, you can create the sensor as follows
    /// # let sensor = unsafe { StaticValueSensor::new(&mut COUNTERS, [0, 0]) };
    /// let sensor = sensor.map(|observations| {
    ///    let sum = observations.iter().sum::<u64>();
    ///    (observations, sum)
    /// });
    /// ```
    #[coverage(off)]
    fn map<ToObservations, F>(self, map_f: F) -> MapSensor<Self, ToObservations, F>
    where
        Self: Sized,
        F: Fn(Self::Observations) -> ToObservations,
    {
        MapSensor::new(self, map_f)
    }
}
impl<T> SensorExt for T where T: Sensor {}

/// Each pool has an associated `Stats` type. They're not very interesting, but I don't want to completely hide them, so I have gathered them here.
pub mod stats {
    use std::fmt::Display;

    #[doc(inline)]
    pub use super::and_sensor_and_pool::AndPoolStats;
    #[doc(inline)]
    pub use super::maximise_each_counter_pool::MaximiseEachCounterPoolStats;
    #[doc(inline)]
    pub use super::most_n_diverse_pool::MostNDiversePoolStats;
    #[doc(inline)]
    pub use super::simplest_to_activate_counter_pool::UniqueCoveragePoolStats;
    #[doc(inline)]
    pub use super::test_failure_pool::TestFailurePoolStats;
    #[doc(inline)]
    pub use super::unique_values_pool::UniqueValuesPoolStats;
    use crate::traits::Stats;
    use crate::{CSVField, ToCSV};

    /// An empty type that can be used for [`Pool::Stats`](crate::Pool::Stats)
    #[derive(Clone, Copy)]
    pub struct EmptyStats;

    impl Display for EmptyStats {
        #[coverage(off)]
        fn fmt(&self, _f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            Ok(())
        }
    }
    impl ToCSV for EmptyStats {
        #[coverage(off)]
        fn csv_headers(&self) -> Vec<CSVField> {
            vec![]
        }
        #[coverage(off)]
        fn to_csv_record(&self) -> Vec<CSVField> {
            vec![]
        }
    }
    impl Stats for EmptyStats {}
}
