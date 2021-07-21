use std::{collections::HashSet, ops::RangeInclusive};

use fuzzcheck_mutators::{alternation::AlternationMutator, integer::CharWithinRangeMutator};
use fuzzcheck_traits::Mutator;

const MAX_CPLX: f64 = 100.;

fn alternation_char_mutator_property_test(ranges: impl IntoIterator<Item = RangeInclusive<char>> + Clone) {
    let m = AlternationMutator::new(ranges.clone().into_iter().map(CharWithinRangeMutator::new).collect());
    let mut arbitrary_step = m.default_arbitrary_step();

    let total_count = ranges.clone().into_iter().fold(0, |acc, range| acc + range.count());

    println!("{}", total_count);

    let mut arbitraries = HashSet::new();
    for i in 0..100 {
        if let Some((x, _cplx)) = m.ordered_arbitrary(&mut arbitrary_step, MAX_CPLX) {
            let is_new = arbitraries.insert(x);
            assert!(is_new);
            let (mut cache, mut mutation_step) = m.validate_value(&x).unwrap();
            let mut mutated = HashSet::new();
            mutated.insert(x);
            let mut x_mut = x;
            for j in 0..100 {
                if let Some((token, _cplx)) = m.ordered_mutate(&mut x_mut, &mut cache, &mut mutation_step, MAX_CPLX) {
                    println!("{}", x_mut);
                    let _is_new = mutated.insert(x_mut); // problem: the mutated char does not depend on its value
                                                         // assert!(is_new);
                    m.unmutate(&mut x_mut, &mut cache, token);
                    assert_eq!(x, x_mut);
                } else {
                    assert!(j >= total_count - 1 && j < total_count + 5);
                    println!("Stopped mutating at {}", j);
                    break;
                }
            }
            for range in ranges.clone().into_iter() {
                for c in range {
                    assert!(mutated.contains(&c));
                }
            }
        } else {
            println!("Stopped arbitraries at {}", i);
            break;
        }
    }
    for range in ranges.into_iter() {
        for c in range {
            assert!(arbitraries.contains(&c));
        }
    }
}

#[test]
fn test_alternation_char_mutator_letters_or_digits() {
    alternation_char_mutator_property_test(['a'..='z', '0'..='9']);
}

#[test]
fn test_alternation_char_mutator_one_range() {
    alternation_char_mutator_property_test(['a'..='z']);
}

#[test]
fn test_alternation_char_mutator_two_ranges_but_one_has_just_one_element() {
    alternation_char_mutator_property_test(['a'..='z', '0'..='0']);
}
