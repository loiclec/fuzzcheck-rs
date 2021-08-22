use crate::sensor_and_pool::Sensor;

pub struct NoopSensor;

impl Sensor for NoopSensor {
    #[no_coverage]
    fn start_recording(&mut self) {}
    #[no_coverage]
    fn stop_recording(&mut self) {}
}
