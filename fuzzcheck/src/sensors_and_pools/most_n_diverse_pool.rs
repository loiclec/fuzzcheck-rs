use std::{
    cmp::Ordering,
    fmt::{self, Display, Formatter},
    path::PathBuf,
};

use crate::bitset::FixedBitSet;

use crate::{
    fenwick_tree::FenwickTree,
    fuzzer::PoolStorageIndex,
    traits::{CorpusDelta, Pool},
    CSVField, ToCSVFields,
};

use super::{compatible_with_iterator_sensor::CompatibleWithIteratorSensor, unique_coverage_pool::gen_f64};

#[derive(Clone)]
pub struct Input {
    nbr_unique_counters: usize,
    unique_counters: FixedBitSet,
    counters: FixedBitSet,
    pool_idx: PoolStorageIndex,
    cplx: f64,
}

pub struct MostNDiversePool {
    name: String,
    max_len: usize,
    inputs: Vec<Input>,
    all_counters: FixedBitSet,
    worst_input_idx: Option<usize>,
    fenwick_tree: FenwickTree,
    rng: fastrand::Rng,
    cache: FixedBitSet,
}

#[derive(Clone)]
pub struct Stats {
    name: String,
    counters: usize,
}
impl MostNDiversePool {
    #[no_coverage]
    pub fn new(name: &str, max_len: usize) -> Self {
        Self {
            name: name.to_owned(),
            max_len,
            inputs: vec![],
            all_counters: FixedBitSet::new(),
            worst_input_idx: None,
            rng: fastrand::Rng::new(),
            fenwick_tree: FenwickTree::new(vec![]),
            cache: FixedBitSet::new(),
        }
    }
}

impl Pool for MostNDiversePool {
    type Stats = Stats;
    #[no_coverage]
    fn len(&self) -> usize {
        self.inputs.len()
    }
    #[no_coverage]
    fn stats(&self) -> Self::Stats {
        Stats {
            name: self.name.clone(),
            counters: self.all_counters.count_ones(),
        }
    }
    #[no_coverage]
    fn get_random_index(&mut self) -> Option<PoolStorageIndex> {
        if self.fenwick_tree.len() == 0 {
            return None;
        }
        let most = self.fenwick_tree.prefix_sum(self.fenwick_tree.len() - 1);
        if most <= 0.0 {
            return None;
        }
        let chosen_weight = gen_f64(&self.rng, 0.0..most);

        // Find the first item which has a weight *higher* than the chosen weight.
        let choice = self.fenwick_tree.first_index_past_prefix_sum(chosen_weight);

        let input = &self.inputs[choice];
        Some(input.pool_idx)
    }
    #[no_coverage]
    fn mark_test_case_as_dead_end(&mut self, _idx: PoolStorageIndex) {
        // TODO
    }
    #[no_coverage]
    fn serialized(&self) -> Vec<(PathBuf, Vec<u8>)> {
        // TODO
        vec![]
    }
}
pub struct ObservationState {
    counters: FixedBitSet,
    nbr_new_counters: usize,
}
impl Default for ObservationState {
    fn default() -> Self {
        Self {
            counters: FixedBitSet::with_capacity(2617),
            nbr_new_counters: 0,
        }
    }
}
impl CompatibleWithIteratorSensor for MostNDiversePool {
    type Observation = (usize, u64);

    type ObservationState = ObservationState;
    #[no_coverage]
    fn observe(&mut self, observation: &Self::Observation, _input_complexity: f64, state: &mut Self::ObservationState) {
        let ObservationState {
            counters,
            nbr_new_counters,
        } = state;
        let (idx, _) = observation;
        // if *idx >= counters.len() {
        //     counters.grow(2617);
        // }
        counters.insert(*idx);
        // if !self.all_counters.contains(*idx) {
        //     *nbr_new_counters += 1;
        // }
    }
    #[no_coverage]
    fn finish_observing(&mut self, state: &mut Self::ObservationState, _input_complexity: f64) {
        let ObservationState {
            counters,
            nbr_new_counters,
        } = state;
        self.cache.clone_from(counters);
        // let mut unique_counters = counters.clone();
        self.cache.difference_with(&self.all_counters);

        *nbr_new_counters = self.cache.count_ones();
        self.cache.clear();
    }

    #[no_coverage]
    fn is_interesting(&self, state: &Self::ObservationState, input_complexity: f64) -> bool {
        let ObservationState {
            counters,
            nbr_new_counters,
        } = state;
        if self.inputs.len() < self.max_len && *nbr_new_counters > 0 {
            return true;
        }
        if let Some(worst_input) = self.worst_input_idx.map(
            #[no_coverage]
            |idx| &self.inputs[idx],
        ) {
            if (*nbr_new_counters > worst_input.nbr_unique_counters)
                || (*nbr_new_counters == worst_input.nbr_unique_counters && worst_input.cplx > input_complexity)
            {
                return true;
            }
        }

        let mut common_unique_counters = FixedBitSet::with_capacity(counters.len());
        for input in &self.inputs {
            if *nbr_new_counters > 0 || input.cplx > input_complexity {
                common_unique_counters.clone_from(&counters);
                common_unique_counters.intersect_with(&input.unique_counters);
                let common_uniq_counters = common_unique_counters.count_ones();
                let nbr_new_counters = common_uniq_counters + nbr_new_counters;

                if (nbr_new_counters > input.nbr_unique_counters)
                    || (nbr_new_counters == input.nbr_unique_counters && input.cplx > input_complexity)
                {
                    return true;
                }
            }
        }

        return false;
    }
    #[no_coverage]
    fn add(
        &mut self,
        data: PoolStorageIndex,
        input_complexity: f64,
        state: Self::ObservationState,
    ) -> Vec<CorpusDelta> {
        let ObservationState {
            counters,
            nbr_new_counters,
        } = state;
        let unique_counters = {
            let mut x = counters.clone();
            x.difference_with(&self.all_counters);
            x
        };
        assert!(unique_counters.count_ones() == nbr_new_counters);

        let new_input = Input {
            nbr_unique_counters: nbr_new_counters,
            unique_counters,
            counters: counters.clone(),
            pool_idx: data,
            cplx: input_complexity,
        };

        if self.inputs.len() < self.max_len && nbr_new_counters > 0 {
            self.inputs.push(new_input);
            self.recompute_state_from_inputs_vec();
            return vec![CorpusDelta {
                path: PathBuf::new().join(&self.name),
                add: true,
                remove: vec![],
            }];
        }

        if let Some(worst_input) = self.worst_input_idx.map(
            #[no_coverage]
            |idx| &mut self.inputs[idx],
        ) {
            if (nbr_new_counters > worst_input.nbr_unique_counters)
                || (nbr_new_counters == worst_input.nbr_unique_counters && worst_input.cplx > input_complexity)
            {
                let worst_input_data = worst_input.pool_idx;
                *worst_input = new_input;
                self.recompute_state_from_inputs_vec();
                return vec![CorpusDelta {
                    path: PathBuf::new().join(&self.name),
                    add: true,
                    remove: vec![worst_input_data],
                }];
            }
        }

        let mut common_unique_counters = FixedBitSet::with_capacity(counters.len());
        for input in &mut self.inputs {
            if nbr_new_counters > 0 || input.cplx > input_complexity {
                common_unique_counters.clone_from(&counters);
                common_unique_counters.intersect_with(&input.unique_counters);
                let common_uniq_counters = common_unique_counters.count_ones();
                let nbr_new_counters = common_uniq_counters + nbr_new_counters;

                if (nbr_new_counters > input.nbr_unique_counters)
                    || (nbr_new_counters == input.nbr_unique_counters && input.cplx > input_complexity)
                {
                    let input_data = input.pool_idx;
                    *input = new_input;
                    self.recompute_state_from_inputs_vec();
                    return vec![CorpusDelta {
                        path: PathBuf::new().join(&self.name),
                        add: true,
                        remove: vec![input_data],
                    }];
                }
            }
        }

        unreachable!()
    }
}

impl MostNDiversePool {
    #[no_coverage]
    fn recompute_state_from_inputs_vec(&mut self) {
        let mut all_counters = FixedBitSet::new();
        let mut unique_counters = FixedBitSet::new();
        for input in &self.inputs {
            for bit in input.counters.ones() {
                if !all_counters.contains(bit) {
                    unique_counters.grow(bit * 2 + 1);
                    unique_counters.insert(bit);
                } else if unique_counters.contains(bit) {
                    unique_counters.toggle(bit);
                }
            }
            all_counters.union_with(&input.counters);
        }
        self.all_counters = all_counters;

        for input in &mut self.inputs {
            input.unique_counters.clone_from(&input.counters);
            input.unique_counters.intersect_with(&unique_counters);
            input.nbr_unique_counters = input.unique_counters.count_ones();
        }
        self.worst_input_idx = self
            .inputs
            .iter()
            .enumerate()
            .min_by(
                #[no_coverage]
                |x, y| {
                    (x.1.nbr_unique_counters, -x.1.cplx)
                        .partial_cmp(&(y.1.nbr_unique_counters, -y.1.cplx))
                        .unwrap_or(Ordering::Equal)
                },
            )
            .map(
                #[no_coverage]
                |x| x.0,
            );
        self.fenwick_tree = FenwickTree::new(self.inputs.iter().map(|x| x.nbr_unique_counters as f64).collect());
    }
}

impl ToCSVFields for Stats {
    #[no_coverage]
    fn csv_headers(&self) -> Vec<CSVField> {
        vec![]
    }
    #[no_coverage]
    fn to_csv_record(&self) -> Vec<CSVField> {
        vec![]
    }
}
impl Display for Stats {
    #[no_coverage]
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}({})", self.name, self.counters)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[no_coverage]
    fn test_most_n_diverse_pool() {
        let mut pool = MostNDiversePool::new("diverse5", 5);

        run(&mut pool, vec![1], 10.0);
        run(&mut pool, vec![2], 10.0);
        run(&mut pool, vec![3], 10.0);
        run(&mut pool, vec![4], 10.0);
        run(&mut pool, vec![5], 10.0);

        run(&mut pool, vec![6], 10.0);
        run(&mut pool, vec![6], 9.0);

        run(&mut pool, vec![1, 2], 10.0);
        run(&mut pool, vec![1, 2, 3], 10.0);
    }

    #[no_coverage]
    fn run(pool: &mut MostNDiversePool, observations: Vec<usize>, cplx: f64) {
        let mut obs_state = ObservationState::default();
        for observation in &observations {
            pool.observe(&(*observation, 1), cplx, &mut obs_state);
        }
        if pool.is_interesting(&obs_state, cplx) {
            pool.add(PoolStorageIndex::mock(0), cplx, obs_state);
            println!(
                "input_count: {} worst_idx: {:?}, all_counters: {}",
                pool.inputs.len(),
                pool.worst_input_idx,
                pool.all_counters.count_ones()
            );
            for input in &pool.inputs {
                println!(
                    "\tuniq_counters: {:?}",
                    input.unique_counters.ones().collect::<Vec<_>>()
                );
                // println!("\tall_counters: {:?}", input.counters.ones().collect::<Vec<_>>());
            }
        } else {
            println!("not interesting: {:?}", observations);
        }
    }
}