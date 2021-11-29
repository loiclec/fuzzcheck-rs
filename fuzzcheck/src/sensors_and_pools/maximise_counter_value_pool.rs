use crate::data_structures::{Slab, SlabKey};
use crate::fenwick_tree::FenwickTree;
use crate::fuzzer::PoolStorageIndex;
use crate::sensors_and_pools::compatible_with_iterator_sensor::CompatibleWithIteratorSensor;
use crate::traits::{CorpusDelta, Pool, SaveToStatsFolder, Stats};
use crate::{CSVField, ToCSV};
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
/// It is [compatible with](crate::CompatibleWithSensor) the following sensors:
/// * [`CodeCoverageSensor`](crate::sensors_and_pools::CodeCoverageSensor)
/// * [`ArrayOfCounters`](crate::sensors_and_pools::ArrayOfCounters)
/// * any other sensor whose [observation handler](crate::Sensor::ObservationHandler) is a `&'a mut dyn FnMut((usize, u64))`
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

impl CompatibleWithIteratorSensor for MaximiseCounterValuePool {
    type Observation = (usize, u64);
    type ObservationState = Vec<(usize, u64)>;

    #[no_coverage]
    fn start_observing(&mut self) -> Self::ObservationState {
        <_>::default()
    }

    #[no_coverage]
    fn observe(
        &mut self,
        &(index, counter): &Self::Observation,
        input_complexity: f64,
        state: &mut Self::ObservationState,
    ) {
        let pool_counter = self.highest_counts[index];
        if pool_counter < counter {
            state.push((index, counter));
        } else if pool_counter == counter {
            if let Some(candidate_key) = self.best_input_for_counter[index] {
                if self.inputs[candidate_key].cplx > input_complexity {
                    state.push((index, counter));
                }
            } else {
            }
        }
    }

    #[no_coverage]
    fn finish_observing(&mut self, _state: &mut Self::ObservationState, _input_complexity: f64) {}

    #[no_coverage]
    fn is_interesting(&self, observation_state: &Self::ObservationState, _input_complexity: f64) -> bool {
        !observation_state.is_empty()
    }

    #[no_coverage]
    fn add(
        &mut self,
        input_idx: PoolStorageIndex,
        complexity: f64,
        observation_state: Self::ObservationState,
    ) -> Vec<CorpusDelta> {
        let highest_for_counters = observation_state;
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
            idx: input_idx,
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
    use crate::sensors_and_pools::compatible_with_iterator_sensor::CompatibleWithIteratorSensor;
    use crate::traits::Pool;

    #[test]
    fn test_basic_pool_1() {
        let mut pool = MaximiseCounterValuePool::new("a", 5);
        println!("{:?}", pool);
        let index = pool.get_random_index();
        println!("{:?}", index);

        println!("event: {:?}", pool.add(PoolStorageIndex::mock(0), 1.21, vec![(1, 2)]));
        println!("pool: {:?}", pool);
        let index = pool.get_random_index();
        println!("{:?}", index);

        // replace
        println!("event: {:?}", pool.add(PoolStorageIndex::mock(0), 1.11, vec![(1, 2)]));

        println!("pool: {:?}", pool);
        let index = pool.get_random_index();
        println!("{:?}", index);
    }

    #[test]
    fn test_basic_pool_2() {
        let mut pool = MaximiseCounterValuePool::new("b", 5);

        let _ = pool.add(PoolStorageIndex::mock(0), 1.21, vec![(1, 4)]);
        let _ = pool.add(PoolStorageIndex::mock(1), 2.21, vec![(2, 2)]);
        println!("event: {:?}", pool.add(PoolStorageIndex::mock(2), 3.21, vec![(3, 2)]));
        println!("pool: {:?}", pool);
        let index = pool.get_random_index();
        println!("{:?}", index);

        // replace
        println!(
            "event: {:?}",
            pool.add(PoolStorageIndex::mock(3), 1.11, vec![(2, 3), (3, 3)])
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
            pool.add(PoolStorageIndex::mock(5), 4.41, vec![(0, 3), (3, 4), (4, 1)])
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
            pool.add(
                PoolStorageIndex::mock(6),
                0.11,
                vec![(0, 3), (3, 4), (4, 1), (1, 7), (2, 8)]
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
        println!("event: {:?}", pool.add(PoolStorageIndex::mock(7), 1.51, vec![(0, 10)]));
        println!("pool: {:?}", pool);

        let mut map = HashMap::new();
        for _ in 0..10000 {
            let index = pool.get_random_index().unwrap();
            *map.entry(index).or_insert(0) += 1;
        }
        println!("{:?}", map);
    }
}
