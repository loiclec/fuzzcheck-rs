use fuzzcheck_common::FuzzerEvent;
use std::{fmt::Display, hash::Hash, path::PathBuf};

use crate::mutators::either::Either;

#[derive(Default, Clone, Copy)]
pub struct EmptyStats;
impl Display for EmptyStats {
    #[no_coverage]
    fn fmt(&self, _f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Ok(())
    }
}

#[derive(Debug)]
pub struct CorpusDelta<T, Idx> {
    pub path: PathBuf,
    pub add: Option<(T, Idx)>,
    pub remove: Vec<Idx>,
}

impl<T, Idx> CorpusDelta<T, Idx> {
    #[no_coverage]
    pub fn convert<U>(self, convert_f: impl FnOnce(T) -> U) -> CorpusDelta<U, Idx> {
        CorpusDelta {
            path: self.path,
            add: self.add.map(
                #[no_coverage]
                |(x, idx)| (convert_f(x), idx),
            ),
            remove: self.remove,
        }
    }
    #[no_coverage]
    pub fn fuzzer_event(&self) -> FuzzerEvent {
        if self.add.is_some() {
            if self.remove.is_empty() {
                FuzzerEvent::New
            } else {
                FuzzerEvent::Replace(self.remove.len())
            }
        } else {
            if self.remove.is_empty() {
                FuzzerEvent::None
            } else {
                FuzzerEvent::Remove(self.remove.len())
            }
        }
    }
}

pub trait TestCase: Clone {
    fn generation(&self) -> usize;
}

pub trait Sensor {
    type ObservationHandler<'a>;
    fn start_recording(&mut self);
    fn stop_recording(&mut self);

    fn iterate_over_observations(&mut self, handler: Self::ObservationHandler<'_>);
}

pub trait Pool {
    type TestCase: TestCase;
    type Index: Hash + Eq + Clone + Copy;
    type Stats: Default + Display + Clone;

    fn len(&self) -> usize;
    fn stats(&self) -> Self::Stats;

    fn get_random_index(&mut self) -> Option<Self::Index>;
    fn get(&self, idx: Self::Index) -> &Self::TestCase;
    fn get_mut(&mut self, idx: Self::Index) -> &mut Self::TestCase;

    fn retrieve_after_processing(&mut self, idx: Self::Index, generation: usize) -> Option<&mut Self::TestCase>;
    fn mark_test_case_as_dead_end(&mut self, idx: Self::Index);
}

pub trait CompatibleWithSensor<S: Sensor>: Pool {
    fn process(
        &mut self,
        sensor: &mut S,
        get_input_ref: Either<Self::Index, &Self::TestCase>,
        clone_input: &impl Fn(&Self::TestCase) -> Self::TestCase,
        complexity: f64,
        event_handler: impl FnMut(CorpusDelta<&Self::TestCase, Self::Index>, Self::Stats) -> Result<(), std::io::Error>,
    ) -> Result<(), std::io::Error>;

    fn minify(
        &mut self,
        sensor: &mut S,
        target_len: usize,
        event_handler: impl FnMut(CorpusDelta<&Self::TestCase, Self::Index>, Self::Stats) -> Result<(), std::io::Error>,
    ) -> Result<(), std::io::Error>;
}
