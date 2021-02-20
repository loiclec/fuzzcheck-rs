//! Collection of data structures and algorithms used by the rest of fuzzcheck

extern crate fastrand;

use std::cmp::Ordering;
use std::cmp::PartialOrd;

use std::ops::Index;
use std::ops::IndexMut;
use std::ops::Range;

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
/// Generate a random f64 within the given range
/// The start and end of the range must be finite
/// This is a very naive implementation
#[inline(always)]
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

#[cfg(all(trace_compares, test))]
const SIZE: usize = 0b1 << 30;
#[cfg(trace_compares)]
const L0_SIZE: usize = 0b1 << 24;
#[cfg(trace_compares)]
const L1_SIZE: usize = 0b1 << 18;
#[cfg(trace_compares)]
const L2_SIZE: usize = 0b1 << 12;
#[cfg(trace_compares)]
const L3_SIZE: usize = 0b1 << 6;

#[cfg(trace_compares)]
pub struct HBitSet {
    l0: Vec<u64>,
    l1: Vec<u64>,
    l2: Vec<u64>,
    l3: Vec<u64>,
}

#[cfg(trace_compares)]
impl HBitSet {
    pub fn new() -> Self {
        Self {
            l0: std::iter::repeat(0).take(L0_SIZE).collect(),
            l1: std::iter::repeat(0).take(L1_SIZE).collect(),
            l2: std::iter::repeat(0).take(L2_SIZE).collect(),
            l3: std::iter::repeat(0).take(L3_SIZE).collect(),
        }
    }

    #[inline]
    pub fn set(&mut self, mut idx: usize) {
        // assert!(idx < SIZE);

        let bit = 0b1 << (idx % 64);
        idx /= 64;
        unsafe {
            *self.l0.get_unchecked_mut(idx) |= bit;
        }

        let bit = 0b1 << (idx % 64);
        idx /= 64;
        unsafe {
            *self.l1.get_unchecked_mut(idx) |= bit;
        }

        let bit = 0b1 << (idx % 64);
        idx /= 64;
        unsafe {
            *self.l2.get_unchecked_mut(idx) |= bit;
        }

        let bit = 0b1 << (idx % 64);
        idx /= 64;
        unsafe {
            *self.l3.get_unchecked_mut(idx) |= bit;
        }
    }

    // pub fn test(&self, el: usize) -> bool {
    //     let (idx, bit) = (el / 64, el % 64);

    //     self.l0[idx] & (0b1 << bit) != 0
    // }

    pub fn drain(&mut self, mut f: impl FnMut(u64)) {
        for (idx, map) in self.l3.iter_mut().enumerate() {
            if *map == 0 {
                continue;
            }
            for bit in 0..64 {
                if *map & (0b1 << bit) == 0 {
                    continue;
                }

                let inner_idx = idx * 64 + bit;

                for (idx, map) in unsafe {
                    self.l2
                        .get_unchecked_mut(inner_idx..inner_idx + (64 - bit))
                        .iter_mut()
                        .enumerate()
                } {
                    if *map == 0 {
                        continue;
                    }
                    for bit in 0..64 {
                        if *map & (0b1 << bit) == 0 {
                            continue;
                        }

                        let inner_idx = (inner_idx + idx) * 64 + bit;

                        for (idx, map) in unsafe {
                            self.l1
                                .get_unchecked_mut(inner_idx..inner_idx + (64 - bit))
                                .iter_mut()
                                .enumerate()
                        } {
                            if *map == 0 {
                                continue;
                            }
                            for bit in 0..64 {
                                if *map & (0b1 << bit) == 0 {
                                    continue;
                                }

                                let inner_idx = (inner_idx + idx) * 64 + bit;

                                for (idx, map) in unsafe {
                                    self.l0
                                        .get_unchecked_mut(inner_idx..inner_idx + (64 - bit))
                                        .iter_mut()
                                        .enumerate()
                                } {
                                    if *map == 0 {
                                        continue;
                                    }
                                    let element = ((inner_idx + idx) as u64) * 64;
                                    for bit in 0..64 {
                                        if *map & (0b1 << bit) != 0 {
                                            f(element + bit);
                                        }
                                    }

                                    *map = 0;
                                }
                            }

                            *map = 0;
                        }
                    }

                    *map = 0;
                }
            }

            *map = 0;
        }
    }
}

#[cfg(all(test, trace_compares))]
mod bench_hbitset {
    extern crate test;
    use std::collections::HashSet;

    use super::*;
    use test::Bencher;

    #[test]
    fn test_set_correct() {
        let mut hbitset = HBitSet::new();
        let mut set = HashSet::new();
        for _ in 0..1000_000 {
            let j = fastrand::usize(0..SIZE);
            set.insert(j);
            hbitset.set(j);
            // hbitset.set(i);
            assert!(hbitset.test(j), "{}", j);
            // hbitset.set(SIZE - 1 - i);
            // assert!(hbitset.test(SIZE - 1 - i), "{}", i);
            // assert!(hbitset.test(i - 2000) == false, "{}", i);
        }

        let mut count = 0;
        hbitset.drain(|j| {
            assert!(set.contains(&(j as usize)));
            count += 1;
        });
        assert_eq!(count, set.len());
        // for i in 10_000_001..20_000_000 {
        //     assert!(hbitset.test(i) == false, "{}", i);
        // }
    }

    #[bench]
    fn test_set(b: &mut Bencher) {
        let mut hbitset = HBitSet::new();
        b.iter(|| {
            for _ in 0..10 {
                for i in 0..100_000 {
                    let j = fastrand::usize(0..SIZE);
                    hbitset.set(j);
                }
            }
            std::hint::black_box(&hbitset);
        });
    }
    #[bench]
    fn test_drain(b: &mut Bencher) {
        let mut hbitset = HBitSet::new();
        b.iter(|| {
            for _ in 0..10 {
                for i in 0..10_000 {
                    let j = fastrand::usize(0..SIZE);
                    hbitset.set(j);
                }
                let mut sum = 0;
                hbitset.drain(|_| sum += 1);
                std::hint::black_box(sum);
                assert!(sum > 0);
            }
        });
    }
}
