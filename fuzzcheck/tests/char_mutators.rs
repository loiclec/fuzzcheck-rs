use fuzzcheck::mutators::char::CharWithinRangeMutator;
use fuzzcheck::mutators::testing_utilities::*;

#[test]
fn other_test_char_mutator() {
    test_mutator(CharWithinRangeMutator::new('a'..='z'), 100.0, 100.0, true, 100, 100);
    test_mutator(CharWithinRangeMutator::new('a'..='z'), 1.0, 1.0, true, 100, 100);
    test_mutator(CharWithinRangeMutator::new('a'..='z'), 100.0, 1.0, true, 100, 100);
    test_mutator(CharWithinRangeMutator::new('a'..='a'), 100.0, 100.0, true, 100, 100);
    test_mutator(CharWithinRangeMutator::new('a'..='b'), 100.0, 100.0, true, 100, 100);
}
