use std::cmp::Ordering;

use crate::mutators::gen_f64;

#[derive(Clone)]
pub struct FenwickTree {
    storage: Vec<f64>,
}

#[inline(always)]
#[coverage(off)]
fn least_significant_bit(i: usize) -> usize {
    i & (1_usize.wrapping_add(!i))
}
#[inline(always)]
#[coverage(off)]
fn get_parent(i: usize) -> usize {
    i - least_significant_bit(i)
}
#[inline(always)]
#[coverage(off)]
fn get_next(i: usize) -> usize {
    i + least_significant_bit(i)
}

impl FenwickTree {
    #[coverage(off)]
    pub fn new(mut xs: Vec<f64>) -> Self {
        let mut i = 1;
        while i < xs.len() {
            let j = get_next(i);
            if j < xs.len() {
                xs[j] += xs[i];
            }
            i += 1;
        }
        Self { storage: xs }
    }
    #[coverage(off)]
    pub fn len(&self) -> usize {
        self.storage.len()
    }
    #[coverage(off)]
    pub fn prefix_sum(&self, mut idx: usize) -> f64 {
        assert!(!self.storage.is_empty());
        let mut sum = self.storage[0];
        while idx != 0 {
            sum += self.storage[idx];
            idx = get_parent(idx);
        }
        sum
    }
    #[coverage(off)]
    pub fn update(&mut self, mut idx: usize, delta: f64) {
        if idx == 0 {
            self.storage[idx] += delta;
            return;
        }
        while idx < self.storage.len() {
            self.storage[idx] += delta;
            idx = get_next(idx);
        }
    }
    // Find the first item which has a prefix sum *higher* than the chosen weight.
    #[coverage(off)]
    pub fn first_index_past_prefix_sum(&self, chosen_weight: f64) -> usize {
        binary_search(
            self.len(),
            #[coverage(off)]
            |idx| {
                if self.prefix_sum(idx) <= chosen_weight {
                    Ordering::Less
                } else {
                    Ordering::Greater
                }
            },
        )
        .unwrap_err()
    }
}

#[coverage(off)]
pub fn binary_search<F>(mut size: usize, mut f: F) -> Result<usize, usize>
where
    F: FnMut(usize) -> Ordering,
{
    let mut left = 0;
    let mut right = size;
    while left < right {
        let mid = left + size / 2;
        let cmp = f(mid);
        if cmp == Ordering::Less {
            left = mid + 1;
        } else if cmp == Ordering::Greater {
            right = mid;
        } else {
            return Ok(mid);
        }
        size = right - left;
    }
    Err(left)
}

/*
Note: I can pad the tree with zeros if its size is not a power of two
this will make it possible to use this method below to query it
is it worth it?

// Find the largest i with prefix_sum(i) <= value.
// NOTE: Requires that all values are non-negative!
unsigned int rank_query(int value) {
    int i = 0, j = SIZE - 1;
    // j is a power of 2.

    for (; j > 0;  j >>= 1) {
        if (i + j < SIZE && A[i + j] <= value) {
            value -= A[i + j];
            i += j;
        }
    }
    return i;
}
*/

impl FenwickTree {
    #[coverage(off)]
    pub fn sample(&self, rng: &fastrand::Rng) -> Option<usize> {
        if self.len() == 0 {
            return None;
        }
        let most = self.prefix_sum(self.len() - 1);
        if most <= 0.0 {
            return None;
        }
        let chosen_weight = gen_f64(rng, 0.0..most);

        // Find the first item which has a weight *higher* than the chosen weight.
        let choice = self.first_index_past_prefix_sum(chosen_weight);
        Some(choice)
    }
}

#[cfg(test)]
mod tests {
    use super::FenwickTree;

    #[coverage(off)]
    #[test]
    fn test_basic_1() {
        let cumulative_probabilities = vec![2.0, 4.0, 1.0, 0.0, 1.2];
        let mut tree = FenwickTree::new(cumulative_probabilities);
        for i in 0..tree.storage.len() {
            println!("{}", tree.prefix_sum(i));
        }
        println!("===");
        tree.update(0, -0.5);
        for i in 0..tree.storage.len() {
            println!("{}", tree.prefix_sum(i));
        }
        println!("===");
        tree.update(1, 0.5);
        for i in 0..tree.storage.len() {
            println!("{}", tree.prefix_sum(i));
        }
        println!("===");
        tree.update(3, 1.);
        for i in 0..tree.storage.len() {
            println!("{}", tree.prefix_sum(i));
        }
    }
}
