//! Collection of data structures and algorithms used by the rest of fuzzcheck

extern crate fastrand;

use std::cmp::Ordering;
use std::cmp::PartialOrd;
use std::fmt;
use std::fmt::Debug;
use std::hash::Hash;
use std::ops::Index;
use std::ops::IndexMut;

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

impl<T: Debug> Debug for Slab<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let storage = self.keys().map(|k| &self[k]).collect::<Vec<_>>();
        f.debug_struct("Slab")
            .field("storage", &storage)
            .field("available_slots", &self.available_slots.len())
            .finish()
    }
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
    // #[no_coverage]
    // pub fn is_empty(&self) -> bool {
    //     self.len() == 0
    // }
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
    // #[no_coverage]
    // pub fn next_key(&self) -> SlabKey<T> {
    //     if let Some(&slot) = self.available_slots.last() {
    //         SlabKey::new(slot)
    //     } else {
    //         SlabKey::new(self.storage.len())
    //     }
    // }
    // #[no_coverage]
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
    #[no_coverage]
    fn index(&self, key: SlabKey<T>) -> &Self::Output {
        &self.storage[key.key]
    }
}
impl<T> IndexMut<SlabKey<T>> for Slab<T> {
    #[inline(always)]
    #[no_coverage]
    fn index_mut(&mut self, key: SlabKey<T>) -> &mut Self::Output {
        &mut self.storage[key.key]
    }
}

impl<T> Slab<T> {
    #[no_coverage]
    pub fn keys(&self) -> impl Iterator<Item = SlabKey<T>> {
        let available_slots = self.available_slots.clone();
        (0..self.storage.len())
            .into_iter()
            .filter(move |i| !available_slots.contains(i))
            .map(|raw_key| SlabKey::new(raw_key))
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
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let storage = self.keys().map(|k| &self[k]).collect::<Vec<_>>();
        f.debug_struct("Slab")
            .field("storage", &storage)
            .field("available_slots", &self.available_slots.len())
            .finish()
    }
}

impl<T> RcSlab<T> {
    #[no_coverage]
    pub fn new() -> Self {
        Self {
            storage: Vec::with_capacity(1000),
            available_slots: Vec::with_capacity(32),
        }
    }
    // #[no_coverage]
    // pub fn len(&self) -> usize {
    //     self.storage.len() - self.available_slots.len()
    // }
    // #[no_coverage]
    // pub fn is_empty(&self) -> bool {
    //     self.len() == 0
    // }
    // #[no_coverage]
    // pub fn get_nth_key(&self, n: usize) -> SlabKey<T> {
    //     let mut idx = n;
    //     for &i in &self.available_slots {
    //         if i <= idx {
    //             idx += 1;
    //         }
    //     }
    //     SlabKey::new(idx)
    // }

    #[no_coverage]
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
    #[no_coverage]
    pub fn remove(&mut self, key: usize) {
        let slot = &mut self.storage[key];
        assert!(slot.ref_count > 0);
        slot.ref_count -= 1;
        if slot.ref_count == 0 {
            self.available_slots.push(key);
            self.available_slots.sort();
        }
    }
    #[no_coverage]
    pub fn next_slot(&self) -> usize {
        if let Some(&slot) = self.available_slots.last() {
            slot
        } else {
            self.storage.len()
        }
    }
    #[no_coverage]
    pub fn get_mut(&mut self, key: usize) -> Option<&mut T> {
        // O(n) but in practice very fast because there will be almost no available slots
        if self.available_slots.contains(&key) {
            None
        } else {
            Some(&mut self.storage[key].data)
        }
    }
}

impl<T> Index<usize> for RcSlab<T> {
    type Output = T;
    #[inline(always)]
    #[no_coverage]
    fn index(&self, key: usize) -> &Self::Output {
        &self.storage[key].data
    }
}
impl<T> IndexMut<usize> for RcSlab<T> {
    #[inline(always)]
    #[no_coverage]
    fn index_mut(&mut self, key: usize) -> &mut Self::Output {
        &mut self.storage[key].data
    }
}

impl<T> RcSlab<T> {
    #[no_coverage]
    pub fn keys(&self) -> impl Iterator<Item = usize> + '_ {
        (0..self.storage.len())
            .into_iter()
            .filter(move |i| !self.available_slots.contains(i))
    }
}
