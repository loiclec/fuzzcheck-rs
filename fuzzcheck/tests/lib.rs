#![feature(no_coverage)]
#![feature(trivial_bounds)]

use fuzzcheck::mutators::vector::VecMutator;
use fuzzcheck::{CrossoverSubValueProvider, DefaultMutator, Mutator};
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
    let m = VecMutator::<(u8, u16), _>::new(<(u8, u16)>::default_mutator(), 5..=10);
    let (value, _) = m.random_arbitrary(1000.0);
    let cache = m.validate_value(&value).unwrap();
    println!("{:?}", value);
    let all_paths = m.all_paths(&value, &cache);
    for (key, subpaths) in all_paths.iter() {
        println!("{:?} : {:?}", key, subpaths.len());
    }
    let mut subvalue_provider = CrossoverSubValueProvider::from(&m, &value, &cache, &all_paths);
}
