use fuzzcheck_mutators::{integer::U8Mutator, option::OptionMutator};

#[test]
fn test_option() {
    let m = OptionMutator::new(U8Mutator::default());
    fuzzcheck_mutators::testing_utilities::test_mutator(m, 100.0, 100.0, false, 500, 500);
}
