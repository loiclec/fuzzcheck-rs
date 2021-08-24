use std::{
    fmt::{Debug, Display},
    marker::PhantomData,
};

use ahash::AHashSet;

use crate::{
    code_coverage_sensor::CodeCoverageSensor,
    data_structures::{Slab, SlabKey, WeightedIndex},
    mutators::either::Either,
    sensor_and_pool::{CorpusDelta, Pool, SensorAndPool, TestCase},
};

#[derive(Clone, Default)]
pub(crate) struct Stats {
    size: usize,
    total_counts: u64,
}
impl Display for Stats {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "p2: {}\ttotal_count:{}\t", self.size, self.total_counts)
    }
}

#[derive(Debug)]
pub(crate) struct CounterMaximizingPoolEvent<T> {
    added_key: Option<SlabKey<Input<T>>>,
    removed_keys: Vec<SlabKey<Input<T>>>,
}

#[derive(Debug)]
pub(crate) struct Input<T> {
    best_for_counters: AHashSet<usize>,
    cplx: f64,
    data: T,
    score: f64,
}

pub(crate) struct CounterMaximizingPool<T> {
    complexities: Vec<f64>,
    highest_counts: Vec<u64>,
    inputs: Slab<Input<T>>,
    best_input_for_counter: Vec<Option<SlabKey<Input<T>>>>,
    // also use a fenwwick tree here?
    cumulative_score_inputs: Vec<f64>,
    stats: Stats,
    rng: fastrand::Rng,
}
impl<T: Debug> Debug for CounterMaximizingPool<T> {
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
    pub(crate) fn new(size: usize) -> Self {
        Self {
            complexities: vec![0.0; size],
            highest_counts: vec![0; size],
            inputs: Slab::new(),
            best_input_for_counter: vec![None; size],
            cumulative_score_inputs: vec![],
            stats: Stats {
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

    fn len(&self) -> usize {
        self.inputs.len()
    }

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

    fn get(&self, idx: Self::Index) -> &Self::TestCase {
        &self.inputs[idx].data
    }

    fn get_mut(&mut self, idx: Self::Index) -> &mut Self::TestCase {
        &mut self.inputs[idx].data
    }

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

    fn mark_test_case_as_dead_end(&mut self, idx: Self::Index) {
        self.inputs[idx].score = 0.0;
        self.update_stats();
    }
}
impl<T: TestCase> CounterMaximizingPool<T> {
    fn add(&mut self, input: T, cplx: f64, highest_for_counters: Vec<(usize, u64)>) -> CounterMaximizingPoolEvent<T> {
        let input = Input {
            best_for_counters: highest_for_counters.iter().map(|x| x.0).collect(),
            cplx,
            data: input,
            score: highest_for_counters.len() as f64,
        };
        let input_key = self.inputs.insert(input);
        let mut event = CounterMaximizingPoolEvent {
            added_key: Some(input_key),
            removed_keys: vec![],
        };
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
                    event.removed_keys.push(*previous_best_key);
                }
                *previous_best_key = input_key;
            } else {
                *previous_best_key = Some(input_key);
            }
        }
        for &removed_key in &event.removed_keys {
            self.inputs.remove(removed_key);
        }
        self.update_stats();
        event
    }

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

pub(crate) struct MaximizingCoverageSensorAndPool<T> {
    _phantom: PhantomData<T>,
}
impl<T: TestCase> SensorAndPool for MaximizingCoverageSensorAndPool<T> {
    type Sensor = CodeCoverageSensor;
    type Pool = CounterMaximizingPool<T>;
    type TestCase = T;
    type Event = CounterMaximizingPoolEvent<T>;
    type Stats = Stats;

    fn process(
        sensor: &mut Self::Sensor,
        pool: &mut Self::Pool,
        stats: &mut Self::Stats,
        get_input_ref: Either<<Self::Pool as Pool>::Index, &Self::TestCase>,
        clone_input: &impl Fn(&Self::TestCase) -> Self::TestCase,
        complexity: f64,
        mut event_handler: impl FnMut(
            CorpusDelta<&Self::TestCase, <Self::Pool as Pool>::Index>,
            &Self::Stats,
        ) -> Result<(), std::io::Error>,
    ) -> Result<(), std::io::Error> {
        let mut new_highest = vec![];
        unsafe {
            for i in 0..sensor.coverage.len() {
                sensor.iterate_over_collected_features(
                    i,
                    #[no_coverage]
                    |index, counter| {
                        let pool_counter = pool.highest_counts[index];
                        if pool_counter < counter {
                            new_highest.push((index, counter));
                        } else if pool_counter == counter {
                            if let Some(candidate_key) = pool.best_input_for_counter[index] {
                                if pool.inputs[candidate_key].cplx > complexity {
                                    new_highest.push((index, counter));
                                }
                            } else {
                            }
                        }
                    },
                );
            }
        }
        if !new_highest.is_empty() {
            let input = match get_input_ref {
                Either::Left(key) => clone_input(&pool.inputs[key].data),
                Either::Right(input) => clone_input(input),
            };
            let event = pool.add(input, complexity, new_highest);
            let delta = Self::get_corpus_delta_from_event(pool, event);
            *stats = pool.stats.clone();
            event_handler(delta, stats)?;
        }
        Ok(())
    }

    fn minify(
        sensor: &mut Self::Sensor,
        pool: &mut Self::Pool,
        stats: &mut Self::Stats,
        target_len: usize,
        event_handler: impl FnMut(CorpusDelta<&Self::TestCase, <Self::Pool as Pool>::Index>, &Self::Stats),
    ) {
        todo!()
    }

    fn get_corpus_delta_from_event<'a>(
        pool: &'a Self::Pool,
        event: Self::Event,
    ) -> CorpusDelta<&'a Self::TestCase, <Self::Pool as Pool>::Index> {
        CorpusDelta {
            add: event.added_key.map(|key| (&pool.inputs[key].data, key)),
            remove: event.removed_keys,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use crate::sensor_and_pool::Pool;

    use super::CounterMaximizingPool;

    #[test]
    fn test_basic_pool_1() {
        let mut pool = CounterMaximizingPool::<f64>::new(5);
        println!("{:?}", pool);
        let index = pool.get_random_index();
        println!("{:?}", index);

        let event = pool.add(1.2, 1.21, vec![(1, 2)]);
        println!("event: {:?}", event);
        println!("pool: {:?}", pool);
        let index = pool.get_random_index();
        println!("{:?}", index);

        // replace
        let event = pool.add(1.1, 1.11, vec![(1, 2)]);
        println!("event: {:?}", event);
        println!("pool: {:?}", pool);
        let index = pool.get_random_index();
        println!("{:?}", index);
    }

    #[test]
    fn test_basic_pool_2() {
        let mut pool = CounterMaximizingPool::<f64>::new(5);

        let _ = pool.add(1.2, 1.21, vec![(1, 4)]);
        let _ = pool.add(2.2, 2.21, vec![(2, 2)]);
        let event = pool.add(3.2, 3.21, vec![(3, 2)]);
        println!("event: {:?}", event);
        println!("pool: {:?}", pool);
        let index = pool.get_random_index();
        println!("{:?}", index);

        // replace
        let event = pool.add(1.1, 1.11, vec![(2, 3), (3, 3)]);
        println!("event: {:?}", event);
        println!("pool: {:?}", pool);

        let mut map = HashMap::new();
        for _ in 0..100 {
            let index = pool.get_random_index().unwrap();
            *map.entry(index).or_insert(0) += 1;
        }
        println!("{:?}", map);

        // replace
        let event = pool.add(4.1, 4.41, vec![(0, 3), (3, 4), (4, 1)]);
        println!("event: {:?}", event);
        println!("pool: {:?}", pool);

        let mut map = HashMap::new();
        for _ in 0..10000 {
            let index = pool.get_random_index().unwrap();
            *map.entry(index).or_insert(0) += 1;
        }
        println!("{:?}", map);

        // replace
        let event = pool.add(0.1, 0.11, vec![(0, 3), (3, 4), (4, 1), (1, 7), (2, 8)]);
        println!("event: {:?}", event);
        println!("pool: {:?}", pool);

        let mut map = HashMap::new();
        for _ in 0..10000 {
            let index = pool.get_random_index().unwrap();
            *map.entry(index).or_insert(0) += 1;
        }
        println!("{:?}", map);

        // replace
        let event = pool.add(1.5, 1.51, vec![(0, 10)]);
        println!("event: {:?}", event);
        println!("pool: {:?}", pool);

        let mut map = HashMap::new();
        for _ in 0..10000 {
            let index = pool.get_random_index().unwrap();
            *map.entry(index).or_insert(0) += 1;
        }
        println!("{:?}", map);
    }
}
