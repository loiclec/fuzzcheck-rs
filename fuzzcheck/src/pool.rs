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
//! 3. It has the same size as the smallest input containing a particular
//! Feature, but it is estimated that it will be higher-scoring than that
//! previous smallest input.
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
//! For example, “edge” features with different intensities but the same
//! Program Counter (PC) belong to the same group, and divide the score of the
//! group among themselves.
//!
//! Let's say Group G3 correspond to edge features of the PC “467” and that
//! there are 5 different features belonging to that group: F1, F2, F3, F4, F5.
//! The score associated with each feature is `(group.score() / 5)`.
//! Now imagine the feature f4 appears in 3 different inputs. Each input will
//! thus gain ((group.score() / 5) / 3) from having the feature f4.

use std::cmp::Ordering;
use std::collections::BTreeSet;
#[cfg(feature = "ui")]
use std::collections::HashMap;
use std::fmt;
use std::ops::Range;

extern crate fastrand;
#[cfg(feature = "ui")]
use backtrace::BacktraceSymbol;
use fastrand::Rng;

use crate::data_structures::{Slab, SlabKey, WeightedIndex};
use crate::fuzzer::AnalysisResult;
use crate::world::WorldAction;
use crate::{Feature, FuzzedInput, Mutator};
use fuzzcheck_common::FuzzerEvent;

/// Index of an input in the Pool
pub enum PoolIndex<T: Clone, M: Mutator<T>> {
    Normal(SlabKey<Input<T, M>>),
    Favored,
    LowestStack,
}

impl<T: Clone, M: Mutator<T>> Clone for PoolIndex<T, M> {
    fn clone(&self) -> Self {
        match self {
            PoolIndex::Normal(idx) => PoolIndex::Normal(*idx),
            PoolIndex::Favored => PoolIndex::Favored,
            PoolIndex::LowestStack => PoolIndex::LowestStack,
        }
    }
}
impl<T: Clone, M: Mutator<T>> Copy for PoolIndex<T, M> {}

/**
 * An element stored in the pool, containing its value, cache, mutation step,
 * as well as analysed code coverage and computed score.
*/
pub struct Input<T: Clone, M: Mutator<T>> {
    /// The keys of the features for which there are no simpler inputs in the
    /// pool reaching the feature.
    least_complex_for_features: BTreeSet<SlabKey<AnalyzedFeature<T, M>>>,
    /// Holds the key of each [FeatureInPool] associated with this input.
    all_features: Vec<SlabKey<AnalyzedFeature<T, M>>>,
    /// The computed score of the input
    score: f64,
    /// Data associated with the input: value, cache, and mutation step
    data: FuzzedInput<T, M>,
    /// Cached complexity of the value.
    ///
    /// It should always be equal to [mutator.complexity(&self.data.value, &self.data.cache)](Mutator::complexity)
    complexity: f64,
    /// The corresponding index of the input in [pool.inputs](self::Pool::inputs)
    idx_in_pool: usize,
}

/**
    An analysis of the role of a feature in the pool.

    It contains the feature itself, a reference to the group of the feature,
    the list of inputs hitting this feature, as well as a reference to the
    least complex of these inputs.
*/
pub struct AnalyzedFeature<T: Clone, M: Mutator<T>> {
    pub key: SlabKey<AnalyzedFeature<T, M>>,
    pub(crate) feature: Feature,
    group_key: SlabKey<FeatureGroup>,
    inputs: Vec<SlabKey<Input<T, M>>>,
    least_complex_input: SlabKey<Input<T, M>>,
    pub least_complexity: f64,
    /// cache used when deleting inputs to know how to evolve the score of inputs
    old_multiplicity: usize,
}

impl<T: Clone, M: Mutator<T>> AnalyzedFeature<T, M> {
    fn new(
        key: SlabKey<Self>,
        feature: Feature,
        group_key: SlabKey<FeatureGroup>,
        inputs: Vec<SlabKey<Input<T, M>>>,
        least_complex_input: SlabKey<Input<T, M>>,
        least_complexity: f64,
    ) -> Self {
        let old_multiplicity = inputs.len();
        Self {
            key,
            feature,
            group_key,
            inputs,
            least_complex_input,
            least_complexity,
            old_multiplicity,
        }
    }
}

/**
    A reference to a FeatureInPool that can be used for fast searching and sorting.

    It contains a SlabKey to the FeatureInPool and a copy of the feature. By storing
    a copy of the feature, we can avoid indexing the slab and accessing the feature
    which saves time.
*/
pub struct AnalyzedFeatureRef<T: Clone, M: Mutator<T>> {
    pub key: SlabKey<AnalyzedFeature<T, M>>,
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
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
struct FeatureGroupId {
    id: Feature,
}

impl Feature {
    fn group_id(self) -> FeatureGroupId {
        FeatureGroupId {
            // erase last 8 bits, which stand for the payload
            id: self.erasing_payload(),
        }
    }
}

pub struct FeatureGroup {
    id: FeatureGroupId,
    /// Indices of the features belonging to the group in the vector `pool.features`.
    idcs: Range<usize>,
    /// cache used when adding or remocing features to know how to evolve the score of affected inputs
    old_size: usize,
}
impl FeatureGroup {
    fn new(id: FeatureGroupId, idcs: Range<usize>) -> Self {
        let old_size = idcs.end - idcs.start;
        Self { id, idcs, old_size }
    }
    pub fn size(&self) -> usize {
        self.idcs.end - self.idcs.start
    }
}

pub struct LowestStackInput<T: Clone, M: Mutator<T>> {
    input: FuzzedInput<T, M>,
    stack_depth: usize,
    generation: usize,
}

pub struct Pool<T: Clone, M: Mutator<T>> {
    pub features: Vec<AnalyzedFeatureRef<T, M>>,
    pub slab_features: Slab<AnalyzedFeature<T, M>>,

    feature_groups: Vec<SlabKey<FeatureGroup>>,
    pub slab_feature_groups: Slab<FeatureGroup>,

    inputs: Vec<SlabKey<Input<T, M>>>,
    slab_inputs: Slab<Input<T, M>>,

    favored_input: Option<FuzzedInput<T, M>>,
    lowest_stack_input: Option<LowestStackInput<T, M>>,

    pub average_complexity: f64,
    cumulative_weights: Vec<f64>,
    rng: Rng,
}

impl<T: Clone, M: Mutator<T>> Pool<T, M> {
    pub fn default() -> Self {
        let rng = fastrand::Rng::new();
        Pool {
            features: Vec::new(),
            slab_features: Slab::new(),

            feature_groups: Vec::default(),
            slab_feature_groups: Slab::new(),

            inputs: Vec::default(),
            slab_inputs: Slab::new(),

            favored_input: None,
            lowest_stack_input: None,

            average_complexity: 0.0,
            cumulative_weights: Vec::default(),
            rng,
        }
    }

    pub(crate) fn add_favored_input(&mut self, data: FuzzedInput<T, M>) {
        self.favored_input = Some(data);
    }

    pub fn score(&self) -> f64 {
        *self.cumulative_weights.last().unwrap_or(&0.0)
    }

    pub fn lowest_stack(&self) -> usize {
        self.lowest_stack_input.as_ref().map_or(usize::MAX, |x| x.stack_depth)
    }

    #[allow(clippy::too_many_lines)]
    pub(crate) fn add(
        &mut self,
        data: FuzzedInput<T, M>,
        complexity: f64,
        result: AnalysisResult<T, M>,
        _generation: usize,
    ) -> (Vec<WorldAction<T>>, Option<SlabKey<Input<T, M>>>) {
        let AnalysisResult {
            existing_features,
            new_features,
            _lowest_stack: _,
        } = result;

        let mut actions: Vec<WorldAction<T>> = Vec::new();

        // TODO: reenable stack tracing
        // if lowest_stack < self.lowest_stack() {
        //     let new = LowestStackInput {
        //         input: data.clone(),
        //         stack_depth: lowest_stack,
        //         generation,
        //     };
        //     let old = self.lowest_stack_input.replace(new);

        //     actions.push(WorldAction::Add(data.value.clone()));
        //     if let Some(old) = old {
        //         actions.push(WorldAction::Remove(old.input.value))
        //     }
        //     actions.push(WorldAction::ReportEvent(FuzzerEvent::ReplaceLowestStack(lowest_stack)));
        // }

        if existing_features.is_empty() && new_features.is_empty() {
            return (actions, None);
        }

        let element_key: SlabKey<Input<T, M>> = {
            let element = Input {
                least_complex_for_features: BTreeSet::default(),
                all_features: vec![],
                score: 0.0,
                data,
                complexity,
                idx_in_pool: self.inputs.len(),
            };
            let i_key = self.slab_inputs.insert(element);
            self.inputs.push(i_key);

            i_key
        };

        let mut to_delete: Vec<SlabKey<Input<T, M>>> = vec![];

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
                if affected_element.complexity >= complexity {
                    // add (element, feature_key) to list [(Element, [Feature])]
                    // binary search element there, then add feature to the end of it

                    // TODO: change this!
                    // instead, make list of elements to remove feature_key from
                    // and then process them all at once?
                    // and also for each element in this list a list of features to delete
                    affected_element.least_complex_for_features.remove(feature_key);
                    if affected_element.least_complex_for_features.is_empty() {
                        // then this will only be called once by element
                        to_delete.push(*input_key);
                    }
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

        let element = &mut self.slab_inputs[element_key];
        for &f in new_features.iter() {
            let f_key = self.slab_features.next_key();

            let new_feature_for_iter = AnalyzedFeatureRef { key: f_key, feature: f };
            let group_key = Self::insert_feature(
                &mut self.features,
                &mut self.feature_groups,
                &mut self.slab_feature_groups,
                new_feature_for_iter,
            );

            let analyzed_f = AnalyzedFeature::new(f_key, f, group_key, vec![element_key], element_key, complexity);
            self.slab_features.insert(analyzed_f);

            element.all_features.push(f_key);
            element.least_complex_for_features.insert(f_key);
        }

        to_delete.sort();
        to_delete.dedup();

        let deleted_values: Vec<_> = to_delete
            .iter()
            .map(|&key| self.slab_inputs[key].data.value.clone())
            .collect();

        self.delete_elements(to_delete, element_key);

        // iterate over new elements and change score for new group sizes
        let mut new_features_iter = new_features.iter().peekable();

        while let Some(&&next_feature) = new_features_iter.peek() {
            let feature_for_iter_idx = self
                .features
                .binary_search_by_key(&next_feature, |f| f.feature)
                .unwrap();
            let feature_for_iter = &self.features[feature_for_iter_idx];
            let group = {
                let analyzed_feature = &mut self.slab_features[feature_for_iter.key];
                &mut self.slab_feature_groups[analyzed_feature.group_key]
            };

            for f_ref in self.features[group.idcs.clone()].iter() {
                let feature_key = f_ref.key;
                let analyzed_feature = &mut self.slab_features[feature_key];

                let old_feature_score = Self::score_of_feature(group.old_size, analyzed_feature.old_multiplicity);
                let new_feature_score = Self::score_of_feature(group.size(), analyzed_feature.inputs.len());
                let change_in_score = new_feature_score - old_feature_score;

                for &input_key in &analyzed_feature.inputs {
                    if input_key != element_key {
                        let element_with_feature = &mut self.slab_inputs[input_key];
                        element_with_feature.score += change_in_score;
                    }
                }

                // reset old_multiplicity as it is not needed anymore and will need to be correct
                // for the next call to pool.add
                analyzed_feature.old_multiplicity = analyzed_feature.inputs.len();
            }

            let prev_feature = self.slab_features[feature_for_iter.key].feature;

            while let Some(&&next_feature) = new_features_iter.peek() {
                let feature_for_iter_idx = self
                    .features
                    .binary_search_by_key(&next_feature, |f| f.feature)
                    .unwrap();
                let feature_for_iter = &self.features[feature_for_iter_idx];

                if feature_for_iter.feature.group_id() == prev_feature.group_id() {
                    let _ = new_features_iter.next();
                    continue;
                } else {
                    break;
                }
            }

            group.old_size = group.size();
        }

        for feature_key in existing_features.iter() {
            let analyzed_feature = &mut self.slab_features[*feature_key];

            let group = &self.slab_feature_groups[analyzed_feature.group_key];

            let old_feature_score = Self::score_of_feature(group.old_size, analyzed_feature.old_multiplicity);
            let new_feature_score = Self::score_of_feature(group.size(), analyzed_feature.inputs.len());

            let change_in_score = new_feature_score - old_feature_score;

            for &input_key in &analyzed_feature.inputs {
                if input_key != element_key {
                    let element_with_feature = &mut self.slab_inputs[input_key];
                    element_with_feature.score += change_in_score;
                }
            }
            analyzed_feature.old_multiplicity = analyzed_feature.inputs.len();
        }

        let element = &mut self.slab_inputs[element_key];

        for f_key in &element.all_features {
            let analyzed_feature = &mut self.slab_features[*f_key];
            let group = &self.slab_feature_groups[analyzed_feature.group_key];
            let feature_score = Self::score_of_feature(group.size(), analyzed_feature.inputs.len());
            element.score += feature_score;
        }

        let value = element.data.value.clone();

        if deleted_values.is_empty() {
            actions.push(WorldAction::ReportEvent(FuzzerEvent::New));
            actions.push(WorldAction::Add(value));
        } else {
            actions.push(WorldAction::ReportEvent(FuzzerEvent::Replace(deleted_values.len())));
        }

        for i in deleted_values {
            actions.push(WorldAction::Remove(i));
        }

        self.update_stats();

        // self.sanity_check();

        (actions, Some(element_key))
    }

    pub fn delete_elements(
        &mut self,
        to_delete: Vec<SlabKey<Input<T, M>>>,
        should_not_update_key: SlabKey<Input<T, M>>,
    ) {
        for &to_delete_key in &to_delete {
            let to_swap_idx = self.inputs.len() - 1;
            let to_swap_key = *self.inputs.last().unwrap();
            // println!("will delete input with key {}", to_delete_key);
            let to_delete_idx = self.slab_inputs[to_delete_key].idx_in_pool;

            let to_swap_el = &mut self.slab_inputs[to_swap_key];
            to_swap_el.idx_in_pool = to_delete_idx;

            self.inputs.swap(to_delete_idx, to_swap_idx);
            self.inputs.pop();

            let to_delete_el = &self.slab_inputs[to_delete_key];
            // to_delete_el.idx_in_pool = to_swap_idx; // not necessary, element will be deleted

            // TODO: not ideal to clone all features
            let all_features = to_delete_el.all_features.clone();

            for &f_key in &all_features {
                let analyzed_f = &mut self.slab_features[f_key];

                let idx_to_delete_key = analyzed_f.inputs.iter().position(|&x| x == to_delete_key).unwrap();
                analyzed_f.inputs.swap_remove(idx_to_delete_key);

                let group = &self.slab_feature_groups[analyzed_f.group_key];

                // note: assume that group size hasn't changed. this is true because we are not adding or removing features
                let new_feature_score = Self::score_of_feature(group.old_size, analyzed_f.inputs.len());
                let old_feature_score = Self::score_of_feature(group.old_size, analyzed_f.old_multiplicity);
                let change_in_score = new_feature_score - old_feature_score;

                for input_key in &analyzed_f.inputs {
                    if *input_key != should_not_update_key {
                        let element_with_feature = &mut self.slab_inputs[*input_key];
                        element_with_feature.score += change_in_score;
                    }
                }
                analyzed_f.old_multiplicity = analyzed_f.inputs.len();
            }
            self.slab_inputs.remove(to_delete_key);
        }
    }

    pub fn delete_element(&mut self, to_delete_key: SlabKey<Input<T, M>>) {
        //          TODO:
        // * remove element from the list of inputs
        // * iter through all features and remove the input from their list of inputs
        // * iter through all features and, if they have no corresponding inputs, remove them from the pool

        //          To remove a feature from the pool:
        // * iter through all features in its group and update the score of their inputs, because the group size has changed
        // * no need to have a special case for the removed feature, we're removing it because there are no inputs that contain it, so we don't need to update their scores
        // * remove the feature from the list and the slab
        // * update the indices and old_size of the group
        // * also update the indices of all the following groups

        // * iter through all features, and update the score of each affected input because the feature multiplicity has changed
        // * update the feature old multiplicity
        // * remove element from the slab of inputs

        // 1. remove element from the list of inputs
        let to_swap_idx = self.inputs.len() - 1;
        let to_swap_key = *self.inputs.last().unwrap();
        // println!("will delete input with key {}", to_delete_key);
        let to_delete_idx = self.slab_inputs[to_delete_key].idx_in_pool;

        let to_swap_el = &mut self.slab_inputs[to_swap_key];
        to_swap_el.idx_in_pool = to_delete_idx;

        self.inputs.swap(to_delete_idx, to_swap_idx);
        self.inputs.pop();

        let to_delete_el = &self.slab_inputs[to_delete_key];
        // to_delete_el.idx_in_pool = to_swap_idx; // not necessary, element will be deleted

        // 2. iter through all features and remove the input from their list of inputs
        let all_features = to_delete_el.all_features.clone();

        for &f_key in &all_features {
            let analyzed_f = &mut self.slab_features[f_key];
            let idx_to_delete_key = analyzed_f.inputs.iter().position(|&x| x == to_delete_key).unwrap();
            analyzed_f.inputs.swap_remove(idx_to_delete_key); // this updates new multiplicity
        }

        // 3. iter through all features and, if they have no corresponding inputs, remove them from the pool
        for &f_key in &all_features {
            let analyzed_f = &self.slab_features[f_key];
            if !analyzed_f.inputs.is_empty() {
                continue;
            }

            let group = &self.slab_feature_groups[analyzed_f.group_key];
            //          To remove a feature from the pool:
            // 1. iter through all features in its group and update the score of their inputs, because the group size has changed
            for f_ref in self.features[group.idcs.clone()].iter() {
                let feature_key = f_ref.key;
                let analyzed_feature = &mut self.slab_features[feature_key];

                let old_feature_score = Self::score_of_feature(group.old_size, analyzed_feature.old_multiplicity); // feature multiplicity did
                let new_feature_score = Self::score_of_feature(group.old_size - 1, analyzed_feature.old_multiplicity); // not change yet
                let change_in_score = new_feature_score - old_feature_score;

                for &input_key in &analyzed_feature.inputs {
                    let element_with_feature = &mut self.slab_inputs[input_key];
                    element_with_feature.score += change_in_score;
                }
            }

            let analyzed_f = &self.slab_features[f_key];
            // 2. remove the feature from the list and the slab
            let idx_f = self
                .features
                .binary_search_by_key(&analyzed_f.feature, |f| f.feature)
                .unwrap();
            self.features.remove(idx_f);

            let key = analyzed_f.key;
            self.slab_features.remove(key);

            // 3. update the indices and old_size of the group
            let analyzed_f = &self.slab_features[f_key];
            let group = &mut self.slab_feature_groups[analyzed_f.group_key];
            group.idcs.end -= 1;
            group.old_size = group.size();
            //let group_index = self.feature_groups.binary_search_by_key
            // 4. also update the indices of all the following groups
            let id = group.id;
            let slab_feature_groups = &self.slab_feature_groups;

            let group_index = self
                .feature_groups
                .binary_search_by_key(&id, |g| slab_feature_groups[*g].id)
                .unwrap();
            for group_key in self.feature_groups[group_index + 1..].iter_mut() {
                let group = &mut self.slab_feature_groups[*group_key];
                group.idcs.end -= 1;
                group.idcs.start -= 1;
            }
        }
        // 4. iter through all features, and update the score of each affected input because the feature multiplicity has changed
        for &f_key in &all_features {
            let analyzed_feature = &mut self.slab_features[f_key];

            let group = &self.slab_feature_groups[analyzed_feature.group_key];

            let old_feature_score = Self::score_of_feature(group.old_size, analyzed_feature.old_multiplicity);
            let new_feature_score = Self::score_of_feature(group.old_size, analyzed_feature.inputs.len());

            let change_in_score = new_feature_score - old_feature_score;

            for &input_key in &analyzed_feature.inputs {
                let element_with_feature = &mut self.slab_inputs[input_key];
                element_with_feature.score += change_in_score;
            }
            // 5. update the feature old multiplicity
            analyzed_feature.old_multiplicity = analyzed_feature.inputs.len();
        }

        // 6. remove element from the slab of inputs
        self.slab_inputs.remove(to_delete_key);
    }

    pub(crate) fn remove_lowest_scoring_input(&mut self) -> Vec<WorldAction<T>> {
        let slab = &self.slab_inputs;
        let pick_key = self
            .inputs
            .iter()
            .min_by(|&&k1, &&k2| slab[k1].score.partial_cmp(&slab[k2].score).unwrap_or(Ordering::Less))
            .copied()
            .unwrap();

        let deleted_value = self.slab_inputs[pick_key].data.value.clone();

        self.delete_element(pick_key);

        let mut actions: Vec<WorldAction<T>> = Vec::new();
        actions.push(WorldAction::ReportEvent(FuzzerEvent::Remove));
        actions.push(WorldAction::Remove(deleted_value));

        self.update_stats();

        actions
    }

    /// Returns the index of the group of the feature
    fn insert_feature(
        features: &mut Vec<AnalyzedFeatureRef<T, M>>,
        feature_groups: &mut Vec<SlabKey<FeatureGroup>>,
        slab_feature_groups: &mut Slab<FeatureGroup>,
        new_feature_for_iter: AnalyzedFeatureRef<T, M>,
    ) -> SlabKey<FeatureGroup> {
        // TODO: CHANGE THIS, too slow
        let insertion_idx = sorted_insert(features, new_feature_for_iter, |other_f| {
            new_feature_for_iter.feature < other_f.feature
        });

        let group_of_new_feature = new_feature_for_iter.feature.group_id();

        let (group_index, group_key) = // group ids correspond to feature ids, and are sorted in the same way, so we can use binary search
            match feature_groups.binary_search_by_key(&group_of_new_feature, |g| slab_feature_groups[*g].id) {
                Ok(group_idx) => {
                    let group_key = feature_groups[group_idx];
                    let group = &mut slab_feature_groups[group_key];
                    if group.idcs.start == insertion_idx + 1 {
                        group.idcs.start -= 1;
                    } else if group.idcs.contains(&insertion_idx) || group.idcs.end == insertion_idx {
                        group.idcs.end += 1;
                    } else {
                        unreachable!();
                    }
                    (group_idx, group_key)
                }
                Err(group_insertion_index) => {
                    let group = FeatureGroup::new(group_of_new_feature, insertion_idx..(insertion_idx + 1));
                    let group_key = slab_feature_groups.insert(group);
                    feature_groups.insert(group_insertion_index, group_key);
                    (group_insertion_index, group_key)
                }
            };

        for group_key in feature_groups[group_index + 1..].iter_mut() {
            let group = &mut slab_feature_groups[*group_key];
            group.idcs.end += 1;
            group.idcs.start += 1;
        }

        group_key
    }

    pub fn score_of_feature(group_size: usize, exact_feature_multiplicity: usize) -> f64 {
        score_for_group_size(group_size) / (group_size as f64 * exact_feature_multiplicity as f64)
    }

    /// Returns the index of an interesting input in the pool
    pub fn random_index(&mut self) -> Option<PoolIndex<T, M>> {
        if self.favored_input.is_some() && (self.rng.u8(0..4) == 0 || self.inputs.is_empty()) {
            Some(PoolIndex::Favored)
        } else if self.lowest_stack_input.is_some() && self.rng.u8(0..10) == 0 {
            Some(PoolIndex::LowestStack)
        } else if self.cumulative_weights.last().unwrap_or(&0.0) > &0.0 {
            let dist = WeightedIndex {
                cumulative_weights: &self.cumulative_weights,
            };
            let x = dist.sample(&mut self.rng);
            let key = self.inputs[x];
            Some(PoolIndex::Normal(key))
        } else {
            None
        }
    }

    pub fn len(&self) -> usize {
        self.inputs.len()
    }

    /// Update global statistics of the pool following a change in its content
    fn update_stats(&mut self) {
        let slab = &self.slab_inputs;
        self.cumulative_weights = self
            .inputs
            .iter()
            .map(|&key| &slab[key])
            .scan(0.0, |state, x| {
                *state += x.score;
                Some(*state)
            })
            .collect();

        self.average_complexity = self
            .inputs
            .iter()
            .map(|&key| &slab[key])
            .fold(0.0, |c, x| c + x.complexity)
            / self.inputs.len() as f64;
    }

    /// Get the input at the given index along with its complexity and the number of mutations tried on this input
    pub(crate) fn get_ref(&self, idx: PoolIndex<T, M>) -> &'_ FuzzedInput<T, M> {
        match idx {
            PoolIndex::Normal(key) => &self.slab_inputs[key].data,
            PoolIndex::Favored => self.favored_input.as_ref().unwrap(),
            PoolIndex::LowestStack => self.lowest_stack_input.as_ref().map(|x| &x.input).unwrap(),
        }
    }
    /// Get the input at the given index along with its complexity and the number of mutations tried on this input
    pub(crate) fn get(&mut self, idx: PoolIndex<T, M>) -> &'_ mut FuzzedInput<T, M> {
        match idx {
            PoolIndex::Normal(key) => &mut self.slab_inputs[key].data,
            PoolIndex::Favored => self.favored_input.as_mut().unwrap(),
            PoolIndex::LowestStack => self.lowest_stack_input.as_mut().map(|x| &mut x.input).unwrap(),
        }
    }

    pub(crate) fn retrieve_source_input_for_unmutate(
        &mut self,
        idx: PoolIndex<T, M>,
        generation: usize,
    ) -> Option<&'_ mut FuzzedInput<T, M>> {
        match idx {
            PoolIndex::Normal(key) => self.slab_inputs.get_mut(key).map(|input| &mut input.data),
            PoolIndex::Favored => Some(self.get(idx)),
            PoolIndex::LowestStack => {
                if let Some(lsi) = self.lowest_stack_input.as_mut() {
                    if lsi.generation < generation {
                        Some(&mut lsi.input)
                    } else {
                        None
                    }
                } else {
                    None
                }
            }
        }
    }

    pub(crate) fn mark_input_as_dead_end(&mut self, idx: PoolIndex<T, M>) {
        match idx {
            PoolIndex::Normal(key) => {
                let input = &mut self.slab_inputs[key];
                input.score = 0.0;
            }
            PoolIndex::Favored => {
                self.favored_input = None;
            }
            PoolIndex::LowestStack => {
                self.lowest_stack_input = None;
            }
        }
        self.update_stats()
    }

    #[cfg(feature = "ui")]
    pub(crate) fn send_coverage_information_for_input(
        &self,
        key: SlabKey<Input<T, M>>,
        coverage_map: &HashMap<Feature, Vec<BacktraceSymbol>>,
    ) -> WorldAction<T> {
        let input = &self.slab_inputs[key];
        let mut coverage = HashMap::new();

        for feature_key in &input.all_features {
            let feature = &self.slab_features[*feature_key];
            let feature_group = feature.feature.erasing_payload();
            coverage.insert(feature_group, coverage_map[&feature_group].clone());
        }
        let coverage: Vec<_> = coverage
            .iter()
            .map(|(_, symbols)| {
                symbols
                    .iter()
                    .map(|symbol| {
                        (
                            symbol.filename().map(|p| p.to_str().unwrap().to_owned()),
                            symbol.lineno(),
                            symbol.colno(),
                        )
                    })
                    .collect()
            })
            .collect();
        WorldAction::ReportCoverage {
            input: input.data.value.clone(),
            coverage_map: coverage,
        }
    }

    #[cfg(test)]
    fn print_recap(&self) {
        println!("recap inputs:");
        for &input_key in &self.inputs {
            let input = &self.slab_inputs[input_key];
            println!(
                "input with key {:?} has cplx {:.2}, score {:.2}, idx {}, and features: {:?}",
                input_key, input.complexity, input.score, input.idx_in_pool, input.all_features
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
        for (i, group_key) in self.feature_groups.iter().enumerate() {
            let group = &self.slab_feature_groups[*group_key];
            let slab = &self.slab_features;
            println!(
                "group {} has features {:?}",
                i,
                self.features[group.idcs.clone()]
                    .iter()
                    .map(|f| &slab[f.key].key)
                    .collect::<Vec<_>>()
            );
        }
        println!("---");
    }

    #[cfg(test)]
    fn sanity_check(&self) {
        let slab = &self.slab_features;

        self.print_recap();

        let fs = self
            .features
            .iter()
            .map(|f_iter| self.slab_features[f_iter.key].feature)
            .collect::<Vec<_>>();
        assert!(fs.is_sorted());

        let slab_groups = &self.slab_feature_groups;
        assert!(self.feature_groups.iter().is_sorted_by_key(|&g| slab_groups[g].id));
        assert!(self
            .feature_groups
            .iter()
            .is_sorted_by_key(|&g| slab_groups[g].idcs.start));
        assert!(self
            .feature_groups
            .iter()
            .is_sorted_by_key(|&g| slab_groups[g].idcs.end));
        assert!(self
            .feature_groups
            .windows(2)
            .all(|gs| slab_groups[gs[0]].idcs.end == slab_groups[gs[1]].idcs.start));
        assert!(slab_groups[*self.feature_groups.last().unwrap()].idcs.end == self.features.len());

        for f_iter in &self.features {
            let f_key = f_iter.key;
            let analyzed_f = &self.slab_features[f_key];
            for input_key in &analyzed_f.inputs {
                let input = &self.slab_inputs[*input_key];
                assert!(input.all_features.contains(&f_key));
            }
        }

        for input_key in &self.inputs {
            let input = &self.slab_inputs[*input_key];
            assert!(input.score > 0.0);
            let expected_input_score = input.all_features.iter().fold(0.0, |c, &fk| {
                let f = &slab[fk];
                let slab_groups = &self.slab_feature_groups;
                let group = self
                    .feature_groups
                    .iter()
                    .map(|&g| &slab_groups[g])
                    .find(|g| g.id == f.feature.group_id())
                    .unwrap();
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
                assert_eq!(analyzed_f.least_complexity, input.complexity);
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

        let mut dedupped_inputs = self.inputs.clone();
        dedupped_inputs.sort();
        dedupped_inputs.dedup();
        assert_eq!(dedupped_inputs.len(), self.inputs.len());

        // let mut dedupped_features = self.features.clone();
        // dedupped_features.sort();
        // dedupped_features.dedup();
        // assert_eq!(dedupped_features.len(), self.features.len());

        for g_key in &self.feature_groups {
            let g = &self.slab_feature_groups[*g_key];
            let slab = &self.slab_features;
            assert!(self.features[g.idcs.clone()]
                .iter()
                .map(|f| &slab[f.key])
                .all(|f| f.feature.group_id() == g.id));
        }
    }
}

/// Add the element in the correct place in the sorted vector
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

fn score_for_group_size(size: usize) -> f64 {
    const SCORES: [f64; 16] = [
        1.0, 1.1, 1.2, 1.3, 1.4, 1.5, 1.55, 1.6, 1.65, 1.7, 1.75, 1.8, 1.85, 1.9, 1.95, 2.0,
    ];
    if size < 16 {
        SCORES[size]
    } else {
        2.0
    }
    // 1.0
}

// ===============================================================
// ==================== Trait implementations ====================
// ===============================================================

impl<T: Clone, M: Mutator<T>> Clone for AnalyzedFeature<T, M> {
    fn clone(&self) -> Self {
        Self {
            key: self.key,
            feature: self.feature,
            group_key: self.group_key,
            inputs: self.inputs.clone(),
            least_complex_input: self.least_complex_input,
            least_complexity: self.least_complexity,
            old_multiplicity: self.old_multiplicity,
        }
    }
}
impl<T: Clone, M: Mutator<T>> PartialEq for AnalyzedFeature<T, M> {
    fn eq(&self, other: &Self) -> bool {
        self.key == other.key
            && self.feature == other.feature
            && self.inputs == other.inputs
            && self.least_complex_input == other.least_complex_input
            && self.least_complexity == other.least_complexity
            && self.old_multiplicity == other.old_multiplicity
    }
}
impl<T: Clone, M: Mutator<T>> fmt::Debug for AnalyzedFeature<T, M> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Feature {{ {:?}, f: {:#b}, inputs: {:?}, least_cplx: {:.2}, old_mult: {} }}",
            self.key, self.feature.0, self.inputs, self.least_complexity, self.old_multiplicity
        )
    }
}
impl<T: Clone, M: Mutator<T>> PartialEq for AnalyzedFeatureRef<T, M> {
    fn eq(&self, other: &Self) -> bool {
        self.feature == other.feature
    }
}
impl<T: Clone, M: Mutator<T>> Eq for AnalyzedFeatureRef<T, M> {}
impl<T: Clone, M: Mutator<T>> PartialOrd for AnalyzedFeatureRef<T, M> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.feature.partial_cmp(&other.feature)
    }
}
impl<T: Clone, M: Mutator<T>> Ord for AnalyzedFeatureRef<T, M> {
    fn cmp(&self, other: &Self) -> Ordering {
        self.feature.cmp(&other.feature)
    }
}

impl<T: Clone, M: Mutator<T>> Clone for AnalyzedFeatureRef<T, M> {
    fn clone(&self) -> Self {
        Self {
            key: self.key,
            feature: self.feature,
        }
    }
}

impl<T: Clone, M: Mutator<T>> Copy for AnalyzedFeatureRef<T, M> {}

// TODO: include testing the returned WorldAction
// TODO: write unit tests as data, read them from files
// TODO: write tests for adding inputs that are not simplest for any feature but are predicted to have a greater score
#[cfg(test)]
mod tests {
    use super::*;

    fn mock(cplx: f64) -> FuzzedInput<f64, VoidMutator> {
        FuzzedInput::new(cplx, (), ())
    }

    fn edge_f(pc_guard: usize, intensity: u16) -> Feature {
        Feature::edge(pc_guard, intensity)
    }

    type FK = SlabKey<AnalyzedFeature<f64, VoidMutator>>;

    #[test]
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

            let mut pool = Pool::<f64, VoidMutator>::default();

            for i in 0..fastrand::usize(0..100) {
                let nbr_new_features = if new_features.is_empty() {
                    0
                } else {
                    if i == 0 {
                        fastrand::usize(1..new_features.len())
                    } else {
                        fastrand::usize(0..new_features.len())
                    }
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
                    _lowest_stack: 0,
                };
                // println!("adding input of cplx {:.2} with new features {:?} and existing features {:?}", cplx1, new_features_1, existing_features_1);
                let _ = pool.add(mock(cplx1), cplx1, analysis_result, 0);
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

    // #[test]
    // fn test_features() {
    //     let x1 = Feature::edge(37, 3);
    //     assert_eq!(x1.score(), 1.0);
    //     println!("{:.x}", x1.0);

    //     let x2 = Feature::edge(std::usize::MAX, 255);
    //     assert_eq!(x2.score(), 1.0);
    //     println!("{:.x}", x2.0);

    //     assert!(x1 < x2);

    //     let y1 = Feature::instruction(56, 89, 88);
    //     assert_eq!(y1.score(), 0.1);
    //     println!("{:.x}", y1.0);

    //     assert!(y1 > x1);

    //     let y2 = Feature::instruction(76, 89, 88);
    //     assert_eq!(y2.score(), 0.1);
    //     println!("{:.x}", y2.0);

    //     assert!(y2 > y1);
    // }

    #[derive(Clone, Copy, Debug)]
    pub struct VoidMutator {}

    impl Mutator<f64> for VoidMutator {
        type Cache = ();
        type MutationStep = ();
        type ArbitraryStep = ();
        type UnmutateToken = ();

        fn default_arbitrary_step(&self) -> Self::ArbitraryStep {
            ()
        }

        fn validate_value(&self, _value: &f64) -> Option<(Self::Cache, Self::MutationStep)> {
            Some(((), ()))
        }
        fn max_complexity(&self) -> f64 {
            0.0
        }

        fn min_complexity(&self) -> f64 {
            0.0
        }

        fn complexity(&self, _value: &f64, _cache: &Self::Cache) -> f64 {
            0.0
        }

        fn ordered_arbitrary(&self, _step: &mut Self::ArbitraryStep, _max_cplx: f64) -> Option<(f64, f64)> {
            todo!()
        }

        fn random_arbitrary(&self, _max_cplx: f64) -> (f64, f64) {
            (0.0, 0.0)
        }

        fn ordered_mutate(
            &self,
            _value: &mut f64,
            _cache: &Self::Cache,
            _step: &mut Self::MutationStep,
            _max_cplx: f64,
        ) -> Option<(Self::UnmutateToken, f64)> {
            Some(((), 0.0))
        }

        fn random_mutate(&self, _value: &mut f64, _cache: &Self::Cache, _max_cplx: f64) -> (Self::UnmutateToken, f64) {
            ((), 0.0)
        }

        fn unmutate(&self, _value: &mut f64, _t: Self::UnmutateToken) {}
    }
}
