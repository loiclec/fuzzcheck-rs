use std::ops::RangeInclusive;

use fuzzcheck_mutators::{
    alternation::AlternationMutator, integer::CharWithinRangeMutator, testing_utilities::test_mutator,
};

fn test_alternation_char_helper(ranges: impl IntoIterator<Item = RangeInclusive<char>> + Clone) {
    let m = AlternationMutator::new(ranges.clone().into_iter().map(CharWithinRangeMutator::new).collect());
    test_mutator(m, 100.0, 100.0, false, 100, 1000);
    let m = AlternationMutator::new(ranges.clone().into_iter().map(CharWithinRangeMutator::new).collect());
    test_mutator(m, 1.0, 100.0, false, 100, 1000);
    let m = AlternationMutator::new(ranges.into_iter().map(CharWithinRangeMutator::new).collect());
    test_mutator(m, 1.0, 1.0, false, 100, 1000);
}

#[test]
fn test_alternation_char() {
    test_alternation_char_helper(['a'..='z', '0'..='0']);
    test_alternation_char_helper(['a'..='z']);
    test_alternation_char_helper(['a'..='z', '0'..='9']);
    test_alternation_char_helper(['a'..='z', 'a'..='b', 'a'..='c', '0'..='9', '0'..='5']);
}
