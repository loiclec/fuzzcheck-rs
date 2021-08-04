use std::{collections::HashSet, ops::RangeBounds};

use fuzzcheck_mutators::integer_within_range::I8WithinRangeMutator;
use crate::Mutator;

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
