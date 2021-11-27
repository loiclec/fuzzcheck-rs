use std::collections::HashSet;

use fuzzcheck::mutators::{integer::U8Mutator, vector::VecMutator};
use fuzzcheck::DefaultMutator;
use fuzzcheck::Mutator;
#[test]
fn test_vector_mutator() {
    // let m = VecMutator::new(U8Mutator::default(), 0..=10);
    // fuzzcheck_mutators::testing_utilities::test_mutator(m, 100.0, 100.0, false, 500, 500);
    // let m = VecMutator::new(U8Mutator::default(), 0..=10);
    // fuzzcheck_mutators::testing_utilities::test_mutator(m, 20000.0, 20000.0, false, 500, 500);
    // let m = VecMutator::new(U8Mutator::default(), 10..=20);
    // fuzzcheck_mutators::testing_utilities::test_mutator(m, 10000.0, 10000.0, false, 500, 500);
    // // todo: test with an unlimited range

    let m = VecMutator::new(VecMutator::new(U8Mutator::default(), 0..=usize::MAX), 0..=usize::MAX);
    fuzzcheck::mutators::testing_utilities::test_mutator(m, 500.0, 500.0, false, 100, 200);
}

#[test]
fn test_vector_explore() {
    // let m = VecMutator::new(VecMutator::new(U8Mutator::default(), 0..=5), 0..=5);
    let m = VecMutator::new(<Option<u16>>::default_mutator(), 0..=32); //VecMutator::new(VecMutator::new(U8Mutator::default(), 0..=5), 0..=10);
    let mut step = m.default_arbitrary_step();
    // let (x, cplx) = m.ordered_arbitrary(&mut step, 1000.0).unwrap();
    // println!("{:?}", x);
    // println!("cplx: {}", cplx);
    let mut sum = 0;
    let mut total = 0;
    for _ in 0..10 {
        if let Some((mut x, _cplx)) = m.ordered_arbitrary(&mut step, 1000.0) {
            assert!((0..=32).contains(&x.len()));
            // println!("{:?}", x);
            // println!("cplx: {}", cplx);
            let mut cache = m.validate_value(&x).unwrap();
            let mut step = m.default_mutation_step(&x, &cache);
            let mut all = HashSet::new();
            for i in 0..10_000 {
                total += 1;
                if let Some((token, _cplx)) = m.ordered_mutate(&mut x, &mut cache, &mut step, 1000.) {
                    all.insert(x.clone());
                    // println!("{:?}", x);
                    // println!("\t{:?}", x);
                    assert!((0..=32).contains(&x.len()), "{}", x.len());
                    m.unmutate(&mut x, &mut cache, token);
                    assert!((0..=32).contains(&x.len()), "{}", x.len());
                } else {
                    println!("!!!!!!! STOP at {} !!!!!!", i);
                    break;
                }
                // let (token, _) = m.random_mutate(&mut x, &mut cache, 1000.);
                // assert!((0..=32).contains(&x.len()));
                // all.insert(x.clone());
                // m.unmutate(&mut x, &mut cache, token);
            }
            sum += all.len();
            println!("===");
        } else {
            break;
        }
    }
    println!("{}", sum as f64 / total as f64);
}

#[test]
fn test_vector_explore2() {
    let m = VecMutator::new(<()>::default_mutator(), 0..=usize::MAX); //VecMutator::new(VecMutator::new(U8Mutator::default(), 0..=5), 0..=10);
    let mut step = m.default_arbitrary_step();
    for j in 0..36 {
        if let Some((mut x, _cplx)) = m.ordered_arbitrary(&mut step, 32.0) {
            println!("{} {:?}", x.len(), x);
            let mut cache = m.validate_value(&x).unwrap();
            let mut step = m.default_mutation_step(&x, &cache);
            for i in 0..40 {
                if let Some((token, _cplx)) = m.ordered_mutate(&mut x, &mut cache, &mut step, 32.) {
                    println!("{} {:?}", x.len(), x);
                    m.unmutate(&mut x, &mut cache, token);
                } else {
                    println!("!!!!!!! STOP at {} !!!!!!", i);
                    break;
                }
            }
            println!("===");
        } else {
            println!("no more arbitraries!! {}", j);
            break;
        }
    }
}
