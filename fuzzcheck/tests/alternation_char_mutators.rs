use std::ops::RangeInclusive;

use fuzzcheck::mutators::{
    alternation::AlternationMutator, char::CharWithinRangeMutator, testing_utilities::test_mutator,
};

fn test_alternation_char_helper(ranges: impl IntoIterator<Item = RangeInclusive<char>> + Clone) {
    let m = AlternationMutator::new(ranges.clone().into_iter().map(CharWithinRangeMutator::new).collect());
    test_mutator(m, 100.0, 100.0, false, true, 100, 1000);
    let m = AlternationMutator::new(ranges.clone().into_iter().map(CharWithinRangeMutator::new).collect());
    test_mutator(m, 1.0, 100.0, false, true, 100, 1000);
    let m = AlternationMutator::new(ranges.into_iter().map(CharWithinRangeMutator::new).collect());
    test_mutator(m, 1.0, 1.0, false, true, 100, 1000);
}

#[test]
fn test_alternation_char() {
    test_alternation_char_helper(['a'..='z', '0'..='0']);
    test_alternation_char_helper(['a'..='z']);
    test_alternation_char_helper(['a'..='z', '0'..='9']);
    // this will fail because the submutators give different complexities byt the letter 'a' is
    // a possibility for all three first choices.
    // test_alternation_char_helper(['a'..='z', 'a'..='b', 'a'..='c', '0'..='9', '0'..='5']);
}
