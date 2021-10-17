//! The [Pool] is responsible for storing and updating inputs along with
//! their associated code coverage.
//!
//! It assigns a score for each input based on how unique its associated code
//! coverage is. And it can randomly select an input with a probability that
//! is proportional to its score relative to all the other ones.
//!
//! # [Feature]: a unit of code coverage
//!
//! The code coverage of an input is a set of [Feature]. A [Feature] is a value
//! that identifies some behavior of the code that was run. For example, it
//! could say “This edge was reached this many times” or “This comparison
//! instruction was called with these arguments”. In practice, features are not
//! perfectly precise. They won't count the exact number of times a code edge
//! was reached, or record the exact arguments passed to an instruction.
//! This is purely due to performance reasons. The end consequence is that the
//! fuzzer may think that an input is less interesting than it really is.
//!
//! # Policy for adding and removing inputs from the pool
//!
//! The pool will strive to keep as few inputs as possible, and will
//! prioritize small high-scoring inputs over large low-scoring ones. It does
//! so in a couple ways.
//!
//! First, an input will only be added if:
//!
//! 1. It contains a new feature, not seen by any other input in the pool; or
//! 2. It is the smallest input that contains a particular Feature; or
//!
//! Second, following a pool update, any input in the pool that does not meet
//! the above conditions anymore will be removed from the pool.
//!
//! # Scoring of an input
//!
//! The score of an input is computed to be as fair as possible. This
//! is currently done by assigning a score to each Feature and distributing
//! that score to each input containing that feature. For example, if a
//! thousand inputs all contain the feature F1, then they will all derive
//! a thousandth of F1’s score from it. On the other hand, if only two inputs
//! contain the feature F2, then they will each get half of F2’s score from it.
//! In short, an input’s final score is the sum of the score of each of its
//! features divided by their frequencies.
//!
//! ## Feature Groups
//!
//! Additionally, different but similar features also share a commong score.
//! For example, features with different counter values but the same location
//! belong to the same group, and divide the score of the group among themselves.
//!
//! Let's say Group G3 correspond to edge features of the PC “467” and that
//! there are 5 different features belonging to that group: F1, F2, F3, F4, F5.
//! The score associated with each feature is `(group.score() / 5)`.
//! Now imagine the feature f4 appears in 3 different inputs. Each input will
//! thus gain ((group.score() / 5) / 3) from having the feature f4.

use super::compatible_with_iterator_sensor::CompatibleWithIteratorSensor;
use crate::data_structures::{Slab, SlabKey};
use crate::fenwick_tree::FenwickTree;
use crate::fuzzer::PoolStorageIndex;
use crate::traits::{CorpusDelta, Pool};
use crate::{CSVField, ToCSVFields};
use ahash::{AHashMap, AHashSet};
use fastrand::Rng;
use owo_colors::OwoColorize;
use serde_json::{Number, Value};
use std::cmp::Ordering;
use std::fmt::Display;
use std::hash::Hash;
use std::ops::Range;
use std::path::{Path, PathBuf};

/**
 * A unit of code coverage.
 */
#[derive(Debug)]
#[repr(transparent)]
pub struct FeatureIdx(pub usize);

impl Clone for FeatureIdx {
    #[inline(always)]
    #[no_coverage]
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}
impl Copy for FeatureIdx {}
impl Hash for FeatureIdx {
    #[inline(always)]
    #[no_coverage]
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.0.hash(state);
    }
}
impl PartialEq for FeatureIdx {
    #[inline(always)]
    #[no_coverage]
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
    #[inline(always)]
    #[no_coverage]
    fn ne(&self, other: &Self) -> bool {
        self.0 != other.0
    }
}
impl Eq for FeatureIdx {}

impl FeatureIdx {
    #[inline(always)]
    #[no_coverage]
    pub(crate) fn new(index: usize) -> Self {
        Self(index)
    }

    #[inline(always)]
    #[no_coverage]
    fn group_id(self) -> FeatureGroupId {
        FeatureGroupId { id: Self(self.0) }
    }
}

/**
 * An element stored in the pool, containing its value, cache, mutation step,
 * as well as analysed code coverage and computed score.
*/
pub struct Input {
    /// The keys of the features for which there are no simpler inputs in the
    /// pool reaching the feature.
    least_complex_for_features: AHashSet<FeatureIdx>,
    /// Holds the key of each [FeatureInPool] associated with this input.
    all_features: Vec<FeatureIdx>,
    /// The computed score of the input
    pub score: f64,
    /// Data associated with the input: value, cache, and mutation step
    data: PoolStorageIndex,
    /// Cached complexity of the value.
    ///
    /// It should always be equal to [mutator.complexity(&self.data.value, &self.data.cache)](Mutator::complexity)
    complexity: f64,

    number_times_chosen: usize,
}

/**
    An analysis of the role of a feature in the pool.

    It contains the feature itself, a reference to the group of the feature,
    the list of inputs hitting this feature, as well as a reference to the
    least complex of these inputs.
*/
pub struct AnalyzedFeature {
    pub key: FeatureIdx,
    inputs: Vec<SlabKey<Input>>,
    pub least_complex_input: SlabKey<Input>,
    pub least_complexity: f64,
    pub score: f64,
}

impl AnalyzedFeature {
    #[no_coverage]
    fn new(
        key: FeatureIdx,
        group: &FeatureGroup,
        inputs: Vec<SlabKey<Input>>,
        least_complex_input: SlabKey<Input>,
        least_complexity: f64,
    ) -> Self {
        let score = UniqueCoveragePool::score_of_feature(group.size(), inputs.len());
        Self {
            key,
            inputs,
            least_complex_input,
            least_complexity,
            score,
        }
    }
}

/**
    A reference to a FeatureInPool that can be used for fast searching and sorting.

    It contains a SlabKey to the FeatureInPool and a copy of the feature. By storing
    a copy of the feature, we can avoid indexing the slab and accessing the feature
    which saves time.
*/

#[derive(Clone)]
#[repr(transparent)]
pub struct AnalyzedFeatureRef {
    pub least_complexity: Option<f64>,
}

/**
    A unique identifier for a feature group.

    The identifier itself is a `Feature` whose payload has been removed.
    So it is a Feature with only a tag and an id.

    Because the payload is in the lower bits of the feature, we have the nice
    property that if two features f1 and f2 are related by: f1 < f2, the groups
    of those features are related by: g1 <= g2.

    Another way to put it is that a group identifier is equal to the first
    feature that could belong to that group (the one with a payload of 0).
*/
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub struct FeatureGroupId {
    id: FeatureIdx,
}

pub struct FeatureGroup {
    features: AHashSet<FeatureIdx>,
}
impl FeatureGroup {
    #[no_coverage]
    pub fn size(&self) -> usize {
        self.features.len()
    }
}
impl Default for FeatureGroup {
    #[no_coverage]
    fn default() -> Self {
        Self {
            features: Default::default(),
        }
    }
}

pub struct UniqueCoveragePool {
    pub name: String,

    pub features: Vec<AnalyzedFeatureRef>,

    pub slab_features: AHashMap<FeatureIdx, AnalyzedFeature>,

    pub feature_groups: AHashMap<FeatureGroupId, FeatureGroup>,

    slab_inputs: Slab<Input>,

    pub average_complexity: f64,
    pub total_score: f64,
    pub ranked_inputs: FenwickTree,

    rng: Rng,

    pub existing_features: Vec<FeatureIdx>,
    pub new_features: Vec<FeatureIdx>,
}

impl UniqueCoveragePool {
    #[no_coverage]
    pub fn new(name: &str, nbr_features: usize) -> Self {
        UniqueCoveragePool {
            name: name.to_string(),
            features: vec![AnalyzedFeatureRef { least_complexity: None }; nbr_features],
            slab_features: AHashMap::with_hasher(ahash::RandomState::with_seeds(0, 0, 0, 0)),

            feature_groups: AHashMap::with_hasher(ahash::RandomState::with_seeds(0, 0, 0, 0)),

            slab_inputs: Slab::new(),

            average_complexity: 0.0,
            total_score: 0.0,
            ranked_inputs: FenwickTree::new(vec![]),

            rng: fastrand::Rng::new(),

            existing_features: vec![],
            new_features: vec![],
        }
    }

    #[no_coverage]
    pub fn score(&self) -> f64 {
        self.total_score
    }

    #[allow(clippy::too_many_lines, clippy::type_complexity)]
    #[no_coverage]
    pub(crate) fn add(
        &mut self,
        data: PoolStorageIndex,
        complexity: f64,
        result: AnalysisResult,
    ) -> Option<(CorpusDelta, <Self as Pool>::Stats)> {
        let AnalysisResult {
            existing_features,
            new_features,
        } = result;

        if existing_features.is_empty() && new_features.is_empty() {
            return None;
        }

        let element = Input {
            least_complex_for_features: AHashSet::with_hasher(ahash::RandomState::with_seeds(0, 0, 0, 0)),
            all_features: vec![],
            score: 0.0,
            data,
            complexity,
            number_times_chosen: 1,
        };
        let element_key = self.slab_inputs.insert(element);

        let mut to_delete: AHashSet<SlabKey<Input>> = AHashSet::with_hasher(ahash::RandomState::with_seeds(0, 0, 0, 0));

        // 1. Update the `element.least_complex_for_features` fields of the elements affected
        // by a change in the `least_complexity` of the features in `existing_features`.
        // 1.1. If it turns out that an element is now no longer the least complex for any feature,
        // then add it to the list of elements to delete
        for feature_key in existing_features.iter() {
            let feature = self.slab_features.get_mut(feature_key).unwrap();

            if feature.least_complexity < complexity {
                continue;
            }

            for input_key in &feature.inputs {
                let affected_element = &mut self.slab_inputs[*input_key];
                affected_element.least_complex_for_features.remove(feature_key);
                if affected_element.least_complex_for_features.is_empty() {
                    to_delete.insert(*input_key);
                }
            }
            let element = &mut self.slab_inputs[element_key];

            element.least_complex_for_features.insert(*feature_key);
            feature.least_complex_input = element_key;
            self.features[feature_key.0] = AnalyzedFeatureRef {
                least_complexity: Some(complexity),
            };
            feature.least_complexity = complexity;
        }

        for feature_key in existing_features.iter() {
            let feature = self.slab_features.get_mut(feature_key).unwrap();
            let element = &mut self.slab_inputs[element_key];

            element.all_features.push(*feature_key);
            feature.inputs.push(element_key);
        }

        let mut affected_groups = AHashSet::with_hasher(ahash::RandomState::with_seeds(0, 0, 0, 0));

        // Now add in the new features
        let element = &mut self.slab_inputs[element_key];
        for &f in new_features.iter() {
            let new_feature_for_iter = AnalyzedFeatureRef {
                least_complexity: Some(complexity),
            };
            self.features[f.0] = new_feature_for_iter;

            let group_id = f.group_id();
            let group = self.feature_groups.entry(group_id).or_default();
            group.features.insert(f);

            let analyzed_f = AnalyzedFeature::new(f, group, vec![element_key], element_key, complexity);
            self.slab_features.insert(f, analyzed_f);

            element.all_features.push(f);
            element.least_complex_for_features.insert(f);

            affected_groups.insert(group_id);
        }

        let mut affected_features = AHashSet::<FeatureIdx>::new();
        for group_id in &affected_groups {
            let group = &self.feature_groups[&group_id];
            for feature_key in &group.features {
                affected_features.insert(*feature_key);
            }
        }

        let deleted_values: Vec<_> = to_delete
            .iter()
            .map(
                #[no_coverage]
                |&key| key,
            )
            .collect();
        let deleted_pool_storage_indices = deleted_values
            .iter()
            .map(
                #[no_coverage]
                |key| self.slab_inputs[*key].data,
            )
            .collect::<Vec<_>>();

        self.delete_elements(to_delete, &mut affected_groups, &mut affected_features, false);

        // now track the features whose scores are affected by the existing features
        for feature_key in existing_features.iter() {
            affected_features.insert(*feature_key);
        }
        // and update the score of every affected input
        for feature_key in affected_features.into_iter() {
            let feature = self.slab_features.get_mut(&feature_key).unwrap();
            let group = &self.feature_groups[&feature_key.group_id()];

            let old_score = feature.score;
            feature.score = Self::score_of_feature(group.size(), feature.inputs.len());
            let change_in_score = feature.score - old_score;

            for &input_key in &feature.inputs {
                let element_with_feature = &mut self.slab_inputs[input_key];
                element_with_feature.score += change_in_score;
            }
        }

        let element = &mut self.slab_inputs[element_key];
        element.score = 0.0;
        for f_key in &element.all_features {
            let analyzed_feature = self.slab_features.get_mut(f_key).unwrap();
            let group = &self.feature_groups[&f_key.group_id()];
            let feature_score = Self::score_of_feature(group.size(), analyzed_feature.inputs.len());
            element.score += feature_score;
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

    #[no_coverage]
    pub fn delete_elements(
        &mut self,
        to_delete: AHashSet<SlabKey<Input>>,
        affected_group: &mut AHashSet<FeatureGroupId>, // for now we assume that no feature is removed, which is incorrect for corpus reduction
        affected_features: &mut AHashSet<FeatureIdx>,
        may_remove_feature: bool,
    ) {
        for &to_delete_key in &to_delete {
            let to_delete_el = &self.slab_inputs[to_delete_key];

            for f in &to_delete_el.all_features {
                affected_features.insert(*f);
            }

            for &f_key in &to_delete_el.all_features {
                let analyzed_f = self.slab_features.get_mut(&f_key).unwrap();

                let idx_to_delete_key = analyzed_f
                    .inputs
                    .iter()
                    .position(
                        #[no_coverage]
                        |&x| x == to_delete_key,
                    )
                    .unwrap();
                analyzed_f.inputs.swap_remove(idx_to_delete_key);
            }

            if may_remove_feature {
                // iter through all features and, if they have no corresponding inputs, remove them from the pool
                for &f_key in &to_delete_el.all_features {
                    let analyzed_f = &self.slab_features[&f_key];
                    if !analyzed_f.inputs.is_empty() {
                        continue;
                    }
                    // remove the feature from the list and the slab
                    self.slab_features.remove(&f_key);
                    self.features[f_key.0].least_complexity = None;

                    // update the group and mark it as affected
                    // let analyzed_f = &self.slab_features[&f_key];
                    let group = self.feature_groups.get_mut(&f_key.group_id()).unwrap();
                    group.features.remove(&f_key);

                    affected_group.insert(f_key.group_id());
                    affected_features.remove(&f_key);
                }
            }

            self.slab_inputs.remove(to_delete_key);
        }
    }

    #[no_coverage]
    pub fn score_of_feature(group_size: usize, exact_feature_multiplicity: usize) -> f64 {
        score_for_group_size(group_size) / (group_size as f64 * exact_feature_multiplicity as f64)
    }

    /// Update global statistics of the pool following a change in its content
    #[no_coverage]
    fn update_self_stats(&mut self) {
        let slab = &self.slab_inputs;

        let ranked_inputs = self
            .slab_inputs
            .keys()
            .map(
                #[no_coverage]
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
                #[no_coverage]
                |key| slab[key].score,
            )
            .sum();

        self.average_complexity = self
            .slab_inputs
            .keys()
            .map(
                #[no_coverage]
                |key| &slab[key],
            )
            .fold(
                0.0,
                #[no_coverage]
                |c, x| c + x.complexity,
            )
            / self.slab_inputs.len() as f64;
    }

    #[cfg(test)]
    #[no_coverage]
    fn print_recap(&self) {
        println!("recap inputs:");
        for input_key in self.slab_inputs.keys() {
            let input = &self.slab_inputs[input_key];
            println!(
                "input with key {:?} has cplx {:.2}, score {:.2}, and features: {:?}",
                input_key, input.complexity, input.score, input.all_features
            );
            println!("        and is best for {:?}", input.least_complex_for_features);
        }
        println!("recap features:");
        for (f_idx, f) in &self.slab_features {
            println!("feature {:?}’s inputs: {:?}", f_idx, f.inputs);
        }
        println!("recap groups:");
        for (i, (_, group)) in self.feature_groups.iter().enumerate() {
            println!(
                "group {} has features {:?}",
                i,
                group.features.iter().collect::<Vec<_>>()
            );
        }
        println!("---");
    }

    #[cfg(test)]
    #[no_coverage]
    fn sanity_check(&self) {
        let slab = &self.slab_features;

        self.print_recap();

        for (f_key, f) in &self.slab_features {
            for input_key in &f.inputs {
                let input = &self.slab_inputs[*input_key];
                assert!(input.all_features.contains(&f_key));
            }
        }

        for input_key in self.slab_inputs.keys() {
            let input = &self.slab_inputs[input_key];
            assert!(input.score > 0.0);
            let expected_input_score = input.all_features.iter().fold(0.0, |c, fk| {
                let f = &slab[fk];
                let group = &self.feature_groups[&fk.group_id()];
                c + Self::score_of_feature(group.size(), f.inputs.len())
            });
            assert!(
                (input.score - expected_input_score).abs() < 0.01,
                "{:.2} != {:.2}",
                input.score,
                expected_input_score
            );
            assert!(!input.least_complex_for_features.is_empty());

            for f_key in &input.least_complex_for_features {
                let analyzed_f = &self.slab_features[f_key];

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

        // let mut dedupped_features = self.features.clone();
        // dedupped_features.sort();
        // dedupped_features.dedup();
        // assert_eq!(dedupped_features.len(), self.features.len());
    }
}

impl Pool for UniqueCoveragePool {
    type Stats = UniqueCoveragePoolStats;

    #[no_coverage]
    fn stats(&self) -> Self::Stats {
        UniqueCoveragePoolStats {
            name: self.name.clone(),
            score: self.score(),
            pool_size: self.slab_inputs.len(),
            avg_cplx: self.average_complexity,
            coverage: (self.feature_groups.len(), self.features.len()),
        }
    }

    #[no_coverage]
    fn len(&self) -> usize {
        self.slab_inputs.len()
    }

    #[no_coverage]
    fn get_random_index(&mut self) -> Option<PoolStorageIndex> {
        if self.ranked_inputs.len() == 0 {
            return None;
        }
        let most = self.ranked_inputs.prefix_sum(self.ranked_inputs.len() - 1);
        if most <= 0.0 {
            return None;
        }
        let chosen_weight = gen_f64(&self.rng, 0.0..most);

        // Find the first item which has a weight *higher* than the chosen weight.
        let choice = self.ranked_inputs.first_index_past_prefix_sum(chosen_weight);

        let key = self.slab_inputs.get_nth_key(choice);

        let input = &mut self.slab_inputs[key];
        let old_rank = input.score / (input.number_times_chosen as f64);
        input.number_times_chosen += 1;
        let new_rank = input.score / (input.number_times_chosen as f64);

        let delta = new_rank - old_rank;
        self.ranked_inputs.update(choice, delta);
        Some(input.data)
    }

    #[no_coverage]
    fn mark_test_case_as_dead_end(&mut self, idx: PoolStorageIndex) {
        for input_key in self.slab_inputs.keys() {
            let input = &mut self.slab_inputs[input_key];
            if input.data == idx {
                input.score = 0.0;
                break;
            }
        }
        self.update_self_stats()
    }

    #[no_coverage]
    fn serialized(&self) -> Vec<(PathBuf, Vec<u8>)> {
        let path = PathBuf::new().join(format!("{}.json", &self.name));

        let all_hit_counters = self
            .features
            .iter()
            .enumerate()
            .filter(|(_, x)| x.least_complexity.is_some())
            .map(|(idx, _)| idx)
            .collect::<Vec<_>>();

        let best_for_counter = self
            .features
            .iter()
            .enumerate()
            .filter(|(_, x)| x.least_complexity.is_some())
            .map(|(idx, _)| {
                let f = &self.slab_features[&FeatureIdx::new(idx)];
                let key = f.least_complex_input;
                let input = &self.slab_inputs[key].data;
                (idx, *input)
            })
            .collect::<Vec<_>>();

        let mut ranked_inputs = self
            .slab_inputs
            .keys()
            .map(|key| {
                let input = &self.slab_inputs[key];
                (input.data, input.score)
            })
            .collect::<Vec<_>>();
        ranked_inputs.sort_by(|x, y| x.1.partial_cmp(&y.1).unwrap_or(Ordering::Equal));
        let ranked_inputs = ranked_inputs.into_iter().map(|x| x.0).collect();

        let counters_for_input = self
            .slab_inputs
            .keys()
            .map(|key| {
                let input = &self.slab_inputs[key];
                (input.data, input.all_features.iter().map(|x| x.0).collect())
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
    }
}

#[derive(serde::Serialize, serde::Deserialize)]
struct SerializedUniqCov {
    all_hit_counters: Vec<usize>,
    best_for_counter: Vec<(usize, PoolStorageIndex)>,
    ranked_inputs: Vec<PoolStorageIndex>,
    counters_for_input: Vec<(PoolStorageIndex, Vec<usize>)>,
}

#[inline(always)]
#[no_coverage]
pub fn gen_f64(rng: &fastrand::Rng, range: Range<f64>) -> f64 {
    range.start + rng.f64() * (range.end - range.start)
}

#[no_coverage]
fn score_for_group_size(size: usize) -> f64 {
    const SCORES: [f64; 17] = [
        0.0, 1.0, 1.1, 1.2, 1.3, 1.4, 1.5, 1.55, 1.6, 1.65, 1.7, 1.75, 1.8, 1.85, 1.9, 1.95, 2.0,
    ];
    if size < 16 {
        SCORES[size]
    } else {
        2.0
    }
}

// ===============================================================
// ==================== Trait implementations ====================
// ===============================================================

impl Clone for AnalyzedFeature {
    #[no_coverage]
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
    #[no_coverage]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            format!(
                "{}({} cov: {}/{} cplx: {:.2})",
                self.name, self.pool_size, self.coverage.0, self.coverage.1, self.avg_cplx
            )
            .bright_green()
        )
    }
}
impl ToCSVFields for UniqueCoveragePoolStats {
    #[no_coverage]
    fn csv_headers(&self) -> Vec<CSVField> {
        vec![
            CSVField::String(format!("{}-size", self.name)),
            CSVField::String(format!("{}-percent-coverage", self.name)),
            CSVField::String(format!("{}-avg-cplx", self.name)),
        ]
    }
    #[no_coverage]
    fn to_csv_record(&self) -> Vec<CSVField> {
        vec![
            CSVField::Integer(self.pool_size as isize),
            CSVField::Integer(self.coverage.0 as isize),
            CSVField::Float(self.avg_cplx),
        ]
    }
}

#[derive(Default)]
pub struct UniqueCoveragePoolObservationState {
    is_interesting: bool,
    analysis_result: AnalysisResult,
}
#[derive(Default)]
pub struct AnalysisResult {
    pub(crate) existing_features: Vec<FeatureIdx>,
    pub(crate) new_features: Vec<FeatureIdx>,
}

impl CompatibleWithIteratorSensor for UniqueCoveragePool {
    type Observation = (usize, u64);
    type ObservationState = UniqueCoveragePoolObservationState;

    #[no_coverage]
    fn observe(
        &mut self,
        &(index, _counter): &Self::Observation,
        input_complexity: f64,
        state: &mut Self::ObservationState,
    ) {
        let feature_index = FeatureIdx::new(index);
        let AnalyzedFeatureRef { least_complexity } = unsafe { self.features.get_unchecked(feature_index.0) };
        if let Some(prev_least_complexity) = least_complexity {
            self.existing_features.push(feature_index);
            if input_complexity < *prev_least_complexity {
                state.is_interesting = true;
            }
        } else {
            self.new_features.push(feature_index);
            state.is_interesting = true;
        }
    }
    #[no_coverage]
    fn finish_observing(&mut self, state: &mut Self::ObservationState, _input_complexity: f64) {
        if state.is_interesting {
            state.analysis_result.new_features = self.new_features.clone();
            state.analysis_result.existing_features = self.existing_features.clone();
        }
        self.new_features.clear();
        self.existing_features.clear();
    }
    #[no_coverage]
    fn is_interesting(&self, observation_state: &Self::ObservationState, _input_complexity: f64) -> bool {
        observation_state.is_interesting
    }
    #[no_coverage]
    fn add(
        &mut self,
        data: PoolStorageIndex,
        complexity: f64,
        observation_state: Self::ObservationState,
    ) -> Vec<CorpusDelta> {
        let result = observation_state.analysis_result;
        self.add(data, complexity, result)
            .map(
                #[no_coverage]
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

    #[no_coverage]
    fn edge_f(index: usize, intensity: u16) -> FeatureIdx {
        FeatureIdx(index * 64 + intensity as usize)
    }

    #[test]
    #[no_coverage]
    fn property_test() {
        use std::iter::FromIterator;

        let mut list_features = vec![];
        for i in 0..3 {
            for j in 0..3 {
                list_features.push(edge_f(i, j));
            }
        }

        for _ in 0..1000 {
            let mut new_features: AHashSet<_, ahash::RandomState> = AHashSet::from_iter(list_features.iter());
            let mut added_features: Vec<FeatureIdx> = vec![];

            let mut pool = UniqueCoveragePool::new("cov", 1024);

            for i in 0..fastrand::usize(0..100) {
                let nbr_new_features = if new_features.is_empty() {
                    0
                } else if i == 0 {
                    fastrand::usize(1..new_features.len())
                } else {
                    fastrand::usize(0..new_features.len())
                };
                let new_features_1: Vec<_> = {
                    let mut fs = new_features.iter().map(|&&f| f).collect::<Vec<_>>();

                    fastrand::shuffle(&mut fs);
                    fs[0..nbr_new_features].to_vec()
                };

                for f in &new_features_1 {
                    new_features.remove(f);
                }
                let nbr_existing_features = if new_features_1.is_empty() {
                    if added_features.len() > 1 {
                        fastrand::usize(1..added_features.len())
                    } else {
                        1
                    }
                } else if added_features.is_empty() {
                    0
                } else {
                    fastrand::usize(0..added_features.len())
                };

                let existing_features_1: Vec<FeatureIdx> = {
                    let mut fs = added_features.clone();
                    fastrand::shuffle(&mut fs);
                    fs[0..nbr_existing_features].to_vec()
                };

                let max_cplx: f64 = if !existing_features_1.is_empty() && new_features_1.is_empty() {
                    let idx = fastrand::usize(0..existing_features_1.len());
                    let fs = existing_features_1
                        .iter()
                        .map(|&f_key| pool.slab_features[&f_key].least_complexity)
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
                for f in new_features_1.iter() {
                    added_features.push(*f);
                }

                let prev_score = pool.score();
                let analysis_result = AnalysisResult {
                    existing_features: existing_features_1,
                    new_features: new_features_1,
                };
                // println!("adding input of cplx {:.2} with new features {:?} and existing features {:?}", cplx1, new_features_1, existing_features_1);
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

        #[no_coverage]
        fn default_arbitrary_step(&self) -> Self::ArbitraryStep {}

        #[no_coverage]
        fn validate_value(&self, _value: &f64) -> Option<(Self::Cache, Self::MutationStep)> {
            Some(((), ()))
        }
        #[no_coverage]
        fn max_complexity(&self) -> f64 {
            0.0
        }

        #[no_coverage]
        fn min_complexity(&self) -> f64 {
            0.0
        }

        #[no_coverage]
        fn complexity(&self, _value: &f64, _cache: &Self::Cache) -> f64 {
            0.0
        }

        #[no_coverage]
        fn ordered_arbitrary(&self, _step: &mut Self::ArbitraryStep, _max_cplx: f64) -> Option<(f64, f64)> {
            todo!()
        }

        #[no_coverage]
        fn random_arbitrary(&self, _max_cplx: f64) -> (f64, f64) {
            (0.0, 0.0)
        }

        #[no_coverage]
        fn ordered_mutate(
            &self,
            _value: &mut f64,
            _cache: &mut Self::Cache,
            _step: &mut Self::MutationStep,
            _max_cplx: f64,
        ) -> Option<(Self::UnmutateToken, f64)> {
            Some(((), 0.0))
        }

        #[no_coverage]
        fn random_mutate(
            &self,
            _value: &mut f64,
            _cache: &mut Self::Cache,
            _max_cplx: f64,
        ) -> (Self::UnmutateToken, f64) {
            ((), 0.0)
        }

        #[no_coverage]
        fn unmutate(&self, _value: &mut f64, _cache: &mut Self::Cache, _t: Self::UnmutateToken) {}
    }
}
