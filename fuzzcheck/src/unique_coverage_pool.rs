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

use crate::coverage_sensor_and_pool::AnalysisResult;
use crate::data_structures::{Slab, SlabKey, WeightedIndex};
use crate::sensor_and_pool::{Pool, TestCase};
use crate::Feature;
use ahash::{AHashMap, AHashSet};
use fastrand::Rng;
use std::cmp::Ordering;
use std::fmt;
use std::hash::Hash;
use std::ops::{Range, RangeInclusive};

pub struct UniqueCoveragePoolEvent<T> {
    pub added_key: Option<SlabKey<Input<T>>>,
    pub removed_keys: Vec<SlabKey<Input<T>>>,
}

/**
 * An element stored in the pool, containing its value, cache, mutation step,
 * as well as analysed code coverage and computed score.
*/
pub struct Input<T> {
    /// The keys of the features for which there are no simpler inputs in the
    /// pool reaching the feature.
    least_complex_for_features: AHashSet<SlabKey<AnalyzedFeature<T>>>,
    /// Holds the key of each [FeatureInPool] associated with this input.
    all_features: Vec<SlabKey<AnalyzedFeature<T>>>,
    /// The computed score of the input
    pub score: f64,
    /// Data associated with the input: value, cache, and mutation step
    data: T,
    /// Cached complexity of the value.
    ///
    /// It should always be equal to [mutator.complexity(&self.data.value, &self.data.cache)](Mutator::complexity)
    complexity: f64,
}

/**
    An analysis of the role of a feature in the pool.

    It contains the feature itself, a reference to the group of the feature,
    the list of inputs hitting this feature, as well as a reference to the
    least complex of these inputs.
*/
pub struct AnalyzedFeature<T> {
    pub key: SlabKey<AnalyzedFeature<T>>,
    pub(crate) feature: Feature,
    inputs: Vec<SlabKey<Input<T>>>,
    pub least_complex_input: SlabKey<Input<T>>,
    pub least_complexity: f64,
    pub score: f64,
}

impl<T: TestCase> AnalyzedFeature<T> {
    #[no_coverage]
    fn new(
        key: SlabKey<Self>,
        feature: Feature,
        group: &FeatureGroup<T>,
        inputs: Vec<SlabKey<Input<T>>>,
        least_complex_input: SlabKey<Input<T>>,
        least_complexity: f64,
    ) -> Self {
        let score = UniqueCoveragePool::<T>::score_of_feature(group.size(), inputs.len());
        Self {
            key,
            feature,
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
pub struct AnalyzedFeatureRef<T> {
    pub key: SlabKey<AnalyzedFeature<T>>,
    pub(crate) feature: Feature,
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
#[derive(Clone, Copy, Eq, Debug, Hash)]
pub struct FeatureGroupId {
    id: Feature,
}
impl PartialEq for FeatureGroupId {
    #[inline(always)]
    #[no_coverage]
    fn eq(&self, other: &Self) -> bool {
        self.id.eq(&other.id)
    }
    #[inline(always)]
    #[no_coverage]
    fn ne(&self, other: &Self) -> bool {
        self.id.ne(&other.id)
    }
}
impl PartialOrd for FeatureGroupId {
    #[inline(always)]
    #[no_coverage]
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.id.partial_cmp(&other.id)
    }
    #[inline(always)]
    #[no_coverage]
    fn lt(&self, other: &Self) -> bool {
        self.id.lt(&other.id)
    }
    #[inline(always)]
    #[no_coverage]
    fn le(&self, other: &Self) -> bool {
        self.id.le(&other.id)
    }
    #[inline(always)]
    #[no_coverage]
    fn gt(&self, other: &Self) -> bool {
        self.id.gt(&other.id)
    }
    #[inline(always)]
    #[no_coverage]
    fn ge(&self, other: &Self) -> bool {
        self.id.ge(&other.id)
    }
}
impl Ord for FeatureGroupId {
    #[inline(always)]
    #[no_coverage]
    fn cmp(&self, other: &Self) -> Ordering {
        self.id.cmp(&other.id)
    }
    // I don't write the other methods, I don't think they are used here
}

impl Feature {
    #[no_coverage]
    fn group_id(self) -> FeatureGroupId {
        FeatureGroupId {
            // erase last 8 bits, which stand for the payload
            id: self.erasing_payload(),
        }
    }
}

pub struct FeatureGroup<T> {
    features: AHashSet<SlabKey<AnalyzedFeature<T>>>,
}
impl<T> FeatureGroup<T> {
    #[no_coverage]
    pub fn size(&self) -> usize {
        self.features.len()
    }
}
impl<T> Default for FeatureGroup<T> {
    #[no_coverage]
    fn default() -> Self {
        Self {
            features: Default::default(),
        }
    }
}

pub struct UniqueCoveragePool<T: TestCase> {
    pub features: Vec<AnalyzedFeatureRef<T>>,
    pub slab_features: Slab<AnalyzedFeature<T>>,

    pub feature_groups: AHashMap<FeatureGroupId, FeatureGroup<T>>,

    slab_inputs: Slab<Input<T>>,

    pub average_complexity: f64,
    cumulative_weights: Vec<f64>,

    pub features_range_for_coverage_index: Vec<Range<usize>>,

    rng: Rng,
}

impl<T: TestCase> UniqueCoveragePool<T> {
    #[no_coverage]
    pub fn new(sensor_index_ranges: &[RangeInclusive<usize>]) -> Self {
        let rng = fastrand::Rng::new();
        let mut pool = UniqueCoveragePool {
            features: Vec::new(),
            slab_features: Slab::new(),

            feature_groups: AHashMap::with_hasher(ahash::RandomState::with_seeds(0, 0, 0, 0)),

            // inputs: Vec::default(),
            slab_inputs: Slab::new(),

            average_complexity: 0.0,
            cumulative_weights: Vec::default(),

            features_range_for_coverage_index: Vec::default(),

            rng,
        };
        pool.update_feature_ranges_for_coverage(sensor_index_ranges);
        pool
    }

    #[no_coverage]
    pub fn score(&self) -> f64 {
        *self.cumulative_weights.last().unwrap_or(&0.0)
    }

    #[allow(clippy::too_many_lines, clippy::type_complexity)]
    #[no_coverage]
    pub(crate) fn add(
        &mut self,
        data: T,
        complexity: f64,
        result: AnalysisResult<T>,
        sensor_index_ranges: &[RangeInclusive<usize>],
    ) -> (Option<UniqueCoveragePoolEvent<T>>, Option<SlabKey<Input<T>>>) {
        let AnalysisResult {
            existing_features,
            new_features,
        } = result;

        if existing_features.is_empty() && new_features.is_empty() {
            return (None, None);
        }

        let element = Input {
            least_complex_for_features: AHashSet::with_hasher(ahash::RandomState::with_seeds(0, 0, 0, 0)),
            all_features: vec![],
            score: 0.0,
            data,
            complexity,
        };
        let element_key = self.slab_inputs.insert(element);

        let mut to_delete: AHashSet<SlabKey<Input<T>>> =
            AHashSet::with_hasher(ahash::RandomState::with_seeds(0, 0, 0, 0));

        // 1. Update the `element.least_complex_for_features` fields of the elements affected
        // by a change in the `least_complexity` of the features in `existing_features`.
        // 1.1. If it turns out that an element is now no longer the least complex for any feature,
        // then add it to the list of elements to delete
        for feature_key in existing_features.iter() {
            let feature = &mut self.slab_features[*feature_key];

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
            feature.least_complexity = complexity;
        }

        for feature_key in existing_features.iter() {
            let feature = &mut self.slab_features[*feature_key];
            let element = &mut self.slab_inputs[element_key];

            element.all_features.push(*feature_key);
            feature.inputs.push(element_key);
        }

        let mut affected_groups = AHashSet::with_hasher(ahash::RandomState::with_seeds(0, 0, 0, 0));

        // Now add in the new features
        let element = &mut self.slab_inputs[element_key];
        for &f in new_features.iter() {
            let f_key = self.slab_features.next_key();
            let new_feature_for_iter = AnalyzedFeatureRef { key: f_key, feature: f };
            sorted_insert(
                &mut self.features,
                new_feature_for_iter,
                #[no_coverage]
                |other_f| new_feature_for_iter.feature < other_f.feature,
            );

            let group_id = new_feature_for_iter.feature.group_id();
            let group = self.feature_groups.entry(group_id).or_default();
            group.features.insert(f_key);

            let analyzed_f = AnalyzedFeature::new(f_key, f, group, vec![element_key], element_key, complexity);
            self.slab_features.insert(analyzed_f);

            element.all_features.push(f_key);
            element.least_complex_for_features.insert(f_key);

            affected_groups.insert(group_id);
        }

        let mut affected_features = AHashSet::<SlabKey<AnalyzedFeature<T>>>::new();
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

        self.delete_elements(to_delete, &mut affected_groups, &mut affected_features, false);

        // now track the features whose scores are affected by the existing features
        for feature_key in existing_features.iter() {
            affected_features.insert(*feature_key);
        }
        // and update the score of every affected input
        for feature_key in affected_features.into_iter() {
            let feature = &mut self.slab_features[feature_key];
            let group = &self.feature_groups[&feature.feature.group_id()];

            let old_score = feature.score;
            feature.score = Self::score_of_feature(group.size(), feature.inputs.len());
            let change_in_score = feature.score - old_score;

            for &input_key in &feature.inputs {
                let element_with_feature = &mut self.slab_inputs[input_key];
                element_with_feature.score += change_in_score;
            }
        }

        let element = &mut self.slab_inputs[element_key];
        // TODO: test if this is necessary
        element.score = 0.0;
        for f_key in &element.all_features {
            let analyzed_feature = &mut self.slab_features[*f_key];
            let group = &self.feature_groups[&analyzed_feature.feature.group_id()];
            let feature_score = Self::score_of_feature(group.size(), analyzed_feature.inputs.len());
            element.score += feature_score;
        }

        let event = UniqueCoveragePoolEvent {
            added_key: Some(element_key),
            removed_keys: deleted_values,
        };

        self.update_stats();
        self.update_feature_ranges_for_coverage(sensor_index_ranges);

        // self.sanity_check();

        (Some(event), Some(element_key))
    }

    #[no_coverage]
    pub fn delete_elements(
        &mut self,
        to_delete: AHashSet<SlabKey<Input<T>>>,
        affected_group: &mut AHashSet<FeatureGroupId>, // for now we assume that no feature is removed, which is incorrect for corpus reduction
        affected_features: &mut AHashSet<SlabKey<AnalyzedFeature<T>>>,
        may_remove_feature: bool,
    ) {
        for &to_delete_key in &to_delete {
            let to_delete_el = &self.slab_inputs[to_delete_key];

            for f in &to_delete_el.all_features {
                affected_features.insert(*f);
            }

            for &f_key in &to_delete_el.all_features {
                let analyzed_f = &mut self.slab_features[f_key];

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
                    let analyzed_f = &self.slab_features[f_key];
                    if !analyzed_f.inputs.is_empty() {
                        continue;
                    }
                    // remove the feature from the list and the slab
                    let idx_f = self
                        .features
                        .binary_search_by_key(
                            &analyzed_f.feature,
                            #[no_coverage]
                            |f| f.feature,
                        )
                        .unwrap();
                    self.features.remove(idx_f);

                    let key = analyzed_f.key;
                    self.slab_features.remove(key);

                    // update the group and mark it as affected
                    let analyzed_f = &self.slab_features[f_key];
                    let group = self.feature_groups.get_mut(&analyzed_f.feature.group_id()).unwrap();
                    group.features.remove(&f_key);

                    affected_group.insert(analyzed_f.feature.group_id());
                }
            }

            self.slab_inputs.remove(to_delete_key);
        }
    }
    #[no_coverage]
    pub(crate) fn remove_lowest_scoring_input(&mut self) -> Option<UniqueCoveragePoolEvent<T>> {
        let slab = &self.slab_inputs;
        let pick_key = self
            .slab_inputs
            .keys()
            .min_by(
                #[no_coverage]
                |&k1, &k2| slab[k1].score.partial_cmp(&slab[k2].score).unwrap_or(Ordering::Less),
            )
            .unwrap();
        let mut to_delete = AHashSet::with_hasher(ahash::RandomState::with_seeds(0, 0, 0, 0));
        to_delete.insert(pick_key);
        let mut affected_groups = AHashSet::with_hasher(ahash::RandomState::with_seeds(0, 0, 0, 0));
        let mut affected_features = AHashSet::with_hasher(ahash::RandomState::with_seeds(0, 0, 0, 0));
        self.delete_elements(to_delete, &mut affected_groups, &mut affected_features, true);

        for group_id in affected_groups {
            let group = &self.feature_groups[&group_id];
            for feature_key in &group.features {
                affected_features.insert(*feature_key);
            }
        }

        for feature_key in affected_features.into_iter() {
            let feature = &mut self.slab_features[feature_key];
            let group = &self.feature_groups[&feature.feature.group_id()];

            let old_score = feature.score;
            feature.score = Self::score_of_feature(group.size(), feature.inputs.len());
            let change_in_score = feature.score - old_score;

            for &input_key in &feature.inputs {
                let element_with_feature = &mut self.slab_inputs[input_key];
                element_with_feature.score += change_in_score;
            }
        }

        let event = UniqueCoveragePoolEvent {
            added_key: None,
            removed_keys: vec![pick_key],
        };

        self.update_stats();

        Some(event)
    }

    #[no_coverage]
    pub fn score_of_feature(group_size: usize, exact_feature_multiplicity: usize) -> f64 {
        score_for_group_size(group_size) / (group_size as f64 * exact_feature_multiplicity as f64)
    }

    /// Update global statistics of the pool following a change in its content
    #[no_coverage]
    fn update_stats(&mut self) {
        let slab = &self.slab_inputs;
        self.cumulative_weights = self
            .slab_inputs
            .keys()
            .map(
                #[no_coverage]
                |key| &slab[key],
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

    #[no_coverage]
    pub fn update_feature_ranges_for_coverage(&mut self, indexes: &[RangeInclusive<usize>]) {
        let mut idx = self.features.len();
        self.features_range_for_coverage_index.clear();
        for index_range in indexes.iter().rev() {
            let first_feature = Feature::new(*index_range.start(), 0);
            // let last_feature = Feature::new(*index_range.end(), 0);

            if let Some(first_index) = self.features[..idx].iter().rposition(|&f| f.feature < first_feature) {
                self.features_range_for_coverage_index.push(first_index + 1..idx);
                idx = first_index + 1;
            } else {
                // TODO
                self.features_range_for_coverage_index.push(0..idx);
                idx = 0;
            }
        }
        self.features_range_for_coverage_index.reverse();
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
        for &f_iter in &self.features {
            let f_key = f_iter.key;
            let analyzed_f = &self.slab_features[f_key];
            println!("feature {:?}’s inputs: {:?}", f_key, analyzed_f.inputs);
        }
        println!("recap groups:");
        for (i, (_, group)) in self.feature_groups.iter().enumerate() {
            let slab = &self.slab_features;
            println!(
                "group {} has features {:?}",
                i,
                group.features.iter().map(|f| &slab[*f]).collect::<Vec<_>>()
            );
        }
        println!("---");
    }

    #[cfg(test)]
    #[no_coverage]
    fn sanity_check(&self) {
        let slab = &self.slab_features;

        self.print_recap();

        let fs = self
            .features
            .iter()
            .map(|f_iter| self.slab_features[f_iter.key].feature)
            .collect::<Vec<_>>();
        assert!(fs.is_sorted());

        for f_iter in &self.features {
            let f_key = f_iter.key;
            let analyzed_f = &self.slab_features[f_key];
            for input_key in &analyzed_f.inputs {
                let input = &self.slab_inputs[*input_key];
                assert!(input.all_features.contains(&f_key));
            }
        }

        for input_key in self.slab_inputs.keys() {
            let input = &self.slab_inputs[input_key];
            assert!(input.score > 0.0);
            let expected_input_score = input.all_features.iter().fold(0.0, |c, &fk| {
                let f = &slab[fk];
                let group = &self.feature_groups[&f.feature.group_id()];
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
                let analyzed_f = &self.slab_features[*f_key];

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

impl<T: TestCase> Pool for UniqueCoveragePool<T> {
    type TestCase = T;
    type Index = SlabKey<Input<T>>;

    #[no_coverage]
    fn len(&self) -> usize {
        self.slab_inputs.len()
    }

    #[no_coverage]
    fn get_random_index(&self) -> Option<Self::Index> {
        if self.cumulative_weights.last().unwrap_or(&0.0) > &0.0 {
            let dist = WeightedIndex {
                cumulative_weights: &self.cumulative_weights,
            };
            let x = dist.sample(&self.rng);
            let key = self.slab_inputs.get_nth_key(x);
            Some(key)
        } else {
            None
        }
    }

    #[no_coverage]
    fn get(&self, idx: Self::Index) -> &Self::TestCase {
        &self.slab_inputs[idx].data
    }

    #[no_coverage]
    fn get_mut(&mut self, idx: Self::Index) -> &mut Self::TestCase {
        &mut self.slab_inputs[idx].data
    }

    #[no_coverage]
    fn retrieve_after_processing(&mut self, idx: Self::Index, generation: usize) -> Option<&mut Self::TestCase> {
        if let Some(input) = self.slab_inputs.get_mut(idx) {
            assert!(
                input.data.generation() == generation,
                "{} {}",
                input.data.generation(),
                generation
            );
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
    fn mark_test_case_as_dead_end(&mut self, idx: SlabKey<Input<T>>) {
        let input = &mut self.slab_inputs[idx];
        input.score = 0.0;
        self.update_stats()
    }
}

/// Add the element in the correct place in the sorted vector
#[no_coverage]
fn sorted_insert<T, F>(vec: &mut Vec<T>, element: T, is_before: F) -> usize
where
    F: Fn(&T) -> bool,
{
    let mut insertion = 0;
    for e in vec.iter() {
        if is_before(e) {
            break;
        }
        insertion += 1;
    }
    vec.insert(insertion, element);
    insertion
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

impl<T> Clone for AnalyzedFeature<T> {
    #[no_coverage]
    fn clone(&self) -> Self {
        Self {
            key: self.key,
            feature: self.feature,
            inputs: self.inputs.clone(),
            least_complex_input: self.least_complex_input,
            least_complexity: self.least_complexity,
            score: self.score,
        }
    }
}
impl<T> PartialEq for AnalyzedFeature<T> {
    #[no_coverage]
    fn eq(&self, other: &Self) -> bool {
        self.key == other.key
            && self.feature == other.feature
            && self.inputs == other.inputs
            && self.least_complex_input == other.least_complex_input
            && self.least_complexity == other.least_complexity
    }
}
impl<T> fmt::Debug for AnalyzedFeature<T> {
    #[no_coverage]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Feature {{ {:?}, f: {:#b}, inputs: {:?}, least_cplx: {:.2}, score: {:.2} }}",
            self.key, self.feature.0, self.inputs, self.least_complexity, self.score
        )
    }
}
impl<T> PartialEq for AnalyzedFeatureRef<T> {
    #[no_coverage]
    fn eq(&self, other: &Self) -> bool {
        self.feature == other.feature
    }
}
impl<T> Eq for AnalyzedFeatureRef<T> {}
impl<T> PartialOrd for AnalyzedFeatureRef<T> {
    #[no_coverage]
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.feature.partial_cmp(&other.feature)
    }
}
impl<T> Ord for AnalyzedFeatureRef<T> {
    #[no_coverage]
    fn cmp(&self, other: &Self) -> Ordering {
        self.feature.cmp(&other.feature)
    }
}

impl<T> Clone for AnalyzedFeatureRef<T> {
    #[no_coverage]
    fn clone(&self) -> Self {
        Self {
            key: self.key,
            feature: self.feature,
        }
    }
}

impl<T> Copy for AnalyzedFeatureRef<T> {}

#[cfg(test)]
mod tests {
    use crate::Mutator;

    use super::*;
    use std::collections::BTreeSet;

    #[no_coverage]
    fn mock(cplx: f64) -> f64 {
        cplx
    }

    #[no_coverage]
    fn edge_f(index: usize, intensity: u16) -> Feature {
        Feature::new(index, intensity as u64)
    }

    type FK = SlabKey<AnalyzedFeature<f64>>;

    impl TestCase for f64 {
        #[no_coverage]
        fn generation(&self) -> usize {
            0
        }
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
            let mut new_features = BTreeSet::from_iter(list_features.iter());
            let mut added_features: Vec<FK> = vec![];

            let mut pool = UniqueCoveragePool::<f64>::new(&[]);

            for i in 0..fastrand::usize(0..100) {
                let nbr_new_features = if new_features.is_empty() {
                    0
                } else if i == 0 {
                    fastrand::usize(1..new_features.len())
                } else {
                    fastrand::usize(0..new_features.len())
                };
                let mut new_features_1: Vec<_> = {
                    let mut fs = new_features.iter().map(|&&f| f).collect::<Vec<_>>();

                    fastrand::shuffle(&mut fs);
                    fs[0..nbr_new_features].to_vec()
                };

                new_features_1.sort();
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

                let mut existing_features_1: Vec<FK> = {
                    let mut fs = added_features.clone();
                    fastrand::shuffle(&mut fs);
                    fs[0..nbr_existing_features].to_vec()
                };

                let slab = &pool.slab_features;
                existing_features_1.sort_by(|&fk1, &fk2| slab[fk1].feature.cmp(&slab[fk2].feature));

                let max_cplx: f64 = if !existing_features_1.is_empty() && new_features_1.is_empty() {
                    let idx = fastrand::usize(0..existing_features_1.len());
                    let fs = existing_features_1
                        .iter()
                        .map(|&f_key| pool.slab_features[f_key].least_complexity)
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
                for _ in 0..new_features_1.len() {
                    added_features.push(added_features.last().map_or(FK::new(0), |x| FK::new(x.key + 1)));
                }

                let prev_score = pool.score();
                let analysis_result = AnalysisResult {
                    existing_features: existing_features_1,
                    new_features: new_features_1,
                };
                // println!("adding input of cplx {:.2} with new features {:?} and existing features {:?}", cplx1, new_features_1, existing_features_1);
                let _ = pool.add(mock(cplx1), cplx1, analysis_result, &[]);
                // pool.print_recap();
                pool.sanity_check();
                assert!(
                    (pool.score() - prev_score) > -0.01,
                    "{:.3} > {:.3}",
                    prev_score,
                    pool.score()
                );
            }
            for _ in 0..pool.len() {
                let prev_score = pool.score();
                let _ = pool.remove_lowest_scoring_input();
                pool.sanity_check();
                assert!(
                    (prev_score - pool.score()) > -0.01,
                    "{:.3} < {:.3}",
                    prev_score,
                    pool.score()
                );
            }
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
