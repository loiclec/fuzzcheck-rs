use std::collections::HashMap;
use std::collections::HashSet;

use std::cmp::Ordering;
use std::cmp::PartialOrd;

use rand::rngs::ThreadRng;
use rand::Rng;

use rand::distributions::uniform::{UniformFloat, UniformSampler};
use rand::distributions::Distribution;

use serde::{Serialize, Deserialize};

use crate::input::InputGenerator;
use crate::weighted_index::WeightedIndex;
use crate::world::FuzzerEvent;
use crate::world::World;

#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy, Serialize, Deserialize)]
pub enum Feature {
    Edge(EdgeFeature),
    Comparison(ComparisonFeature),
    Indir(IndirFeature),
}

#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy, Serialize, Deserialize)]
pub struct EdgeFeature {
    pc_guard: usize,
    intensity: u8,
}
#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy, Serialize, Deserialize)]
pub struct IndirFeature {
    pub caller: usize,
    pub callee: usize,
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

impl EdgeFeature {
    pub fn new(pc_guard: usize, counter: u16) -> EdgeFeature {
        EdgeFeature {
            pc_guard,
            intensity: score_from_counter(counter),
        }
    }
}

#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy, Serialize, Deserialize)]
pub struct ComparisonFeature {
    pc: usize,
    id: u8,
}

impl ComparisonFeature {
    pub fn new(pc: usize, arg1: u64, arg2: u64) -> ComparisonFeature {
        ComparisonFeature {
            pc,
            id: score_from_counter((arg1 ^ arg2).count_ones() as u16),
        }
    }
}

impl Feature {
    fn score(&self) -> f64 {
        match self {
            Feature::Edge(_) => 1.0,
            Feature::Comparison(_) => 0.1,
            Feature::Indir(_) => 1.0,
        }
    }
}

pub enum InputPoolIndex {
    Normal(usize),
    Favored,
}

#[derive(Debug, PartialEq, Clone)]
pub struct InputPoolElement<T: Clone> {
    id: usize,
    pub input: T,
    pub complexity: f64,
    features: Vec<Feature>,
    score: f64,
    least_complex_for_features: HashSet<Feature>
}

impl<T: Clone> InputPoolElement<T> {
    pub fn new(input: T, complexity: f64, features: Vec<Feature>) -> InputPoolElement<T> {
        InputPoolElement {
            id: 0,
            input,
            complexity,
            features,
            score: 0.0,
            least_complex_for_features: HashSet::new()
        }
    }
}

#[derive(Debug)]
pub struct InputPool<T: Clone> {
    pub inputs: Vec<Option<InputPoolElement<T>>>,
    pub favored_input: Option<InputPoolElement<T>>,
    cumulative_weights: Vec<f64>,
    pub inputs_of_feature: HashMap<Feature, Vec<usize>>,
    pub average_complexity: f64,
    rng: ThreadRng
}

impl<T: Clone> InputPool<T> {
    pub fn new() -> Self {
        InputPool {
            inputs: Vec::new(),
            favored_input: None,
            cumulative_weights: vec![],
            inputs_of_feature: HashMap::new(),
            average_complexity: 0.0,
            rng: rand::thread_rng(),
        }
    }

    pub fn score(&self) -> f64 {
        *self.cumulative_weights.last().unwrap_or(&0.0)
    }

    pub fn get(&self, idx: InputPoolIndex) -> &InputPoolElement<T> {
        match idx {
            InputPoolIndex::Normal(idx) => &self.inputs[idx].as_ref().unwrap(),
            InputPoolIndex::Favored => &self.favored_input.as_ref().unwrap(),
        }
    }
    
    pub fn add<G>(&mut self, mut element: InputPoolElement<T>) -> impl FnOnce(&mut World<T, G>) -> Result<(), std::io::Error>
    where
        G: InputGenerator<Input = T>,
    {
        /* Goals:
        1. Find for which of its features the new element is the least complex (TODO: phrasing)
        2. Delete elements that are not worth keeping anymore
        3. Update the score of every element that shares a common feature with the new one
        4. Update the list of inputs for each feature
        5. Find the score of the new element
        6. Keep the metadata of the pool consistent (cumulative weights, average_complexity)
        7. Inform the external world of the actions taken in this function
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
        let inputs_to_delete: Vec<_> = to_delete.iter().map(|idx| self.inputs[*idx].as_ref().unwrap().input.clone()).collect();

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
                                            + feature.score() / ((inputs_of_feature.len()+1) as f64);
            }

            // Goal 4.
            // Push, assuming sorted array, then I have as sorted list of all
            // the inputs containing the feature, which is handy for other opeartions
            let inputs = &mut self.inputs; 
            sorted_push(inputs_of_feature, element.id, |e| 
                inputs[*e].as_ref().map(|x| x.complexity).unwrap_or(-1.0) < element.complexity
            );

            // Goal 5.
            element.score += feature.score() / (inputs_of_feature.len() as f64);
        }
        
        self.inputs.push(Some(element.clone()));

        // Goal 6.
        self.update_metadata();

        // Goal 7.
        move |w: &mut World<T, G>| {
            for i in &inputs_to_delete {
                w.remove_from_output_corpus(i.clone())?;
            }
            if !inputs_to_delete.is_empty() {
                w.report_event(FuzzerEvent::Deleted(inputs_to_delete.len()), Option::None);
            }
            Ok(())
        }
    }

    fn update_metadata(&mut self) {
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
            self.average_complexity = self.inputs.iter().filter_map(|x| x.as_ref()).fold(0.0, |c, x| c + x.complexity) / len as f64;
        }
    }

    pub fn random_index(&mut self) -> InputPoolIndex {
        if self.favored_input.is_some() && (self.rng.gen_bool(0.25) || self.inputs.is_empty()) {
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

    pub fn remove_lowest<G>(&mut self) -> impl FnOnce(&mut World<T, G>) -> Result<(), std::io::Error>
        where
        G: InputGenerator<Input = T>,
    {
        let input_to_delete: Option<T>;

        let e = self.inputs.iter()
            .filter_map(|x| x.as_ref())
            .min_by(|x, y| PartialOrd::partial_cmp(&x.score, &y.score).unwrap_or(Ordering::Equal))
            .cloned();

        if let Some(e) = e {
            self.remove_input_id(e.id);
            input_to_delete = Some(e.input);

            for f in e.features.iter() {
                if let Some(new_lowest_cplx_id_for_f) = self.inputs_of_feature.get(f).map(|x| x.last().copied()).flatten() {
                    let new_lowest_e_for_f = &mut self.inputs[new_lowest_cplx_id_for_f].as_mut().unwrap();
                    if new_lowest_e_for_f.least_complex_for_features.contains(f) { continue }
                    new_lowest_e_for_f.least_complex_for_features.insert(*f);
                }
            }
        } else {
            input_to_delete = None;
        }

        self.update_metadata();

        move |w: &mut World<T, G>| {
            if let Some(input_to_delete) = input_to_delete {
                w.remove_from_output_corpus(input_to_delete)?
            }
            Ok(())
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

fn sorted_push<T: PartialEq, F>(vec: &mut Vec<T>, element: T, is_before: F) where F: Fn(&T) -> bool {
    let mut insertion = 0;
    for e in vec.iter() {
        if is_before(e) { break }
        insertion += 1;
    }
    vec.insert(insertion, element);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn edge_f(pc_guard: usize, intensity: u8) -> Feature {
        Feature::Edge(EdgeFeature { pc_guard, intensity })
    }
    
    #[test]
    fn new_pool() {
        let pool = InputPool::<u8>::new();
        assert!(pool.inputs.is_empty());
        assert!(pool.favored_input.is_none());
        assert!(pool.cumulative_weights.is_empty());
        assert!(pool.inputs_of_feature.is_empty());
    }

    #[test]
    fn new_element() {
        let features = vec![edge_f(0, 1), edge_f(1, 1)];
        let element = InputPoolElement::<u8>::new(23, 10.0, features);

        assert_eq!(element.score, 0.0);
    }

    #[test]
    fn add_one_element() {
        let mut pool = InputPool::<u8>::new();

        let (f1, f2) = (edge_f(0, 1), edge_f(1, 1));

        let features = vec![f1, f2];
        let element = InputPoolElement::<u8>::new(255, 10.0, features.clone());

        let _ = pool.add::<U8Gen>(element.clone());

        let predicted_element_in_pool = InputPoolElement::<u8> {
            id: 0,
            input: element.input,
            complexity: element.complexity,
            features: features.clone(),
            score: 2.0,
            least_complex_for_features: features.iter().cloned().collect()
        };

        assert_eq!(pool.inputs.len(), 1);
        assert_eq!(pool.inputs[0].as_ref().unwrap().clone(), predicted_element_in_pool);

        // some of the score of the features
        assert_eq!(pool.cumulative_weights, vec![2.0]);
        
        let mut predicted_inputs_of_feature = HashMap::<Feature, Vec<usize>>::new();
        predicted_inputs_of_feature.insert(f1, vec![0]);
        predicted_inputs_of_feature.insert(f2, vec![0]);
        assert_eq!(pool.inputs_of_feature, predicted_inputs_of_feature);
    
        assert_eq!(pool.average_complexity, element.complexity);
    }

    #[test]
    fn add_two_elements_with_entirely_different_features() {
        let mut pool = InputPool::<u8>::new();

        let (f1, f2, f3, f4) = (edge_f(0, 1), edge_f(1, 1), edge_f(2, 1), edge_f(3, 1));

        let features_1 = vec![f1, f2];
        let element_1 = InputPoolElement::<u8>::new(255, 10.0, features_1.clone());

        let features_2 = vec![f3, f4];
        let element_2 = InputPoolElement::<u8>::new(254, 25.0, features_2.clone());

        let _ = pool.add::<U8Gen>(element_1.clone());
        let _ = pool.add::<U8Gen>(element_2.clone());


        let predicted_element_1_in_pool = InputPoolElement::<u8> {
            id: 0,
            input: element_1.input,
            complexity: element_1.complexity,
            features: features_1.clone(),
            score: 2.0,
            least_complex_for_features: features_1.iter().cloned().collect()
        };

        let predicted_element_2_in_pool = InputPoolElement::<u8> {
            id: 1,
            input: element_2.input,
            complexity: element_2.complexity,
            features: features_2.clone(),
            score: 2.0,
            least_complex_for_features: features_2.iter().cloned().collect()
        };
        
        assert_eq!(pool.inputs.len(), 2);
        assert_eq!(pool.inputs[0].as_ref().unwrap().clone(), predicted_element_1_in_pool);
        assert_eq!(pool.inputs[1].as_ref().unwrap().clone(), predicted_element_2_in_pool);

        // some of the score of the features
        assert_eq!(pool.cumulative_weights, vec![2.0, 4.0]);
        
        let mut predicted_inputs_of_feature = HashMap::<Feature, Vec<usize>>::new();
        predicted_inputs_of_feature.insert(f1, vec![0]);
        predicted_inputs_of_feature.insert(f2, vec![0]);
        predicted_inputs_of_feature.insert(f3, vec![1]);
        predicted_inputs_of_feature.insert(f4, vec![1]);

        assert_eq!(pool.inputs_of_feature, predicted_inputs_of_feature);
        assert_eq!(pool.average_complexity, (element_1.complexity + element_2.complexity) / 2.0);
    }

    #[test]
    fn add_two_elements_with_one_common_feature_1() {
        let mut pool = InputPool::<u8>::new();

        let (f1, f2, f3) = (edge_f(0, 1), edge_f(1, 1), edge_f(3, 1));


        let features_1 = vec![f1, f2];
        let element_1 = InputPoolElement::<u8>::new(255, 10.0, features_1.clone());

        let features_2 = vec![f1, f3];
        let element_2 = InputPoolElement::<u8>::new(254, 25.0, features_2.clone());

        let _ = pool.add::<U8Gen>(element_1.clone());
        let _ = pool.add::<U8Gen>(element_2.clone());


        let predicted_element_1_in_pool = InputPoolElement::<u8> {
            id: 0,
            input: element_1.input,
            complexity: element_1.complexity,
            features: features_1.clone(),
            score: 1.5,
            least_complex_for_features: features_1.iter().cloned().collect()
        };

        let predicted_element_2_in_pool = InputPoolElement::<u8> {
            id: 1,
            input: element_2.input,
            complexity: element_2.complexity,
            features: features_2.clone(),
            score: 1.5,
            least_complex_for_features: vec![f3].iter().cloned().collect(),
        };
        
        assert_eq!(pool.inputs.len(), 2);
        assert_eq!(pool.inputs[0].as_ref().unwrap().clone(), predicted_element_1_in_pool);
        assert_eq!(pool.inputs[1].as_ref().unwrap().clone(), predicted_element_2_in_pool);

        // some of the score of the features
        assert_eq!(pool.cumulative_weights, vec![1.5, 3.0]);
        
        let mut predicted_inputs_of_feature = HashMap::<Feature, Vec<usize>>::new();
        predicted_inputs_of_feature.insert(f1, vec![1, 0]);
        predicted_inputs_of_feature.insert(f2, vec![0]);
        predicted_inputs_of_feature.insert(f3, vec![1]);

        assert_eq!(pool.inputs_of_feature, predicted_inputs_of_feature);
        assert_eq!(pool.average_complexity, (element_1.complexity + element_2.complexity) / 2.0);
    }

    #[test]
    fn add_two_elements_with_one_common_feature_2() {
        let mut pool = InputPool::<u8>::new();

        let (f1, f2, f3) = (edge_f(0, 1), edge_f(1, 1), edge_f(3, 1));


        let features_1 = vec![f1, f2];
        let element_1 = InputPoolElement::<u8>::new(255, 25.0, features_1.clone());

        let features_2 = vec![f1, f3];
        let element_2 = InputPoolElement::<u8>::new(254, 10.0, features_2.clone());

        let _ = pool.add::<U8Gen>(element_1.clone());
        let _ = pool.add::<U8Gen>(element_2.clone());


        let predicted_element_1_in_pool = InputPoolElement::<u8> {
            id: 0,
            input: element_1.input,
            complexity: element_1.complexity,
            features: features_1.clone(),
            score: 1.5,
            least_complex_for_features: vec![f2].iter().cloned().collect(),
        };

        let predicted_element_2_in_pool = InputPoolElement::<u8> {
            id: 1,
            input: element_2.input,
            complexity: element_2.complexity,
            features: features_2.clone(),
            score: 1.5,
            least_complex_for_features: features_2.iter().cloned().collect(),
        };
        
        assert_eq!(pool.inputs.len(), 2);
        assert_eq!(pool.inputs[0].as_ref().unwrap().clone(), predicted_element_1_in_pool);
        assert_eq!(pool.inputs[1].as_ref().unwrap().clone(), predicted_element_2_in_pool);

        // some of the score of the features
        assert_eq!(pool.cumulative_weights, vec![1.5, 3.0]);
        
        let mut predicted_inputs_of_feature = HashMap::<Feature, Vec<usize>>::new();
        predicted_inputs_of_feature.insert(f1, vec![0, 1]);
        predicted_inputs_of_feature.insert(f2, vec![0]);
        predicted_inputs_of_feature.insert(f3, vec![1]);

        assert_eq!(pool.inputs_of_feature, predicted_inputs_of_feature);
        assert_eq!(pool.average_complexity, (element_1.complexity + element_2.complexity) / 2.0);
    }

    #[test]
    fn add_two_elements_with_identical_features() {
        let mut pool = InputPool::<u8>::new();

        let (f1, f2) = (edge_f(0, 1), edge_f(1, 1));


        let features = vec![f1, f2];
        let element_1 = InputPoolElement::<u8>::new(255, 25.0, features.clone());

        let element_2 = InputPoolElement::<u8>::new(254, 10.0, features.clone());

        let _ = pool.add::<U8Gen>(element_1.clone());
        let _ = pool.add::<U8Gen>(element_2.clone());

        let predicted_element_in_pool = InputPoolElement::<u8> {
            id: 1,
            input: element_2.input,
            complexity: element_2.complexity,
            features: features.clone(),
            score: 2.0,
            least_complex_for_features: features.iter().cloned().collect(),
        };
        
        assert_eq!(pool.inputs.len(), 2);
        assert!(pool.inputs[0].is_none());
        assert_eq!(pool.inputs[1].as_ref().unwrap().clone(), predicted_element_in_pool);

        // some of the score of the features
        assert_eq!(pool.cumulative_weights, vec![0.0, 2.0]);
        
        let mut predicted_inputs_of_feature = HashMap::<Feature, Vec<usize>>::new();
        predicted_inputs_of_feature.insert(f1, vec![1]);
        predicted_inputs_of_feature.insert(f2, vec![1]);

        assert_eq!(pool.inputs_of_feature, predicted_inputs_of_feature);
        assert_eq!(pool.average_complexity, element_2.complexity);
    }

    #[test]
    fn add_three_elements_the_last_one_replaces_the_two_first() {
        let mut pool = InputPool::<u8>::new();

        let (f1, f2, f3, f4) = (edge_f(0, 1), edge_f(1, 1), edge_f(3, 1), edge_f(4, 1));

        let features_1 = vec![f1, f2];
        let element_1 = InputPoolElement::<u8>::new(255, 20.0, features_1.clone());

        let features_2 = vec![f3, f4];
        let element_2 = InputPoolElement::<u8>::new(254, 25.0, features_2.clone());

        let features_3 = vec![f1, f2, f3, f4];
        let element_3 = InputPoolElement::<u8>::new(253, 15.0, features_3.clone());

        let _ = pool.add::<U8Gen>(element_1.clone());
        let _ = pool.add::<U8Gen>(element_2.clone());
        let _ = pool.add::<U8Gen>(element_3.clone());

        let predicted_element_in_pool = InputPoolElement::<u8> {
            id: 2,
            input: element_3.input,
            complexity: element_3.complexity,
            features: features_3.clone(),
            score: 4.0,
            least_complex_for_features: features_3.iter().cloned().collect(),
        };
        
        assert_eq!(pool.inputs.len(), 3);
        assert!(pool.inputs[0].is_none());
        assert!(pool.inputs[1].is_none());
        assert_eq!(pool.inputs[2].as_ref().unwrap().clone(), predicted_element_in_pool);

        // some of the score of the features
        assert_eq!(pool.cumulative_weights, vec![0.0, 0.0, 4.0]);
        
        let mut predicted_inputs_of_feature = HashMap::<Feature, Vec<usize>>::new();
        predicted_inputs_of_feature.insert(f1, vec![2]);
        predicted_inputs_of_feature.insert(f2, vec![2]);
        predicted_inputs_of_feature.insert(f3, vec![2]);
        predicted_inputs_of_feature.insert(f4, vec![2]);

        assert_eq!(pool.inputs_of_feature, predicted_inputs_of_feature);
        assert_eq!(pool.average_complexity, element_3.complexity);
    }

    #[test]
    fn add_three_elements_with_some_common_features_and_equal_complexities() {
        let mut pool = InputPool::<u8>::new();

        let (f1, f2, f3) = (edge_f(0, 1), edge_f(1, 1), edge_f(3, 1));

        let features_1 = vec![f1, f2];
        let element_1 = InputPoolElement::<u8>::new(255, 20.0, features_1.clone());

        let features_2 = vec![f1, f3];
        let element_2 = InputPoolElement::<u8>::new(254, 20.0, features_2.clone());

        let features_3 = vec![f2, f3];
        let element_3 = InputPoolElement::<u8>::new(253, 20.0, features_3.clone());

        let _ = pool.add::<U8Gen>(element_1.clone());
        let _ = pool.add::<U8Gen>(element_2.clone());
        let _ = pool.add::<U8Gen>(element_3.clone());

        let predicted_element_1_in_pool = InputPoolElement::<u8> {
            id: 1,
            input: element_2.input,
            complexity: element_2.complexity,
            features: features_2.clone(),
            score: 1.5,
            least_complex_for_features: vec![f1].iter().cloned().collect(),
        };
        let predicted_element_2_in_pool = InputPoolElement::<u8> {
            id: 2,
            input: element_3.input,
            complexity: element_3.complexity,
            features: features_3.clone(),
            score: 1.5,
            least_complex_for_features: vec![f2, f3].iter().cloned().collect(),
        };
        
        assert_eq!(pool.inputs.len(), 3);
        assert!(pool.inputs[0].is_none());
        assert_eq!(pool.inputs[1].as_ref().unwrap().clone(), predicted_element_1_in_pool);
        assert_eq!(pool.inputs[2].as_ref().unwrap().clone(), predicted_element_2_in_pool);

        // some of the score of the features
        assert_eq!(pool.cumulative_weights, vec![0.0, 1.5, 3.0]);
        
        let mut predicted_inputs_of_feature = HashMap::<Feature, Vec<usize>>::new();
        predicted_inputs_of_feature.insert(f1, vec![1]);
        predicted_inputs_of_feature.insert(f2, vec![2]);
        predicted_inputs_of_feature.insert(f3, vec![1, 2]);

        assert_eq!(pool.inputs_of_feature, predicted_inputs_of_feature);
        assert_eq!(pool.average_complexity, (element_2.complexity + element_3.complexity) / 2.0);
    }

    #[test]
    fn test_add_three_elements_the_last_one_deletes_the_first_and_it_is_known_after_the_first_analyzed_feature() {
        let mut pool = InputPool::<u8>::new();

        let (f1, f2, f3) = (edge_f(0, 1), edge_f(1, 1), edge_f(3, 1));

        let features_1 = vec![f1, f2];
        let element_1 = InputPoolElement::<u8>::new(255, 20.0, features_1.clone());

        let features_2 = vec![f2, f3];
        let element_2 = InputPoolElement::<u8>::new(254, 20.0, features_2.clone());

        let features_3 = vec![f1, f2];
        let element_3 = InputPoolElement::<u8>::new(253, 20.0, features_3.clone());

        let _ = pool.add::<U8Gen>(element_1.clone());
        let _ = pool.add::<U8Gen>(element_2.clone());
        let _ = pool.add::<U8Gen>(element_3.clone());

        let predicted_element_1_in_pool = InputPoolElement::<u8> {
            id: 1,
            input: element_2.input,
            complexity: element_2.complexity,
            features: features_2.clone(),
            score: 1.5,
            least_complex_for_features: vec![f3].iter().cloned().collect(),
        };
        let predicted_element_2_in_pool = InputPoolElement::<u8> {
            id: 2,
            input: element_3.input,
            complexity: element_3.complexity,
            features: features_3.clone(),
            score: 1.5,
            least_complex_for_features: vec![f1, f2].iter().cloned().collect(),
        };
        
        assert_eq!(pool.inputs.len(), 3);
        assert!(pool.inputs[0].is_none());
        assert_eq!(pool.inputs[1].as_ref().unwrap().clone(), predicted_element_1_in_pool);
        assert_eq!(pool.inputs[2].as_ref().unwrap().clone(), predicted_element_2_in_pool);

        // some of the score of the features
        assert_eq!(pool.cumulative_weights, vec![0.0, 1.5, 3.0]);
        
        let mut predicted_inputs_of_feature = HashMap::<Feature, Vec<usize>>::new();
        predicted_inputs_of_feature.insert(f1, vec![2]);
        predicted_inputs_of_feature.insert(f2, vec![1, 2]);
        predicted_inputs_of_feature.insert(f3, vec![1]);

        assert_eq!(pool.inputs_of_feature, predicted_inputs_of_feature);
        assert_eq!(pool.average_complexity, (element_2.complexity + element_3.complexity) / 2.0);
    }

    #[test]
    fn test_remove_lowest_one_element() {
        let mut pool = InputPool::<u8>::new();

        let (f1, f2) = (edge_f(0, 1), edge_f(1, 1));

        let features = vec![f1, f2];
        let element = InputPoolElement::<u8>::new(255, 10.0, features.clone());

        let _ = pool.add::<U8Gen>(element.clone());

        let _ = pool.remove_lowest::<U8Gen>();

        assert_eq!(pool.inputs.len(), 1);
        assert!(pool.inputs[0].is_none());
        assert_eq!(pool.cumulative_weights, vec![0.0]);
        assert_eq!(pool.inputs_of_feature, HashMap::new());
        assert_eq!(pool.average_complexity, 0.0);
    }

    #[test]
    fn test_remove_lowest_three_elements() {
        let mut pool = InputPool::<u8>::new();

        let (f1, f2) = (edge_f(0, 1), edge_f(1, 1));

        let features_1 = vec![f1, f2];
        let features_2 = vec![f1];

        let element_1 = InputPoolElement::<u8>::new(255, 10.0, features_1.clone());
        let element_2 = InputPoolElement::<u8>::new(254, 10.0, features_2.clone());

        let _ = pool.add::<U8Gen>(element_1.clone());
        let _ = pool.add::<U8Gen>(element_2.clone());
        let _ = pool.remove_lowest::<U8Gen>();

        let predicted_element_in_pool = InputPoolElement::<u8> {
            id: 0,
            input: element_1.input,
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
        assert_eq!(pool.cumulative_weights, vec![2.0, 2.0]);
        assert_eq!(pool.inputs_of_feature, predicted_inputs_of_feature);
        assert_eq!(pool.average_complexity, 10.0);
    }

    use std::hash::{Hash, Hasher};

    struct U8Gen { }

    impl InputGenerator for U8Gen {
        type Input = u8;
        fn complexity(_input: &Self::Input) -> f64 { 1.0 }
        fn hash<H>(input: &Self::Input, state: &mut H) where H: Hasher { input.hash(state) }
        fn new_input(&mut self, _max_cplx: f64) -> Self::Input { 0 }
        fn mutate(&mut self, _input: &mut Self::Input, _spare_cplx: f64) -> bool { true }
        fn from_data(data: &Vec<u8>) -> Option<Self::Input> { data.first().copied() }
        fn to_data(input: &Self::Input) -> Vec<u8> { vec![*input] }
    }
}
