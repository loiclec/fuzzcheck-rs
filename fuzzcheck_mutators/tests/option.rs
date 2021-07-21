use fuzzcheck_mutators::{integer::U8Mutator, option::OptionMutator};
use fuzzcheck_traits::Mutator;
use std::collections::HashSet;

const MAX_CPLX: f64 = 100.;
#[test]
fn test_option() {
    let m = OptionMutator::new(U8Mutator::default());
    let mut arbitrary_step = m.default_arbitrary_step();

    let mut arbitraries = HashSet::new();
    for i in 0..500 {
        if let Some((x, _cplx)) = m.ordered_arbitrary(&mut arbitrary_step, MAX_CPLX) {
            let is_new = arbitraries.insert(x);
            assert!(is_new);
            let (mut cache, mut mutation_step) = m.validate_value(&x).unwrap();
            let mut mutated = HashSet::new();
            mutated.insert(x);
            let mut x_mut = x;
            for j in 0..500 {
                if let Some((token, _cplx)) = m.ordered_mutate(&mut x_mut, &mut cache, &mut mutation_step, MAX_CPLX) {
                    // println!("{:?}", x_mut);
                    let _ = mutated.insert(x_mut);
                    // assert!(is_mutated_new);
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
