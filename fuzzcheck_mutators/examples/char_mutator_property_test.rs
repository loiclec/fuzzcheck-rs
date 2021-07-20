use std::collections::HashSet;

use fuzzcheck_mutators::CharWithinRangeMutator;
use fuzzcheck_traits::Mutator;

const MAX_CPLX: f64 = 100.;

fn main() {
    let m = CharWithinRangeMutator::new('a'..='z');
    let mut arbitrary_step = m.default_arbitrary_step();

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
                    // println!("{}", x_mut);
                    let is_new = mutated.insert(x_mut); // problem: the mutated char does not depend on its value
                    assert!(is_new);
                    m.unmutate(&mut x_mut, &mut cache, token);
                    assert_eq!(x, x_mut);
                } else {
                    println!("Stopped mutating at {}", j);
                    break;
                }
            }
        } else {
            println!("Stopped arbitraries at {}", i);
            break;
        }
    }
}
