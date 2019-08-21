use hashbrown::HashMap;
use hashbrown::HashSet;

use std::cmp::Ordering;
use std::cmp::PartialOrd;

use rand::rngs::ThreadRng;
use rand::Rng;

use rand::distributions::uniform::{UniformFloat, UniformSampler};
use rand::distributions::Distribution;

// use serde::{Deserialize, Serialize};

use std::hash::{Hash, Hasher};

use crate::weighted_index::WeightedIndex;
use crate::world::{WorldAction, FuzzerEvent};

#[derive(Clone)]
enum MetadataChange {
    Remove(usize),
    Add(usize, Vec<Feature>),
    ReportEvent(FuzzerEvent),
}

#[derive(Debug, Clone, Copy/*, Serialize, Deserialize*/)]
pub struct Feature {
    id: u32,
    payload: u16,
    tag: u16,
}

impl PartialEq for Feature {
    fn eq(&self, other: &Self) -> bool {
        let a = unsafe { & *(self as *const Feature as *const u64) };
        let b = unsafe { & *(other as *const Feature as *const u64) };
        *a == *b
    }
}
impl Eq for Feature { }

impl Hash for Feature {
    fn hash<H: Hasher>(&self, state: &mut H) {
        let a = unsafe { & *(self as *const Feature as *const u64) };
        (*a).hash(state);
    }
}

impl Feature {
    pub fn edge(pc_guard: usize, counter: u16) -> Feature {
        Feature {
            id: (pc_guard % core::u32::MAX as usize) as u32,
            payload: score_from_counter(counter) as u16,
            tag: 0,
        }
    }
    pub fn indir(caller_xor_callee: usize) -> Feature {
        Feature {
            id: (caller_xor_callee % core::u32::MAX as usize) as u32,
            payload: 0,
            tag: 1,
        }
    }
    pub fn comparison(pc: usize, arg1: u64, arg2: u64) -> Feature {
        Feature {
            id: (pc % core::u32::MAX as usize) as u32 ,
            payload: score_from_counter((arg1 ^ arg2).count_ones() as u16) as u16,
            tag: 2,
        }
    }
}

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
    fn score(&self) -> f64 {
        match self.tag {
            0 => 1.0,
            1 => 1.0,
            2 => 0.1,
            _ => unreachable!(),
        }
    }
}


pub enum InputPoolIndex {
    Normal(usize),
    Favored,
}

#[derive(Debug, PartialEq, Clone)]
struct InputMetadata {
    id: usize,
    complexity: f64,
    features: Vec<Feature>,
    score: f64,
    least_complex_for_features: HashSet<Feature>,
}

impl InputMetadata {
    pub fn new(complexity: f64, features: Vec<Feature>) -> InputMetadata {
        InputMetadata {
            id: 0,
            complexity,
            features,
            score: 0.0,
            least_complex_for_features: HashSet::new(),
        }
    }
}

pub struct InputPool<T: Clone> {
    pub inputs: Vec<Option<T>>,
    pub favored_input: Option<(T, f64)>,
    metadata: InputMetadataPool,
    pub size: usize,
    pub average_complexity: f64,
    pub cumulative_weights: Vec<f64>,
    rng: ThreadRng,
}

impl<T: Clone> InputPool<T> {
    pub fn new() -> Self {
        Self {
            inputs: vec![],
            favored_input: None,
            metadata: InputMetadataPool::new(),
            size: 0,
            average_complexity: 0.0,
            cumulative_weights: vec![],
            rng: rand::thread_rng(),
        }
    }

    pub fn add_favored_input(&mut self, input: T, cplx: f64) {
        self.favored_input = Some((input, cplx));
    }

    fn convert_partial_world_actions(&self, actions: &Vec<MetadataChange>) -> Vec<WorldAction<T>> {
        actions.into_iter().map(|p| {
            match p {
                MetadataChange::Add(x, fs) => {
                    WorldAction::Add(self.inputs[*x].as_ref().unwrap().clone(), fs.clone())
                },
                MetadataChange::Remove(x) => {
                    WorldAction::Remove(self.inputs[*x].as_ref().unwrap().clone())
                },
                MetadataChange::ReportEvent(e) => {
                    WorldAction::ReportEvent(e.clone())
                }
            }
        }).collect()
    }

    pub fn add(&mut self, input: T, cplx: f64, features: Vec<Feature>) -> Vec<WorldAction<T>> {
        let actions = self.metadata.add(InputMetadata::new(cplx, features));
        self.inputs.push(Some(input));
        self.update_stats();

        let full_actions = self.convert_partial_world_actions(&actions);
        for a in actions.iter() {
            match a {
                MetadataChange::Remove(i) => self.inputs[*i] = None,
                _ => continue,
            }
        }
        full_actions
    }

    pub fn remove_lowest(&mut self) -> Vec<WorldAction<T>> {
        let actions = self.metadata.remove_lowest();
        self.update_stats();

        let full_actions = self.convert_partial_world_actions(&actions);
        for a in actions.iter() {
            match a {
                MetadataChange::Remove(i) => self.inputs[*i] = None,
                _ => continue,
            }
        }
        full_actions
    }

    pub fn score(&self) -> f64 {
        *self.cumulative_weights.last().unwrap_or(&0.0)
    }

    pub fn random_index(&mut self) -> InputPoolIndex {
        if self.favored_input.is_some() && (self.rng.gen_bool(0.25) || self.metadata.inputs.is_empty()) {
            InputPoolIndex::Favored
        } else {
            let weight_distr = UniformFloat::new(0.0, self.cumulative_weights.last().unwrap_or(&0.0));
            let dist = WeightedIndex {
                cumulative_weights: self.cumulative_weights.clone(),
                weight_distribution: weight_distr,
            };
            let x = dist.sample(&mut self.rng);
            InputPoolIndex::Normal(x)
        }
    }

    fn update_stats(&mut self) {
        self.cumulative_weights = self
            .metadata
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
                .metadata
                .inputs
                .iter()
                .filter_map(|x| x.as_ref())
                .fold(0.0, |c, x| c + x.complexity)
                / len as f64;
        }
        self.size = len;
    }

    pub fn get(&self, idx: InputPoolIndex) -> (T, f64) {
        match idx {
            InputPoolIndex::Normal(idx) => (self.inputs[idx].as_ref().unwrap().clone(), self.metadata.inputs[idx].as_ref().unwrap().complexity - 1.0),
            InputPoolIndex::Favored => self.favored_input.as_ref().unwrap().clone(),
        }
    }

    pub fn least_complex_input_for_feature(&mut self, f: Feature) -> Option<(f64, f64)> {
        let inputs = &self.metadata.inputs;
        if let Some(input) = self.metadata.inputs_of_feature.get(&f).map(|x| inputs[*x.last().unwrap()].as_ref().unwrap()) {
            Some((input.complexity, input.score))
        } else {
            None
        }
    }

    pub fn predicted_feature_score(&self, f: Feature) -> f64 {
        self.metadata.inputs_of_feature
            .get(&f)
            .map(|x| f.score() / (x.len() + 1) as f64)
            .unwrap_or(f.score())
    }
}

#[derive(Debug)]
struct InputMetadataPool {
    inputs: Vec<Option<InputMetadata>>,
    inputs_of_feature: HashMap<Feature, Vec<usize>>,
}

impl InputMetadataPool {
    fn new() -> Self {
        InputMetadataPool {
            inputs: Vec::new(),
            inputs_of_feature: HashMap::new(),
        }
    }

    fn add(&mut self, mut element: InputMetadata) -> Vec<MetadataChange> {
        /* Goals:
        1. Find for which of its features the new element is the least complex (TODO: phrasing)
        2. Delete elements that are not worth keeping anymore
        3. Update the score of every element that shares a common feature with the new one
        4. Update the list of inputs for each feature
        5. Find the score of the new element
        6. Return the actions taken in this function
        */

        // An element's id is its index in the pool. We already know that the element
        // will be added at the end of the pool, so its id is the length of the pool.
        element.id = self.inputs.len();

        // The following loop is for Goal 1 and 2.
        let mut to_delete: Vec<usize> = vec![];
        for feature in element.features.iter() {
            let inputs_of_feature = self.inputs_of_feature.entry(*feature).or_default();

            // This loop also computes for which features the new element is the least
            // complex in the pool to contain them (TODO: phrasing)
            let mut least_complex_for_this_feature = true;

            // Go through every element affected by the addition of the new input to the pool
            // which is every element that shares a common feature with it.
            for id in inputs_of_feature.iter() {
                // Note that it is likely that affected_element has already been
                // processed for a previous feature, and has already been marked
                // for deletion. We go through the loop anyway and will add it
                // again to the `to_delete` list. Then, duplicate elements in the
                // list will be removed. Ref: #PBeq2fKehxcEz
                let affected_element = &mut self.inputs[*id].as_mut().unwrap();

                // if the complexity of that element is higher than the new input,
                // myaybe it needs to be deleted. We need to make sure it is still the
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

        // See: #PBeq2fKehxcEz
        to_delete.sort();
        to_delete.dedup();

        // Goal 7: We save the inputs that are deleted in order to inform the external world of that action later
        let inputs_to_delete: Vec<_> = to_delete
            .iter()
            .map(|idx| self.inputs[*idx].as_ref().unwrap().id)
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
            for id in inputs_of_feature.iter() {
                let element_with_feature = &mut self.inputs[*id].as_mut().unwrap();
                element_with_feature.score = element_with_feature.score
                    - feature.score() / (inputs_of_feature.len() as f64)
                    + feature.score() / ((inputs_of_feature.len() + 1) as f64);
            }

            // Goal 4.
            // Push, assuming sorted array, then I have as sorted list of all
            // the inputs containing the feature, which is handy for other opeartions
            let inputs = &mut self.inputs;
            sorted_push(inputs_of_feature, element.id, |e| {
                inputs[*e].as_ref().map(|x| x.complexity).unwrap_or(-1.0) < element.complexity
            });

            // Goal 5.
            element.score += feature.score() / (inputs_of_feature.len() as f64);
        }

        self.inputs.push(Some(element.clone()));

        // Goal 6.
        let mut actions: Vec<MetadataChange> = Vec::new();
        
        actions.push(MetadataChange::Add(element.id, vec![]));

        for i in &inputs_to_delete {
            actions.push(MetadataChange::Remove(i.clone()));
        }
        if !inputs_to_delete.is_empty() {
            actions.push(MetadataChange::ReportEvent(FuzzerEvent::Deleted(inputs_to_delete.len())));
        }
        
        actions
    }

    fn remove_lowest(&mut self) -> Vec<MetadataChange> {
        let input_to_delete: Option<usize>;

        let e = self
            .inputs
            .iter()
            .filter_map(|x| x.as_ref())
            .min_by(|x, y| PartialOrd::partial_cmp(&x.score, &y.score).unwrap_or(Ordering::Equal))
            .cloned();

        if let Some(e) = e {
            self.remove_input_id(e.id);
            input_to_delete = Some(e.id);

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
            vec![MetadataChange::Remove(input_to_delete)]
        } else {
            vec![]
        }
    }

    fn remove_input_id(&mut self, id: usize) {
        let e = self.inputs[id].as_ref().unwrap().clone();
        assert_eq!(e.id, id);
        for f in e.features.iter() {
            self.inputs_of_feature.entry(*f).and_modify(|x| {
                x.remove_item(&e.id);
            });
            let new_mult = self.inputs_of_feature[f].len();
            for j in self.inputs_of_feature[f].iter() {
                let mut e = &mut self.inputs[*j].as_mut().unwrap();
                e.score = e.score - (f.score() / (new_mult + 1) as f64) + (f.score() / new_mult as f64);
            }
            if new_mult == 0 {
                self.inputs_of_feature.remove(f);
            }
        }
        self.inputs[e.id] = None;
    }
}

fn sorted_push<T: PartialEq, F>(vec: &mut Vec<T>, element: T, is_before: F)
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


// TODO: tests on InputPool, not InputMetadataPool. Including testing the returned WorldAction 
// TODO: write unit tests as data, read them from files
#[cfg(test)]
mod tests {
    use super::*;

    fn edge_f(pc_guard: usize, intensity: u16) -> Feature {
        Feature::edge(pc_guard, intensity)
    }

    #[test]
    fn new_pool() {
        let pool = InputMetadataPool::new();
        assert!(pool.inputs.is_empty());
        assert!(pool.inputs_of_feature.is_empty());
    }

    #[test]
    fn new_element() {
        let features = vec![edge_f(0, 1), edge_f(1, 1)];
        let element = InputMetadata::new(10.0, features);

        assert_eq!(element.score, 0.0);
    }

    #[test]
    fn add_one_element() {
        let mut pool = InputMetadataPool::new();

        let (f1, f2) = (edge_f(0, 1), edge_f(1, 1));

        let features = vec![f1, f2];
        let element = InputMetadata::new(10.0, features.clone());

        let _ = pool.add(element.clone());

        let predicted_element_in_pool = InputMetadata {
            id: 0,
            complexity: element.complexity,
            features: features.clone(),
            score: 2.0,
            least_complex_for_features: features.iter().cloned().collect(),
        };

        assert_eq!(pool.inputs.len(), 1);
        assert_eq!(pool.inputs[0].as_ref().unwrap().clone(), predicted_element_in_pool);

        let mut predicted_inputs_of_feature = HashMap::<Feature, Vec<usize>>::new();
        predicted_inputs_of_feature.insert(f1, vec![0]);
        predicted_inputs_of_feature.insert(f2, vec![0]);
        assert_eq!(pool.inputs_of_feature, predicted_inputs_of_feature);
    }

    #[test]
    fn add_two_elements_with_entirely_different_features() {
        let mut pool = InputMetadataPool::new();

        let (f1, f2, f3, f4) = (edge_f(0, 1), edge_f(1, 1), edge_f(2, 1), edge_f(3, 1));

        let features_1 = vec![f1, f2];
        let element_1 = InputMetadata::new(10.0, features_1.clone());

        let features_2 = vec![f3, f4];
        let element_2 = InputMetadata::new(25.0, features_2.clone());

        let _ = pool.add(element_1.clone());
        let _ = pool.add(element_2.clone());

        let predicted_element_1_in_pool = InputMetadata {
            id: 0,
            complexity: element_1.complexity,
            features: features_1.clone(),
            score: 2.0,
            least_complex_for_features: features_1.iter().cloned().collect(),
        };

        let predicted_element_2_in_pool = InputMetadata {
            id: 1,
            complexity: element_2.complexity,
            features: features_2.clone(),
            score: 2.0,
            least_complex_for_features: features_2.iter().cloned().collect(),
        };

        assert_eq!(pool.inputs.len(), 2);
        assert_eq!(pool.inputs[0].as_ref().unwrap().clone(), predicted_element_1_in_pool);
        assert_eq!(pool.inputs[1].as_ref().unwrap().clone(), predicted_element_2_in_pool);


        let mut predicted_inputs_of_feature = HashMap::<Feature, Vec<usize>>::new();
        predicted_inputs_of_feature.insert(f1, vec![0]);
        predicted_inputs_of_feature.insert(f2, vec![0]);
        predicted_inputs_of_feature.insert(f3, vec![1]);
        predicted_inputs_of_feature.insert(f4, vec![1]);

        assert_eq!(pool.inputs_of_feature, predicted_inputs_of_feature);
    }

    #[test]
    fn add_two_elements_with_one_common_feature_1() {
        let mut pool = InputMetadataPool::new();

        let (f1, f2, f3) = (edge_f(0, 1), edge_f(1, 1), edge_f(3, 1));

        let features_1 = vec![f1, f2];
        let element_1 = InputMetadata::new(10.0, features_1.clone());

        let features_2 = vec![f1, f3];
        let element_2 = InputMetadata::new(25.0, features_2.clone());

        let _ = pool.add(element_1.clone());
        let _ = pool.add(element_2.clone());

        let predicted_element_1_in_pool = InputMetadata {
            id: 0,
            complexity: element_1.complexity,
            features: features_1.clone(),
            score: 1.5,
            least_complex_for_features: features_1.iter().cloned().collect(),
        };

        let predicted_element_2_in_pool = InputMetadata {
            id: 1,
            complexity: element_2.complexity,
            features: features_2.clone(),
            score: 1.5,
            least_complex_for_features: vec![f3].iter().cloned().collect(),
        };

        assert_eq!(pool.inputs.len(), 2);
        assert_eq!(pool.inputs[0].as_ref().unwrap().clone(), predicted_element_1_in_pool);
        assert_eq!(pool.inputs[1].as_ref().unwrap().clone(), predicted_element_2_in_pool);


        let mut predicted_inputs_of_feature = HashMap::<Feature, Vec<usize>>::new();
        predicted_inputs_of_feature.insert(f1, vec![1, 0]);
        predicted_inputs_of_feature.insert(f2, vec![0]);
        predicted_inputs_of_feature.insert(f3, vec![1]);

        assert_eq!(pool.inputs_of_feature, predicted_inputs_of_feature);
    }

    #[test]
    fn add_two_elements_with_one_common_feature_2() {
        let mut pool = InputMetadataPool::new();

        let (f1, f2, f3) = (edge_f(0, 1), edge_f(1, 1), edge_f(3, 1));

        let features_1 = vec![f1, f2];
        let element_1 = InputMetadata::new(25.0, features_1.clone());

        let features_2 = vec![f1, f3];
        let element_2 = InputMetadata::new(10.0, features_2.clone());

        let _ = pool.add(element_1.clone());
        let _ = pool.add(element_2.clone());

        let predicted_element_1_in_pool = InputMetadata {
            id: 0,
            complexity: element_1.complexity,
            features: features_1.clone(),
            score: 1.5,
            least_complex_for_features: vec![f2].iter().cloned().collect(),
        };

        let predicted_element_2_in_pool = InputMetadata {
            id: 1,
            complexity: element_2.complexity,
            features: features_2.clone(),
            score: 1.5,
            least_complex_for_features: features_2.iter().cloned().collect(),
        };

        assert_eq!(pool.inputs.len(), 2);
        assert_eq!(pool.inputs[0].as_ref().unwrap().clone(), predicted_element_1_in_pool);
        assert_eq!(pool.inputs[1].as_ref().unwrap().clone(), predicted_element_2_in_pool);


        let mut predicted_inputs_of_feature = HashMap::<Feature, Vec<usize>>::new();
        predicted_inputs_of_feature.insert(f1, vec![0, 1]);
        predicted_inputs_of_feature.insert(f2, vec![0]);
        predicted_inputs_of_feature.insert(f3, vec![1]);

        assert_eq!(pool.inputs_of_feature, predicted_inputs_of_feature);
    }

    #[test]
    fn add_two_elements_with_identical_features() {
        let mut pool = InputMetadataPool::new();

        let (f1, f2) = (edge_f(0, 1), edge_f(1, 1));

        let features = vec![f1, f2];
        let element_1 = InputMetadata::new(25.0, features.clone());

        let element_2 = InputMetadata::new(10.0, features.clone());

        let _ = pool.add(element_1.clone());
        let _ = pool.add(element_2.clone());

        let predicted_element_in_pool = InputMetadata {
            id: 1,
            complexity: element_2.complexity,
            features: features.clone(),
            score: 2.0,
            least_complex_for_features: features.iter().cloned().collect(),
        };

        assert_eq!(pool.inputs.len(), 2);
        assert!(pool.inputs[0].is_none());
        assert_eq!(pool.inputs[1].as_ref().unwrap().clone(), predicted_element_in_pool);


        let mut predicted_inputs_of_feature = HashMap::<Feature, Vec<usize>>::new();
        predicted_inputs_of_feature.insert(f1, vec![1]);
        predicted_inputs_of_feature.insert(f2, vec![1]);

        assert_eq!(pool.inputs_of_feature, predicted_inputs_of_feature);
    }

    #[test]
    fn add_three_elements_the_last_one_replaces_the_two_first() {
        let mut pool = InputMetadataPool::new();

        let (f1, f2, f3, f4) = (edge_f(0, 1), edge_f(1, 1), edge_f(3, 1), edge_f(4, 1));

        let features_1 = vec![f1, f2];
        let element_1 = InputMetadata::new(20.0, features_1.clone());

        let features_2 = vec![f3, f4];
        let element_2 = InputMetadata::new(25.0, features_2.clone());

        let features_3 = vec![f1, f2, f3, f4];
        let element_3 = InputMetadata::new(15.0, features_3.clone());

        let _ = pool.add(element_1.clone());
        let _ = pool.add(element_2.clone());
        let _ = pool.add(element_3.clone());

        let predicted_element_in_pool = InputMetadata {
            id: 2,
            complexity: element_3.complexity,
            features: features_3.clone(),
            score: 4.0,
            least_complex_for_features: features_3.iter().cloned().collect(),
        };

        assert_eq!(pool.inputs.len(), 3);
        assert!(pool.inputs[0].is_none());
        assert!(pool.inputs[1].is_none());
        assert_eq!(pool.inputs[2].as_ref().unwrap().clone(), predicted_element_in_pool);


        let mut predicted_inputs_of_feature = HashMap::<Feature, Vec<usize>>::new();
        predicted_inputs_of_feature.insert(f1, vec![2]);
        predicted_inputs_of_feature.insert(f2, vec![2]);
        predicted_inputs_of_feature.insert(f3, vec![2]);
        predicted_inputs_of_feature.insert(f4, vec![2]);

        assert_eq!(pool.inputs_of_feature, predicted_inputs_of_feature);
    }

    #[test]
    fn add_three_elements_with_some_common_features_and_equal_complexities() {
        let mut pool = InputMetadataPool::new();

        let (f1, f2, f3) = (edge_f(0, 1), edge_f(1, 1), edge_f(3, 1));

        let features_1 = vec![f1, f2];
        let element_1 = InputMetadata::new(20.0, features_1.clone());

        let features_2 = vec![f1, f3];
        let element_2 = InputMetadata::new(20.0, features_2.clone());

        let features_3 = vec![f2, f3];
        let element_3 = InputMetadata::new(20.0, features_3.clone());

        let _ = pool.add(element_1.clone());
        let _ = pool.add(element_2.clone());
        let _ = pool.add(element_3.clone());

        let predicted_element_1_in_pool = InputMetadata {
            id: 1,
            complexity: element_2.complexity,
            features: features_2.clone(),
            score: 1.5,
            least_complex_for_features: vec![f1].iter().cloned().collect(),
        };
        let predicted_element_2_in_pool = InputMetadata {
            id: 2,
            complexity: element_3.complexity,
            features: features_3.clone(),
            score: 1.5,
            least_complex_for_features: vec![f2, f3].iter().cloned().collect(),
        };

        assert_eq!(pool.inputs.len(), 3);
        assert!(pool.inputs[0].is_none());
        assert_eq!(pool.inputs[1].as_ref().unwrap().clone(), predicted_element_1_in_pool);
        assert_eq!(pool.inputs[2].as_ref().unwrap().clone(), predicted_element_2_in_pool);


        let mut predicted_inputs_of_feature = HashMap::<Feature, Vec<usize>>::new();
        predicted_inputs_of_feature.insert(f1, vec![1]);
        predicted_inputs_of_feature.insert(f2, vec![2]);
        predicted_inputs_of_feature.insert(f3, vec![1, 2]);

        assert_eq!(pool.inputs_of_feature, predicted_inputs_of_feature);
    }

    #[test]
    fn test_add_three_elements_the_last_one_deletes_the_first_and_it_is_known_after_the_first_analyzed_feature() {
        let mut pool = InputMetadataPool::new();

        let (f1, f2, f3) = (edge_f(0, 1), edge_f(1, 1), edge_f(3, 1));

        let features_1 = vec![f1, f2];
        let element_1 = InputMetadata::new(20.0, features_1.clone());

        let features_2 = vec![f2, f3];
        let element_2 = InputMetadata::new(20.0, features_2.clone());

        let features_3 = vec![f1, f2];
        let element_3 = InputMetadata::new(20.0, features_3.clone());

        let _ = pool.add(element_1.clone());
        let _ = pool.add(element_2.clone());
        let _ = pool.add(element_3.clone());

        let predicted_element_1_in_pool = InputMetadata {
            id: 1,
            complexity: element_2.complexity,
            features: features_2.clone(),
            score: 1.5,
            least_complex_for_features: vec![f3].iter().cloned().collect(),
        };
        let predicted_element_2_in_pool = InputMetadata {
            id: 2,
            complexity: element_3.complexity,
            features: features_3.clone(),
            score: 1.5,
            least_complex_for_features: vec![f1, f2].iter().cloned().collect(),
        };

        assert_eq!(pool.inputs.len(), 3);
        assert!(pool.inputs[0].is_none());
        assert_eq!(pool.inputs[1].as_ref().unwrap().clone(), predicted_element_1_in_pool);
        assert_eq!(pool.inputs[2].as_ref().unwrap().clone(), predicted_element_2_in_pool);


        let mut predicted_inputs_of_feature = HashMap::<Feature, Vec<usize>>::new();
        predicted_inputs_of_feature.insert(f1, vec![2]);
        predicted_inputs_of_feature.insert(f2, vec![1, 2]);
        predicted_inputs_of_feature.insert(f3, vec![1]);

        assert_eq!(pool.inputs_of_feature, predicted_inputs_of_feature);
    }

    #[test]
    fn test_remove_lowest_one_element() {
        let mut pool = InputMetadataPool::new();

        let (f1, f2) = (edge_f(0, 1), edge_f(1, 1));

        let features = vec![f1, f2];
        let element = InputMetadata::new(10.0, features.clone());

        let _ = pool.add(element.clone());

        let _ = pool.remove_lowest();

        assert_eq!(pool.inputs.len(), 1);
        assert!(pool.inputs[0].is_none());
        assert_eq!(pool.inputs_of_feature, HashMap::new());
    }

    #[test]
    fn test_remove_lowest_three_elements() {
        let mut pool = InputMetadataPool::new();

        let (f1, f2) = (edge_f(0, 1), edge_f(1, 1));

        let features_1 = vec![f1, f2];
        let features_2 = vec![f1];

        let element_1 = InputMetadata::new(10.0, features_1.clone());
        let element_2 = InputMetadata::new(10.0, features_2.clone());

        let _ = pool.add(element_1.clone());
        let _ = pool.add(element_2.clone());
        let _ = pool.remove_lowest();

        let predicted_element_in_pool = InputMetadata {
            id: 0,
            complexity: element_1.complexity,
            features: features_1.clone(),
            score: 2.0,
            least_complex_for_features: vec![f1, f2].iter().cloned().collect(),
        };

        let mut predicted_inputs_of_feature = HashMap::<Feature, Vec<usize>>::new();
        predicted_inputs_of_feature.insert(f1, vec![0]);
        predicted_inputs_of_feature.insert(f2, vec![0]);

        assert_eq!(pool.inputs.len(), 2);
        assert_eq!(pool.inputs[0].as_ref().unwrap().clone(), predicted_element_in_pool);
        assert!(pool.inputs[1].is_none());
        assert_eq!(pool.inputs_of_feature, predicted_inputs_of_feature);
    }
}
