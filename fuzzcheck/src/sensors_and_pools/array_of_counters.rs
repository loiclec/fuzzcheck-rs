use crate::traits::Sensor;

pub struct ArrayOfCounters<const N: usize> {
    start: *mut u64,
}
impl<const N: usize> ArrayOfCounters<N> {
    pub fn new(xs: &'static mut [u64; N]) -> Self {
        Self { start: xs.as_mut_ptr() }
    }
}

impl<const N: usize> Sensor for ArrayOfCounters<N> {
    type ObservationHandler<'a> = &'a mut dyn FnMut((usize, u64));

    fn start_recording(&mut self) {
        unsafe {
            let slice = std::slice::from_raw_parts_mut(self.start, N);
            for x in slice.iter_mut() {
                *x = 0;
            }
        }
    }

    fn stop_recording(&mut self) {}

    fn iterate_over_observations(&mut self, handler: Self::ObservationHandler<'_>) {
        unsafe {
            let slice = std::slice::from_raw_parts(self.start, N);
            for (i, &x) in slice.iter().enumerate() {
                if x != 0 {
                    handler((i, x))
                }
            }
        }
    }
}
