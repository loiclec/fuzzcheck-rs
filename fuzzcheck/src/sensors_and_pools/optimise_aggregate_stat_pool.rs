use crate::traits::Stats;
use crate::CompatibleWithObservations;
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
    input_id: PoolStorageIndex,
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
            Some(best.1.input_id)
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

impl<M> OptimiseAggregateStatPool<M> {
    fn add_if_value_is_maximal(&mut self, value: u64, complexity: f64, input_id: PoolStorageIndex) -> Vec<CorpusDelta> {
        let is_interesting = if let Some((counter, cur_input)) = &self.current_best {
            value > *counter || (value == *counter && cur_input.complexity > complexity)
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
                vec![best.1.input_id]
            } else {
                vec![]
            },
        };
        let new = Input { input_id, complexity };
        self.current_best = Some((value, new));
        vec![delta]
    }
}

impl<I> CompatibleWithObservations<I> for OptimiseAggregateStatPool<SumOfCounterValues>
where
    I: IntoIterator<Item = (usize, u64)>,
{
    fn process(&mut self, input_id: PoolStorageIndex, observations: I, complexity: f64) -> Vec<CorpusDelta> {
        let mut sum_counters = 0;
        for (_, counter) in observations.into_iter() {
            sum_counters += counter;
        }
        self.add_if_value_is_maximal(sum_counters, complexity, input_id)
    }
}
impl<I> CompatibleWithObservations<I> for OptimiseAggregateStatPool<NumberOfActivatedCounters>
where
    I: IntoIterator<Item = (usize, u64)>,
{
    fn process(&mut self, input_id: PoolStorageIndex, observations: I, complexity: f64) -> Vec<CorpusDelta> {
        let nbr_activated_counters = observations.into_iter().count();
        self.add_if_value_is_maximal(nbr_activated_counters as u64, complexity, input_id)
    }
}
