// This file is derived from the lupine crate, which is licensed under MIT
// and available at https://github.com/greglaurent/lupine/
/*
Permission is hereby granted, free of charge, to any
person obtaining a copy of this software and associated
documentation files (the "Software"), to deal in the
Software without restriction, including without
limitation the rights to use, copy, modify, merge,
publish, distribute, sublicense, and/or sell copies of
the Software, and to permit persons to whom the Software
is furnished to do so, subject to the following
conditions:

The above copyright notice and this permission notice
shall be included in all copies or substantial portions
of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF
ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED
TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A
PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT
SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY
CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION
OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR
IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER
DEALINGS IN THE SOFTWARE.
 */
use std::hash::{Hash, Hasher};
use std::marker::PhantomData;

use ahash::AHasher;
use bit_vec::BitVec;

const FALSE_POS_PROB: f64 = -1.0;
const LN_2: f64 = core::f64::consts::LN_2;
const LN_2_SQR: f64 = LN_2 * LN_2;

/// Representation of a bloom filter
pub struct BloomFilter<T: ?Sized> {
    k: u64,
    m: usize,
    hashers: [AHasher; 2],
    pub bitmap: BitVec,
    _phantom: PhantomData<T>,
}

impl<T: ?Sized> BloomFilter<T> {
    /// Returns a BloomFilter with an optimized k and m
    ///
    /// # Arguments
    ///
    /// * 'size' - A usize that sets the size of the filter
    /// * 'false_pos_rate' - The acceptable false positive rate
    #[coverage(off)]
    pub fn new(size: usize, false_pos_rate: f64) -> Self {
        let k = Self::optimal_k(false_pos_rate);
        let m = Self::optimal_m(false_pos_rate, size);
        let bitmap = BitVec::from_elem(m, false);
        let hashers = [
            AHasher::new_with_keys(fastrand::u128(..), fastrand::u128(..)),
            AHasher::new_with_keys(fastrand::u128(..), fastrand::u128(..)),
        ];
        BloomFilter {
            k,
            m,
            hashers,
            bitmap,
            _phantom: PhantomData,
        }
    }

    /// Calculate optimal m value for the filter
    /// where m is the optimal number of bits in the bit array
    /// while preventing overfill
    ///
    /// where P is the probability of false positives
    /// and n is the acceptable false postive rate
    /// k = ( -( n * lnP ) / (ln2)^2 )
    #[coverage(off)]
    fn optimal_m(false_pos_rate: f64, size: usize) -> usize {
        ((size as f64 * FALSE_POS_PROB * false_pos_rate.ln()) / LN_2_SQR).ceil() as usize
    }

    /// Calculate optimal k value for the filter
    /// where k is the number of functions to hash input T
    /// yielding k indices into the bit array
    ///
    /// where P is the probability of false positives
    /// k = ( - lnP / ln2 )
    #[coverage(off)]
    fn optimal_k(false_pos_rate: f64) -> u64 {
        (false_pos_rate.ln() * FALSE_POS_PROB / LN_2).ceil() as u64
    }

    /// Hash values T for Bloomfilter
    #[coverage(off)]
    fn hash(&self, t: &T) -> (u64, u64)
    where
        T: Hash,
    {
        let hash1 = &mut self.hashers[0].clone();
        let hash2 = &mut self.hashers[1].clone();

        t.hash(hash1);
        t.hash(hash2);

        (hash1.finish(), hash2.finish())
    }

    /// Retrieve the index of indexes by simulating
    /// more than 2 hashers
    ///
    /// Prevent Overflow:
    /// wrapping_add: wrapping add around at the boundary type
    /// wrapping_mul: wrapping mult around at the boundary type
    #[coverage(off)]
    fn find_index(&self, i: u64, hash1: u64, hash2: u64) -> usize {
        hash1.wrapping_add((i).wrapping_mul(hash2)) as usize % self.m
    }

    /// Insert T into the BloomFilter index
    #[coverage(off)]
    pub fn insert(&mut self, t: &T)
    where
        T: Hash,
    {
        let (hash1, hash2) = self.hash(t);

        for i in 0..self.k {
            let index = self.find_index(i, hash1, hash2);
            self.bitmap.set(index, true);
        }
    }

    /// Check if t of type T is in the BloomFilter index
    #[coverage(off)]
    pub fn contains(&mut self, t: &T) -> bool
    where
        T: Hash,
    {
        let (hash1, hash2) = self.hash(t);

        for i in 0..self.k {
            let index = self.find_index(i, hash1, hash2);
            if !self.bitmap.get(index).unwrap() {
                return false;
            }
        }
        true
    }
}
