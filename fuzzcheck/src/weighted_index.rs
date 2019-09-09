//! Weighted index sampling
//!
//! This module provides an implementation for sampling indices.
//! Todo: give credit

use core::cmp::PartialOrd;
use rand::distributions::uniform::{SampleUniform, UniformSampler};
use rand::distributions::Distribution;
use rand::Rng;

#[derive(Debug, Clone)]
pub struct WeightedIndex<X: SampleUniform + PartialOrd> {
    pub cumulative_weights: Vec<X>,
    pub weight_distribution: X::Sampler,
}

impl<X> Distribution<usize> for WeightedIndex<X>
where
    X: SampleUniform + PartialOrd,
{
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> usize {
        use ::core::cmp::Ordering;
        let chosen_weight = self.weight_distribution.sample(rng);
        // Find the first item which has a weight *higher* than the chosen weight.
        self.cumulative_weights
            .binary_search_by(|w| {
                if *w <= chosen_weight {
                    Ordering::Less
                } else {
                    Ordering::Greater
                }
            })
            .unwrap_err()
    }
}
