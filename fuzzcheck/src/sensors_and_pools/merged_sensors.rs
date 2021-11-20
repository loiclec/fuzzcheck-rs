use std::marker::PhantomData;

use crate::{traits::SaveToStatsFolder, Sensor};

/// A sensor that combines two sensors of the same kind into one.
///
/// Unlike [`AndSensor`](crate::sensors_and_pools::AndSensor), the two sensors
/// must share the same [observation handler](crate::Sensor::ObservationHandler),
/// which must be of type `&'a mut dyn FnMut(O)`.
///
/// The resulting sensor keeps the same observation handler as the two sensors, and
/// is therefore more likely to be compatible with the same pools.
///
/// You can use this sensor to extend the [`CodeCoverageSensor`](crate::sensors_and_pools::CodeCoverageSensor)
/// with your own observations. But be careful to make your sensor’s counter ids unique.
/// For example, you can write:
/// ```no_run
/// use fuzzcheck::sensors_and_pools::{MergedSensors, CodeCoverageSensor, ArrayOfCounters, SimplestToActivateCounterPool};
///
/// static mut COUNTERS: [u64; 2] = [0; 2];
/// let coverage_sensor = CodeCoverageSensor::observing_only_files_from_current_dir();
/// let my_sensor = ArrayOfCounters::new(unsafe { &mut COUNTERS }).offset_counter_id_by(coverage_sensor.count_instrumented);
/// let total_nbr_counters = coverage_sensor.count_instrumented + my_sensor.len();
/// let sensor = MergedSensors::new(coverage_sensor, my_sensor);
/// let pool = SimplestToActivateCounterPool::new("simplest_cov", total_nbr_counters);
/// ```
/// Then you can add your own instrumentation:
/// ```
/// # static mut COUNTERS: [u64; 2] = [0; 2];
/// # fn cond1() -> bool { true }
/// # fn cond2() -> bool { true }
/// fn foo() {
///     let mut x: u64 = 0;
///     if cond1() {
///         x += 1;    
///     }
///     if cond2() {
///         x += 1;
///     }
///     // ...
///     unsafe {
///         // This counter is activated only when x > 1 .
///         // Since we've merged our sensor with the code coverage sensor, the fuzzer
///         // will treat the case where x > 1 as a distinct new code coverage point
///         //
///         // Also note that, in this case, if we add a pool that maximises the
///         // number of code coverage hits, then it will also maximise the value of `x`.
///         // This is not always what we want.
///         COUNTERS[0] = x.saturating_sub(1);
///     }
/// }
/// ```
pub struct MergedSensors<S1, S2, O>
where
    O: 'static,
    S1: for<'a> Sensor<ObservationHandler<'a> = &'a mut dyn FnMut(O)>,
    S2: for<'a> Sensor<ObservationHandler<'a> = &'a mut dyn FnMut(O)>,
{
    s1: S1,
    s2: S2,
    _phantom: PhantomData<O>,
}

impl<S1, S2, O> MergedSensors<S1, S2, O>
where
    O: 'static,
    S1: for<'a> Sensor<ObservationHandler<'a> = &'a mut dyn FnMut(O)>,
    S2: for<'a> Sensor<ObservationHandler<'a> = &'a mut dyn FnMut(O)>,
{
    #[no_coverage]
    pub fn new(s1: S1, s2: S2) -> Self {
        Self {
            s1,
            s2,
            _phantom: PhantomData,
        }
    }
}

impl<S1, S2, O> SaveToStatsFolder for MergedSensors<S1, S2, O>
where
    O: 'static,
    S1: for<'a> Sensor<ObservationHandler<'a> = &'a mut dyn FnMut(O)>,
    S2: for<'a> Sensor<ObservationHandler<'a> = &'a mut dyn FnMut(O)>,
{
    #[no_coverage]
    fn save_to_stats_folder(&self) -> Vec<(std::path::PathBuf, Vec<u8>)> {
        let mut xs = self.s1.save_to_stats_folder();
        xs.extend(self.s2.save_to_stats_folder());
        xs
    }
}
impl<S1, S2, O> Sensor for MergedSensors<S1, S2, O>
where
    O: 'static,
    S1: for<'a> Sensor<ObservationHandler<'a> = &'a mut dyn FnMut(O)>,
    S2: for<'a> Sensor<ObservationHandler<'a> = &'a mut dyn FnMut(O)>,
{
    type ObservationHandler<'a> = &'a mut dyn FnMut(O);
    #[no_coverage]
    fn start_recording(&mut self) {
        self.s1.start_recording();
        self.s2.start_recording();
    }
    #[no_coverage]
    fn stop_recording(&mut self) {
        self.s1.stop_recording();
        self.s2.stop_recording();
    }
    #[no_coverage]
    fn iterate_over_observations(&mut self, handler: Self::ObservationHandler<'_>) {
        self.s1.iterate_over_observations(handler);
        self.s2.iterate_over_observations(handler);
    }
}
