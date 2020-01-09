//! The `InputPool` is responsible for storing and updating inputs along with
//! their associated code coverage information. It assigns a score for each
//! input based on how unique its associated code coverage is. And it can
//! randomly select an input with a probability that is proportional to its
//! score relative to all the other ones.
//!
//! # Feature: a unit of code coverage
//!
//! The code coverage of an input is a set of `Feature`. A `Feature` is a value
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
//! The input pool will strive to keep as few inputs as possible, and will
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
//! It is not a perfectly fair system because the score of each feature is
//! currently wrong in many cases. For example, a single comparison instruction
//! can currently yield 16 different features for just one input. If that
//! happens, the score of those features will be too high and the input will be
//! over-rated. On the other hand, if it yields only 1 feature, it will be
//! under-rated. My intuition is that all these features could be grouped by
//! the address of their common comparison instruction, and that they should
//! share a common score that increases sub-linearly with the number of
//! features in the group. But it is difficult to implement efficiently.
//!

use std::cmp::Ordering;
use std::cmp::PartialOrd;

use rand::rngs::SmallRng;
use rand::{Rng, SeedableRng};

use rand::distributions::uniform::{UniformFloat, UniformSampler};
use rand::distributions::Distribution;

use ahash::{AHashMap, AHashSet};

use std::hash::{Hash, Hasher};

use crate::input::FuzzedInput;
use crate::input::UnifiedFuzzedInput;

use crate::weighted_index::WeightedIndex;
use crate::world::{FuzzerEvent, WorldAction};

/// A unit of code coverage
#[repr(align(8))]
#[derive(Debug, Clone, Copy)]
pub struct Feature {
    /// Identifier for a group of features.
    ///
    /// For example, it could uniquely identify a control flow edge,
    /// or a specific instruction in the text of the program.
    id: u32,

    /// Data associated with the feature.
    ///
    /// For example, it could contain the arguments to the instruction
    /// specified by `id`, or the number of times that the edge specified
    /// by `id` was reached.
    payload: u16,
    /// An identifier for the type of the feature
    ///
    /// * `0` control flow edge feature
    /// * `1`: indirect call feature
    /// * `2`: instruction feature
    /// * `3..`: undefined
    tag: u16,
}

impl PartialEq for Feature {
    fn eq(&self, other: &Self) -> bool {
        let a = unsafe { &*(self as *const Feature as *const u64) };
        let b = unsafe { &*(other as *const Feature as *const u64) };
        *a == *b
    }
}
impl Eq for Feature {}

impl Hash for Feature {
    fn hash<H: Hasher>(&self, state: &mut H) {
        let a = unsafe { &*(self as *const Feature as *const u64) };
        (*a).hash(state);
    }
}

impl Feature {
    /// Create a “control flow edge” feature identified by the given `pc_guard`
    /// whose payload is the intensity of the given `counter`.
    pub fn edge(pc_guard: usize, counter: u16) -> Feature {
        Feature {
            id: (pc_guard % core::u32::MAX as usize) as u32,
            payload: u16::from(score_from_counter(counter)),
            tag: 0,
        }
    }
    /// Create an “indirect call” feature identified by the given `caller_xor_callee`
    pub fn indir(caller_xor_callee: usize) -> Feature {
        Feature {
            id: (caller_xor_callee % core::u32::MAX as usize) as u32,
            payload: 0,
            tag: 1,
        }
    }
    /// Create an “instructon” feature identified by the given `pc` whose payload
    /// is a ~hash of the two arguments.
    pub fn instruction(pc: usize, arg1: u64, arg2: u64) -> Feature {
        Feature {
            id: (pc % core::u32::MAX as usize) as u32,
            payload: u16::from(score_from_counter((arg1 ^ arg2).count_ones() as u16)),
            tag: 2,
        }
    }
}

/// “Hash” a u16 into a number between 0 and 16.
///
/// So that similar numbers have the same hash, and very different
/// numbers have a greater hash.
fn score_from_counter(counter: u16) -> u8 {
    if counter == core::u16::MAX {
        16
    } else if counter <= 3 {
        counter as u8
    } else {
        (16 - counter.leading_zeros() + 1) as u8
    }
}

impl Feature {
    // Returns the code coverage score associated with the feature
    fn score(self) -> f64 {
        match self.tag {
            0 => 1.0,
            1 => 1.0,
            2 => 0.1,
            _ => unreachable!(),
        }
    }
}

/// Index of an input in the InputPool
#[derive(Debug, Clone, Copy)]
pub enum InputPoolIndex {
    Normal(usize),
    Favored,
}

/// Cached information about an input that is useful for analyzing it
// #[derive(Debug)]
pub struct Input<I: FuzzedInput> {
    /// Index of the input in the `InputPool`
    id: usize,
    /// Set of features triggered by feeding the input to the test function
    features: Vec<Feature>,
    /// Code coverage score of the input
    score: f64,
    /// Subset of the input‘s features for which there is no simpler input in the pool that also contains them
    least_complex_for_features: AHashSet<Feature>,

    data: UnifiedFuzzedInput<I>,

    complexity: f64,
}
impl<I: FuzzedInput> Clone for Input<I> {
    fn clone(&self) -> Self {
        Input {
            id: self.id,
            features: self.features.clone(),
            score: self.score,
            least_complex_for_features: self.least_complex_for_features.clone(),
            data: self.data.clone(),
            complexity: self.complexity,
        }
    }
}

impl<I: FuzzedInput> Input<I> {
    pub fn new(data: UnifiedFuzzedInput<I>, features: Vec<Feature>) -> Self {
        let complexity = data.complexity();
        Self {
            id: 0,
            features,
            score: 0.0,
            least_complex_for_features: AHashSet::new(),
            data,
            complexity,
        }
    }
}

/// The `InputPool` stores and rates inputs based on their associated code coverage.
pub struct InputPool<I: FuzzedInput> {
    /// List of all the inputs.
    ///
    /// A None in this vector is an input that was removed.
    pub inputs: Vec<Option<Input<I>>>,
    /// A special input that is given an artificially high score
    ///
    /// It is used for the input minifying function.
    pub favored_input: Option<UnifiedFuzzedInput<I>>,
    /// Number of inputs in the pool.
    ///
    /// It is equal to the number of `Some` values in `self.inputs`
    pub size: usize,
    /// The average complexity of the inputs
    pub average_complexity: f64,
    /// A map that lists all the inputs that contain a particular feature
    ///
    /// The keys contain all encountered features.
    /// The values are the ids of the inputs that contain the corresponding feature
    inputs_of_feature: AHashMap<Feature, Vec<usize>>,
    /// A vector used to quickly pick a random input based on its relative score
    ///
    /// The value at index i is equal to the value at i-1, plus the score of self.inputs[i].
    /// So it is a scan of the scores of the inputs.
    pub cumulative_weights: Vec<f64>,
    /// A random number generator, used for picking random inputs.
    rng: SmallRng,
}

impl<I: FuzzedInput> InputPool<I> {
    pub fn new() -> Self {
        Self {
            inputs: vec![],
            favored_input: None,
            size: 0,
            average_complexity: 0.0,
            inputs_of_feature: AHashMap::new(),
            cumulative_weights: Vec::new(),
            rng: SmallRng::from_entropy(),
        }
    }

    pub fn add_favored_input(&mut self, data: UnifiedFuzzedInput<I>) {
        self.favored_input = Some(data);
    }

    /// Add the given input to the pool.
    ///
    /// Recomputes the score of every input in the pool following the deletion.
    ///
    /// Returns the list of actions that the world has to handle to stay in sync with the pool
    pub fn add(&mut self, data: UnifiedFuzzedInput<I>, features: Vec<Feature>) -> Vec<WorldAction<I::Value>> {
        /* Goals:
        1. Find for which of its features the new element is the least complex
        2. Delete elements that are not worth keeping anymore
        3. Update the score of every element that shares a common feature with the new one
        4. Update the list of inputs for each feature
        5. Find the score of the new element
        6. Return the actions taken in this function

        The update to the pool is incremental. Not everything is recomputed from scratch.
        This is why (3) says:
            “Update the score of every element that shares a common feature with the new one”
        instead of:
            “Recompute the score of every input”

        TODO: insert new input in a gap instead of adding it to the end
        */

        let mut element = Input::new(data, features);

        // An element's id is its index in the pool. We already know that the element
        // will be added at the end of the pool, so its id is the length of the pool.
        // TODO: change that
        element.id = self.inputs.len();

        // The following loop is for Goal 1 and 2.
        let mut to_delete: Vec<usize> = vec![];
        for feature in element.features.iter() {
            let inputs_of_feature = self.inputs_of_feature.entry(*feature).or_default();

            let mut least_complex_for_this_feature = true;

            // Go through every element affected by the addition of the new input to the pool,
            // which is every element that shares a common feature with it.
            for id in inputs_of_feature.iter() {
                // Note that it is likely that affected_element has already been
                // processed for a previous feature, and has already been marked
                // for deletion. We go through the loop anyway and will add it
                // again to the `to_delete` list. Then, duplicate elements in the
                // list will be removed. Ref: #PBeq2fKehxcEz
                let affected_element = &mut self.inputs[*id].as_mut().unwrap();

                // if the complexity of that element is higher than the new input,
                // maybe it needs to be deleted. We need to make sure it is still the
                // smallest for any feature and is worth keeping.
                if affected_element.complexity >= element.complexity {
                    // The following line will not do anything in most cases, because
                    // it is unlikely that the affected element is the least complex
                    // for any particular feature.
                    affected_element.least_complex_for_features.remove(feature);

                    // If the element is not worth keeping, we add its `id` to a list of
                    // elements to delete. We batch deletions for simplicity and speed.
                    if affected_element.least_complex_for_features.is_empty() {
                        // Goal 2.
                        to_delete.push(*id);
                    }
                } else {
                    // One of the affected element for this feature is less complex than the new one.
                    // This means the new element is not the least complex for this feature.
                    least_complex_for_this_feature = false;
                }
            }

            if least_complex_for_this_feature {
                // Goal 1.
                element.least_complex_for_features.insert(*feature);
            }
        }
        // Goal 2.
        // TODO: this could be replaced by iterating over every input and removing those are not
        // simplest for any feature. Would this be simpler/faster? Maybe in some case.
        // See: #PBeq2fKehxcEz
        to_delete.sort();
        to_delete.dedup();

        // Goal 7: We save the inputs that are deleted in order to inform the external world of that action later
        let inputs_to_delete: Vec<_> = to_delete
            .iter()
            .map(|idx| self.inputs[*idx].as_ref().unwrap().data.value.clone())
            .collect();

        // Actually delete the elements marked for deletion
        to_delete.iter().for_each(|id| self.remove_input_id(*id));

        // The following loop is for Goal 3, 4, and 5.
        for feature in element.features.iter() {
            let inputs_of_feature = self.inputs_of_feature.entry(*feature).or_default();

            // Goal 3.
            // Updating the score of each affected element is done by subtracting the
            // previous portion of the score caused by the feature, and adding the new one
            // The portion of the score caused by a feature is `feature.score() / nbr_inputs_containing_feature`
            let feature_score = feature.score();
            let number_of_inputs_sharing_feature = inputs_of_feature.len() as f64;
            for id in inputs_of_feature.iter() {
                let element_with_feature = &mut self.inputs[*id].as_mut().unwrap();
                element_with_feature.score = element_with_feature.score
                    - feature_score / number_of_inputs_sharing_feature
                    + feature_score / (number_of_inputs_sharing_feature + 1.0)
            }

            // Goal 4.
            // Push, assuming sorted array, then I have as sorted list of all
            // the inputs containing the feature, which is handy for other opeartions
            let inputs = &mut self.inputs;
            sorted_insert(inputs_of_feature, element.id, |e| {
                inputs[*e].as_ref().map(|x| x.complexity).unwrap_or(-1.0) < element.complexity
            });

            // Goal 5.                      // equivalent to the current number of inputs sharing the feature
            element.score += feature_score / (number_of_inputs_sharing_feature + 1.0);
        }

        let value = element.data.value.clone();
        self.inputs.push(Some(element));

        // Goal 6.
        let mut actions: Vec<WorldAction<I::Value>> = Vec::new();

        actions.push(WorldAction::Add(value, vec![]));

        if !inputs_to_delete.is_empty() {
            actions.push(WorldAction::ReportEvent(FuzzerEvent::Deleted(inputs_to_delete.len())));
        }

        for i in inputs_to_delete.into_iter() {
            actions.push(WorldAction::Remove(i));
        }

        self.update_stats();

        actions
    }

    /// Removes the lowest ranking input in the pool
    ///
    /// Recomputes the score of every input in the pool following the deletion.
    ///
    /// Returns the list of actions that the world has to handle to stay in sync with the pool
    pub fn remove_lowest(&mut self) -> Vec<WorldAction<I::Value>> {
        let actions = {
            let input_to_delete: Option<I::Value>;

            let e = self
                .inputs
                .iter()
                .filter_map(|x| x.as_ref())
                .min_by(|x, y| PartialOrd::partial_cmp(&x.score, &y.score).unwrap_or(Ordering::Equal))
                .cloned(); // TODO: not ideal? don't care?

            if let Some(e) = e {
                self.remove_input_id(e.id);
                input_to_delete = Some(e.data.value);

                for f in e.features.iter() {
                    if let Some(new_lowest_cplx_id_for_f) =
                        self.inputs_of_feature.get(f).map(|x| x.last().copied()).flatten()
                    {
                        let new_lowest_e_for_f = &mut self.inputs[new_lowest_cplx_id_for_f].as_mut().unwrap();
                        if new_lowest_e_for_f.least_complex_for_features.contains(f) {
                            continue;
                        }
                        new_lowest_e_for_f.least_complex_for_features.insert(*f);
                    }
                }
            } else {
                input_to_delete = None;
            }

            if let Some(input_to_delete) = input_to_delete {
                vec![WorldAction::Remove(input_to_delete)]
            } else {
                vec![]
            }
        };

        self.update_stats();

        actions
    }

    /// Returns the combined score of every input in the pool
    ///
    /// It can be interpreted as the total score of the fuzzing process
    pub fn score(&self) -> f64 {
        *self.cumulative_weights.last().unwrap_or(&0.0)
    }

    /// Returns the index of an interesting input in the pool
    pub fn random_index(&mut self) -> InputPoolIndex {
        if self.favored_input.is_some() && (self.rng.gen_bool(0.25) || self.inputs.is_empty()) {
            InputPoolIndex::Favored
        } else {
            let weight_distr = UniformFloat::new(0.0, self.cumulative_weights.last().unwrap_or(&0.0));
            let dist = WeightedIndex {
                cumulative_weights: &self.cumulative_weights,
                weight_distribution: weight_distr,
            };
            let x = dist.sample(&mut self.rng);
            InputPoolIndex::Normal(x)
        }
    }

    /// Update global statistics of the input pool following a change in its content
    fn update_stats(&mut self) {
        self.cumulative_weights = self
            .inputs
            .iter()
            .scan(0.0, |state, x| {
                *state += x.as_ref().map(|x| x.score).unwrap_or(0.0);
                Some(*state)
            })
            .collect();

        let len = self.inputs.iter().fold(0, |c, x| if x.is_some() { c + 1 } else { c });
        if len == 0 {
            self.average_complexity = 0.0;
        } else {
            self.average_complexity = self
                .inputs
                .iter()
                .filter_map(|x| x.as_ref())
                .fold(0.0, |c, x| c + x.complexity)
                / len as f64;
        }
        self.size = len;
    }

    /// Get the input at the given index along with its complexity and the number of mutations tried on this input
    pub fn get_ref(&self, idx: InputPoolIndex) -> &'_ UnifiedFuzzedInput<I> {
        match idx {
            InputPoolIndex::Normal(idx) => &self.inputs[idx].as_ref().unwrap().data,
            InputPoolIndex::Favored => self.favored_input.as_ref().unwrap(),
        }
    }
    /// Get the input at the given index along with its complexity and the number of mutations tried on this input
    pub fn get(&mut self, idx: InputPoolIndex) -> &'_ mut UnifiedFuzzedInput<I> {
        match idx {
            InputPoolIndex::Normal(idx) => &mut self.inputs[idx].as_mut().unwrap().data,
            InputPoolIndex::Favored => self.favored_input.as_mut().unwrap(),
        }
    }
    /// Get the input at the given index along with its complexity and the number of mutations tried on this input
    pub fn get_opt(&mut self, idx: InputPoolIndex) -> Option<&'_ mut UnifiedFuzzedInput<I>> {
        match idx {
            InputPoolIndex::Normal(idx) => self.inputs[idx].as_mut().map(|x| &mut x.data),
            InputPoolIndex::Favored => self.favored_input.as_mut(),
        }
    }

    /// Return the predicted feature score for the given feature, as well as the
    /// complexity of the simplest input that contains this feature.
    ///
    /// The predicted score is an underestimate. It is based on the scenario where the
    /// multiplicity of the feature would grow by 1 following a new addition in the pool.
    /// But in reality, a new addition into the pool may delete existing inputs, which would
    /// decrease the multiplicity of the feature, and lead to a higher score.
    pub fn predicted_feature_score_and_least_complex_input_for_feature(
        &mut self,
        f: Feature,
    ) -> (f64, Option<(f64, f64)>) {
        if let Some(inputs_of_feature) = self.inputs_of_feature.get(&f) {
            let x = self.inputs[*inputs_of_feature.last().unwrap()].as_ref().unwrap();
            let predicted_score = f.score() / (inputs_of_feature.len() + 1) as f64;
            let least_complex = Some((x.complexity, x.score));
            (predicted_score, least_complex)
        } else {
            (f.score(), None)
        }
    }

    /// Removes the input of the given id from the pool
    ///
    /// Updates the pool accordingly to keep the scores correct,
    /// but does not update the global statistics managed by `self.update_stats()`
    fn remove_input_id(&mut self, id: usize) {
        let input_features = self.inputs[id].as_ref().unwrap().features.clone();

        for f in input_features.iter() {
            let f_score = f.score();
            let pool_inputs = &mut self.inputs;

            let mut mult: usize = 0;
            // For every input that shares a common feature with the one that's being deleted,
            // we update its score accordingly. We know that its score will increase because the
            // multiplicity of one of its features, `f`, went down by one.

            // At this point, inputs_of_feature has not been updated to remove the deletd input. So we
            // do that. Then, we update the score of each affected input by removing the old score
            // attributed to that feature (f_score / old_mult) and adding the new score (f_score / new_mult)
            // to that feature, f_score / (new_mult+1), then add the new score, f_score / new_mult.
            self.inputs_of_feature.entry(*f).and_modify(|x| {
                let old_mult = x.len();
                x.remove_item(&id);
                let new_mult = x.len();
                for j in x.iter() {
                    let mut e = &mut pool_inputs[*j].as_mut().unwrap();
                    e.score = e.score - (f_score / old_mult as f64) + (f_score / new_mult as f64);
                }
                mult = new_mult;
            });

            if mult == 0 {
                self.inputs_of_feature.remove(f);
            }
        }

        self.inputs[id] = None;
    }
}

/// Add the element in the correct place in the sorted vector
fn sorted_insert<T: PartialEq, F>(vec: &mut Vec<T>, element: T, is_before: F)
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
}

// TODO: include testing the returned WorldAction
// TODO: write unit tests as data, read them from files
// TODO: write tests for adding inputs that are not simplest for any feature but are predicted to have a greater score
#[cfg(test)]
mod tests {
    use super::*;

    fn equal_input(a: Input<FuzzedVoid>, b: Input<FuzzedVoid>) -> bool {
        a.id == b.id
            && a.features == b.features
            && a.score == b.score
            && a.least_complex_for_features == b.least_complex_for_features
            && a.data.value == b.data.value
            && a.data.state == b.data.state
            && a.complexity == b.complexity
    }

    fn mock(cplx: f64) -> UnifiedFuzzedInput<FuzzedVoid> {
        UnifiedFuzzedInput::new((cplx, ()))
    }

    fn edge_f(pc_guard: usize, intensity: u16) -> Feature {
        Feature::edge(pc_guard, intensity)
    }

    #[test]
    fn new_pool() {
        let pool = InputPool::<FuzzedVoid>::new();
        assert!(pool.inputs.is_empty());
        assert!(pool.inputs_of_feature.is_empty());
    }

    #[test]
    fn new_element() {
        let features = vec![edge_f(0, 1), edge_f(1, 1)];
        let element = Input::new(mock(10.0), features);

        assert_eq!(element.score.classify(), std::num::FpCategory::Zero);
    }

    #[test]
    fn add_one_element() {
        let mut pool = InputPool::<FuzzedVoid>::new();

        let (f1, f2) = (edge_f(0, 1), edge_f(1, 1));

        let features = vec![f1, f2];
        let element = Input::new(mock(10.0), features.clone());

        let _ = pool.add(mock(10.0), element.features);

        let predicted_element_in_pool = Input {
            id: 0,
            complexity: element.complexity,
            features: features.clone(),
            score: 2.0,
            least_complex_for_features: features.iter().cloned().collect(),
            data: mock(10.0),
        };

        assert_eq!(pool.inputs.len(), 1);
        assert!(equal_input(
            pool.inputs[0].as_ref().unwrap().clone(),
            predicted_element_in_pool
        ));

        let mut predicted_inputs_of_feature = AHashMap::<Feature, Vec<usize>>::new();
        predicted_inputs_of_feature.insert(f1, vec![0]);
        predicted_inputs_of_feature.insert(f2, vec![0]);
        assert_eq!(pool.inputs_of_feature, predicted_inputs_of_feature);
    }

    #[test]
    fn add_two_elements_with_entirely_different_features() {
        let mut pool = InputPool::<FuzzedVoid>::new();

        let (f1, f2, f3, f4) = (edge_f(0, 1), edge_f(1, 1), edge_f(2, 1), edge_f(3, 1));

        let features_1 = vec![f1, f2];
        let element_1 = Input::new(mock(10.0), features_1.clone());

        let features_2 = vec![f3, f4];
        let element_2 = Input::new(mock(25.0), features_2.clone());

        let _ = pool.add(element_1.data, element_1.features);
        let _ = pool.add(element_2.data, element_2.features);

        let predicted_element_1_in_pool = Input {
            id: 0,
            complexity: element_1.complexity,
            features: features_1.clone(),
            score: 2.0,
            least_complex_for_features: features_1.iter().cloned().collect(),
            data: element_1.data,
        };

        let predicted_element_2_in_pool = Input {
            id: 1,
            complexity: element_2.complexity,
            features: features_2.clone(),
            score: 2.0,
            least_complex_for_features: features_2.iter().cloned().collect(),
            data: element_2.data,
        };

        assert_eq!(pool.inputs.len(), 2);
        assert!(equal_input(
            pool.inputs[0].as_ref().unwrap().clone(),
            predicted_element_1_in_pool
        ));
        assert!(equal_input(
            pool.inputs[1].as_ref().unwrap().clone(),
            predicted_element_2_in_pool
        ));

        let mut predicted_inputs_of_feature = AHashMap::<Feature, Vec<usize>>::new();
        predicted_inputs_of_feature.insert(f1, vec![0]);
        predicted_inputs_of_feature.insert(f2, vec![0]);
        predicted_inputs_of_feature.insert(f3, vec![1]);
        predicted_inputs_of_feature.insert(f4, vec![1]);

        assert_eq!(pool.inputs_of_feature, predicted_inputs_of_feature);
    }

    #[test]
    fn add_two_elements_with_one_common_feature_1() {
        let mut pool = InputPool::<FuzzedVoid>::new();

        let (f1, f2, f3) = (edge_f(0, 1), edge_f(1, 1), edge_f(3, 1));

        let features_1 = vec![f1, f2];
        let element_1 = Input::new(mock(10.0), features_1.clone());

        let features_2 = vec![f1, f3];
        let element_2 = Input::new(mock(25.0), features_2.clone());

        let _ = pool.add(element_1.data, element_1.features);
        let _ = pool.add(element_2.data, element_2.features);

        let predicted_element_1_in_pool = Input {
            id: 0,
            complexity: element_1.complexity,
            features: features_1.clone(),
            score: 1.5,
            least_complex_for_features: features_1.iter().cloned().collect(),
            data: element_1.data,
        };

        let predicted_element_2_in_pool = Input {
            id: 1,
            complexity: element_2.complexity,
            features: features_2,
            score: 1.5,
            least_complex_for_features: vec![f3].iter().cloned().collect(),
            data: element_2.data,
        };

        assert_eq!(pool.inputs.len(), 2);
        assert!(equal_input(
            pool.inputs[0].as_ref().unwrap().clone(),
            predicted_element_1_in_pool
        ));
        assert!(equal_input(
            pool.inputs[1].as_ref().unwrap().clone(),
            predicted_element_2_in_pool
        ));

        let mut predicted_inputs_of_feature = AHashMap::<Feature, Vec<usize>>::new();
        predicted_inputs_of_feature.insert(f1, vec![1, 0]);
        predicted_inputs_of_feature.insert(f2, vec![0]);
        predicted_inputs_of_feature.insert(f3, vec![1]);

        assert_eq!(pool.inputs_of_feature, predicted_inputs_of_feature);
    }

    #[test]
    fn add_two_elements_with_one_common_feature_2() {
        let mut pool = InputPool::<FuzzedVoid>::new();

        let (f1, f2, f3) = (edge_f(0, 1), edge_f(1, 1), edge_f(3, 1));

        let features_1 = vec![f1, f2];
        let element_1 = Input::new(mock(25.0), features_1.clone());

        let features_2 = vec![f1, f3];
        let element_2 = Input::new(mock(10.0), features_2.clone());

        let _ = pool.add(element_1.data, element_1.features);
        let _ = pool.add(element_2.data, element_2.features);

        let predicted_element_1_in_pool = Input {
            id: 0,
            complexity: element_1.complexity,
            features: features_1,
            score: 1.5,
            least_complex_for_features: vec![f2].iter().cloned().collect(),
            data: element_1.data,
        };

        let predicted_element_2_in_pool = Input {
            id: 1,
            complexity: element_2.complexity,
            features: features_2.clone(),
            score: 1.5,
            least_complex_for_features: features_2.iter().cloned().collect(),
            data: element_2.data,
        };

        assert_eq!(pool.inputs.len(), 2);
        assert!(equal_input(
            pool.inputs[0].as_ref().unwrap().clone(),
            predicted_element_1_in_pool
        ));
        assert!(equal_input(
            pool.inputs[1].as_ref().unwrap().clone(),
            predicted_element_2_in_pool
        ));

        let mut predicted_inputs_of_feature = AHashMap::<Feature, Vec<usize>>::new();
        predicted_inputs_of_feature.insert(f1, vec![0, 1]);
        predicted_inputs_of_feature.insert(f2, vec![0]);
        predicted_inputs_of_feature.insert(f3, vec![1]);

        assert_eq!(pool.inputs_of_feature, predicted_inputs_of_feature);
    }

    #[test]
    fn add_two_elements_with_identical_features() {
        let mut pool = InputPool::<FuzzedVoid>::new();

        let (f1, f2) = (edge_f(0, 1), edge_f(1, 1));

        let features = vec![f1, f2];
        let element_1 = Input::new(mock(25.0), features.clone());

        let element_2 = Input::new(mock(10.0), features.clone());

        let _ = pool.add(element_1.data, element_1.features);
        let _ = pool.add(element_2.data, element_2.features);

        let predicted_element_in_pool = Input {
            id: 1,
            complexity: element_2.complexity,
            features: features.clone(),
            score: 2.0,
            least_complex_for_features: features.iter().cloned().collect(),
            data: element_2.data,
        };

        assert_eq!(pool.inputs.len(), 2);
        assert!(pool.inputs[0].is_none());
        assert!(equal_input(
            pool.inputs[1].as_ref().unwrap().clone(),
            predicted_element_in_pool
        ));

        let mut predicted_inputs_of_feature = AHashMap::<Feature, Vec<usize>>::new();
        predicted_inputs_of_feature.insert(f1, vec![1]);
        predicted_inputs_of_feature.insert(f2, vec![1]);

        assert_eq!(pool.inputs_of_feature, predicted_inputs_of_feature);
    }

    #[test]
    fn add_three_elements_the_last_one_replaces_the_two_first() {
        let mut pool = InputPool::<FuzzedVoid>::new();

        let (f1, f2, f3, f4) = (edge_f(0, 1), edge_f(1, 1), edge_f(3, 1), edge_f(4, 1));

        let features_1 = vec![f1, f2];
        let element_1 = Input::new(mock(20.0), features_1);

        let features_2 = vec![f3, f4];
        let element_2 = Input::new(mock(25.0), features_2);

        let features_3 = vec![f1, f2, f3, f4];
        let element_3 = Input::new(mock(15.0), features_3.clone());

        let _ = pool.add(element_1.data, element_1.features);
        let _ = pool.add(element_2.data, element_2.features);
        let _ = pool.add(element_3.data, element_3.features);

        let predicted_element_in_pool = Input {
            id: 2,
            complexity: element_3.complexity,
            features: features_3.clone(),
            score: 4.0,
            least_complex_for_features: features_3.iter().cloned().collect(),
            data: element_3.data,
        };

        assert_eq!(pool.inputs.len(), 3);
        assert!(pool.inputs[0].is_none());
        assert!(pool.inputs[1].is_none());
        assert!(equal_input(
            pool.inputs[2].as_ref().unwrap().clone(),
            predicted_element_in_pool
        ));

        let mut predicted_inputs_of_feature = AHashMap::<Feature, Vec<usize>>::new();
        predicted_inputs_of_feature.insert(f1, vec![2]);
        predicted_inputs_of_feature.insert(f2, vec![2]);
        predicted_inputs_of_feature.insert(f3, vec![2]);
        predicted_inputs_of_feature.insert(f4, vec![2]);

        assert_eq!(pool.inputs_of_feature, predicted_inputs_of_feature);
    }

    #[test]
    fn add_three_elements_with_some_common_features_and_equal_complexities() {
        let mut pool = InputPool::<FuzzedVoid>::new();

        let (f1, f2, f3) = (edge_f(0, 1), edge_f(1, 1), edge_f(3, 1));

        let features_1 = vec![f1, f2];
        let element_1 = Input::new(mock(20.0), features_1);

        let features_2 = vec![f1, f3];
        let element_2 = Input::new(mock(20.0), features_2.clone());

        let features_3 = vec![f2, f3];
        let element_3 = Input::new(mock(20.0), features_3.clone());

        let _ = pool.add(element_1.data, element_1.features);
        let _ = pool.add(element_2.data, element_2.features);
        let _ = pool.add(element_3.data, element_3.features);

        let predicted_element_1_in_pool = Input {
            id: 1,
            complexity: element_2.complexity,
            features: features_2,
            score: 1.5,
            least_complex_for_features: vec![f1].iter().cloned().collect(),
            data: element_2.data,
        };
        let predicted_element_2_in_pool = Input {
            id: 2,
            complexity: element_3.complexity,
            features: features_3,
            score: 1.5,
            least_complex_for_features: vec![f2, f3].iter().cloned().collect(),
            data: element_3.data,
        };

        assert_eq!(pool.inputs.len(), 3);
        assert!(pool.inputs[0].is_none());
        assert!(equal_input(
            pool.inputs[1].as_ref().unwrap().clone(),
            predicted_element_1_in_pool
        ));
        assert!(equal_input(
            pool.inputs[2].as_ref().unwrap().clone(),
            predicted_element_2_in_pool
        ));

        let mut predicted_inputs_of_feature = AHashMap::<Feature, Vec<usize>>::new();
        predicted_inputs_of_feature.insert(f1, vec![1]);
        predicted_inputs_of_feature.insert(f2, vec![2]);
        predicted_inputs_of_feature.insert(f3, vec![1, 2]);

        assert_eq!(pool.inputs_of_feature, predicted_inputs_of_feature);
    }

    #[test]
    fn test_add_three_elements_the_last_one_deletes_the_first_and_it_is_known_after_the_first_analyzed_feature() {
        let mut pool = InputPool::<FuzzedVoid>::new();

        let (f1, f2, f3) = (edge_f(0, 1), edge_f(1, 1), edge_f(3, 1));

        let features_1 = vec![f1, f2];
        let element_1 = Input::new(mock(20.0), features_1);

        let features_2 = vec![f2, f3];
        let element_2 = Input::new(mock(20.0), features_2.clone());

        let features_3 = vec![f1, f2];
        let element_3 = Input::new(mock(20.0), features_3.clone());

        let _ = pool.add(mock(20.0), element_1.features);
        let _ = pool.add(mock(20.0), element_2.features);
        let _ = pool.add(mock(20.0), element_3.features);

        let predicted_element_1_in_pool = Input {
            id: 1,
            complexity: element_2.complexity,
            features: features_2,
            score: 1.5,
            least_complex_for_features: vec![f3].iter().cloned().collect(),
            data: element_2.data,
        };
        let predicted_element_2_in_pool = Input {
            id: 2,
            complexity: element_3.complexity,
            features: features_3,
            score: 1.5,
            least_complex_for_features: vec![f1, f2].iter().cloned().collect(),
            data: element_3.data,
        };

        assert_eq!(pool.inputs.len(), 3);
        assert!(pool.inputs[0].is_none());
        assert!(equal_input(
            pool.inputs[1].as_ref().unwrap().clone(),
            predicted_element_1_in_pool
        ));
        assert!(equal_input(
            pool.inputs[2].as_ref().unwrap().clone(),
            predicted_element_2_in_pool
        ));

        let mut predicted_inputs_of_feature = AHashMap::<Feature, Vec<usize>>::new();
        predicted_inputs_of_feature.insert(f1, vec![2]);
        predicted_inputs_of_feature.insert(f2, vec![1, 2]);
        predicted_inputs_of_feature.insert(f3, vec![1]);

        assert_eq!(pool.inputs_of_feature, predicted_inputs_of_feature);
    }

    #[test]
    fn test_remove_lowest_one_element() {
        let mut pool = InputPool::<FuzzedVoid>::new();

        let (f1, f2) = (edge_f(0, 1), edge_f(1, 1));

        let features = vec![f1, f2];
        let element = Input::new(mock(20.0), features);

        let _ = pool.add(element.data, element.features);

        let _ = pool.remove_lowest();

        assert_eq!(pool.inputs.len(), 1);
        assert!(pool.inputs[0].is_none());
        assert_eq!(pool.inputs_of_feature, AHashMap::new());
    }

    #[test]
    fn test_remove_lowest_three_elements() {
        let mut pool = InputPool::<FuzzedVoid>::new();

        let (f1, f2) = (edge_f(0, 1), edge_f(1, 1));

        let features_1 = vec![f1, f2];
        let features_2 = vec![f1];

        let element_1 = Input::new(mock(20.0), features_1.clone());
        let element_2 = Input::new(mock(20.0), features_2);

        let _ = pool.add(element_1.data, element_1.features);
        let _ = pool.add(element_2.data, element_2.features);
        let _ = pool.remove_lowest();

        let predicted_element_in_pool = Input {
            id: 0,
            complexity: element_1.complexity,
            features: features_1,
            score: 2.0,
            least_complex_for_features: vec![f1, f2].iter().cloned().collect(),
            data: element_1.data,
        };

        let mut predicted_inputs_of_feature = AHashMap::<Feature, Vec<usize>>::new();
        predicted_inputs_of_feature.insert(f1, vec![0]);
        predicted_inputs_of_feature.insert(f2, vec![0]);

        assert_eq!(pool.inputs.len(), 2);
        assert!(equal_input(
            pool.inputs[0].as_ref().unwrap().clone(),
            predicted_element_in_pool
        ));
        assert!(pool.inputs[1].is_none());
        assert_eq!(pool.inputs_of_feature, predicted_inputs_of_feature);
    }

    use std::hash::Hasher;

    #[derive(Clone, Copy, Debug)]
    pub enum FuzzedVoid {}

    impl FuzzedInput for FuzzedVoid {
        type Value = f64;
        type State = ();
        type UnmutateToken = ();

        fn default() -> Self::Value {
            0.0
        }

        fn state_from_value(_value: &Self::Value) -> Self::State {}

        fn arbitrary(_seed: usize, _max_cplx: f64) -> Self::Value {
            0.0
        }

        fn max_complexity() -> f64 {
            std::f64::INFINITY
        }

        fn min_complexity() -> f64 {
            0.0
        }

        fn hash_value<H: Hasher>(_value: &Self::Value, _state: &mut H) {}

        fn complexity(value: &Self::Value, _state: &Self::State) -> f64 {
            *value
        }

        fn mutate(_value: &mut Self::Value, _state: &mut Self::State, _max_cplx: f64) -> Self::UnmutateToken {}

        fn unmutate(_value: &mut Self::Value, _state: &mut Self::State, _t: Self::UnmutateToken) {}

        fn from_data(_data: &[u8]) -> Option<Self::Value> {
            None
        }
        fn to_data(_value: &Self::Value) -> Vec<u8> {
            vec![]
        }
    }

    impl Copy for UnifiedFuzzedInput<FuzzedVoid> {}
}
