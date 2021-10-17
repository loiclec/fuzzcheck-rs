use std::path::PathBuf;

use crate::traits::Sensor;

pub struct NoopSensor;

impl Sensor for NoopSensor {
    type ObservationHandler<'a> = ();
    #[no_coverage]
    fn start_recording(&mut self) {}
    #[no_coverage]
    fn stop_recording(&mut self) {}

    #[no_coverage]
    fn iterate_over_observations(&mut self, _handler: Self::ObservationHandler<'_>) {}

    #[no_coverage]
    fn serialized(&self) -> Vec<(PathBuf, Vec<u8>)> {
        vec![]
    }
}
