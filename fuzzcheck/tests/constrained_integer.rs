use std::{collections::HashSet, ops::RangeBounds};

use fuzzcheck::mutators::integer_within_range::I8WithinRangeMutator;
use fuzzcheck::mutators::testing_utilities::test_mutator;
use fuzzcheck::Mutator;

fn test_arbitrary_for_int_range_mutator(range: impl RangeBounds<i8> + IntoIterator<Item = i8> + Clone) {
    let m = I8WithinRangeMutator::new(range.clone());
    for _ in 0..1000 {
        let x = m.random_arbitrary(100.0).0;
        assert!(range.contains(&x), "{}", x);
    }
    let mut step = 0;
    let mut all_generated = HashSet::new();
    while let Some((x, _)) = m.ordered_arbitrary(&mut step, 100.0) {
        let is_new = all_generated.insert(x);
        assert!(is_new);
    }
    for x in range {
        assert!(all_generated.contains(&x));
    }
}
#[test]
fn test_arbitrary_constrained_signed_integer_8() {
    test_arbitrary_for_int_range_mutator(-128..12);
    test_arbitrary_for_int_range_mutator(5..10);
    test_arbitrary_for_int_range_mutator(0..=0);
    test_arbitrary_for_int_range_mutator(-128..=127);
    test_arbitrary_for_int_range_mutator(-100..50);
}

#[test]
fn test_mutate_constrained_signed_integer_8() {
    let mutator = I8WithinRangeMutator::new(-128..127);
    test_mutator(mutator, 1000., 1000., false, true, 100, 100);

    let mutator = I8WithinRangeMutator::new(-128..=127);
    let mut x = 0;
    let mut x_cache = mutator.validate_value(&x).unwrap();
    let mut x_step = mutator.default_mutation_step(&x, &x_cache);
    let mut set = HashSet::new();
    for _ in 0..256 {
        let (t, _c) = mutator
            .ordered_mutate(
                &mut x,
                &mut x_cache,
                &mut x_step,
                &fuzzcheck::subvalue_provider::EmptySubValueProvider,
                100.0,
            )
            .unwrap();
        set.insert(x);
        mutator.unmutate(&mut x, &mut x_cache, t);
    }
    let mut set = set.into_iter().collect::<Vec<_>>();
    set.sort();
    println!("{} {set:?}", set.len());

    let mutator = I8WithinRangeMutator::new(-12..17);
    let mut x = 0;
    let mut x_cache = mutator.validate_value(&x).unwrap();
    let mut x_step = mutator.default_mutation_step(&x, &x_cache);
    let mut set = HashSet::new();
    for _ in 0..(12 + 16) {
        let (t, _c) = mutator
            .ordered_mutate(
                &mut x,
                &mut x_cache,
                &mut x_step,
                &fuzzcheck::subvalue_provider::EmptySubValueProvider,
                100.0,
            )
            .unwrap();
        set.insert(x);
        mutator.unmutate(&mut x, &mut x_cache, t);
    }
    let mut set = set.into_iter().collect::<Vec<_>>();
    set.sort();
    println!("{} {set:?}", set.len());
}
