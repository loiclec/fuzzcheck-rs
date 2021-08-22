use fuzzcheck_common::FuzzerEvent;
use std::{fmt::Display, hash::Hash};

use crate::mutators::either::Either;

#[derive(Default, Clone, Copy)]
pub struct EmptyStats;
impl Display for EmptyStats {
    #[no_coverage]
    fn fmt(&self, _f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Ok(())
    }
}

pub struct CorpusDelta<T, Idx> {
    pub add: Option<(T, Idx)>,
    pub remove: Vec<Idx>,
}
impl<T, Idx> Default for CorpusDelta<T, Idx> {
    #[no_coverage]
    fn default() -> Self {
        Self {
            add: Default::default(),
            remove: Default::default(),
        }
    }
}

impl<T, Idx> CorpusDelta<T, Idx> {
    #[no_coverage]
    pub fn convert<U>(self, convert_f: impl FnOnce(T) -> U) -> CorpusDelta<U, Idx> {
        CorpusDelta {
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

pub trait TestCase {
    fn generation(&self) -> usize;
}

pub trait Sensor {
    fn start_recording(&mut self);
    fn stop_recording(&mut self);
}

pub trait Pool {
    type TestCase: TestCase;
    type Index: Hash + Eq + Clone + Copy;

    fn len(&self) -> usize;

    fn get_random_index(&self) -> Option<Self::Index>;
    fn get(&self, idx: Self::Index) -> &Self::TestCase;
    fn get_mut(&mut self, idx: Self::Index) -> &mut Self::TestCase;

    fn retrieve_after_processing(&mut self, idx: Self::Index, generation: usize) -> Option<&mut Self::TestCase>;
    fn mark_test_case_as_dead_end(&mut self, idx: Self::Index);
}

pub trait SensorAndPool {
    type Sensor: Sensor;
    type Pool: Pool<TestCase = Self::TestCase>;

    type TestCase: TestCase;
    type Event;
    type Stats: Default + Display + Clone;

    fn process(
        sensor: &mut Self::Sensor,
        pool: &mut Self::Pool,
        stats: &mut Self::Stats,
        get_input_ref: Either<<Self::Pool as Pool>::Index, &Self::TestCase>,
        clone_input: &impl Fn(&Self::TestCase) -> Self::TestCase,
        complexity: f64,
        event_handler: impl FnMut(
            CorpusDelta<&Self::TestCase, <Self::Pool as Pool>::Index>,
            &Self::Stats,
        ) -> Result<(), std::io::Error>,
    ) -> Result<(), std::io::Error>;

    fn minify(
        sensor: &mut Self::Sensor,
        pool: &mut Self::Pool,
        stats: &mut Self::Stats,
        target_len: usize,
        event_handler: impl FnMut(CorpusDelta<&Self::TestCase, <Self::Pool as Pool>::Index>, &Self::Stats),
    );

    fn get_corpus_delta_from_event<'a>(
        pool: &'a Self::Pool,
        event: Self::Event,
    ) -> CorpusDelta<&'a Self::TestCase, <Self::Pool as Pool>::Index>;
}
