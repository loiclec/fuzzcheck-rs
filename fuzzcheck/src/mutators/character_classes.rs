use std::any::Any;
use std::ops::{Range, RangeInclusive};

use fastrand::Rng;

use super::size_to_cplxity;
use crate::Mutator;

/// Mutator for a `char` within a list of ranges
#[derive(Debug)]
pub struct CharacterMutator {
    ranges: Vec<RangeInclusive<char>>,
    total_length: u32,
    lengths: Vec<Range<u32>>,
    search_space_complexity: f64,
    max_cplx: f64,
    min_cplx: f64,
    rng: Rng,
}
impl CharacterMutator {
    #[coverage(off)]
    pub fn new(ranges: Vec<RangeInclusive<char>>) -> Self {
        let ranges = ranges.into_iter().filter(|r| !r.is_empty()).collect::<Vec<_>>();
        assert!(!ranges.is_empty());
        let total_length = ranges.iter().fold(
            0,
            #[coverage(off)]
            |x, y| x + y.clone().count(),
        ) as u32;
        let lengths = ranges
            .iter()
            .scan(
                0u32,
                #[coverage(off)]
                |x, y| {
                    let start = *x;
                    let end = *x + y.clone().count() as u32;
                    *x = end;
                    Some(start..end)
                },
            )
            .collect::<Vec<_>>();
        let min_cplx = if total_length == 1 {
            let c = *ranges[0].start();
            Self::complexity_of_value(c)
        } else {
            8.0
        };
        let max_cplx = if total_length == 1 {
            let c = *ranges[0].start();
            Self::complexity_of_value(c)
        } else {
            32.0
        };
        let rng = Rng::new();
        Self {
            ranges,
            total_length,
            lengths,
            search_space_complexity: size_to_cplxity(total_length as usize),
            max_cplx,
            min_cplx,
            rng,
        }
    }

    #[coverage(off)]
    pub fn get_char(&self, idx: u32) -> Option<char> {
        for (len_range, range) in self.lengths.iter().zip(&self.ranges) {
            if len_range.contains(&idx) {
                return char::from_u32(*range.start() as u32 + (idx - len_range.start));
            }
        }
        panic!()
    }
}
impl CharacterMutator {
    fn complexity_of_value(c: char) -> f64 {
        (c.len_utf8() * 8) as f64
    }
}
impl Mutator<char> for CharacterMutator {
    #[doc(hidden)]
    type Cache = ();
    #[doc(hidden)]
    type MutationStep = u64;
    #[doc(hidden)]
    type ArbitraryStep = u64;
    #[doc(hidden)]
    type UnmutateToken = char;

    #[doc(hidden)]
    #[coverage(off)]
    fn initialize(&self) {}

    #[doc(hidden)]
    #[coverage(off)]
    fn default_arbitrary_step(&self) -> Self::ArbitraryStep {
        0
    }

    #[doc(hidden)]
    #[coverage(off)]
    fn is_valid(&self, value: &char) -> bool {
        self.ranges.iter().any(
            #[coverage(off)]
            |range| range.contains(value),
        )
    }

    #[doc(hidden)]
    #[coverage(off)]
    fn validate_value(&self, value: &char) -> Option<Self::Cache> {
        if self.ranges.iter().any(
            #[coverage(off)]
            |range| range.contains(value),
        ) {
            Some(())
        } else {
            None
        }
    }
    #[doc(hidden)]
    #[coverage(off)]
    fn default_mutation_step(&self, _value: &char, _cache: &Self::Cache) -> Self::MutationStep {
        0
    }

    #[doc(hidden)]
    #[coverage(off)]
    fn global_search_space_complexity(&self) -> f64 {
        self.search_space_complexity
    }

    #[doc(hidden)]
    #[coverage(off)]
    fn max_complexity(&self) -> f64 {
        self.max_cplx
    }
    #[doc(hidden)]
    #[coverage(off)]
    fn min_complexity(&self) -> f64 {
        self.min_cplx
    }
    #[doc(hidden)]
    #[coverage(off)]
    fn complexity(&self, value: &char, _cache: &Self::Cache) -> f64 {
        Self::complexity_of_value(*value)
    }
    #[doc(hidden)]
    #[coverage(off)]
    fn ordered_arbitrary(&self, step: &mut Self::ArbitraryStep, max_cplx: f64) -> Option<(char, f64)> {
        if *step == self.total_length as u64 {
            return None;
        }
        let idx = crate::mutators::integer::binary_search_arbitrary_u32(0, self.total_length - 1, *step);
        *step += 1;

        if let Some(c) = self.get_char(idx) {
            Some((c, Self::complexity_of_value(c)))
        } else {
            self.ordered_arbitrary(step, max_cplx)
        }
    }
    #[doc(hidden)]
    #[coverage(off)]
    fn random_arbitrary(&self, max_cplx: f64) -> (char, f64) {
        let idx = self.rng.u32(..self.total_length);
        if let Some(c) = self.get_char(idx) {
            (c, Self::complexity_of_value(c))
        } else {
            self.random_arbitrary(max_cplx)
        }
    }
    #[doc(hidden)]
    #[coverage(off)]
    fn ordered_mutate(
        &self,
        value: &mut char,
        cache: &mut Self::Cache,
        step: &mut Self::MutationStep,
        subvalue_provider: &dyn crate::SubValueProvider,
        max_cplx: f64,
    ) -> Option<(Self::UnmutateToken, f64)> {
        if *step == self.total_length as u64 {
            return None;
        }
        let idx = crate::mutators::integer::binary_search_arbitrary_u32(0, self.total_length - 1, *step);
        *step += 1;

        if let Some(mut c) = self.get_char(idx) {
            std::mem::swap(value, &mut c);
            Some((c, Self::complexity_of_value(*value)))
        } else {
            self.ordered_mutate(value, cache, step, subvalue_provider, max_cplx)
        }
    }
    #[doc(hidden)]
    #[coverage(off)]
    fn random_mutate(&self, value: &mut char, cache: &mut Self::Cache, max_cplx: f64) -> (Self::UnmutateToken, f64) {
        let idx = self.rng.u32(..self.total_length);
        if let Some(mut c) = self.get_char(idx) {
            std::mem::swap(value, &mut c);
            (c, Self::complexity_of_value(*value))
        } else {
            self.random_mutate(value, cache, max_cplx)
        }
    }
    #[doc(hidden)]
    #[coverage(off)]
    fn unmutate(&self, value: &mut char, _cache: &mut Self::Cache, t: Self::UnmutateToken) {
        *value = t;
    }

    #[doc(hidden)]
    #[coverage(off)]
    fn visit_subvalues<'a>(&self, _value: &'a char, _cache: &'a Self::Cache, _visit: &mut dyn FnMut(&'a dyn Any, f64)) {
    }
}

#[cfg(test)]
mod tests {
    use crate::mutators::character_classes::CharacterMutator;
    use crate::Mutator;

    #[test]
    #[coverage(off)]
    fn char_classes_test() {
        let chars = vec!['a'..='c', 'f'..='t', '0'..='9'];
        let mutator = CharacterMutator::new(chars);
        println!("{:?}", mutator);

        let mut step = mutator.default_arbitrary_step();
        for i in 0..30 {
            if let Some((ch, _)) = mutator.ordered_arbitrary(&mut step, 10.0) {
                println!("{}: {}", i, ch);
            } else {
                println!("{}: none", i);
            }
        }
    }
}
