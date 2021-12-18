use crate::code_coverage_sensor::CopiedSliceIterObservations;
use crate::data_structures::{Slab, SlabKey};
use crate::fenwick_tree::FenwickTree;
use crate::fuzzer::PoolStorageIndex;
// use crate::sensors_and_pools::compatible_with_iterator_sensor::CompatibleWithIteratorSensor;
use crate::traits::{CorpusDelta, Observations, Pool, SaveToStatsFolder, Stats};
use crate::{CSVField, CompatibleWithObservations, ToCSV};
use ahash::AHashSet;
use nu_ansi_term::Color;
use std::fmt::{Debug, Display};
use std::path::Path;

/// The statistics of a [MaximiseCounterValuePool]
#[derive(Clone)]
pub struct MaximiseCounterValuePoolStats {
    name: String,
    size: usize,
    total_counts: u64,
}

impl Display for MaximiseCounterValuePoolStats {
    #[no_coverage]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            Color::LightPurple.paint(format!("{}({} sum: {})", self.name, self.size, self.total_counts))
        )
    }
}

impl ToCSV for MaximiseCounterValuePoolStats {
    #[no_coverage]
    fn csv_headers(&self) -> Vec<CSVField> {
        vec![
            CSVField::String(format!("{}-count", self.name)),
            CSVField::String(format!("{}-sum", self.name)),
        ]
    }
    #[no_coverage]
    fn to_csv_record(&self) -> Vec<CSVField> {
        vec![
            CSVField::Integer(self.size as isize),
            CSVField::Integer(self.total_counts as isize),
        ]
    }
}
impl Stats for MaximiseCounterValuePoolStats {}

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
/// * [`ArrayOfCounters`](crate::sensors_and_pools::ArrayOfCounters)
/// * any other sensor whose [observations](crate::Sensor::Observations) are given by an iterator of `(usize, u64)`
pub struct MaximiseCounterValuePool {
    name: String,
    complexities: Vec<f64>,
    highest_counts: Vec<u64>,
    inputs: Slab<Input>,
    best_input_for_counter: Vec<Option<SlabKey<Input>>>,
    ranked_inputs: FenwickTree,
    stats: MaximiseCounterValuePoolStats,
    rng: fastrand::Rng,
}
impl Debug for MaximiseCounterValuePool {
    #[no_coverage]
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

impl MaximiseCounterValuePool {
    #[no_coverage]
    pub fn new(name: &str, size: usize) -> Self {
        Self {
            name: name.to_string(),
            complexities: vec![0.0; size],
            highest_counts: vec![0; size],
            inputs: Slab::new(),
            best_input_for_counter: vec![None; size],
            ranked_inputs: FenwickTree::new(vec![]),
            stats: MaximiseCounterValuePoolStats {
                name: name.to_string(),
                size: 0,
                total_counts: 0,
            },
            rng: fastrand::Rng::new(),
        }
    }
}

impl Pool for MaximiseCounterValuePool {
    type Stats = MaximiseCounterValuePoolStats;

    #[no_coverage]
    fn stats(&self) -> Self::Stats {
        self.stats.clone()
    }

    #[no_coverage]
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

impl SaveToStatsFolder for MaximiseCounterValuePool {
    #[no_coverage]
    fn save_to_stats_folder(&self) -> Vec<(std::path::PathBuf, Vec<u8>)> {
        vec![]
    }
}

impl MaximiseCounterValuePool {
    #[no_coverage]
    fn update_stats(&mut self) {
        let inputs = &self.inputs;
        let ranked_inputs = self
            .inputs
            .keys()
            .map(
                #[no_coverage]
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

impl CompatibleWithObservations<CopiedSliceIterObservations<(usize, u64)>> for MaximiseCounterValuePool {
    fn process<'a>(
        &'a mut self,
        input_id: PoolStorageIndex,
        observations: <CopiedSliceIterObservations<(usize, u64)> as Observations>::Concrete<'a>,
        complexity: f64,
    ) -> Vec<CorpusDelta> {
        let mut state = vec![];
        for (index, counter) in observations.into_iter() {
            let pool_counter = self.highest_counts[index];
            if pool_counter < counter {
                state.push((index, counter));
            } else if pool_counter == counter {
                if let Some(candidate_key) = self.best_input_for_counter[index] {
                    if self.inputs[candidate_key].cplx > complexity {
                        state.push((index, counter));
                    }
                }
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
                    #[no_coverage]
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

    use super::MaximiseCounterValuePool;
    use crate::fuzzer::PoolStorageIndex;
    // use crate::sensors_and_pools::IterObservations;
    // use crate::sensors_and_pools::compatible_with_iterator_sensor::CompatibleWithIteratorSensor;
    use crate::traits::CompatibleWithObservations;
    use crate::traits::Pool;

    #[test]
    fn test_basic_pool_1() {
        let mut pool = MaximiseCounterValuePool::new("a", 5);
        println!("{:?}", pool);
        let index = pool.get_random_index();
        println!("{:?}", index);

        println!(
            "event: {:?}",
            pool.process(PoolStorageIndex::mock(0), [(1, 2)].iter().copied(), 1.21)
        );
        println!("pool: {:?}", pool);
        let index = pool.get_random_index();
        println!("{:?}", index);

        // replace
        println!(
            "event: {:?}",
            pool.process(PoolStorageIndex::mock(0), [(1, 2)].iter().copied(), 1.11)
        );

        println!("pool: {:?}", pool);
        let index = pool.get_random_index();
        println!("{:?}", index);
    }

    #[test]
    fn test_basic_pool_2() {
        let mut pool = MaximiseCounterValuePool::new("b", 5);

        let _ = pool.process(PoolStorageIndex::mock(0), [(1, 4)].iter().copied(), 1.21);
        let _ = pool.process(PoolStorageIndex::mock(1), [(2, 2)].iter().copied(), 2.21);
        println!(
            "event: {:?}",
            pool.process(PoolStorageIndex::mock(2), [(3, 2)].iter().copied(), 3.21)
        );
        println!("pool: {:?}", pool);
        let index = pool.get_random_index();
        println!("{:?}", index);

        // replace
        println!(
            "event: {:?}",
            pool.process(PoolStorageIndex::mock(3), [(2, 3), (3, 3)].iter().copied(), 1.11)
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
            pool.process(
                PoolStorageIndex::mock(5),
                [(0, 3), (3, 4), (4, 1)].iter().copied(),
                4.41
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
        println!(
            "event: {:?}",
            pool.process(
                PoolStorageIndex::mock(6),
                [(0, 3), (3, 4), (4, 1), (1, 7), (2, 8)].iter().copied(),
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
        println!(
            "event: {:?}",
            pool.process(PoolStorageIndex::mock(7), [(0, 10)].iter().copied(), 1.51)
        );
        println!("pool: {:?}", pool);

        let mut map = HashMap::new();
        for _ in 0..10000 {
            let index = pool.get_random_index().unwrap();
            *map.entry(index).or_insert(0) += 1;
        }
        println!("{:?}", map);
    }
}
