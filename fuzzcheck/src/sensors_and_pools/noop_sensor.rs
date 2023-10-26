use std::path::PathBuf;

use crate::traits::{SaveToStatsFolder, Sensor};

/// A sensor that does nothing.
///
/// In practice, it is used in conjunction with [`UnitPool`](crate::sensors_and_pools::UnitPool) to
/// favour one particular test case throughout the whole fuzzing run. This is partly how the `minify`
/// command is implemented.
pub struct NoopSensor;

impl Sensor for NoopSensor {
    type Observations = ();
    #[coverage(off)]
    fn start_recording(&mut self) {}
    #[coverage(off)]
    fn stop_recording(&mut self) {}

    #[coverage(off)]
    fn get_observations(&mut self) {}
}
impl SaveToStatsFolder for NoopSensor {
    #[coverage(off)]
    fn save_to_stats_folder(&self) -> Vec<(PathBuf, Vec<u8>)> {
        vec![]
    }
}
