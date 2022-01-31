use std::ops::{Range, RangeInclusive};

use fastrand::Rng;

use crate::Mutator;

/// Mutator for a `char` within a list of ranges
#[derive(Debug)]
pub struct CharacterMutator {
    ranges: Vec<RangeInclusive<char>>,
    total_length: u32,
    lengths: Vec<Range<u32>>,
    cplx: f64,
    rng: Rng,
}
impl CharacterMutator {
    #[no_coverage]
    pub fn new(ranges: Vec<RangeInclusive<char>>) -> Self {
        let total_length = ranges.iter().fold(
            0,
            #[no_coverage]
            |x, y| x + y.clone().count(),
        ) as u32;
        let lengths = ranges
            .iter()
            .scan(
                0u32,
                #[no_coverage]
                |x, y| {
                    let start = *x;
                    let end = *x + y.clone().count() as u32;
                    *x = end;
                    Some(start..end)
                },
            )
            .collect::<Vec<_>>();
        let cplx = 8.0;
        let rng = Rng::new();
        Self {
            ranges,
            total_length,
            lengths,
            cplx,
            rng,
        }
    }

    #[no_coverage]
    pub fn get_char(&self, idx: u32) -> Option<char> {
        for (len_range, range) in self.lengths.iter().zip(&self.ranges) {
            if len_range.contains(&idx) {
                return char::from_u32(*range.start() as u32 + (idx - len_range.start));
            }
        }
        panic!()
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
    type LensPath = !;

    #[doc(hidden)]
    #[no_coverage]
    fn default_arbitrary_step(&self) -> Self::ArbitraryStep {
        0
    }
    #[doc(hidden)]
    #[no_coverage]
    fn validate_value(&self, value: &char) -> Option<Self::Cache> {
        if self.ranges.iter().any(
            #[no_coverage]
            |range| range.contains(value),
        ) {
            Some(())
        } else {
            None
        }
    }
    #[doc(hidden)]
    #[no_coverage]
    fn default_mutation_step(&self, _value: &char, _cache: &Self::Cache) -> Self::MutationStep {
        0
    }

    #[doc(hidden)]
    #[no_coverage]
    fn max_complexity(&self) -> f64 {
        self.cplx
    }
    #[doc(hidden)]
    #[no_coverage]
    fn min_complexity(&self) -> f64 {
        self.cplx
    }
    #[doc(hidden)]
    #[no_coverage]
    fn complexity(&self, _value: &char, _cache: &Self::Cache) -> f64 {
        self.cplx
    }
    #[doc(hidden)]
    #[no_coverage]
    fn ordered_arbitrary(&self, step: &mut Self::ArbitraryStep, max_cplx: f64) -> Option<(char, f64)> {
        if *step == self.total_length as u64 {
            return None;
        }
        let idx = crate::mutators::integer::binary_search_arbitrary_u32(0, self.total_length - 1, *step);
        *step += 1;

        if let Some(c) = self.get_char(idx) {
            Some((c, self.cplx))
        } else {
            self.ordered_arbitrary(step, max_cplx)
        }
    }
    #[doc(hidden)]
    #[no_coverage]
    fn random_arbitrary(&self, max_cplx: f64) -> (char, f64) {
        let idx = self.rng.u32(..self.total_length);
        if let Some(c) = self.get_char(idx) {
            (c, self.cplx)
        } else {
            self.random_arbitrary(max_cplx)
        }
    }
    #[doc(hidden)]
    #[no_coverage]
    fn ordered_mutate(
        &self,
        value: &mut char,
        cache: &mut Self::Cache,
        step: &mut Self::MutationStep,
        max_cplx: f64,
    ) -> Option<(Self::UnmutateToken, f64)> {
        if *step == self.total_length as u64 {
            return None;
        }
        let idx = crate::mutators::integer::binary_search_arbitrary_u32(0, self.total_length - 1, *step);
        *step += 1;

        if let Some(mut c) = self.get_char(idx) {
            std::mem::swap(value, &mut c);
            Some((c, self.cplx))
        } else {
            self.ordered_mutate(value, cache, step, max_cplx)
        }
    }
    #[doc(hidden)]
    #[no_coverage]
    fn random_mutate(&self, value: &mut char, cache: &mut Self::Cache, max_cplx: f64) -> (Self::UnmutateToken, f64) {
        let idx = self.rng.u32(..self.total_length);
        if let Some(mut c) = self.get_char(idx) {
            std::mem::swap(value, &mut c);
            (c, self.cplx)
        } else {
            self.random_mutate(value, cache, max_cplx)
        }
    }
    #[doc(hidden)]
    #[no_coverage]
    fn unmutate(&self, value: &mut char, _cache: &mut Self::Cache, t: Self::UnmutateToken) {
        *value = t;
    }

    #[doc(hidden)]
    #[no_coverage]
    fn lens<'a>(&self, _value: &'a char, _cache: &Self::Cache, _path: &Self::LensPath) -> &'a dyn std::any::Any {
        unreachable!()
    }

    #[doc(hidden)]
    #[no_coverage]
    fn all_paths(
        &self,
        _value: &char,
        _cache: &Self::Cache,
        _register_path: &mut dyn FnMut(std::any::TypeId, Self::LensPath),
    ) {
    }

    #[doc(hidden)]
    #[no_coverage]
    fn crossover_mutate(
        &self,
        value: &mut char,
        cache: &mut Self::Cache,
        _subvalue_provider: &dyn crate::SubValueProvider,
        max_cplx: f64,
    ) -> (Self::UnmutateToken, f64) {
        self.random_mutate(value, cache, max_cplx)
    }
}

#[cfg(test)]
mod tests {
    use crate::{mutators::character_classes::CharacterMutator, Mutator};

    #[test]
    #[no_coverage]
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
