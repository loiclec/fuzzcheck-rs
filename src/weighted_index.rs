
//! Weighted index sampling
//! 
//! This module provides two implementations for sampling indices:
//! 
//! *   [`WeightedIndex`] allows `O(log N)` sampling
//! *   [`alias_method::WeightedIndex`] allows `O(1)` sampling, but with
//!      much greater set-up cost
//!      
//! [`alias_method::WeightedIndex`]: alias_method/struct.WeightedIndex.html

use rand::Rng;
use rand::distributions::Distribution;
use rand::distributions::uniform::{UniformSampler, SampleUniform};
use core::cmp::PartialOrd;

/// A distribution using weighted sampling to pick a discretely selected
/// item.
///
/// Sampling a `WeightedIndex` distribution returns the index of a randomly
/// selected element from the iterator used when the `WeightedIndex` was
/// created. The chance of a given element being picked is proportional to the
/// value of the element. The weights can use any type `X` for which an
/// implementation of [`Uniform<X>`] exists.
///
/// # Performance
///
/// A `WeightedIndex<X>` contains a `Vec<X>` and a [`Uniform<X>`] and so its
/// size is the sum of the size of those objects, possibly plus some alignment.
///
/// Creating a `WeightedIndex<X>` will allocate enough space to hold `N - 1`
/// weights of type `X`, where `N` is the number of weights. However, since
/// `Vec` doesn't guarantee a particular growth strategy, additional memory
/// might be allocated but not used. Since the `WeightedIndex` object also
/// contains, this might cause additional allocations, though for primitive
/// types, ['Uniform<X>`] doesn't allocate any memory.
///
/// Time complexity of sampling from `WeightedIndex` is `O(log N)` where
/// `N` is the number of weights.
///
/// Sampling from `WeightedIndex` will result in a single call to
/// `Uniform<X>::sample` (method of the [`Distribution`] trait), which typically
/// will request a single value from the underlying [`RngCore`], though the
/// exact number depends on the implementaiton of `Uniform<X>::sample`.
///
/// # Example
///
/// ```
/// use rand::prelude::*;
/// use rand::distributions::WeightedIndex;
///
/// let choices = ['a', 'b', 'c'];
/// let weights = [2,   1,   1];
/// let dist = WeightedIndex::new(&weights).unwrap();
/// let mut rng = thread_rng();
/// for _ in 0..100 {
///     // 50% chance to print 'a', 25% chance to print 'b', 25% chance to print 'c'
///     println!("{}", choices[dist.sample(&mut rng)]);
/// }
///
/// let items = [('a', 0), ('b', 3), ('c', 7)];
/// let dist2 = WeightedIndex::new(items.iter().map(|item| item.1)).unwrap();
/// for _ in 0..100 {
///     // 0% chance to print 'a', 30% chance to print 'b', 70% chance to print 'c'
///     println!("{}", items[dist2.sample(&mut rng)].0);
/// }
/// ```
///
/// [`Uniform<X>`]: crate::distributions::uniform::Uniform
/// [`RngCore`]: crate::RngCore
#[derive(Debug, Clone)]
pub struct WeightedIndex<X: SampleUniform + PartialOrd> {
    pub cumulative_weights: Vec<X>,
    pub weight_distribution: X::Sampler,
}


impl<X> Distribution<usize> for WeightedIndex<X> where
    X: SampleUniform + PartialOrd {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> usize {
        use ::core::cmp::Ordering;
        let chosen_weight = self.weight_distribution.sample(rng);
        // Find the first item which has a weight *higher* than the chosen weight.
        self.cumulative_weights.binary_search_by(
            |w| if *w <= chosen_weight { Ordering::Less } else { Ordering::Greater }).unwrap_err()
    }
}
