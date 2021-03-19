use fuzzcheck_mutators::fuzzcheck_traits::Mutator;
use fuzzcheck_mutators::*;

fn main() {
    let m = I64WithinRangeMutator::new(-100..=6700);
    let mut step = 0;
    let mut results = vec![];
    for i in 0..300 {
        if let Some((value, _)) = m.ordered_arbitrary(&mut step, f64::INFINITY) {
            println!("i: {}", value);
            results.push(value);
        } else {
            println!("{}: None", i);
            break;
        }
    }
    results.sort();
    println!("{:?}", results);
    results.clear();
    let original = 99;
    let mut value = original;
    let mut step = 0;
    for i in 0..300 {
        if let Some(t) = m.ordered_mutate(&mut value, &mut (), &mut step, f64::INFINITY) {
            println!("i: {}", value);
            results.push(value);
            m.unmutate(&mut value, &mut (), t);
            assert!(value == original);
        } else {
            println!("{}: None", i);
            break;
        }
    }
    results.sort();
    println!("{:?}", results);
}
