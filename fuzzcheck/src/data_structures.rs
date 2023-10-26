//! Collection of data structures and algorithms used by the rest of fuzzcheck

extern crate fastrand;

use std::cmp::{Ordering, PartialOrd};
use std::fmt;
use std::fmt::Debug;
use std::hash::Hash;
use std::ops::{Index, IndexMut};

// ========= Slab ============

pub struct SlabKey<T> {
    pub key: usize,
    phantom: std::marker::PhantomData<T>,
}

impl<T> SlabKey<T> {
    #[coverage(off)]
    pub fn new(key: usize) -> Self {
        Self {
            key,
            phantom: std::marker::PhantomData,
        }
    }
}

impl<T> Copy for SlabKey<T> {}

impl<T> Clone for SlabKey<T> {
    #[coverage(off)]
    fn clone(&self) -> Self {
        Self::new(self.key)
    }
}

impl<T> PartialEq for SlabKey<T> {
    #[coverage(off)]
    fn eq(&self, other: &Self) -> bool {
        self.key == other.key
    }
}

impl<T> Eq for SlabKey<T> {}

impl<T> fmt::Debug for SlabKey<T> {
    #[coverage(off)]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "k{}", self.key)
    }
}

impl<T> PartialOrd for SlabKey<T> {
    #[coverage(off)]
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.key.cmp(&other.key))
    }
}
impl<T> Ord for SlabKey<T> {
    #[coverage(off)]
    fn cmp(&self, other: &Self) -> Ordering {
        self.key.cmp(&other.key)
    }
}

impl<T> Hash for SlabKey<T> {
    #[coverage(off)]
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

impl<T: Debug> Debug for Slab<T> {
    #[coverage(off)]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let storage = self
            .keys()
            .map(
                #[coverage(off)]
                |k| &self[k],
            )
            .collect::<Vec<_>>();
        f.debug_struct("Slab")
            .field("storage", &storage)
            .field("available_slots", &self.available_slots.len())
            .finish()
    }
}

impl<T> Slab<T> {
    #[coverage(off)]
    pub fn new() -> Self {
        Self {
            storage: Vec::with_capacity(1000),
            available_slots: Vec::with_capacity(32),
        }
    }
    #[coverage(off)]
    pub fn len(&self) -> usize {
        self.storage.len() - self.available_slots.len()
    }
    // #[coverage(off)]
    // pub fn is_empty(&self) -> bool {
    //     self.len() == 0
    // }
    #[coverage(off)]
    pub fn get_nth_key(&self, n: usize) -> SlabKey<T> {
        let mut idx = n;
        for &i in &self.available_slots {
            if i <= idx {
                idx += 1;
            }
        }
        SlabKey::new(idx)
    }

    #[coverage(off)]
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
    #[coverage(off)]
    pub fn remove(&mut self, key: SlabKey<T>) {
        self.available_slots.push(key.key);
        self.available_slots.sort_unstable();
    }
    // #[coverage(off)]
    // pub fn next_key(&self) -> SlabKey<T> {
    //     if let Some(&slot) = self.available_slots.last() {
    //         SlabKey::new(slot)
    //     } else {
    //         SlabKey::new(self.storage.len())
    //     }
    // }
    // #[coverage(off)]
    // pub fn get_mut(&mut self, key: SlabKey<T>) -> Option<&mut T> {
    //     // O(n) but in practice very fast because there will be almost no available slots
    //     if self.available_slots.contains(&key.key) {
    //         None
    //     } else {
    //         Some(&mut self.storage[key.key])
    //     }
    // }
}

impl<T> Index<SlabKey<T>> for Slab<T> {
    type Output = T;
    #[inline(always)]
    #[coverage(off)]
    fn index(&self, key: SlabKey<T>) -> &Self::Output {
        &self.storage[key.key]
    }
}
impl<T> IndexMut<SlabKey<T>> for Slab<T> {
    #[inline(always)]
    #[coverage(off)]
    fn index_mut(&mut self, key: SlabKey<T>) -> &mut Self::Output {
        &mut self.storage[key.key]
    }
}

impl<T> Slab<T> {
    #[coverage(off)]
    pub fn keys(&self) -> impl Iterator<Item = SlabKey<T>> {
        let available_slots = self.available_slots.clone();
        (0..self.storage.len())
            .into_iter()
            .filter(
                #[coverage(off)]
                move |i| !available_slots.contains(i),
            )
            .map(
                #[coverage(off)]
                |raw_key| SlabKey::new(raw_key),
            )
    }
}

// ========== RcSlab ===========
// Like a Slab, but each entry is ref-counted

#[derive(Debug)]
struct RcSlabSlot<T> {
    data: T,
    ref_count: usize,
}

/**
 * Pre-allocated reference-counted storage for a uniform data type.
 */
pub struct RcSlab<T> {
    storage: Vec<RcSlabSlot<T>>,
    available_slots: Vec<usize>,
}

impl<T: Debug> Debug for RcSlab<T> {
    #[coverage(off)]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let storage = self
            .keys()
            .map(
                #[coverage(off)]
                |k| &self[k],
            )
            .collect::<Vec<_>>();
        f.debug_struct("Slab")
            .field("storage", &storage)
            .field("available_slots", &self.available_slots.len())
            .finish()
    }
}

impl<T> RcSlab<T> {
    #[coverage(off)]
    pub fn new() -> Self {
        Self {
            storage: Vec::with_capacity(1000),
            available_slots: Vec::with_capacity(32),
        }
    }
    // #[coverage(off)]
    // pub fn len(&self) -> usize {
    //     self.storage.len() - self.available_slots.len()
    // }
    // #[coverage(off)]
    // pub fn is_empty(&self) -> bool {
    //     self.len() == 0
    // }
    // #[coverage(off)]
    // pub fn get_nth_key(&self, n: usize) -> SlabKey<T> {
    //     let mut idx = n;
    //     for &i in &self.available_slots {
    //         if i <= idx {
    //             idx += 1;
    //         }
    //     }
    //     SlabKey::new(idx)
    // }

    #[coverage(off)]
    pub fn insert(&mut self, x: T, ref_count: usize) -> usize {
        if let Some(&slot) = self.available_slots.last() {
            self.available_slots.pop();
            self.storage[slot] = RcSlabSlot { data: x, ref_count };
            slot
        } else {
            self.storage.push(RcSlabSlot { data: x, ref_count });
            self.storage.len() - 1
        }
    }
    #[coverage(off)]
    pub fn remove(&mut self, key: usize) {
        let slot = &mut self.storage[key];
        assert!(slot.ref_count > 0);
        slot.ref_count -= 1;
        if slot.ref_count == 0 {
            self.available_slots.push(key);
            self.available_slots.sort_unstable();
        }
    }
    #[coverage(off)]
    pub fn next_slot(&self) -> usize {
        if let Some(&slot) = self.available_slots.last() {
            slot
        } else {
            self.storage.len()
        }
    }
    #[coverage(off)]
    pub fn get_mut(&mut self, key: usize) -> Option<&mut T> {
        // O(n) but in practice very fast because there will be almost no available slots
        if self.available_slots.contains(&key) {
            None
        } else {
            Some(&mut self.storage[key].data)
        }
    }

    #[coverage(off)]
    pub fn get_mut_and_ref(&mut self, key1: usize, key2: usize) -> Option<(&mut T, &T)> {
        if key1 == key2 {
            panic!("key1 must be different than key2");
        }
        if self.available_slots.contains(&key1) || self.available_slots.contains(&key2) {
            None
        } else {
            if key1 < key2 {
                let (slice1, slice2) = self.storage.split_at_mut(key1 + 1);
                let a = &mut slice1[key1].data;
                let b = &mut slice2[key2 - key1 - 1].data;
                Some((a, b))
            } else {
                // key1 > key2
                let (slice1, slice2) = self.storage.split_at_mut(key2 + 1);
                let b = &mut slice1[key2].data;
                let a = &mut slice2[key1 - key2 - 1].data;
                Some((a, b))
            }
        }
    }
}

impl<T> Index<usize> for RcSlab<T> {
    type Output = T;
    #[inline(always)]
    #[coverage(off)]
    fn index(&self, key: usize) -> &Self::Output {
        &self.storage[key].data
    }
}
impl<T> IndexMut<usize> for RcSlab<T> {
    #[inline(always)]
    #[coverage(off)]
    fn index_mut(&mut self, key: usize) -> &mut Self::Output {
        &mut self.storage[key].data
    }
}

impl<T> RcSlab<T> {
    #[coverage(off)]
    pub fn keys(&self) -> impl Iterator<Item = usize> + '_ {
        (0..self.storage.len())
            .into_iter()
            .filter(move |i| !self.available_slots.contains(i))
    }
}
