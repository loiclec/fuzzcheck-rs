use fuzzcheck::mutators::{integer::U8Mutator, option::OptionMutator};

#[test]
fn test_option() {
    let m = OptionMutator::new(U8Mutator::default());
    fuzzcheck::mutators::testing_utilities::test_mutator(m, 100.0, 100.0, false, true, 500, 500);
}
