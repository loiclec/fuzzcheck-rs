use std::path::PathBuf;

use crate::traits::{SaveToStatsFolder, Sensor};

/// A sensor that does nothing.
///
/// In practice, it is used in conjunction with [`UnitPool`](crate::sensors_and_pools::UnitPool) to
/// favour one particular test case throughout the whole fuzzing run. This is partly how the `minify`
/// (test case minify) command is implemented.
pub struct NoopSensor;

impl Sensor for NoopSensor {
    type ObservationHandler<'a> = ();
    #[no_coverage]
    fn start_recording(&mut self) {}
    #[no_coverage]
    fn stop_recording(&mut self) {}

    #[no_coverage]
    fn iterate_over_observations(&mut self, _handler: Self::ObservationHandler<'_>) {}
}
impl SaveToStatsFolder for NoopSensor {
    #[no_coverage]
    fn save_to_stats_folder(&self) -> Vec<(PathBuf, Vec<u8>)> {
        vec![]
    }
}
