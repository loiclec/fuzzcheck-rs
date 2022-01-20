use crate::{SaveToStatsFolder, Sensor};

/// A custom sensor whose observations are given by a mutable static value.
///
/// In the example below, we use a `StaticValueSensor` to maximise
/// the value of a variable used in the test function.
/// ```
/// use fuzzcheck::sensors_and_pools::{StaticValueSensor, MaximiseObservationPool};
/// use fuzzcheck::{Arguments, ReasonForStopping};
///
/// // the “COUNT” variable must be a static item
/// static mut COUNT: usize = 0;
///
/// fn test_function(xs: &[u8]) -> bool {
///     if xs.len() == 6 {
///         let mut number_correct_guesses = 0;
///         if xs[0] == 98  { number_correct_guesses += 1 }
///         if xs[1] == 18  { number_correct_guesses += 1 }
///         if xs[2] == 9   { number_correct_guesses += 1 }
///         if xs[3] == 203 { number_correct_guesses += 1 }
///         if xs[4] == 45  { number_correct_guesses += 1 }
///         if xs[5] == 165 { number_correct_guesses += 1 }
///             
///         // here, record the value of number_correct_guesses in COUNT
///         unsafe { COUNT = number_correct_guesses; }
///         
///         number_correct_guesses != 6
///     } else {
///         true
///     }
/// }
/// // You can create the sensor as follows.
/// // It is unsafe because of the access to the global mutable variable.
/// // After each run of the test function, the sensor resets `COUNT`
/// // to the second argument (here: 0). It is best if you don't access
/// // `COUNT` outside of the test function.
/// let sensor = unsafe { StaticValueSensor::new(&mut COUNT, 0) };
///
/// // The sensor can be paired with any pool which is compatible with
/// // observations of type `usize`. For example, we can use:
/// let pool = MaximiseObservationPool::<usize>::new("maximise_count");
///
/// // then launch fuzzcheck with this sensor and pool
/// let result = fuzzcheck::fuzz_test(test_function)
///     .default_mutator()
///     .serde_serializer()
///     .sensor_and_pool(sensor, pool)
///     .arguments(Arguments::for_internal_documentation_test())
///     .stop_after_first_test_failure(true)
///     .launch();
///
/// assert!(matches!(
///     result.reason_for_stopping,
///     ReasonForStopping::TestFailure(x)
///         if matches!(
///             x.as_slice(),
///             [98, 18, 9, 203, 45, 165]
///         )
/// ));
/// ```
pub struct StaticValueSensor<T>
where
    T: 'static + Clone,
{
    value: *mut T,
    default_value: T,
}
impl<T> StaticValueSensor<T>
where
    T: 'static + Clone,
{
    pub fn new(value: &'static mut T, default_value: T) -> Self {
        Self { value, default_value }
    }
}
impl<T> SaveToStatsFolder for StaticValueSensor<T>
where
    T: 'static + Clone,
{
    fn save_to_stats_folder(&self) -> Vec<(std::path::PathBuf, Vec<u8>)> {
        vec![]
    }
}
impl<T: 'static> Sensor for StaticValueSensor<T>
where
    T: Clone,
{
    type Observations = T;

    fn start_recording(&mut self) {
        unsafe { *self.value = self.default_value.clone() };
    }

    fn stop_recording(&mut self) {
        // unsafe {
        //     self.recorded_value = (*self.value).clone();
        // }
    }

    fn get_observations(&mut self) -> Self::Observations {
        unsafe { (*self.value).clone() }
    }
}
