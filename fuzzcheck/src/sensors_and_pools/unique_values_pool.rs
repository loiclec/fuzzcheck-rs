use crate::data_structures::{Slab, SlabKey};
use crate::fenwick_tree::FenwickTree;
use crate::fuzzer::PoolStorageIndex;
use crate::sensors_and_pools::compatible_with_iterator_sensor::CompatibleWithIteratorSensor;
use crate::traits::{CorpusDelta, Pool, SaveToStatsFolder, Stats};
use crate::ToCSV;
use ahash::{AHashMap, AHashSet};
use std::fmt::{Debug, Display};
use std::path::Path;

#[derive(Clone, Default)]
pub struct UniqueValuesPoolStats {
    pub name: String,
    pub size: usize,
}
impl ToCSV for UniqueValuesPoolStats {
    fn csv_headers(&self) -> Vec<crate::CSVField> {
        vec![]
    }

    fn to_csv_record(&self) -> Vec<crate::CSVField> {
        vec![]
    }
}

impl Display for UniqueValuesPoolStats {
    #[no_coverage]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            nu_ansi_term::Color::Blue.paint(format!("{}({})", self.name, self.size))
        )
    }
}
impl Stats for UniqueValuesPoolStats {}

#[derive(Debug)]
struct Input {
    best_for_values: AHashSet<(usize, u64)>,
    data: PoolStorageIndex,
    score: f64,
    number_times_chosen: usize,
}

/// A pool that stores an input for each different value of each sensor counter
pub struct UniqueValuesPool {
    name: String,
    complexities: Vec<AHashMap<u64, f64>>,
    inputs: Slab<Input>,
    best_input_for_value: Vec<AHashMap<u64, SlabKey<Input>>>,
    ranked_inputs: FenwickTree,
    stats: UniqueValuesPoolStats,
    rng: fastrand::Rng,
}
impl Debug for UniqueValuesPool {
    #[no_coverage]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("UniqueValuesPool")
            .field("complexities", &self.complexities)
            // .field("highest_counts", &self.highest_counts)
            .field("inputs", &self.inputs)
            // .field("best_input_for_counter", &self.best_input_for_counter)
            // .field("cumulative_score_inputs", &self.ranked_inputs)
            .finish()
    }
}

impl UniqueValuesPool {
    #[no_coverage]
    pub fn new(name: &str, size: usize) -> Self {
        Self {
            name: name.to_string(),
            complexities: vec![AHashMap::new(); size],
            inputs: Slab::new(),
            best_input_for_value: vec![AHashMap::new(); size],
            ranked_inputs: FenwickTree::new(vec![]),
            stats: UniqueValuesPoolStats {
                name: name.to_string(),
                size: 0,
            },
            rng: fastrand::Rng::new(),
        }
    }
}

impl Pool for UniqueValuesPool {
    type Stats = UniqueValuesPoolStats;

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
        let data = self.inputs[key].data;
        Some(data)
    }
}
impl SaveToStatsFolder for UniqueValuesPool {
    #[no_coverage]
    fn save_to_stats_folder(&self) -> Vec<(std::path::PathBuf, Vec<u8>)> {
        vec![]
    }
}

impl UniqueValuesPool {
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
    }
}

impl CompatibleWithIteratorSensor for UniqueValuesPool {
    type Observation = (usize, u64);
    type ObservationState = Vec<(usize, u64)>;

    #[no_coverage]
    fn start_observing(&mut self) -> Self::ObservationState {
        vec![]
    }
    #[no_coverage]
    fn observe(
        &mut self,
        &(index, counter): &Self::Observation,
        input_complexity: f64,
        state: &mut Self::ObservationState,
    ) {
        if let Some(&previous_cplx) = self.complexities[index].get(&counter) {
            if previous_cplx > input_complexity {
                // already exists but this one is better
                state.push((index, counter));
            }
        } else {
            state.push((index, counter));
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
        data: PoolStorageIndex,
        complexity: f64,
        observation_state: Self::ObservationState,
    ) -> Vec<CorpusDelta> {
        let new_observations = observation_state;
        let cplx = complexity;
        let input = data;
        let input = Input {
            best_for_values: new_observations.iter().copied().collect(),
            data: input,
            score: new_observations.len() as f64,
            number_times_chosen: 1,
        };
        let input_key = self.inputs.insert(input);

        let mut removed_keys = vec![];

        for &(counter, id) in &new_observations {
            self.complexities[counter].insert(id, cplx);

            let previous_best_key = self.best_input_for_value[counter].get_mut(&id);
            if let Some(previous_best_key) = previous_best_key {
                let previous_best = &mut self.inputs[*previous_best_key];
                let was_present_in_set = previous_best.best_for_values.remove(&(counter, id));
                assert!(was_present_in_set);
                previous_best.score = previous_best.best_for_values.len() as f64;
                if previous_best.best_for_values.is_empty() {
                    removed_keys.push(*previous_best_key);
                }
                *previous_best_key = input_key;
            } else {
                self.best_input_for_value[counter].insert(id, input_key);
            }
        }
        for &removed_key in &removed_keys {
            self.inputs.remove(removed_key);
        }
        let removed_keys = removed_keys.into_iter().map(|k| self.inputs[k].data).collect();
        self.update_stats();
        return vec![CorpusDelta {
            path: Path::new(&self.name).to_path_buf(),
            add: true,
            remove: removed_keys,
        }];
    }
}
