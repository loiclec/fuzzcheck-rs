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
/// The [Observations](crate::Sensor::Observations) of this sensor is a reference to the array.
/// Note that most pools provided by fuzzcheck are compatible with iterators over values of type `&'a (usize, u64)`
/// where the first element of the tuple is strictly larger than its predecessors and the second element of the
/// tuple is guaranteed to be greater than 0.
///
/// Therefore, if you wish to use an `ArrayOfCounters` with these pools, you need to wrap it in a sensor that
/// calls `enumerate()` on the observations and filter out its zero elements. You can do so as follows:
/// ```
/// use fuzzcheck::SensorExt;
/// use fuzzcheck::sensors_and_pools::ArrayOfCounters;
/// use fuzzcheck::sensors_and_pools::SimplestToActivateCounterPool;
/// // the “counters” array must be a static item
/// static mut COUNTERS: [u64; 2] = [0; 2];
///
/// // inside the fuzz test, you can create the sensor as follows
/// let sensor = ArrayOfCounters::new(unsafe { &mut COUNTERS });
///
/// let sensor = sensor.map(|o| o.into_iter().copied().filter(|&o| o != 0).enumerate().collect::<Vec<_>>());
/// // now this sensor is compatible with `SimplestToActivateCounterPool`:
/// let pool = SimplestToActivateCounterPool::new("simplest_cov_custom", 2);
/// # let sensor_and_pool: Box<dyn fuzzcheck::SensorAndPool> = Box::new((sensor, pool));
/// ```
///
pub struct ArrayOfCounters<T, const N: usize> {
    start: *mut T,
}

impl<T, const N: usize> ArrayOfCounters<T, N> {
    #[no_coverage]
    pub fn new(xs: &'static mut [T; N]) -> Self {
        Self { start: xs.as_mut_ptr() }
    }
    #[no_coverage]
    pub fn offset_counter_id_by(self) -> Self {
        Self { start: self.start }
    }
    #[no_coverage]
    pub fn len(&self) -> usize {
        N
    }
}

impl<T, const N: usize> Sensor for ArrayOfCounters<T, N>
where
    T: 'static + Default + Copy,
{
    type Observations = &'static [T];

    #[no_coverage]
    fn start_recording(&mut self) {
        unsafe {
            let slice = std::slice::from_raw_parts_mut(self.start, N);
            for x in slice.iter_mut() {
                *x = T::default();
            }
        }
    }

    #[no_coverage]
    fn stop_recording(&mut self) {}

    #[no_coverage]
    fn get_observations(&mut self) -> Self::Observations {
        unsafe { std::slice::from_raw_parts(self.start, N) }
    }
}
impl<T, const N: usize> SaveToStatsFolder for ArrayOfCounters<T, N> {
    #[no_coverage]
    fn save_to_stats_folder(&self) -> Vec<(PathBuf, Vec<u8>)> {
        vec![]
    }
}
