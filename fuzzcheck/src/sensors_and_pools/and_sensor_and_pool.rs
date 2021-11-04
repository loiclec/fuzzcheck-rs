use std::{fmt::Display, path::PathBuf};

use crate::{
    fuzzer::PoolStorageIndex,
    traits::{CompatibleWithSensor, CorpusDelta, Pool, Sensor},
    CSVField, ToCSV,
};

use super::compatible_with_iterator_sensor::CompatibleWithIteratorSensor;

pub struct AndPool<P1, P2>
where
    P1: Pool,
    P2: Pool,
{
    pub p1: P1,
    pub p2: P2,

    pub ratio_choose_first: u8,
    pub rng: fastrand::Rng,
}

impl<P1, P2> AndPool<P1, P2>
where
    P1: Pool,
    P2: Pool,
{
    #[no_coverage]
    pub fn new(p1: P1, p2: P2, ratio_choose_first: u8) -> Self {
        Self {
            p1,
            p2,
            ratio_choose_first,
            rng: fastrand::Rng::new(),
        }
    }
}
impl<P1, P2> Pool for AndPool<P1, P2>
where
    P1: Pool,
    P2: Pool,
{
    type Stats = AndStats<P1::Stats, P2::Stats>;

    #[no_coverage]
    fn len(&self) -> usize {
        self.p1.len() + self.p2.len()
    }
    #[no_coverage]
    fn stats(&self) -> Self::Stats {
        AndStats {
            stats1: self.p1.stats(),
            stats2: self.p2.stats(),
        }
    }
    #[no_coverage]
    fn get_random_index(&mut self) -> Option<PoolStorageIndex> {
        if self.rng.u8(..) <= self.ratio_choose_first {
            if let Some(idx) = self.p1.get_random_index() {
                Some(idx)
            } else {
                self.p2.get_random_index()
            }
        } else {
            if let Some(idx) = self.p2.get_random_index() {
                Some(idx)
            } else {
                self.p1.get_random_index()
            }
        }
    }

    #[no_coverage]
    fn mark_test_case_as_dead_end(&mut self, idx: PoolStorageIndex) {
        self.p1.mark_test_case_as_dead_end(idx);
        self.p2.mark_test_case_as_dead_end(idx);
    }
    #[no_coverage]
    fn serialized(&self) -> Vec<(std::path::PathBuf, Vec<u8>)> {
        let mut x = self.p1.serialized();
        x.extend(self.p2.serialized());
        x
    }
}

pub struct AndSensor<S1, S2>
where
    S1: Sensor,
    S2: Sensor,
{
    pub s1: S1,
    pub s2: S2,
}
impl<S1, S2> Sensor for AndSensor<S1, S2>
where
    S1: Sensor,
    S2: Sensor,
{
    type ObservationHandler<'a> = (S1::ObservationHandler<'a>, S2::ObservationHandler<'a>);

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
        self.s1.iterate_over_observations(handler.0);
        self.s2.iterate_over_observations(handler.1);
    }

    #[no_coverage]
    fn serialized(&self) -> Vec<(PathBuf, Vec<u8>)> {
        let mut x = self.s1.serialized();
        x.extend(self.s2.serialized());
        x
    }
}

#[derive(Clone)]
pub struct AndStats<S1: Display, S2: Display> {
    pub stats1: S1,
    pub stats2: S2,
}
impl<S1: Display, S2: Display> Display for AndStats<S1, S2> {
    #[no_coverage]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {}", self.stats1, self.stats2)
    }
}

impl<S1, S2, P1, P2> CompatibleWithSensor<AndSensor<S1, S2>> for AndPool<P1, P2>
where
    S1: Sensor,
    S2: Sensor,
    P1: Pool,
    P2: Pool,
    P1: CompatibleWithSensor<S1>,
    P2: CompatibleWithSensor<S2>,
{
    #[no_coverage]
    fn process(
        &mut self,
        input_id: PoolStorageIndex,
        sensor: &mut AndSensor<S1, S2>,
        complexity: f64,
    ) -> Vec<CorpusDelta> {
        let AndPool { p1, p2, .. } = self;
        let mut deltas = p1.process(input_id, &mut sensor.s1, complexity);
        deltas.extend(p2.process(input_id, &mut sensor.s2, complexity));
        deltas
    }
}

impl<P1, P2> CompatibleWithIteratorSensor for AndPool<P1, P2>
where
    P1: CompatibleWithIteratorSensor,
    P2: CompatibleWithIteratorSensor<Observation = P1::Observation>,
{
    type Observation = P1::Observation;
    type ObservationState = (P1::ObservationState, P2::ObservationState);

    #[no_coverage]
    fn start_observing(&mut self) -> Self::ObservationState {
        (self.p1.start_observing(), self.p2.start_observing())
    }

    #[no_coverage]
    fn observe(&mut self, observation: &Self::Observation, input_complexity: f64, state: &mut Self::ObservationState) {
        self.p1.observe(observation, input_complexity, &mut state.0);
        self.p2.observe(observation, input_complexity, &mut state.1);
    }
    #[no_coverage]
    fn is_interesting(&self, observation_state: &Self::ObservationState, input_complexity: f64) -> bool {
        self.p1.is_interesting(&observation_state.0, input_complexity)
            || self.p2.is_interesting(&observation_state.1, input_complexity)
    }
    #[no_coverage]
    fn finish_observing(&mut self, state: &mut Self::ObservationState, input_complexity: f64) {
        self.p1.finish_observing(&mut state.0, input_complexity);
        self.p2.finish_observing(&mut state.1, input_complexity);
    }
    #[no_coverage]
    fn add(
        &mut self,
        input_id: PoolStorageIndex,
        complexity: f64,
        observation_state: Self::ObservationState,
    ) -> Vec<CorpusDelta> {
        let (o1, o2) = observation_state;
        let mut deltas = vec![];
        if self.p1.is_interesting(&o1, complexity) {
            deltas.extend(self.p1.add(input_id, complexity, o1));
        }
        if self.p2.is_interesting(&o2, complexity) {
            deltas.extend(self.p2.add(input_id, complexity, o2));
        }
        deltas
    }
}
impl<S1, S2> ToCSV for AndStats<S1, S2>
where
    S1: Display,
    S2: Display,
    S1: ToCSV,
    S2: ToCSV,
{
    #[no_coverage]
    fn csv_headers(&self) -> Vec<CSVField> {
        let mut h = self.stats1.csv_headers();
        h.extend(self.stats2.csv_headers());
        h
    }

    #[no_coverage]
    fn to_csv_record(&self) -> Vec<CSVField> {
        let mut h = self.stats1.to_csv_record();
        h.extend(self.stats2.to_csv_record());
        h
    }
}
