/*!
Types implementing the [Sensor](crate::Sensor) and [Pool](crate::Pool) traits.
*/

mod and_sensor_and_pool;
pub mod map_sensor;
// mod array_of_counters;
// mod compatible_with_iterator_sensor;
mod maximise_counter_value_pool;
// mod merged_sensors;
mod most_n_diverse_pool;
mod noop_sensor;
mod optimise_aggregate_stat_pool;
mod simplest_to_activate_counter_pool;
mod test_failure_pool;
// mod unique_values_pool;
mod unit_pool;

#[doc(inline)]
pub use crate::code_coverage_sensor::CodeCoverageSensor;
#[doc(inline)]
pub use and_sensor_and_pool::{AndPool, AndSensor, AndSensorAndPool, DifferentSensors, SameSensor};
// #[doc(inline)]
// pub use array_of_counters::ArrayOfCounters;
// #[doc(inline)]
// pub use compatible_with_iterator_sensor::CompatibleWithIteratorSensor;
#[doc(inline)]
pub use maximise_counter_value_pool::MaximiseCounterValuePool;
// #[doc(inline)]
// pub use merged_sensors::MergedSensors;
#[doc(inline)]
pub use most_n_diverse_pool::MostNDiversePool;
#[doc(inline)]
pub use noop_sensor::NoopSensor;
#[doc(inline)]
pub use optimise_aggregate_stat_pool::NumberOfActivatedCounters;
#[doc(inline)]
pub use optimise_aggregate_stat_pool::OptimiseAggregateStatPool;
#[doc(inline)]
pub use optimise_aggregate_stat_pool::SumOfCounterValues;
#[doc(inline)]
pub use simplest_to_activate_counter_pool::SimplestToActivateCounterPool;
#[doc(inline)]
pub use test_failure_pool::TestFailure;
#[doc(inline)]
pub use test_failure_pool::TestFailurePool;
#[doc(inline)]
pub use test_failure_pool::TestFailureSensor;
// #[doc(inline)]
// pub use unique_values_pool::UniqueValuesPool;
#[doc(inline)]
pub use unit_pool::UnitPool;

pub(crate) use test_failure_pool::TEST_FAILURE;

/// Each pool has an associated `Stats` type. They're not very interesting, but I don't want to completely hide them, so I have gathered them here.
pub mod stats {
    use crate::traits::Stats;
    use crate::{CSVField, ToCSV};
    use std::fmt::Display;

    #[doc(inline)]
    pub use super::and_sensor_and_pool::AndPoolStats;
    #[doc(inline)]
    pub use super::maximise_counter_value_pool::MaximiseCounterValuePoolStats;
    #[doc(inline)]
    pub use super::most_n_diverse_pool::MostNDiversePoolStats;
    #[doc(inline)]
    pub use super::simplest_to_activate_counter_pool::UniqueCoveragePoolStats;
    #[doc(inline)]
    pub use super::test_failure_pool::TestFailurePoolStats;
    // #[doc(inline)]
    // pub use super::unique_values_pool::UniqueValuesPoolStats;

    /// An empty type that can be used for [`Pool::Stats`](crate::Pool::Stats)
    #[derive(Clone, Copy)]
    pub struct EmptyStats;

    impl Display for EmptyStats {
        #[no_coverage]
        fn fmt(&self, _f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            Ok(())
        }
    }
    impl ToCSV for EmptyStats {
        #[no_coverage]
        fn csv_headers(&self) -> Vec<CSVField> {
            vec![]
        }
        #[no_coverage]
        fn to_csv_record(&self) -> Vec<CSVField> {
            vec![]
        }
    }
    impl Stats for EmptyStats {}
}
