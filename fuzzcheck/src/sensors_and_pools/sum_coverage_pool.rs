use std::{marker::PhantomData, path::PathBuf};

use crate::traits::{CorpusDelta, EmptyStats, Pool, TestCase};

use super::compatible_with_iterator_sensor::CompatibleWithIteratorSensor;

pub struct SumCounterValues;
pub struct CountNumberOfDifferentCounters;

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct Unit;

#[derive(Clone)]
pub struct Input<T: Clone> {
    data: T,
    complexity: f64,
}
pub struct AggregateCoveragePool<T: Clone, Strategy> {
    name: String,
    current_best: Option<(u64, Input<T>)>,
    current_best_dead_end: bool,
    _phantom: PhantomData<Strategy>,
}
impl<T: TestCase, Strategy> AggregateCoveragePool<T, Strategy> {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            current_best: None,
            current_best_dead_end: false,
            _phantom: PhantomData,
        }
    }
}
impl<T: TestCase, Strategy> Pool for AggregateCoveragePool<T, Strategy> {
    type TestCase = T;
    type Index = Unit;
    type Stats = EmptyStats;

    fn len(&self) -> usize {
        1
    }

    fn stats(&self) -> Self::Stats {
        EmptyStats
    }

    fn get_random_index(&mut self) -> Option<Self::Index> {
        if self.current_best.is_some() && !self.current_best_dead_end {
            Some(Unit)
        } else {
            None
        }
    }

    fn get(&self, _idx: Self::Index) -> &Self::TestCase {
        &self.current_best.as_ref().unwrap().1.data
    }

    fn get_mut(&mut self, _idx: Self::Index) -> &mut Self::TestCase {
        &mut self.current_best.as_mut().unwrap().1.data
    }

    fn retrieve_after_processing(&mut self, _idx: Self::Index, generation: usize) -> Option<&mut Self::TestCase> {
        if self.current_best.as_ref().unwrap().1.data.generation() == generation {
            Some(&mut self.current_best.as_mut().unwrap().1.data)
        } else {
            None
        }
    }

    fn mark_test_case_as_dead_end(&mut self, _idx: Self::Index) {
        self.current_best_dead_end = true;
    }
    fn minify(&mut self, _target_len: usize, _event_handler: impl FnMut(CorpusDelta<&Self::TestCase, Self::Index>, Self::Stats) -> Result<(), std::io::Error>) -> Result<(), std::io::Error> {
        // nothing to do
        Ok(())
    }
}
impl<T: TestCase> CompatibleWithIteratorSensor for AggregateCoveragePool<T, SumCounterValues> {
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
        data: Self::TestCase,
        complexity: f64,
        observation_state: Self::ObservationState,
        mut event_handler: impl FnMut(
            crate::traits::CorpusDelta<&Self::TestCase, Self::Index>,
            Self::Stats,
        ) -> Result<(), std::io::Error>,
    ) -> Result<(), std::io::Error> {
        let delta = CorpusDelta {
            path: PathBuf::new().join(&self.name),
            add: Some((&data, Unit)),
            remove: if self.current_best.is_none() {
                vec![]
            } else {
                vec![Unit]
            },
        };
        event_handler(delta, self.stats())?;
        let new = Input { data, complexity };
        self.current_best = Some((observation_state, new));
        Ok(())
    }
}

impl<T: TestCase> CompatibleWithIteratorSensor for AggregateCoveragePool<T, CountNumberOfDifferentCounters> {
    type Observation = (usize, u64);
    type ObservationState = u64;

    fn observe(&mut self, _observation: &Self::Observation, _input_complexity: f64, state: &mut Self::ObservationState) {
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
        data: Self::TestCase,
        complexity: f64,
        observation_state: Self::ObservationState,
        mut event_handler: impl FnMut(
            crate::traits::CorpusDelta<&Self::TestCase, Self::Index>,
            Self::Stats,
        ) -> Result<(), std::io::Error>,
    ) -> Result<(), std::io::Error> {
        let delta = CorpusDelta {
            path: PathBuf::new().join(&self.name),
            add: Some((&data, Unit)),
            remove: if self.current_best.is_none() {
                vec![]
            } else {
                vec![Unit]
            },
        };
        event_handler(delta, self.stats())?;
        let new = Input { data, complexity };
        self.current_best = Some((observation_state, new));
        Ok(())
    }
}
