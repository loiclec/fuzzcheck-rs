use crate::traits::{Observations, Stats};
use crate::CompatibleWithObservations;
use crate::{
    fuzzer::PoolStorageIndex,
    traits::{CorpusDelta, Pool, SaveToStatsFolder},
    CSVField, ToCSV,
};
use std::fmt::Display;
use std::{fmt::Debug, path::PathBuf};

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
struct Unit;

struct Input {
    input_id: PoolStorageIndex,
    complexity: f64,
}

/// A pool that finds a single test case maximising a value given by a sensor.
pub struct MaximiseSingleValuePool<T> {
    name: String,
    current_best: Option<(T, Input)>,
}
#[derive(Clone)]
pub struct OptimiseAggregateStatPoolStats<T> {
    name: String,
    best: Option<T>,
}
impl<T> Display for OptimiseAggregateStatPoolStats<T>
where
    T: Debug,
{
    #[no_coverage]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}({:?})", self.name, self.best)
    }
}
impl<T> ToCSV for OptimiseAggregateStatPoolStats<T>
where
    T: Debug,
{
    #[no_coverage]
    fn csv_headers(&self) -> Vec<CSVField> {
        vec![CSVField::String(self.name.clone())]
    }
    #[no_coverage]
    fn to_csv_record(&self) -> Vec<CSVField> {
        vec![CSVField::String(format!("{:?}", self.best))]
    }
}
impl<T> Stats for OptimiseAggregateStatPoolStats<T> where T: Debug + 'static {}

impl<T> MaximiseSingleValuePool<T> {
    #[no_coverage]
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            current_best: None,
        }
    }
}
impl<T> Pool for MaximiseSingleValuePool<T>
where
    T: Clone + Debug + 'static,
{
    type Stats = OptimiseAggregateStatPoolStats<T>;

    #[no_coverage]
    fn stats(&self) -> Self::Stats {
        OptimiseAggregateStatPoolStats {
            name: self.name.clone(),
            best: self.current_best.as_ref().map(
                #[no_coverage]
                |z| z.0.clone(),
            ),
        }
    }

    #[no_coverage]
    fn ranked_test_cases(&self) -> Vec<(PoolStorageIndex, f64)> {
        if let Some(best) = &self.current_best {
            vec![(best.1.input_id, 1.0)]
        } else {
            vec![]
        }
    }
}
impl<T> SaveToStatsFolder for MaximiseSingleValuePool<T> {
    #[no_coverage]
    fn save_to_stats_folder(&self) -> Vec<(PathBuf, Vec<u8>)> {
        vec![]
    }
}

impl<T, O> CompatibleWithObservations<O> for MaximiseSingleValuePool<T>
where
    O: for<'a> Observations<Concrete<'a> = T>,
    T: Clone + Debug + PartialOrd + 'static,
{
    fn process<'a>(
        &'a mut self,
        input_id: PoolStorageIndex,
        observations: O::Concrete<'a>,
        complexity: f64,
    ) -> Vec<CorpusDelta> {
        let is_interesting = if let Some((counter, cur_input)) = &self.current_best {
            observations > *counter || (observations == *counter && cur_input.complexity > complexity)
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
        self.current_best = Some((observations, new));
        vec![delta]
    }
}
