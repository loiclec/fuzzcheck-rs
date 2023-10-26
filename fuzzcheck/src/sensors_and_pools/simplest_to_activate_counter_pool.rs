//! A pool that tries to find a minimal test case activating each sensor counter.
//!
//! # Policy for adding and removing inputs from the pool
//!
//! The pool will strive to keep as few inputs as possible, and will
//! prioritize small high-scoring inputs over large low-scoring ones. It does
//! so in a couple ways.
//!
//! First, an input will only be added if:
//!
//! 1. It activates a new counter, not seen by any other input in the pool; or
//! 2. It is the smallest input that activates a particular counter
//!
//! Second, following a pool update, any input in the pool that does not meet
//! the above conditions anymore will be removed from the pool.
//!
//! # Scoring of an input
//!
//! The score of an input is computed to be as fair as possible. This
//! is currently done by assigning a score to each counter and distributing
//! that score to each input activating that counter. For example, if a
//! thousand inputs all activate the counter C1, then they will all derive
//! a thousandth of C1’s score from it. On the other hand, if only two inputs
//! activate the counter C2, then they will each get half of C2’s score from it.
//! In short, an input’s final score is the sum of the score of each of its
//! activated counters divided by their frequencies.
//!

use std::fmt::Display;
use std::hash::Hash;
use std::path::{Path, PathBuf};

use ahash::{AHashMap, AHashSet};
use fastrand::Rng;
use nu_ansi_term::Color;

use crate::data_structures::{Slab, SlabKey};
use crate::fenwick_tree::FenwickTree;
use crate::traits::{CorpusDelta, Pool, SaveToStatsFolder, Stats};
use crate::{CSVField, CompatibleWithObservations, PoolStorageIndex, ToCSV};

#[derive(Debug)]
#[repr(transparent)]
struct CounterIdx(pub usize);

impl Clone for CounterIdx {
    #[inline(always)]
    #[coverage(off)]
    fn clone(&self) -> Self {
        Self(self.0)
    }
}
impl Copy for CounterIdx {}
impl Hash for CounterIdx {
    #[inline(always)]
    #[coverage(off)]
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.0.hash(state);
    }
}
impl PartialEq for CounterIdx {
    #[inline(always)]
    #[coverage(off)]
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
    #[inline(always)]
    #[coverage(off)]
    fn ne(&self, other: &Self) -> bool {
        self.0 != other.0
    }
}
impl Eq for CounterIdx {}

impl CounterIdx {
    #[inline(always)]
    #[coverage(off)]
    pub(crate) fn new(index: usize) -> Self {
        Self(index)
    }
}

/**
 * An element stored in the pool, containing its value, cache, mutation step,
 * as well as analysed code coverage and computed score.
*/
struct Input {
    /// The keys of the counters for which there are no simpler inputs in the
    /// pool activating the counter.
    least_complex_for_counters: AHashSet<CounterIdx>,
    /// Holds the key of each counter associated with this input.
    all_counters: Vec<CounterIdx>,
    /// The computed score of the input
    pub score: f64,
    /// Index in the fuzzer’s storage that points to the input’s data
    data: PoolStorageIndex,
    /// Cached complexity of the value.
    complexity: f64,
    /// The number of times that this input was fed to the test function.
    ///
    /// This is used to prioritise new inputs over old ones.
    number_times_chosen: usize,
}

/**
    An analysis of the role of a counter in the pool.

    It contains the counter itself, the list of inputs activating this counter,
    as well as a reference to the least complex of these inputs.
*/
struct AnalysedCounter {
    key: CounterIdx,
    inputs: Vec<SlabKey<Input>>,
    least_complex_input: SlabKey<Input>,
    least_complexity: f64,
    score: f64,
}

impl AnalysedCounter {
    #[coverage(off)]
    fn new(
        key: CounterIdx,
        inputs: Vec<SlabKey<Input>>,
        least_complex_input: SlabKey<Input>,
        least_complexity: f64,
    ) -> Self {
        let score = SimplestToActivateCounterPool::score_of_counter(inputs.len());
        Self {
            key,
            inputs,
            least_complex_input,
            least_complexity,
            score,
        }
    }
}

/// A pool that tries to find a minimal test case activating each sensor counter.
///
/// It is compatible with any sensor whose [observations](crate::Sensor::Observations)
/// are given by an iterator of `(usize, u64)`. In particular, it is recommended to use it
/// with the [`CodeCoverageSensor`](crate::sensors_and_pools::CodeCoverageSensor).
pub struct SimplestToActivateCounterPool {
    pub name: String,

    least_complexity_for_counter: Vec<f64>,
    analysed_counters: AHashMap<CounterIdx, AnalysedCounter>,
    slab_inputs: Slab<Input>,

    pub average_complexity: f64,
    pub total_score: f64,
    pub ranked_inputs: FenwickTree,

    rng: Rng,
}

impl SimplestToActivateCounterPool {
    #[coverage(off)]
    pub fn new(name: &str, nbr_counters: usize) -> Self {
        SimplestToActivateCounterPool {
            name: name.to_string(),
            least_complexity_for_counter: vec![f64::INFINITY; nbr_counters],
            analysed_counters: AHashMap::with_hasher(ahash::RandomState::with_seeds(0, 0, 0, 0)),

            slab_inputs: Slab::new(),

            average_complexity: 0.0,
            total_score: 0.0,
            ranked_inputs: FenwickTree::new(vec![]),

            rng: fastrand::Rng::new(),
        }
    }

    #[coverage(off)]
    pub fn score(&self) -> f64 {
        self.total_score
    }

    #[allow(clippy::too_many_lines)]
    #[coverage(off)]
    fn add(
        &mut self,
        data: PoolStorageIndex,
        complexity: f64,
        result: AnalysisResult,
    ) -> Option<(CorpusDelta, <Self as Pool>::Stats)> {
        let AnalysisResult {
            existing_counters,
            new_counters,
        } = result;

        if existing_counters.is_empty() && new_counters.is_empty() {
            return None;
        }

        let element = Input {
            least_complex_for_counters: AHashSet::with_hasher(ahash::RandomState::with_seeds(0, 0, 0, 0)),
            all_counters: vec![],
            score: 0.0,
            data,
            complexity,
            number_times_chosen: 1,
        };
        let element_key = self.slab_inputs.insert(element);

        let mut to_delete: AHashSet<SlabKey<Input>> = AHashSet::with_hasher(ahash::RandomState::with_seeds(0, 0, 0, 0));

        // 1. Update the `element.least_complex_for_counters` fields of the elements affected
        // by a change in the `least_complexity` of the counters in `existing_counters`.
        // 1.1. If it turns out that an element is now no longer the least complex for any counter,
        // then add it to the list of elements to delete
        for counter_key in existing_counters.iter() {
            let counter = self.analysed_counters.get_mut(counter_key).unwrap();

            if counter.least_complexity < complexity {
                continue;
            }

            for input_key in &counter.inputs {
                let affected_element = &mut self.slab_inputs[*input_key];
                affected_element.least_complex_for_counters.remove(counter_key);
                if affected_element.least_complex_for_counters.is_empty() {
                    to_delete.insert(*input_key);
                }
            }
            let element = &mut self.slab_inputs[element_key];

            element.least_complex_for_counters.insert(*counter_key);
            counter.least_complex_input = element_key;
            self.least_complexity_for_counter[counter_key.0] = complexity;
            counter.least_complexity = complexity;
        }

        for counter_key in existing_counters.iter() {
            let counter = self.analysed_counters.get_mut(counter_key).unwrap();
            let element = &mut self.slab_inputs[element_key];

            element.all_counters.push(*counter_key);
            counter.inputs.push(element_key);
        }

        // Now add in the new counters
        let element = &mut self.slab_inputs[element_key];
        for &f in new_counters.iter() {
            let new_counter_for_iter = complexity;
            self.least_complexity_for_counter[f.0] = new_counter_for_iter;

            let analyzed_f = AnalysedCounter::new(f, vec![element_key], element_key, complexity);
            self.analysed_counters.insert(f, analyzed_f);

            element.all_counters.push(f);
            element.least_complex_for_counters.insert(f);
        }

        let mut affected_counters = AHashSet::<CounterIdx>::new();

        let deleted_values: Vec<_> = to_delete.iter().copied().collect();
        let deleted_pool_storage_indices = deleted_values
            .iter()
            .map(
                #[coverage(off)]
                |key| self.slab_inputs[*key].data,
            )
            .collect::<Vec<_>>();

        self.delete_elements(to_delete, &mut affected_counters);

        // now track the counters whose scores are affected by the existing counters
        for counter_key in existing_counters.iter() {
            affected_counters.insert(*counter_key);
        }
        // and update the score of every affected input
        for counter_key in affected_counters.into_iter() {
            let counter = self.analysed_counters.get_mut(&counter_key).unwrap();

            let old_score = counter.score;
            counter.score = Self::score_of_counter(counter.inputs.len());
            let change_in_score = counter.score - old_score;

            for &input_key in &counter.inputs {
                let element_with_counter = &mut self.slab_inputs[input_key];
                element_with_counter.score += change_in_score;
            }
        }

        let element = &mut self.slab_inputs[element_key];
        element.score = 0.0;
        for f_key in &element.all_counters {
            let analyzed_counter = self.analysed_counters.get_mut(f_key).unwrap();
            let counter_score = Self::score_of_counter(analyzed_counter.inputs.len());
            element.score += counter_score;
        }

        self.update_self_stats();

        // self.sanity_check();
        let stats = self.stats();
        Some((
            CorpusDelta {
                path: Path::new(&self.name).to_path_buf(),
                add: true,
                remove: deleted_pool_storage_indices,
            },
            stats,
        ))
    }

    #[coverage(off)]
    fn delete_elements(&mut self, to_delete: AHashSet<SlabKey<Input>>, affected_counters: &mut AHashSet<CounterIdx>) {
        for &to_delete_key in &to_delete {
            let to_delete_el = &self.slab_inputs[to_delete_key];

            for f in &to_delete_el.all_counters {
                affected_counters.insert(*f);
            }

            for &f_key in &to_delete_el.all_counters {
                let analyzed_f = self.analysed_counters.get_mut(&f_key).unwrap();

                let idx_to_delete_key = analyzed_f
                    .inputs
                    .iter()
                    .position(
                        #[coverage(off)]
                        |&x| x == to_delete_key,
                    )
                    .unwrap();
                analyzed_f.inputs.swap_remove(idx_to_delete_key);
            }

            self.slab_inputs.remove(to_delete_key);
        }
    }

    #[coverage(off)]
    pub fn score_of_counter(exact_counter_multiplicity: usize) -> f64 {
        1.0 / (exact_counter_multiplicity as f64)
    }

    /// Update global statistics of the pool following a change in its content
    #[coverage(off)]
    fn update_self_stats(&mut self) {
        let slab = &self.slab_inputs;

        let ranked_inputs = self
            .slab_inputs
            .keys()
            .map(
                #[coverage(off)]
                |key| {
                    let input = &slab[key];
                    input.score / (input.number_times_chosen as f64)
                },
            )
            .collect();
        self.ranked_inputs = FenwickTree::new(ranked_inputs);

        self.total_score = self
            .slab_inputs
            .keys()
            .map(
                #[coverage(off)]
                |key| slab[key].score,
            )
            .sum();

        self.average_complexity = self
            .slab_inputs
            .keys()
            .map(
                #[coverage(off)]
                |key| &slab[key],
            )
            .fold(
                0.0,
                #[coverage(off)]
                |c, x| c + x.complexity,
            )
            / self.slab_inputs.len() as f64;
    }

    #[cfg(test)]
    #[coverage(off)]
    fn print_recap(&self) {
        println!("recap inputs:");
        for input_key in self.slab_inputs.keys() {
            let input = &self.slab_inputs[input_key];
            println!(
                "input with key {:?} has cplx {:.2}, score {:.2}, and counters: {:?}",
                input_key, input.complexity, input.score, input.all_counters
            );
            println!("        and is best for {:?}", input.least_complex_for_counters);
        }
        println!("recap counters:");
        for (f_idx, f) in &self.analysed_counters {
            println!("counter {:?}’s inputs: {:?}", f_idx, f.inputs);
        }
        println!("---");
    }

    #[cfg(test)]
    #[coverage(off)]
    fn sanity_check(&self) {
        let slab = &self.analysed_counters;

        self.print_recap();

        for (f_key, f) in &self.analysed_counters {
            for input_key in &f.inputs {
                let input = &self.slab_inputs[*input_key];
                assert!(input.all_counters.contains(f_key));
            }
        }

        for input_key in self.slab_inputs.keys() {
            let input = &self.slab_inputs[input_key];
            assert!(input.score > 0.0);
            let expected_input_score = input.all_counters.iter().fold(0.0, |c, fk| {
                let f = &slab[fk];
                c + Self::score_of_counter(f.inputs.len())
            });
            assert!(
                (input.score - expected_input_score).abs() < 0.01,
                "{:.2} != {:.2}",
                input.score,
                expected_input_score
            );
            assert!(!input.least_complex_for_counters.is_empty());

            for f_key in &input.least_complex_for_counters {
                let analyzed_f = &self.analysed_counters[f_key];

                #[allow(clippy::float_cmp)]
                let equal_cplx = analyzed_f.least_complexity == input.complexity;
                assert!(equal_cplx);
                assert!(analyzed_f.inputs.contains(&input_key));
                assert!(
                    analyzed_f
                        .inputs
                        .iter()
                        .find(|&&key| self.slab_inputs[key].complexity < input.complexity)
                        == None
                );
            }
        }

        let mut dedupped_inputs = self.slab_inputs.keys().collect::<Vec<_>>();
        dedupped_inputs.sort();
        dedupped_inputs.dedup();
        assert_eq!(dedupped_inputs.len(), self.slab_inputs.len());

        // let mut dedupped_counters = self.counters.clone();
        // dedupped_counters.sort();
        // dedupped_counters.dedup();
        // assert_eq!(dedupped_counters.len(), self.counters.len());
    }
}

impl Pool for SimplestToActivateCounterPool {
    type Stats = UniqueCoveragePoolStats;

    #[coverage(off)]
    fn stats(&self) -> Self::Stats {
        UniqueCoveragePoolStats {
            name: self.name.clone(),
            score: self.score(),
            pool_size: self.slab_inputs.len(),
            avg_cplx: self.average_complexity,
            coverage: (self.analysed_counters.len(), self.least_complexity_for_counter.len()),
        }
    }

    #[coverage(off)]
    fn get_random_index(&mut self) -> Option<PoolStorageIndex> {
        let choice = self.ranked_inputs.sample(&self.rng)?;
        let key = self.slab_inputs.get_nth_key(choice);

        let input = &mut self.slab_inputs[key];
        let old_rank = input.score / (input.number_times_chosen as f64);
        input.number_times_chosen += 1;
        let new_rank = input.score / (input.number_times_chosen as f64);

        let delta = new_rank - old_rank;
        self.ranked_inputs.update(choice, delta);
        Some(input.data)
    }
}

impl SaveToStatsFolder for SimplestToActivateCounterPool {
    #[coverage(off)]
    fn save_to_stats_folder(&self) -> Vec<(PathBuf, Vec<u8>)> {
        cfg_if::cfg_if! {
            if #[cfg(feature = "serde_json_serializer")]
            {
                let path = PathBuf::new().join(format!("{}.json", &self.name));

                let all_hit_counters = self
                    .least_complexity_for_counter
                    .iter()
                    .enumerate()
                    .filter(#[coverage(off)] |(_, x)| **x != f64::INFINITY)
                    .map(#[coverage(off)] |(idx, _)| idx)
                    .collect::<Vec<_>>();

                let best_for_counter = self
                    .least_complexity_for_counter
                    .iter()
                    .enumerate()
                    .filter(#[coverage(off)] |(_, x)| **x != f64::INFINITY)
                    .map(#[coverage(off)] |(idx, _)| {
                        let f = &self.analysed_counters[&CounterIdx::new(idx)];
                        let key = f.least_complex_input;
                        let input = &self.slab_inputs[key].data;
                        (idx, *input)
                    })
                    .collect::<Vec<_>>();

                let mut ranked_inputs = self
                    .slab_inputs
                    .keys()
                    .map(#[coverage(off)] |key| {
                        let input = &self.slab_inputs[key];
                        (input.data, input.score)
                    })
                    .collect::<Vec<_>>();
                ranked_inputs.sort_by(#[coverage(off)] |&x, y| x.1.partial_cmp(&y.1).unwrap_or(std::cmp::Ordering::Equal).reverse());
                let ranked_inputs = ranked_inputs.into_iter().map(#[coverage(off)] |x| x.0).collect();

                let counters_for_input = self
                    .slab_inputs
                    .keys()
                    .map(#[coverage(off)] |key| {
                        let input = &self.slab_inputs[key];
                        (input.data, input.all_counters.iter().map(#[coverage(off)] |x| x.0).collect())
                    })
                    .collect::<Vec<_>>();

                let serialized = SerializedUniqCov {
                    all_hit_counters,
                    best_for_counter,
                    ranked_inputs,
                    counters_for_input,
                };

                let content = serde_json::to_vec(&serialized).unwrap();

                vec![(path, content)]
            } else {
                vec![]
            }
        }
    }
}

#[cfg(feature = "serde_json_serializer")]
#[derive(serde::Serialize, serde::Deserialize)]
struct SerializedUniqCov {
    all_hit_counters: Vec<usize>,
    best_for_counter: Vec<(usize, PoolStorageIndex)>,
    ranked_inputs: Vec<PoolStorageIndex>,
    counters_for_input: Vec<(PoolStorageIndex, Vec<usize>)>,
}

// ===============================================================
// ==================== Trait implementations ====================
// ===============================================================

impl Clone for AnalysedCounter {
    #[coverage(off)]
    fn clone(&self) -> Self {
        Self {
            key: self.key,
            inputs: self.inputs.clone(),
            least_complex_input: self.least_complex_input,
            least_complexity: self.least_complexity,
            score: self.score,
        }
    }
}

#[derive(Clone)]
pub struct UniqueCoveragePoolStats {
    pub name: String,
    pub score: f64,
    pub pool_size: usize,
    pub avg_cplx: f64,
    pub coverage: (usize, usize),
}
impl Display for UniqueCoveragePoolStats {
    #[coverage(off)]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            Color::LightGreen.paint(format!(
                "{}({} cov: {}/{} cplx: {:.2})",
                self.name, self.pool_size, self.coverage.0, self.coverage.1, self.avg_cplx
            ))
        )
    }
}
impl ToCSV for UniqueCoveragePoolStats {
    #[coverage(off)]
    fn csv_headers(&self) -> Vec<CSVField> {
        vec![
            CSVField::String(format!("{}-size", self.name)),
            CSVField::String(format!("{}-percent-coverage", self.name)),
            CSVField::String(format!("{}-avg-cplx", self.name)),
        ]
    }
    #[coverage(off)]
    fn to_csv_record(&self) -> Vec<CSVField> {
        vec![
            CSVField::Integer(self.pool_size as isize),
            CSVField::Integer(self.coverage.0 as isize),
            CSVField::Float(self.avg_cplx),
        ]
    }
}
impl Stats for UniqueCoveragePoolStats {}
#[derive(Default)]
pub struct UniqueCoveragePoolObservationState {
    is_interesting: bool,
    // analysis_result: AnalysisResult,
}
#[derive(Default)]
struct AnalysisResult {
    existing_counters: Vec<CounterIdx>,
    new_counters: Vec<CounterIdx>,
}

impl<O> CompatibleWithObservations<O> for SimplestToActivateCounterPool
where
    for<'a> &'a O: IntoIterator<Item = &'a (usize, u64)>,
{
    #[coverage(off)]
    fn process(&mut self, input_id: PoolStorageIndex, observations: &O, complexity: f64) -> Vec<CorpusDelta> {
        let mut state = UniqueCoveragePoolObservationState::default();

        for &(index, _) in observations.into_iter() {
            let prev_least_complexity = *unsafe { self.least_complexity_for_counter.get_unchecked(index) };
            state.is_interesting |= complexity < prev_least_complexity;
        }
        if !state.is_interesting {
            return vec![];
        }
        let mut result = AnalysisResult::default();
        for &(index, _counter) in observations.into_iter() {
            let counter_idx = CounterIdx::new(index);
            let prev_least_complexity = *unsafe { self.least_complexity_for_counter.get_unchecked(counter_idx.0) };
            if prev_least_complexity == f64::INFINITY {
                result.new_counters.push(counter_idx);
            } else {
                result.existing_counters.push(counter_idx);
            }
        }

        let result = result;
        self.add(input_id, complexity, result)
            .map(
                #[coverage(off)]
                |x| x.0,
            )
            .into_iter()
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Mutator;

    #[coverage(off)]
    fn edge_f(index: usize, intensity: u16) -> CounterIdx {
        CounterIdx(index * 64 + intensity as usize)
    }

    #[test]
    #[coverage(off)]
    fn property_test() {
        use std::iter::FromIterator;

        let mut list_counters = vec![];
        for i in 0..3 {
            for j in 0..3 {
                list_counters.push(edge_f(i, j));
            }
        }

        for _ in 0..1000 {
            let mut new_counters: AHashSet<_, ahash::RandomState> = AHashSet::from_iter(list_counters.iter());
            let mut added_counters: Vec<CounterIdx> = vec![];

            let mut pool = SimplestToActivateCounterPool::new("cov", 1024);

            for i in 0..fastrand::usize(0..100) {
                let nbr_new_counters = if new_counters.is_empty() {
                    0
                } else if i == 0 {
                    fastrand::usize(1..new_counters.len())
                } else {
                    fastrand::usize(0..new_counters.len())
                };
                let new_counters_1: Vec<_> = {
                    let mut fs = new_counters
                        .iter()
                        .map(
                            #[coverage(off)]
                            |&&f| f,
                        )
                        .collect::<Vec<_>>();

                    fastrand::shuffle(&mut fs);
                    fs[0..nbr_new_counters].to_vec()
                };

                for f in &new_counters_1 {
                    new_counters.remove(f);
                }
                let nbr_existing_counters = if new_counters_1.is_empty() {
                    if added_counters.len() > 1 {
                        fastrand::usize(1..added_counters.len())
                    } else {
                        1
                    }
                } else if added_counters.is_empty() {
                    0
                } else {
                    fastrand::usize(0..added_counters.len())
                };

                let existing_counters_1: Vec<CounterIdx> = {
                    let mut fs = added_counters.clone();
                    fastrand::shuffle(&mut fs);
                    fs[0..nbr_existing_counters].to_vec()
                };

                let max_cplx: f64 = if !existing_counters_1.is_empty() && new_counters_1.is_empty() {
                    let idx = fastrand::usize(0..existing_counters_1.len());
                    let fs = existing_counters_1
                        .iter()
                        .map(
                            #[coverage(off)]
                            |&f_key| pool.analysed_counters[&f_key].least_complexity,
                        )
                        .collect::<Vec<_>>();
                    fs[idx]
                } else {
                    100.0
                };

                #[allow(clippy::float_cmp)]
                if max_cplx == 1.0 {
                    break;
                }

                let cplx1 = 1.0 + fastrand::f64() * (max_cplx - 1.0);
                for f in new_counters_1.iter() {
                    added_counters.push(*f);
                }

                let prev_score = pool.score();
                let analysis_result = AnalysisResult {
                    existing_counters: existing_counters_1,
                    new_counters: new_counters_1,
                };
                // println!("adding input of cplx {:.2} with new counters {:?} and existing counters {:?}", cplx1, new_counters_1, existing_counters_1);
                let _ = pool.add(PoolStorageIndex::mock(0), cplx1, analysis_result);
                // pool.print_recap();
                pool.sanity_check();
                assert!(
                    (pool.score() - prev_score) > -0.01,
                    "{:.3} > {:.3}",
                    prev_score,
                    pool.score()
                );
            }
            // TODO: restore that
            // for _ in 0..pool.len() {
            //     let prev_score = pool.score();
            //     let _ = pool.remove_lowest_scoring_input();
            //     pool.sanity_check();
            //     assert!(
            //         (prev_score - pool.score()) > -0.01,
            //         "{:.3} < {:.3}",
            //         prev_score,
            //         pool.score()
            //     );
            // }
        }
    }

    #[derive(Clone, Copy, Debug)]
    pub struct VoidMutator {}

    impl Mutator<f64> for VoidMutator {
        type Cache = ();
        type MutationStep = ();
        type ArbitraryStep = ();
        type UnmutateToken = ();

        #[coverage(off)]
        fn initialize(&self) {}

        #[coverage(off)]
        fn default_arbitrary_step(&self) -> Self::ArbitraryStep {}

        #[coverage(off)]
        fn is_valid(&self, _value: &f64) -> bool {
            true
        }

        #[coverage(off)]
        fn validate_value(&self, _value: &f64) -> Option<Self::Cache> {
            Some(())
        }

        #[coverage(off)]
        fn default_mutation_step(&self, _value: &f64, _cache: &Self::Cache) -> Self::MutationStep {}
        #[coverage(off)]
        fn global_search_space_complexity(&self) -> f64 {
            0.0
        }

        #[coverage(off)]
        fn max_complexity(&self) -> f64 {
            0.0
        }

        #[coverage(off)]
        fn min_complexity(&self) -> f64 {
            0.0
        }

        #[coverage(off)]
        fn complexity(&self, _value: &f64, _cache: &Self::Cache) -> f64 {
            0.0
        }

        #[coverage(off)]
        fn ordered_arbitrary(&self, _step: &mut Self::ArbitraryStep, _max_cplx: f64) -> Option<(f64, f64)> {
            todo!()
        }

        #[coverage(off)]
        fn random_arbitrary(&self, _max_cplx: f64) -> (f64, f64) {
            (0.0, 0.0)
        }

        #[coverage(off)]
        fn ordered_mutate(
            &self,
            _value: &mut f64,
            _cache: &mut Self::Cache,
            _step: &mut Self::MutationStep,
            _subvalue_provider: &dyn crate::SubValueProvider,
            _max_cplx: f64,
        ) -> Option<(Self::UnmutateToken, f64)> {
            Some(((), 0.0))
        }

        #[coverage(off)]
        fn random_mutate(
            &self,
            _value: &mut f64,
            _cache: &mut Self::Cache,
            _max_cplx: f64,
        ) -> (Self::UnmutateToken, f64) {
            ((), 0.0)
        }

        #[coverage(off)]
        fn unmutate(&self, _value: &mut f64, _cache: &mut Self::Cache, _t: Self::UnmutateToken) {}

        #[coverage(off)]
        fn visit_subvalues<'a>(
            &self,
            _value: &'a f64,
            _cache: &'a Self::Cache,
            _visit: &mut dyn FnMut(&'a dyn std::any::Any, f64),
        ) {
        }
    }
}
