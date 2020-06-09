//! Collection of data structures and algorithms used by the rest of fuzzcheck

use std::cmp::Ordering;

use core::cmp::PartialOrd;
use rand::distributions::uniform::{SampleUniform, UniformSampler};
use rand::distributions::Distribution;
use rand::Rng;

use std::ops::Index;
use std::ops::IndexMut;

use std::rc::Rc;
use std::cell::{RefCell, RefMut};

use std::fmt;

// ========= LargeStepFindIter ============

/**
 * A pseudo-iterator over a sorted slice that can find a value by jumping over
 * many elements at a time, and then backtracking if necessary.
*/
pub struct LargeStepFindIter<'a, T> {
    slice: &'a [T],
    position: usize,
}

impl<'a, T> LargeStepFindIter<'a, T>
where
    T: Copy,
{
    pub fn new(slice: &'a [T]) -> Self {
        Self { slice, position: 0 }
    }

    fn slow_find<P>(&mut self, cmp: P, start: usize, end: usize) -> Option<T>
    where
        P: Fn(T) -> Ordering,
    {
        for p in start..end {
            self.position = p;
            let el = unsafe { *self.slice.get_unchecked(p) };
            match (&cmp)(el) {
                Ordering::Less => continue,
                Ordering::Equal | Ordering::Greater => return Some(el),
            }
        }
        None
    }

    pub fn find<P>(&mut self, cmp: P) -> Option<T>
    where
        P: Fn(T) -> Ordering,
    {
        let mut step: usize = 8;

        // First check the first element of the slice, as it is often the
        // correct one
        if self.position < self.slice.len() {
            let el = unsafe { *self.slice.get_unchecked(self.position) };
            match (&cmp)(el) {
                Ordering::Less => (),
                Ordering::Equal | Ordering::Greater => return Some(el),
            }
        }

        // then execute first jump
        self.position += step;

        // and repeatedly check the first element and then either backtrack or jump
        while self.position < self.slice.len() {
            let el = unsafe { *self.slice.get_unchecked(self.position) };
            match (&cmp)(el) {
                Ordering::Less => (),
                Ordering::Equal => return Some(el),
                Ordering::Greater => {
                    let start = self.position - step + 1;
                    let end = self.position;
                    if let Some(x) = self.slow_find(&cmp, start, end) {
                        return Some(x);
                    } else {
                        self.position = end;
                    }
                }
            }
            // after each jump, the length of the next jump increases
            // 8, 16, 24, 32, 40, etc.
            step += 8;
            self.position += step;
        }

        // if the last jump went over the upperbound, then perform
        // a slow find over the last elements of the slice
        let start = self.position - step + 1;
        let end = self.slice.len();
        self.slow_find(&cmp, start, end)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_large_step_find_iter() {
        let xs = vec![
            0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 89, 90,
            91,
        ];

        for x in &xs {
            let mut iter = LargeStepFindIter::new(&xs);
            let y = iter.find(|y| y.cmp(x));
            assert_eq!(y, Some(*x));
        }

        let mut iter = LargeStepFindIter::new(&xs);

        let _ = iter.find(|x| x.cmp(&8));

        let y = iter.find(|x| x.cmp(&9));
        assert_eq!(y, Some(9));

        let y = iter.find(|x| x.cmp(&86));
        assert_eq!(y, Some(89));
    }
}

// ========= Slab ============

pub struct SlabKey<T> {
    #[cfg(not(test))]
    key: usize,
    #[cfg(test)]
    pub key: usize,
    phantom: std::marker::PhantomData<T>,
}

impl<T> SlabKey<T> {
    #[cfg(not(test))]
    fn new(key: usize) -> Self {
        Self {
            key,
            phantom: std::marker::PhantomData,
        }
    }

    #[cfg(test)]
    pub fn new(key: usize) -> Self {
        Self {
            key,
            phantom: std::marker::PhantomData,
        }
    }
}

impl<T> SlabKey<T> {
    pub fn invalid() -> Self {
        Self::new(std::usize::MAX)
    }
}

impl<T> Copy for SlabKey<T> {}

impl<T> Clone for SlabKey<T> {
    fn clone(&self) -> Self {
        Self::new(self.key)
    }
}

impl<T> PartialEq for SlabKey<T> {
    fn eq(&self, other: &Self) -> bool {
        self.key == other.key
    }
}

impl<T> Eq for SlabKey<T> {}

impl<T> fmt::Debug for SlabKey<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "k{}", self.key)
    }
}

impl<T> PartialOrd for SlabKey<T> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.key.cmp(&other.key))
    }
}
impl<T> Ord for SlabKey<T> {
    fn cmp(&self, other: &Self) -> Ordering {
        self.key.cmp(&other.key)
    }
}
/**
 * Pre-allocated storage for a uniform data type.
 *
 * An alternative implementation of the `Slab` type by the popular crate `slab`.
 */
pub struct Slab<T> {
    storage: Vec<T>,
    available_slots: Vec<usize>,
}

impl<T> Slab<T> {
    pub fn new() -> Self {
        Self {
            storage: Vec::with_capacity(1000),
            available_slots: Vec::with_capacity(32),
        }
    }

    pub fn insert(&mut self, x: T) -> SlabKey<T> {
        if let Some(&slot) = self.available_slots.last() {
            self.available_slots.pop();
            self.storage[slot] = x;
            SlabKey::new(slot)
        } else {
            self.storage.push(x);
            SlabKey::new(self.storage.len() - 1)
        }
    }
    pub fn remove(&mut self, key: SlabKey<T>) {
        self.available_slots.push(key.key);
    }

    pub fn next_key(&self) -> SlabKey<T> {
        if let Some(&slot) = self.available_slots.last() {
            SlabKey::new(slot)
        } else {
            SlabKey::new(self.storage.len())
        }
    }

    pub fn get_mut(&mut self, key: SlabKey<T>) -> Option<&mut T> {
        // O(n) but in practice very fast because there will be almost no available slots
        if self.available_slots.contains(&key.key) {
            None
        } else {
            Some(unsafe { self.storage.get_unchecked_mut(key.key) })
        }
    }
}

impl<T> Index<SlabKey<T>> for Slab<T> {
    type Output = T;

    fn index(&self, key: SlabKey<T>) -> &Self::Output {
        unsafe { self.storage.get_unchecked(key.key) }
    }
}
impl<T> IndexMut<SlabKey<T>> for Slab<T> {
    fn index_mut(&mut self, key: SlabKey<T>) -> &mut Self::Output {
        unsafe { self.storage.get_unchecked_mut(key.key) }
    }
}

// ========== WeightedIndex ===========

/**
 * A distribution using weighted sampling to pick a discretely selected item.
 *
 * An alternative implementation of the same type by the `rand` crate.
 */
#[derive(Debug, Clone)]
pub struct WeightedIndex<'a, X: SampleUniform + PartialOrd> {
    pub cumulative_weights: &'a Vec<X>,
    pub weight_distribution: X::Sampler,
}

impl<'a, X> Distribution<usize> for WeightedIndex<'a, X>
where
    X: SampleUniform + PartialOrd,
{
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> usize {
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
