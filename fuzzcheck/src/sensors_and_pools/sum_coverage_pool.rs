use std::{fmt::Display, marker::PhantomData, path::PathBuf};

use crate::{
    fuzzer::PoolStorageIndex,
    traits::{CorpusDelta, Pool},
    CSVField, ToCSVFields,
};

use super::compatible_with_iterator_sensor::CompatibleWithIteratorSensor;

pub struct SumCounterValues;
pub struct CountNumberOfDifferentCounters;

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct Unit;

pub struct Input {
    input_idx: PoolStorageIndex,
    complexity: f64,
}
pub struct AggregateCoveragePool<Strategy> {
    name: String,
    current_best: Option<(u64, Input)>,
    current_best_dead_end: bool,
    _phantom: PhantomData<Strategy>,
}
#[derive(Clone)]
pub struct Stats {
    name: String,
    best: u64,
}
impl Display for Stats {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}({})", self.name, self.best)
    }
}
impl ToCSVFields for Stats {
    fn csv_headers(&self) -> Vec<CSVField> {
        vec![CSVField::String(self.name.clone())]
    }

    fn to_csv_record(&self) -> Vec<CSVField> {
        vec![CSVField::Integer(self.best as isize)]
    }
}
impl<Strategy> AggregateCoveragePool<Strategy> {
    #[no_coverage]
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            current_best: None,
            current_best_dead_end: false,
            _phantom: PhantomData,
        }
    }
}
impl<Strategy> Pool for AggregateCoveragePool<Strategy> {
    type Stats = Stats;
    #[no_coverage]
    fn len(&self) -> usize {
        1
    }
    #[no_coverage]
    fn stats(&self) -> Self::Stats {
        Stats {
            name: self.name.clone(),
            best: self.current_best.as_ref().map(|z| z.0).unwrap_or(0),
        }
    }
    #[no_coverage]
    fn get_random_index(&mut self) -> Option<PoolStorageIndex> {
        if let Some(best) = &self.current_best {
            if !self.current_best_dead_end {
                return Some(best.1.input_idx);
            }
        }
        None
    }
    #[no_coverage]
    fn mark_test_case_as_dead_end(&mut self, _idx: PoolStorageIndex) {
        self.current_best_dead_end = true;
    }

    #[no_coverage]
    fn serialized(&self) -> Vec<(PathBuf, Vec<u8>)> {
        vec![]
    }
}
impl CompatibleWithIteratorSensor for AggregateCoveragePool<SumCounterValues> {
    type Observation = (usize, u64);
    type ObservationState = u64;

    #[no_coverage]
    fn start_observing(&mut self) -> Self::ObservationState {
        <_>::default()
    }

    #[no_coverage]
    fn observe(&mut self, observation: &Self::Observation, _input_complexity: f64, state: &mut Self::ObservationState) {
        *state += observation.1;
    }
    #[no_coverage]
    fn finish_observing(&mut self, _state: &mut Self::ObservationState, _input_complexity: f64) {}
    #[no_coverage]
    fn is_interesting(&self, observation_state: &Self::ObservationState, input_complexity: f64) -> bool {
        if let Some((counter, cur_input)) = &self.current_best {
            if *observation_state > *counter {
                true
            } else if *observation_state == *counter && cur_input.complexity > input_complexity {
                true
            } else {
                false
            }
        } else {
            true
        }
    }
    #[no_coverage]
    fn add(
        &mut self,
        input_idx: PoolStorageIndex,
        complexity: f64,
        observation_state: Self::ObservationState,
    ) -> Vec<CorpusDelta> {
        let delta = CorpusDelta {
            path: PathBuf::new().join(&self.name),
            add: true,
            remove: if let Some(best) = &self.current_best {
                vec![best.1.input_idx]
            } else {
                vec![]
            },
        };
        let new = Input { input_idx, complexity };
        self.current_best = Some((observation_state, new));
        vec![delta]
    }
}

impl CompatibleWithIteratorSensor for AggregateCoveragePool<CountNumberOfDifferentCounters> {
    type Observation = (usize, u64);
    type ObservationState = u64;

    #[no_coverage]
    fn start_observing(&mut self) -> Self::ObservationState {
        <_>::default()
    }

    #[no_coverage]
    fn observe(
        &mut self,
        _observation: &Self::Observation,
        _input_complexity: f64,
        state: &mut Self::ObservationState,
    ) {
        *state += 1;
    }
    #[no_coverage]
    fn finish_observing(&mut self, _state: &mut Self::ObservationState, _input_complexity: f64) {}
    #[no_coverage]
    fn is_interesting(&self, observation_state: &Self::ObservationState, input_complexity: f64) -> bool {
        if let Some((counter, cur_input)) = &self.current_best {
            if *observation_state > *counter {
                true
            } else if *observation_state == *counter && cur_input.complexity > input_complexity {
                true
            } else {
                false
            }
        } else {
            true
        }
    }
    #[no_coverage]
    fn add(
        &mut self,
        input_idx: PoolStorageIndex,
        complexity: f64,
        observation_state: Self::ObservationState,
    ) -> Vec<CorpusDelta> {
        let delta = CorpusDelta {
            path: PathBuf::new().join(&self.name),
            add: true,
            remove: if let Some(best) = &self.current_best {
                vec![best.1.input_idx]
            } else {
                vec![]
            },
        };
        let new = Input { input_idx, complexity };
        self.current_best = Some((observation_state, new));
        vec![delta]
    }
}
