use std::{marker::PhantomData, path::PathBuf};

use crate::{
    fuzzer::PoolStorageIndex,
    traits::{CorpusDelta, EmptyStats, Pool},
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
impl<Strategy> AggregateCoveragePool<Strategy> {
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
    type Stats = EmptyStats;

    fn len(&self) -> usize {
        1
    }

    fn stats(&self) -> Self::Stats {
        EmptyStats
    }

    fn get_random_index(&mut self) -> Option<PoolStorageIndex> {
        if let Some(best) = &self.current_best {
            if !self.current_best_dead_end {
                return Some(best.1.input_idx);
            }
        }
        None
    }

    fn mark_test_case_as_dead_end(&mut self, _idx: PoolStorageIndex) {
        self.current_best_dead_end = true;
    }
    fn minify(
        &mut self,
        _target_len: usize,
        _event_handler: impl FnMut(CorpusDelta, Self::Stats) -> Result<(), std::io::Error>,
    ) -> Result<(), std::io::Error> {
        // nothing to do
        Ok(())
    }
}
impl CompatibleWithIteratorSensor for AggregateCoveragePool<SumCounterValues> {
    type Observation = (usize, u64);
    type ObservationState = u64;

    fn observe(&mut self, observation: &Self::Observation, _input_complexity: f64, state: &mut Self::ObservationState) {
        *state += observation.1;
    }

    fn finish_observing(&mut self, _state: &mut Self::ObservationState, _input_complexity: f64) {}

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

    fn observe(
        &mut self,
        _observation: &Self::Observation,
        _input_complexity: f64,
        state: &mut Self::ObservationState,
    ) {
        *state += 1;
    }

    fn finish_observing(&mut self, _state: &mut Self::ObservationState, _input_complexity: f64) {}

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
