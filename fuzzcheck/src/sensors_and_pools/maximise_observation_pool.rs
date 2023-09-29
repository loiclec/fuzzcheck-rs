use std::fmt::{Debug, Display};
use std::path::PathBuf;

use crate::traits::{CorpusDelta, Pool, SaveToStatsFolder, Stats};
use crate::{CSVField, CompatibleWithObservations, PoolStorageIndex, ToCSV};

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
struct Unit;

struct Input {
    input_id: PoolStorageIndex,
    complexity: f64,
}

/// A pool that finds a single test case maximising a value given by a sensor.
pub struct MaximiseObservationPool<T> {
    name: String,
    current_best: Option<(T, Input)>,
}
#[derive(Clone)]
pub struct MaximiseObservationPoolStats<T> {
    name: String,
    best: T,
}
impl<T> Display for MaximiseObservationPoolStats<T>
where
    T: Debug,
{
    #[coverage(off)]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}({:?})", self.name, self.best)
    }
}
impl<T> ToCSV for MaximiseObservationPoolStats<T>
where
    T: Debug,
{
    #[coverage(off)]
    fn csv_headers(&self) -> Vec<CSVField> {
        vec![CSVField::String(self.name.clone())]
    }
    #[coverage(off)]
    fn to_csv_record(&self) -> Vec<CSVField> {
        vec![CSVField::String(format!("{:?}", self.best))]
    }
}
impl<T> Stats for MaximiseObservationPoolStats<T> where T: Debug + Default + 'static {}

impl<T> MaximiseObservationPool<T> {
    #[coverage(off)]
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            current_best: None,
        }
    }
}
impl<T> Pool for MaximiseObservationPool<T>
where
    T: Clone + Debug + Default + 'static,
{
    type Stats = MaximiseObservationPoolStats<T>;

    #[coverage(off)]
    fn stats(&self) -> Self::Stats {
        MaximiseObservationPoolStats {
            name: self.name.clone(),
            best: self
                .current_best
                .as_ref()
                .map(
                    #[coverage(off)]
                    |z| z.0.clone(),
                )
                .unwrap_or_default(),
        }
    }
    #[coverage(off)]
    fn get_random_index(&mut self) -> Option<PoolStorageIndex> {
        if let Some(best) = &self.current_best {
            Some(best.1.input_id)
        } else {
            None
        }
    }
}
impl<T> SaveToStatsFolder for MaximiseObservationPool<T> {
    #[coverage(off)]
    fn save_to_stats_folder(&self) -> Vec<(PathBuf, Vec<u8>)> {
        vec![]
    }
}

impl<T> CompatibleWithObservations<T> for MaximiseObservationPool<T>
where
    T: Clone + Debug + Default + PartialOrd + 'static,
{
    #[coverage(off)]
    fn process(&mut self, input_id: PoolStorageIndex, observations: &T, complexity: f64) -> Vec<CorpusDelta> {
        let observations = observations.clone();
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
