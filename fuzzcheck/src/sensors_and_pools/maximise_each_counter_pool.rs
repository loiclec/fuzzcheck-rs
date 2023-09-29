use std::fmt::{Debug, Display};
use std::path::Path;

use ahash::AHashSet;
use nu_ansi_term::Color;

use crate::data_structures::{Slab, SlabKey};
use crate::fenwick_tree::FenwickTree;
use crate::traits::{CorpusDelta, Pool, SaveToStatsFolder, Stats};
use crate::{CSVField, CompatibleWithObservations, PoolStorageIndex, ToCSV};

/// The statistics of a [MaximiseEachCounterPool]
#[derive(Clone)]
pub struct MaximiseEachCounterPoolStats {
    name: String,
    size: usize,
    total_counts: u64,
}

impl Display for MaximiseEachCounterPoolStats {
    #[coverage(off)]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            Color::LightPurple.paint(format!("{}({} sum: {})", self.name, self.size, self.total_counts))
        )
    }
}

impl ToCSV for MaximiseEachCounterPoolStats {
    #[coverage(off)]
    fn csv_headers(&self) -> Vec<CSVField> {
        vec![
            CSVField::String(format!("{}-count", self.name)),
            CSVField::String(format!("{}-sum", self.name)),
        ]
    }
    #[coverage(off)]
    fn to_csv_record(&self) -> Vec<CSVField> {
        vec![
            CSVField::Integer(self.size as isize),
            CSVField::Integer(self.total_counts as isize),
        ]
    }
}
impl Stats for MaximiseEachCounterPoolStats {}

#[derive(Debug)]
struct Input {
    best_for_counters: AHashSet<usize>,
    cplx: f64,
    idx: PoolStorageIndex,
    score: f64,
    number_times_chosen: usize,
}

/// A pool that tries to find test cases maximizing the value of each counter of a sensor.
///
/// It is [compatible with](crate::CompatibleWithObservations) the following sensors:
/// * [`CodeCoverageSensor`](crate::sensors_and_pools::CodeCoverageSensor)
/// * any other sensor whose [observations](crate::Sensor::Observations) are given by an iterator of `(usize, u64)`
pub struct MaximiseEachCounterPool {
    name: String,
    complexities: Vec<f64>,
    highest_counts: Vec<u64>,
    inputs: Slab<Input>,
    best_input_for_counter: Vec<Option<SlabKey<Input>>>,
    ranked_inputs: FenwickTree,
    stats: MaximiseEachCounterPoolStats,
    rng: fastrand::Rng,
}
impl Debug for MaximiseEachCounterPool {
    #[coverage(off)]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CounterMaximizingPool")
            .field("complexities", &self.complexities)
            .field("highest_counts", &self.highest_counts)
            .field("inputs", &self.inputs)
            .field("best_input_for_counter", &self.best_input_for_counter)
            // .field("cumulative_score_inputs", &self.cumulative_score_inputs)
            .finish()
    }
}

impl MaximiseEachCounterPool {
    #[coverage(off)]
    pub fn new(name: &str, size: usize) -> Self {
        Self {
            name: name.to_string(),
            complexities: vec![0.0; size],
            highest_counts: vec![0; size],
            inputs: Slab::new(),
            best_input_for_counter: vec![None; size],
            ranked_inputs: FenwickTree::new(vec![]),
            stats: MaximiseEachCounterPoolStats {
                name: name.to_string(),
                size: 0,
                total_counts: 0,
            },
            rng: fastrand::Rng::new(),
        }
    }
}

impl Pool for MaximiseEachCounterPool {
    type Stats = MaximiseEachCounterPoolStats;

    #[coverage(off)]
    fn stats(&self) -> Self::Stats {
        self.stats.clone()
    }

    #[coverage(off)]
    fn get_random_index(&mut self) -> Option<PoolStorageIndex> {
        let choice = self.ranked_inputs.sample(&self.rng)?;

        let key = self.inputs.get_nth_key(choice);

        let input = &mut self.inputs[key];
        let old_rank = input.score / (input.number_times_chosen as f64);
        input.number_times_chosen += 1;
        let new_rank = input.score / (input.number_times_chosen as f64);

        let delta = new_rank - old_rank;
        self.ranked_inputs.update(choice, delta);
        Some(input.idx)
    }
}

impl SaveToStatsFolder for MaximiseEachCounterPool {
    #[coverage(off)]
    fn save_to_stats_folder(&self) -> Vec<(std::path::PathBuf, Vec<u8>)> {
        vec![]
    }
}

impl MaximiseEachCounterPool {
    #[coverage(off)]
    fn update_stats(&mut self) {
        let inputs = &self.inputs;
        let ranked_inputs = self
            .inputs
            .keys()
            .map(
                #[coverage(off)]
                |key| {
                    let input = &inputs[key];
                    input.score / (input.number_times_chosen as f64)
                },
            )
            .collect();
        self.ranked_inputs = FenwickTree::new(ranked_inputs);

        self.stats.size = self.inputs.len();
        self.stats.total_counts = self.highest_counts.iter().sum();
    }
}

impl<O> CompatibleWithObservations<O> for MaximiseEachCounterPool
where
    for<'a> &'a O: IntoIterator<Item = &'a (usize, u64)>,
{
    fn process(&mut self, input_id: PoolStorageIndex, observations: &O, complexity: f64) -> Vec<CorpusDelta> {
        let mut state = vec![];
        for &(index, counter) in observations.into_iter() {
            let pool_counter = self.highest_counts[index];
            if pool_counter < counter {
                state.push((index, counter));
            } else if pool_counter == counter
                && let Some(candidate_key) = self.best_input_for_counter[index]
                && self.inputs[candidate_key].cplx > complexity {
                state.push((index, counter));
            }
        }
        if state.is_empty() {
            return vec![];
        }
        let highest_for_counters = state;
        let cplx = complexity;
        let input = Input {
            best_for_counters: highest_for_counters
                .iter()
                .map(
                    #[coverage(off)]
                    |x| x.0,
                )
                .collect(),
            cplx,
            idx: input_id,
            score: highest_for_counters.len() as f64,
            number_times_chosen: 1,
        };
        let input_key = self.inputs.insert(input);

        let mut removed_keys = vec![];

        for &(counter, intensity) in &highest_for_counters {
            assert!(
                self.highest_counts[counter] < intensity
                    || (self.highest_counts[counter] == intensity && self.complexities[counter] > cplx)
            );
            self.complexities[counter] = cplx;
            self.highest_counts[counter] = intensity;

            let previous_best_key = &mut self.best_input_for_counter[counter];
            if let Some(previous_best_key) = previous_best_key {
                let previous_best = &mut self.inputs[*previous_best_key];
                let was_present_in_set = previous_best.best_for_counters.remove(&counter);
                assert!(was_present_in_set);
                previous_best.score = previous_best.best_for_counters.len() as f64;
                if previous_best.best_for_counters.is_empty() {
                    removed_keys.push(*previous_best_key);
                }
                *previous_best_key = input_key;
            } else {
                *previous_best_key = Some(input_key);
            }
        }
        let mut removed_idxs = vec![];
        for &removed_key in &removed_keys {
            removed_idxs.push(self.inputs[removed_key].idx);
            self.inputs.remove(removed_key);
        }

        self.update_stats();

        vec![CorpusDelta {
            path: Path::new(&self.name).to_path_buf(),
            add: true,
            remove: removed_idxs,
        }]
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::MaximiseEachCounterPool;
    use crate::traits::{CompatibleWithObservations, Pool};
    use crate::PoolStorageIndex;

    #[test]
    fn test_basic_pool_1() {
        let mut pool = MaximiseEachCounterPool::new("a", 5);
        println!("{:?}", pool);
        let index = pool.get_random_index();
        println!("{:?}", index);

        println!("event: {:?}", pool.process(PoolStorageIndex::mock(0), &[(1, 2)], 1.21));
        println!("pool: {:?}", pool);
        let index = pool.get_random_index();
        println!("{:?}", index);

        // replace
        println!("event: {:?}", pool.process(PoolStorageIndex::mock(0), &[(1, 2)], 1.11));

        println!("pool: {:?}", pool);
        let index = pool.get_random_index();
        println!("{:?}", index);
    }

    #[test]
    fn test_basic_pool_2() {
        let mut pool = MaximiseEachCounterPool::new("b", 5);

        let _ = pool.process(PoolStorageIndex::mock(0), &[(1, 4)], 1.21);
        let _ = pool.process(PoolStorageIndex::mock(1), &[(2, 2)], 2.21);
        println!("event: {:?}", pool.process(PoolStorageIndex::mock(2), &[(3, 2)], 3.21));
        println!("pool: {:?}", pool);
        let index = pool.get_random_index();
        println!("{:?}", index);

        // replace
        println!(
            "event: {:?}",
            pool.process(PoolStorageIndex::mock(3), &[(2, 3), (3, 3)], 1.11)
        );
        println!("pool: {:?}", pool);

        let mut map = HashMap::new();
        for _ in 0..100 {
            let index = pool.get_random_index().unwrap();
            *map.entry(index).or_insert(0) += 1;
        }
        println!("{:?}", map);

        // replace
        println!(
            "event: {:?}",
            pool.process(PoolStorageIndex::mock(5), &[(0, 3), (3, 4), (4, 1)], 4.41)
        );
        println!("pool: {:?}", pool);

        let mut map = HashMap::new();
        for _ in 0..10000 {
            let index = pool.get_random_index().unwrap();
            *map.entry(index).or_insert(0) += 1;
        }
        println!("{:?}", map);

        // replace
        println!(
            "event: {:?}",
            pool.process(
                PoolStorageIndex::mock(6),
                &[(0, 3), (3, 4), (4, 1), (1, 7), (2, 8)],
                0.11,
            )
        );
        println!("pool: {:?}", pool);

        let mut map = HashMap::new();
        for _ in 0..10000 {
            let index = pool.get_random_index().unwrap();
            *map.entry(index).or_insert(0) += 1;
        }
        println!("{:?}", map);

        // replace
        println!("event: {:?}", pool.process(PoolStorageIndex::mock(7), &[(0, 10)], 1.51));
        println!("pool: {:?}", pool);

        let mut map = HashMap::new();
        for _ in 0..10000 {
            let index = pool.get_random_index().unwrap();
            *map.entry(index).or_insert(0) += 1;
        }
        println!("{:?}", map);
    }
}
