use super::compatible_with_iterator_sensor::CompatibleWithIteratorSensor;
use crate::traits::Stats;
use crate::{
    fuzzer::PoolStorageIndex,
    traits::{CorpusDelta, Pool, SaveToStatsFolder},
    CSVField, ToCSV,
};
use std::{fmt::Display, marker::PhantomData, path::PathBuf};

/// A strategy for [`OptimiseAggregateStatPool`] that maximises the total sum of all counters
pub struct SumOfCounterValues;
/// A strategy for [`OptimiseAggregateStatPool`] that maximises the number of counters that are != 0
pub struct NumberOfActivatedCounters;

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
struct Unit;

struct Input {
    input_idx: PoolStorageIndex,
    complexity: f64,
}

/// A pool that finds a single test case maximising some statistics computed from all of the sensor’s counters.
///
/// The statistics to optimise for is determined by the `Strategy` type parameter, which can be:
/// * [`SumOfCounterValues`] to maximise the total sum of all counters
/// * [`NumberOfActivatedCounters`] to maximise the number of counters that are != 0
///
/// Both strategies make the pool [compatible with](crate::CompatibleWithSensor) sensors whose
/// [observation handler](crate::Sensor::ObservationHandler) is `&'a mut dyn FnMut((usize, u64))`,
/// such as [`CodeCoverageSensor`](crate::sensors_and_pools::CodeCoverageSensor) and
/// [`ArrayOfCounters`](crate::sensors_and_pools::ArrayOfCounters).
pub struct OptimiseAggregateStatPool<Strategy> {
    name: String,
    current_best: Option<(u64, Input)>,
    _phantom: PhantomData<Strategy>,
}
#[derive(Clone)]
pub struct OptimiseAggregateStatPoolStats {
    name: String,
    best: u64,
}
impl Display for OptimiseAggregateStatPoolStats {
    #[no_coverage]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}({})", self.name, self.best)
    }
}
impl ToCSV for OptimiseAggregateStatPoolStats {
    #[no_coverage]
    fn csv_headers(&self) -> Vec<CSVField> {
        vec![CSVField::String(self.name.clone())]
    }
    #[no_coverage]
    fn to_csv_record(&self) -> Vec<CSVField> {
        vec![CSVField::Integer(self.best as isize)]
    }
}
impl Stats for OptimiseAggregateStatPoolStats {}

impl<Strategy> OptimiseAggregateStatPool<Strategy> {
    #[no_coverage]
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            current_best: None,
            _phantom: PhantomData,
        }
    }
}
impl<Strategy> Pool for OptimiseAggregateStatPool<Strategy> {
    type Stats = OptimiseAggregateStatPoolStats;

    #[no_coverage]
    fn stats(&self) -> Self::Stats {
        OptimiseAggregateStatPoolStats {
            name: self.name.clone(),
            best: self
                .current_best
                .as_ref()
                .map(
                    #[no_coverage]
                    |z| z.0,
                )
                .unwrap_or(0),
        }
    }
    #[no_coverage]
    fn get_random_index(&mut self) -> Option<PoolStorageIndex> {
        if let Some(best) = &self.current_best {
            Some(best.1.input_idx)
        } else {
            None
        }
    }
}
impl<T> SaveToStatsFolder for OptimiseAggregateStatPool<T> {
    #[no_coverage]
    fn save_to_stats_folder(&self) -> Vec<(PathBuf, Vec<u8>)> {
        vec![]
    }
}

impl CompatibleWithIteratorSensor for OptimiseAggregateStatPool<SumOfCounterValues> {
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
    fn add_if_interesting(
        &mut self,
        input_idx: PoolStorageIndex,
        complexity: f64,
        observation_state: Self::ObservationState,
    ) -> Vec<CorpusDelta> {
        let is_interesting = if let Some((counter, cur_input)) = &self.current_best {
            observation_state > *counter || (observation_state == *counter && cur_input.complexity > complexity)
        } else {
            true
        };
        if !is_interesting {
            return vec![];
        }

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

impl CompatibleWithIteratorSensor for OptimiseAggregateStatPool<NumberOfActivatedCounters> {
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
    fn add_if_interesting(
        &mut self,
        input_idx: PoolStorageIndex,
        complexity: f64,
        observation_state: Self::ObservationState,
    ) -> Vec<CorpusDelta> {
        let is_interesting = if let Some((counter, cur_input)) = &self.current_best {
            observation_state > *counter || (observation_state == *counter && cur_input.complexity > complexity)
        } else {
            true
        };
        if !is_interesting {
            return vec![];
        }
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
