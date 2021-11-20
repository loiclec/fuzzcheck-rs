use crate::traits::{SaveToStatsFolder, Sensor};
use std::path::PathBuf;

/// A custom sensor consisting of an array of counters that can be manually set.
///
/// ```
/// use fuzzcheck::sensors_and_pools::ArrayOfCounters;
/// // the “counters” array must be a static item
/// static mut COUNTERS: [u64; 2] = [0; 2];
///
/// // inside the fuzz test, you can create the sensor as follows
/// let sensor = ArrayOfCounters::new(unsafe { &mut COUNTERS });
///
/// fn test_function(x: &[bool]) {
///     // you can then manually instrument a test function by changing the values of COUNTERS
///     unsafe {
///         COUNTERS[0] = x.len() as u64;
///     }
///     // ...
///     unsafe {
///         COUNTERS[1] = x.len() as u64;
///     }
///     // ...
/// }
/// ```
/// The [ObservationHandler](crate::Sensor::ObservationHandler) of this sensor has the same type as the one from
/// the default [code coverage sensor](crate::sensors_and_pools::CodeCoverageSensor). Therefore, most [pools](crate::Pool)
/// that are [compatible with](crate::CompatibleWithSensor) the code coverage sensor will also be compatible with
/// the `ArrayOfCounters` sensor.
///
/// You can also use a different pool such as the [`UniqueValuesPool`](crate::sensors_and_pools::UniqueValuesPool)
pub struct ArrayOfCounters<const N: usize> {
    start: *mut u64,
    offset: usize,
}
impl<const N: usize> ArrayOfCounters<N> {
    #[no_coverage]
    pub fn new(xs: &'static mut [u64; N]) -> Self {
        Self {
            start: xs.as_mut_ptr(),
            offset: 0,
        }
    }
    #[no_coverage]
    pub fn offset_counter_id_by(self, offset: usize) -> Self {
        Self {
            start: self.start,
            offset,
        }
    }
    #[no_coverage]
    pub fn len(&self) -> usize {
        N
    }
}

impl<const N: usize> Sensor for ArrayOfCounters<N> {
    type ObservationHandler<'a> = &'a mut dyn FnMut((usize, u64));

    #[no_coverage]
    fn start_recording(&mut self) {
        unsafe {
            let slice = std::slice::from_raw_parts_mut(self.start, N);
            for x in slice.iter_mut() {
                *x = 0;
            }
        }
    }

    #[no_coverage]
    fn stop_recording(&mut self) {}

    #[no_coverage]
    fn iterate_over_observations(&mut self, handler: Self::ObservationHandler<'_>) {
        unsafe {
            let slice = std::slice::from_raw_parts(self.start, N);
            for (i, &x) in slice.iter().enumerate() {
                if x != 0 {
                    handler((self.offset + i, x))
                }
            }
        }
    }
}
impl<const N: usize> SaveToStatsFolder for ArrayOfCounters<N> {
    #[no_coverage]
    fn save_to_stats_folder(&self) -> Vec<(PathBuf, Vec<u8>)> {
        vec![]
    }
}
