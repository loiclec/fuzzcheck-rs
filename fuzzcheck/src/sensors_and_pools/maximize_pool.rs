use crate::data_structures::{Slab, SlabKey, WeightedIndex};
use crate::sensors_and_pools::compatible_with_iterator_sensor::CompatibleWithIteratorSensor;
use crate::traits::{CorpusDelta, Pool, TestCase};
use ahash::AHashSet;
use owo_colors::OwoColorize;
use std::fmt::{Debug, Display};
use std::path::Path;

#[derive(Clone, Default)]
pub struct Stats {
    name: String,
    size: usize,
    total_counts: u64,
}

impl Display for Stats {
    #[no_coverage]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            format!("{}(size: {}  sum: {})", self.name, self.size, self.total_counts).bright_purple()
        )
    }
}

#[derive(Debug)]
pub struct Input<T> {
    best_for_counters: AHashSet<usize>,
    cplx: f64,
    data: T,
    score: f64,
}

pub struct CounterMaximizingPool<T> {
    name: String,
    complexities: Vec<f64>,
    highest_counts: Vec<u64>,
    inputs: Slab<Input<T>>,
    best_input_for_counter: Vec<Option<SlabKey<Input<T>>>>,
    // also use a fenwick tree here?
    cumulative_score_inputs: Vec<f64>,
    stats: Stats,
    rng: fastrand::Rng,
}
impl<T: Debug> Debug for CounterMaximizingPool<T> {
    #[no_coverage]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CounterMaximizingPool")
            .field("complexities", &self.complexities)
            .field("highest_counts", &self.highest_counts)
            .field("inputs", &self.inputs)
            .field("best_input_for_counter", &self.best_input_for_counter)
            .field("cumulative_score_inputs", &self.cumulative_score_inputs)
            .finish()
    }
}

impl<T> CounterMaximizingPool<T> {
    #[no_coverage]
    pub fn new(name: &str, size: usize) -> Self {
        Self {
            name: name.to_string(),
            complexities: vec![0.0; size],
            highest_counts: vec![0; size],
            inputs: Slab::new(),
            best_input_for_counter: vec![None; size],
            cumulative_score_inputs: vec![],
            stats: Stats {
                name: name.to_string(),
                size: 0,
                total_counts: 0,
            },
            rng: fastrand::Rng::new(),
        }
    }
}

impl<T: TestCase> Pool for CounterMaximizingPool<T> {
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
        if self.cumulative_score_inputs.last().unwrap_or(&0.0) > &0.0 {
            let weighted_index = WeightedIndex {
                cumulative_weights: &self.cumulative_score_inputs,
            };
            let index = weighted_index.sample(&self.rng);
            Some(self.inputs.get_nth_key(index))
        } else {
            None
        }
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
impl<T: TestCase> CounterMaximizingPool<T> {
    #[no_coverage]
    fn update_stats(&mut self) {
        let inputs = &self.inputs;
        self.cumulative_score_inputs = self
            .inputs
            .keys()
            .map(
                #[no_coverage]
                |key| &inputs[key],
            )
            .scan(
                0.0,
                #[no_coverage]
                |state, x| {
                    *state += x.score;
                    Some(*state)
                },
            )
            .collect();
        self.stats.size = self.inputs.len();
        self.stats.total_counts = self.highest_counts.iter().sum();
    }
}

impl<T: TestCase> CompatibleWithIteratorSensor for CounterMaximizingPool<T> {
    type Observation = (usize, u64);
    type ObservationState = Vec<(usize, u64)>;

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
        data: Self::TestCase,
        complexity: f64,
        observation_state: Self::ObservationState,
        mut event_handler: impl FnMut(CorpusDelta<&Self::TestCase, Self::Index>, Self::Stats) -> Result<(), std::io::Error>,
    ) -> Result<(), std::io::Error> {
        let highest_for_counters = observation_state;
        let cplx = complexity;
        let input = data;
        let input = Input {
            best_for_counters: highest_for_counters.iter().map(|x| x.0).collect(),
            cplx,
            data: input,
            score: highest_for_counters.len() as f64,
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

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::CounterMaximizingPool;
    use crate::sensors_and_pools::compatible_with_iterator_sensor::CompatibleWithIteratorSensor;
    use crate::traits::Pool;

    #[test]
    fn test_basic_pool_1() {
        let mut pool = CounterMaximizingPool::<f64>::new("a", 5);
        println!("{:?}", pool);
        let index = pool.get_random_index();
        println!("{:?}", index);

        pool.add(1.2, 1.21, vec![(1, 2)], |event, _stats| {
            println!("event: {:?}", event);
            Ok(())
        })
        .unwrap();
        println!("pool: {:?}", pool);
        let index = pool.get_random_index();
        println!("{:?}", index);

        // replace
        pool.add(1.1, 1.11, vec![(1, 2)], |event, _stats| {
            println!("event: {:?}", event);
            Ok(())
        })
        .unwrap();

        println!("pool: {:?}", pool);
        let index = pool.get_random_index();
        println!("{:?}", index);
    }

    #[test]
    fn test_basic_pool_2() {
        let mut pool = CounterMaximizingPool::<f64>::new("b", 5);

        let _ = pool.add(1.2, 1.21, vec![(1, 4)], |_, _| Ok(()));
        let _ = pool.add(2.2, 2.21, vec![(2, 2)], |_, _| Ok(()));
        pool.add(3.2, 3.21, vec![(3, 2)], |event, _stats| {
            println!("event: {:?}", event);
            Ok(())
        })
        .unwrap();
        println!("pool: {:?}", pool);
        let index = pool.get_random_index();
        println!("{:?}", index);

        // replace
        pool.add(1.1, 1.11, vec![(2, 3), (3, 3)], |event, _stats| {
            println!("event: {:?}", event);
            Ok(())
        })
        .unwrap();
        println!("pool: {:?}", pool);

        let mut map = HashMap::new();
        for _ in 0..100 {
            let index = pool.get_random_index().unwrap();
            *map.entry(index).or_insert(0) += 1;
        }
        println!("{:?}", map);

        // replace
        pool.add(4.1, 4.41, vec![(0, 3), (3, 4), (4, 1)], |event, _stats| {
            println!("event: {:?}", event);
            Ok(())
        })
        .unwrap();
        println!("pool: {:?}", pool);

        let mut map = HashMap::new();
        for _ in 0..10000 {
            let index = pool.get_random_index().unwrap();
            *map.entry(index).or_insert(0) += 1;
        }
        println!("{:?}", map);

        // replace
        pool.add(
            0.1,
            0.11,
            vec![(0, 3), (3, 4), (4, 1), (1, 7), (2, 8)],
            |event, _stats| {
                println!("event: {:?}", event);
                Ok(())
            },
        )
        .unwrap();
        println!("pool: {:?}", pool);

        let mut map = HashMap::new();
        for _ in 0..10000 {
            let index = pool.get_random_index().unwrap();
            *map.entry(index).or_insert(0) += 1;
        }
        println!("{:?}", map);

        // replace
        pool.add(1.5, 1.51, vec![(0, 10)], |event, _stats| {
            println!("event: {:?}", event);
            Ok(())
        })
        .unwrap();

        println!("pool: {:?}", pool);

        let mut map = HashMap::new();
        for _ in 0..10000 {
            let index = pool.get_random_index().unwrap();
            *map.entry(index).or_insert(0) += 1;
        }
        println!("{:?}", map);
    }
}
