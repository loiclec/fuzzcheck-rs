use crate::data_structures::{Slab, SlabKey};
use crate::fenwick_tree::FenwickTree;
use crate::sensors_and_pools::compatible_with_iterator_sensor::CompatibleWithIteratorSensor;
use crate::traits::{CorpusDelta, Pool, TestCase};
use ahash::{AHashMap, AHashSet};
use owo_colors::OwoColorize;
use std::fmt::{Debug, Display};
use std::ops::Range;
use std::path::Path;

#[derive(Clone, Default)]
pub struct Stats {
    name: String,
    size: usize,
}

impl Display for Stats {
    #[no_coverage]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", format!("{}({})", self.name, self.size).blue())
    }
}

#[derive(Debug)]
pub struct Input<T> {
    best_for_values: AHashSet<(usize, u64)>,
    data: T,
    score: f64,
    number_times_chosen: usize,
}

pub struct UniqueValuesPool<T> {
    name: String,
    complexities: Vec<AHashMap<u64, f64>>,
    inputs: Slab<Input<T>>,
    best_input_for_value: Vec<AHashMap<u64, SlabKey<Input<T>>>>,
    // also use a fenwick tree here?
    ranked_inputs: FenwickTree,
    stats: Stats,
    rng: fastrand::Rng,
}
impl<T: Debug> Debug for UniqueValuesPool<T> {
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

impl<T> UniqueValuesPool<T> {
    #[no_coverage]
    pub fn new(name: &str, size: usize) -> Self {
        Self {
            name: name.to_string(),
            complexities: vec![AHashMap::new(); size],
            inputs: Slab::new(),
            best_input_for_value: vec![AHashMap::new(); size],
            ranked_inputs: FenwickTree::new(vec![]),
            stats: Stats {
                name: name.to_string(),
                size: 0,
            },
            rng: fastrand::Rng::new(),
        }
    }
}

impl<T: TestCase> Pool for UniqueValuesPool<T> {
    type TestCase = T;
    type Index = SlabKey<Input<T>>;
    type Stats = Stats;

    #[no_coverage]
    fn stats(&self) -> Self::Stats {
        self.stats.clone()
    }

    #[no_coverage]
    fn len(&self) -> usize {
        self.inputs.len()
    }

    #[no_coverage]
    fn get_random_index(&mut self) -> Option<Self::Index> {
        if self.ranked_inputs.len() == 0 {
            return None;
        }
        let most = self.ranked_inputs.prefix_sum(self.ranked_inputs.len() - 1);
        if most <= 0.0 {
            return None;
        }
        let chosen_weight = gen_f64(&self.rng, 0.0..most);
        let choice = self.ranked_inputs.first_index_past_prefix_sum(chosen_weight);
        let key = self.inputs.get_nth_key(choice);

        let input = &mut self.inputs[key];
        let old_rank = input.score / (input.number_times_chosen as f64);
        input.number_times_chosen += 1;
        let new_rank = input.score / (input.number_times_chosen as f64);

        let delta = new_rank - old_rank;
        self.ranked_inputs.update(choice, delta);
        Some(key)
    }

    #[no_coverage]
    fn get(&self, idx: Self::Index) -> &Self::TestCase {
        &self.inputs[idx].data
    }

    #[no_coverage]
    fn get_mut(&mut self, idx: Self::Index) -> &mut Self::TestCase {
        &mut self.inputs[idx].data
    }

    #[no_coverage]
    fn retrieve_after_processing(&mut self, idx: Self::Index, generation: usize) -> Option<&mut Self::TestCase> {
        if let Some(input) = self.inputs.get_mut(idx) {
            if input.data.generation() == generation {
                Some(&mut input.data)
            } else {
                None
            }
        } else {
            None
        }
    }

    #[no_coverage]
    fn mark_test_case_as_dead_end(&mut self, idx: Self::Index) {
        self.inputs[idx].score = 0.0;
        self.update_stats();
    }
    fn minify(
        &mut self,
        _target_len: usize,
        _event_handler: impl FnMut(CorpusDelta<&Self::TestCase, Self::Index>, Self::Stats) -> Result<(), std::io::Error>,
    ) -> Result<(), std::io::Error> {
        // TODO: only keep the `target_len` highest-scoring inputs?
        Ok(())
    }
}
impl<T: TestCase> UniqueValuesPool<T> {
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

impl<T: TestCase> CompatibleWithIteratorSensor for UniqueValuesPool<T> {
    type Observation = (usize, u64);
    type ObservationState = Vec<(usize, u64)>;

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
        data: Self::TestCase,
        complexity: f64,
        observation_state: Self::ObservationState,
        mut event_handler: impl FnMut(CorpusDelta<&Self::TestCase, Self::Index>, Self::Stats) -> Result<(), std::io::Error>,
    ) -> Result<(), std::io::Error> {
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

        self.update_stats();
        let stats = self.stats();
        event_handler(
            CorpusDelta {
                path: Path::new(&self.name).to_path_buf(),
                add: Some((&self.inputs[input_key].data, input_key)),
                remove: removed_keys,
            },
            stats,
        )?;
        Ok(())
    }
}

#[inline(always)]
#[no_coverage]
fn gen_f64(rng: &fastrand::Rng, range: Range<f64>) -> f64 {
    range.start + rng.f64() * (range.end - range.start)
}
