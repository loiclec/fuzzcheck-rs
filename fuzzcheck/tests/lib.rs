#![feature(no_coverage)]
#![feature(trivial_bounds)]

use fuzzcheck::mutators::vector::VecMutator;
use fuzzcheck::{CrossoverArbitraryResult, CrossoverSubValueProvider, DefaultMutator, Mutator};
// mod alternation_char_mutators;
// mod char_mutators;
// mod constrained_integer;
// mod derived_mutually_recursive_structs;
// mod derived_recursive_struct;
// mod expansions;
// #[cfg(feature = "regex_grammar")]
// mod grammar_based_mutators;
// mod option;
// mod vector;

#[test]
fn test_crossover_vec() {
    let m = VecMutator::new(u8::default_mutator(), 0..=10);
    let (value, _) = m.random_arbitrary(100.0);
    let cache = m.validate_value(&value).unwrap();
    println!("{:?}", value);

    let all_paths = m.all_paths(&value, &cache);
    let mut subvalue_provider = CrossoverSubValueProvider::from(&m, &value, &cache, &all_paths);

    for _ in 0..10 {
        let CrossoverArbitraryResult {
            value,
            complexity,
            complexity_from_crossover,
        } = m.crossover_arbitrary(&mut subvalue_provider, 100.0, 200.0);
        println!("{value:?} {complexity:.2} {complexity_from_crossover:.2}");
    }
}
