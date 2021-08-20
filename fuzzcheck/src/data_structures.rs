//! Collection of data structures and algorithms used by the rest of fuzzcheck

extern crate fastrand;

use std::cmp::Ordering;
use std::cmp::PartialOrd;

use std::hash::Hash;
use std::ops::Index;
use std::ops::IndexMut;
use std::ops::Range;

use std::fmt;

// ========= Slab ============

pub struct SlabKey<T> {
    pub key: usize,
    phantom: std::marker::PhantomData<T>,
}

impl<T> SlabKey<T> {
    #[no_coverage]
    pub fn new(key: usize) -> Self {
        Self {
            key,
            phantom: std::marker::PhantomData,
        }
    }
}

impl<T> Copy for SlabKey<T> {}

impl<T> Clone for SlabKey<T> {
    #[no_coverage]
    fn clone(&self) -> Self {
        Self::new(self.key)
    }
}

impl<T> PartialEq for SlabKey<T> {
    #[no_coverage]
    fn eq(&self, other: &Self) -> bool {
        self.key == other.key
    }
}

impl<T> Eq for SlabKey<T> {}

impl<T> fmt::Debug for SlabKey<T> {
    #[no_coverage]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "k{}", self.key)
    }
}

impl<T> PartialOrd for SlabKey<T> {
    #[no_coverage]
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.key.cmp(&other.key))
    }
}
impl<T> Ord for SlabKey<T> {
    #[no_coverage]
    fn cmp(&self, other: &Self) -> Ordering {
        self.key.cmp(&other.key)
    }
}

impl<T> Hash for SlabKey<T> {
    #[no_coverage]
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.key.hash(state);
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
    #[no_coverage]
    pub fn new() -> Self {
        Self {
            storage: Vec::with_capacity(1000),
            available_slots: Vec::with_capacity(32),
        }
    }
    #[no_coverage]
    pub fn len(&self) -> usize {
        self.storage.len() - self.available_slots.len()
    }
    #[no_coverage]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
    #[no_coverage]
    pub fn get_nth_key(&self, n: usize) -> SlabKey<T> {
        let mut idx = n;
        for &i in &self.available_slots {
            if i <= idx {
                idx += 1;
            }
        }
        SlabKey::new(idx)
    }

    #[no_coverage]
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
    #[no_coverage]
    pub fn remove(&mut self, key: SlabKey<T>) {
        self.available_slots.push(key.key);
        self.available_slots.sort();
    }
    #[no_coverage]
    pub fn next_key(&self) -> SlabKey<T> {
        if let Some(&slot) = self.available_slots.last() {
            SlabKey::new(slot)
        } else {
            SlabKey::new(self.storage.len())
        }
    }
    #[no_coverage]
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
    #[no_coverage]
    fn index(&self, key: SlabKey<T>) -> &Self::Output {
        unsafe { self.storage.get_unchecked(key.key) }
    }
}
impl<T> IndexMut<SlabKey<T>> for Slab<T> {
    #[no_coverage]
    fn index_mut(&mut self, key: SlabKey<T>) -> &mut Self::Output {
        unsafe { self.storage.get_unchecked_mut(key.key) }
    }
}

impl<T> Slab<T> {
    #[no_coverage]
    pub fn keys(&self) -> impl Iterator<Item = SlabKey<T>> + '_ {
        (0..self.storage.len())
            .into_iter()
            .filter(move |i| !self.available_slots.contains(i))
            .map(|raw_key| SlabKey::new(raw_key))
    }
}

// ========== WeightedIndex ===========
/// Generate a random f64 within the given range
/// The start and end of the range must be finite
/// This is a very naive implementation
#[inline(always)]
#[no_coverage]
fn gen_f64(rng: &fastrand::Rng, range: Range<f64>) -> f64 {
    range.start + rng.f64() * (range.end - range.start)
}

/**
 * A distribution using weighted sampling to pick a discretely selected item.
 *
 * An alternative implementation of the same type by the `rand` crate.
 */
#[derive(Debug, Clone)]
pub struct WeightedIndex<'a> {
    pub cumulative_weights: &'a Vec<f64>,
}

impl<'a> WeightedIndex<'a> {
    #[no_coverage]
    pub fn sample(&self, rng: &fastrand::Rng) -> usize {
        assert!(!self.cumulative_weights.is_empty());
        if self.cumulative_weights.len() == 1 {
            return 0;
        }

        let range = *self.cumulative_weights.first().unwrap()..*self.cumulative_weights.last().unwrap();
        let chosen_weight = gen_f64(rng, range);
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
