// Note: The following file is largely derived from the fixedbitset
// crate, (crates.io/crates/fixedbitset) of which I have copied the
// license (MIT) below:
//
// Copyright (c) 2015-2017
//
// Permission is hereby granted, free of charge, to any
// person obtaining a copy of this software and associated
// documentation files (the "Software"), to deal in the
// Software without restriction, including without
// limitation the rights to use, copy, modify, merge,
// publish, distribute, sublicense, and/or sell copies of
// the Software, and to permit persons to whom the Software
// is furnished to do so, subject to the following
// conditions:
//
// The above copyright notice and this permission notice
// shall be included in all copies or substantial portions
// of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF
// ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED
// TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A
// PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT
// SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY
// CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION
// OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR
// IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER
// DEALINGS IN THE SOFTWARE.

use std::ops::{BitAnd, BitAndAssign, BitOr, BitOrAssign, BitXor, BitXorAssign};

const BITS: usize = 64;
type Block = u64;

#[inline(always)]
#[coverage(off)]
fn div_rem(x: usize, d: usize) -> (usize, usize) {
    (x / d, x % d)
}

/// `FixedBitSet` is a simple fixed size set of bits that each can
/// be enabled (1 / **true**) or disabled (0 / **false**).
///
/// The bit set has a fixed capacity in terms of enabling bits (and the
/// capacity can grow using the `grow` method).
#[derive(Clone, Debug, Default)]
pub struct FixedBitSet {
    data: Vec<Block>,
    /// length in bits
    length: usize,
}

impl FixedBitSet {
    /// Create a new empty **FixedBitSet**.
    #[coverage(off)]
    pub const fn new() -> Self {
        FixedBitSet {
            data: Vec::new(),
            length: 0,
        }
    }

    /// Create a new **FixedBitSet** with a specific number of bits,
    /// all initially clear.
    #[coverage(off)]
    pub fn with_capacity(bits: usize) -> Self {
        let (mut blocks, rem) = div_rem(bits, BITS);
        blocks += (rem > 0) as usize;
        FixedBitSet {
            data: vec![0; blocks],
            length: bits,
        }
    }

    /// Grow capacity to **bits**, all new bits initialized to zero
    #[coverage(off)]
    pub fn grow(&mut self, bits: usize) {
        if bits > self.length {
            let (mut blocks, rem) = div_rem(bits, BITS);
            blocks += (rem > 0) as usize;
            self.length = bits;
            self.data.resize(blocks, 0);
        }
    }

    /// Return the length of the [`FixedBitSet`] in bits.
    #[inline]
    #[coverage(off)]
    pub fn len(&self) -> usize {
        self.length
    }

    /// Return if the [`FixedBitSet`] is empty.
    #[inline]
    #[coverage(off)]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Return **true** if the bit is enabled in the **FixedBitSet**,
    /// **false** otherwise.
    ///
    /// Note: bits outside the capacity are always disabled.
    ///
    /// Note: Also available with index syntax: `bitset[bit]`.
    #[inline]
    #[coverage(off)]
    pub fn contains(&self, bit: usize) -> bool {
        let (block, i) = div_rem(bit, BITS);
        match self.data.get(block) {
            None => false,
            Some(b) => (b & (1 << i)) != 0,
        }
    }

    /// Clear all bits.
    #[inline]
    #[coverage(off)]
    pub fn clear(&mut self) {
        for elt in &mut self.data {
            *elt = 0
        }
    }

    /// Enable `bit`.
    ///
    /// **Panics** if **bit** is out of bounds.
    #[inline(always)]
    #[coverage(off)]
    pub fn insert(&mut self, bit: usize) {
        assert!(
            bit < self.length,
            "insert at index {} exceeds fixbitset size {}",
            bit,
            self.length
        );
        let (block, i) = div_rem(bit, BITS);
        unsafe {
            *self.data.get_unchecked_mut(block) |= 1 << i;
        }
    }

    /// Enable `bit`, and return its previous value.
    ///
    /// **Panics** if **bit** is out of bounds.
    #[inline]
    #[coverage(off)]
    pub fn put(&mut self, bit: usize) -> bool {
        assert!(
            bit < self.length,
            "put at index {} exceeds fixbitset size {}",
            bit,
            self.length
        );
        let (block, i) = div_rem(bit, BITS);
        unsafe {
            let word = self.data.get_unchecked_mut(block);
            let prev = *word & (1 << i) != 0;
            *word |= 1 << i;
            prev
        }
    }
    /// Toggle `bit` (inverting its state).
    ///
    /// ***Panics*** if **bit** is out of bounds
    #[inline]
    #[coverage(off)]
    pub fn toggle(&mut self, bit: usize) {
        assert!(
            bit < self.length,
            "toggle at index {} exceeds fixbitset size {}",
            bit,
            self.length
        );
        let (block, i) = div_rem(bit, BITS);
        unsafe {
            *self.data.get_unchecked_mut(block) ^= 1 << i;
        }
    }

    /// Count the number of set bits in the given bit range.
    ///
    /// Use `..` to count the whole content of the bitset.
    ///
    /// **Panics** if the range extends past the end of the bitset.
    #[inline]
    #[coverage(off)]
    pub fn count_ones(&self) -> usize {
        let mut sum = 0;
        for block in &self.data {
            sum += block.count_ones();
        }
        sum as usize
    }

    /// Iterates over all enabled bits.
    ///
    /// Iterator element is the index of the `1` bit, type `usize`.
    #[inline]
    #[coverage(off)]
    pub fn ones(&self) -> Ones {
        match self.as_slice().split_first() {
            Some((&block, rem)) => Ones {
                bitset: block,
                block_idx: 0,
                remaining_blocks: rem,
            },
            None => Ones {
                bitset: 0,
                block_idx: 0,
                remaining_blocks: &[],
            },
        }
    }

    /// View the bitset as a slice of `u64` blocks
    #[inline]
    #[coverage(off)]
    pub fn as_slice(&self) -> &[u64] {
        &self.data
    }

    /// In-place union of two `FixedBitSet`s.
    ///
    /// On calling this method, `self`'s capacity may be increased to match `other`'s.
    #[coverage(off)]
    pub fn union_with(&mut self, other: &FixedBitSet) {
        if other.len() >= self.len() {
            self.grow(other.len());
        }
        for (x, y) in self.data.iter_mut().zip(other.data.iter()) {
            *x |= *y;
        }
    }

    /// In-place intersection of two `FixedBitSet`s.
    ///
    /// On calling this method, `self`'s capacity will remain the same as before.
    #[coverage(off)]
    pub fn intersect_with(&mut self, other: &FixedBitSet) {
        for (x, y) in self.data.iter_mut().zip(other.data.iter()) {
            *x &= *y;
        }
        let mn = std::cmp::min(self.data.len(), other.data.len());
        for wd in &mut self.data[mn..] {
            *wd = 0;
        }
    }

    /// In-place difference of two `FixedBitSet`s.
    ///
    /// On calling this method, `self`'s capacity will remain the same as before.
    #[coverage(off)]
    pub fn difference_with(&mut self, other: &FixedBitSet) {
        for (x, y) in self.data.iter_mut().zip(other.data.iter()) {
            *x &= !*y;
        }

        // There's no need to grow self or do any other adjustments.
        //
        // * If self is longer than other, the bits at the end of self won't be affected since other
        //   has them implicitly set to 0.
        // * If other is longer than self, the bits at the end of other are irrelevant since self
        //   has them set to 0 anyway.
    }

    /// In-place symmetric difference of two `FixedBitSet`s.
    ///
    /// On calling this method, `self`'s capacity may be increased to match `other`'s.
    #[coverage(off)]
    pub fn symmetric_difference_with(&mut self, other: &FixedBitSet) {
        if other.len() >= self.len() {
            self.grow(other.len());
        }
        for (x, y) in self.data.iter_mut().zip(other.data.iter()) {
            *x ^= *y;
        }
    }
}

/// An  iterator producing the indices of the set bit in a set.
///
/// This struct is created by the [`FixedBitSet::ones`] method.
pub struct Ones<'a> {
    bitset: Block,
    block_idx: usize,
    remaining_blocks: &'a [Block],
}

impl<'a> Iterator for Ones<'a> {
    type Item = usize; // the bit position of the '1'

    #[inline]
    #[coverage(off)]
    fn next(&mut self) -> Option<Self::Item> {
        while self.bitset == 0 {
            if self.remaining_blocks.is_empty() {
                return None;
            }
            self.bitset = self.remaining_blocks[0];
            self.remaining_blocks = &self.remaining_blocks[1..];
            self.block_idx += 1;
        }
        #[allow(clippy::unnecessary_cast)]
        let t = self.bitset & (0 as Block).wrapping_sub(self.bitset);
        let r = self.bitset.trailing_zeros() as usize;
        self.bitset ^= t;
        Some(self.block_idx * BITS + r)
    }
}

impl<'a> BitAnd for &'a FixedBitSet {
    type Output = FixedBitSet;
    #[coverage(off)]
    fn bitand(self, other: &FixedBitSet) -> FixedBitSet {
        let (short, long) = {
            if self.len() <= other.len() {
                (&self.data, &other.data)
            } else {
                (&other.data, &self.data)
            }
        };
        let mut data = short.clone();
        for (data, block) in data.iter_mut().zip(long.iter()) {
            *data &= *block;
        }
        let len = std::cmp::min(self.len(), other.len());
        FixedBitSet { data, length: len }
    }
}

impl BitAndAssign for FixedBitSet {
    #[coverage(off)]
    fn bitand_assign(&mut self, other: Self) {
        self.intersect_with(&other);
    }
}

impl BitAndAssign<&Self> for FixedBitSet {
    #[coverage(off)]
    fn bitand_assign(&mut self, other: &Self) {
        self.intersect_with(other);
    }
}

impl<'a> BitOr for &'a FixedBitSet {
    type Output = FixedBitSet;
    #[coverage(off)]
    fn bitor(self, other: &FixedBitSet) -> FixedBitSet {
        let (short, long) = {
            if self.len() <= other.len() {
                (&self.data, &other.data)
            } else {
                (&other.data, &self.data)
            }
        };
        let mut data = long.clone();
        for (data, block) in data.iter_mut().zip(short.iter()) {
            *data |= *block;
        }
        let len = std::cmp::max(self.len(), other.len());
        FixedBitSet { data, length: len }
    }
}

impl BitOrAssign for FixedBitSet {
    #[coverage(off)]
    fn bitor_assign(&mut self, other: Self) {
        self.union_with(&other);
    }
}

impl BitOrAssign<&Self> for FixedBitSet {
    #[coverage(off)]
    fn bitor_assign(&mut self, other: &Self) {
        self.union_with(other);
    }
}

impl<'a> BitXor for &'a FixedBitSet {
    type Output = FixedBitSet;
    #[coverage(off)]
    fn bitxor(self, other: &FixedBitSet) -> FixedBitSet {
        let (short, long) = {
            if self.len() <= other.len() {
                (&self.data, &other.data)
            } else {
                (&other.data, &self.data)
            }
        };
        let mut data = long.clone();
        for (data, block) in data.iter_mut().zip(short.iter()) {
            *data ^= *block;
        }
        let len = std::cmp::max(self.len(), other.len());
        FixedBitSet { data, length: len }
    }
}

impl BitXorAssign for FixedBitSet {
    #[coverage(off)]
    fn bitxor_assign(&mut self, other: Self) {
        self.symmetric_difference_with(&other);
    }
}

impl BitXorAssign<&Self> for FixedBitSet {
    #[coverage(off)]
    fn bitxor_assign(&mut self, other: &Self) {
        self.symmetric_difference_with(other);
    }
}
