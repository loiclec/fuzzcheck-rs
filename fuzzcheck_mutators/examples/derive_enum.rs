extern crate fuzzcheck_mutators;

extern crate fuzzcheck_mutators_derive;

use fuzzcheck_mutators::fuzzcheck_derive_mutator;
use fuzzcheck_mutators::fuzzcheck_traits::Mutator;
use fuzzcheck_mutators::DefaultMutator;

// #[fuzzcheck_derive_mutator(DefaultMutator)]
// #[derive(Clone, PartialEq, Eq, Debug)]
// pub enum S {
//     A,
//     B(u8),
// }

#[fuzzcheck_derive_mutator(DefaultMutator)]
#[derive(Clone, PartialEq, Eq)]
pub struct SampleData<A, B, C> {
    a: A,
    b: Vec<B>,
    c: C,
    d: X,
}
#[fuzzcheck_derive_mutator(DefaultMutator)]
#[derive(Clone, PartialEq, Eq)]
pub enum X {
    A(u8),
    B(u8),
    C,
    D(bool),
}

fn main() {
    let mut m = Vec::<SampleData<u8, Option<u8>, bool>>::default_mutator();

    let mut results = vec![];

    let mut ar_step = <_>::default();
    for i in 0..100 {
        if let Some((x, x_cache)) = m.ordered_arbitrary(&mut ar_step, 100.0) {
            let x_step = m.initial_step_from_value(&x);
            results.push((x, x_cache, x_step));
        } else {
            println!("could not generate more than {} arbitraries", i + 1);
            break;
        }
    }
    println!("results len: {}", results.len());
    let mut count = 0;
    for _ in 0..1000 {
        if results.is_empty() {
            println!("cannot mutate anything after {} runs", count);
            break;
        }
        let len = results.len();
        let idx = fastrand::usize(0..len);
        let (x, cache, step) = &mut results[idx];
        let prev_x = x.clone();

        // println!("{:?}", x);
        if let Some(token) = m.ordered_mutate(x, cache, step, 100.0) {
            let next = (x.clone(), cache.clone(), m.initial_step_from_value(x));
            let cplx = m.complexity(x, cache);
            assert!(cplx.is_finite() && cplx > 0.0, "{:.2}", cplx);
            let cache_from_scratch = m.cache_from_value(x);
            let cplx_from_scratch = m.complexity(x, &cache_from_scratch);
            assert!(
                (cplx - cplx_from_scratch).abs() < 0.01,
                "{:.15} != {:.15}",
                cplx,
                cplx_from_scratch
            );
            m.unmutate(x, cache, token);
            assert!(x.clone() == prev_x);
            count += 1;
            results.push(next);
        } else {
            println!("cannot mutate {} anymore", idx);
            results.remove(idx);
        }
    }
    for _ in 0..100_000 {
        if results.is_empty() {
            break;
        }
        let len = results.len();
        let (x, cache, step) = &mut results[fastrand::usize(0..len)];
        let prev_x = x.clone();
        if let Some(token) = m.ordered_mutate(x, cache, step, 100.0) {
            let cplx = m.complexity(x, cache);
            assert!(cplx.is_finite() && cplx > 0.0, "{:.2}", cplx);
            let cache_from_scratch = m.cache_from_value(x);
            let cplx_from_scratch = m.complexity(x, &cache_from_scratch);
            assert!(
                (cplx - cplx_from_scratch).abs() < 0.01,
                "{:.15} != {:.15}",
                cplx,
                cplx_from_scratch
            );
            m.unmutate(x, cache, token);
            assert!(x.clone() == prev_x);
        } else {
            continue;
        }
    }
    println!("{}", results.len());
}
