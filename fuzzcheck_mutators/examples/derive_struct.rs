extern crate fuzzcheck_mutators;

extern crate fuzzcheck_mutators_derive;

use fuzzcheck_mutators::fuzzcheck_derive_mutator;
use fuzzcheck_mutators::fuzzcheck_traits::Mutator;
use fuzzcheck_mutators::DefaultMutator;

#[fuzzcheck_derive_mutator(DefaultMutator)]
#[derive(PartialEq, Eq, Debug, Default)]
pub struct S<A, B, C> {
    pub a: A,
    pub b: Vec<B>,
    pub c: Vec<C>,
}

impl<A, B, C> Clone for S<A, B, C>
where
    A: Clone,
    Vec<B>: Clone,
    Vec<C>: Clone,
{
    fn clone(&self) -> Self {
        Self {
            a: self.a.clone(),
            b: self.b.clone(),
            c: self.c.clone(),
        }
    }
}

fn main() {
    let mut m = S::<u8, u8, u8>::default_mutator();

    let x = S::<u8, u8, u8>::default();
    let x_cache = m.cache_from_value(&x);
    let x_step = m.initial_step_from_value(&x);

    let mut results = vec![(x, x_cache, x_step)];
    let mut ar_step = <_>::default();
    for _ in 0..10 {
        let (x, x_cache) = m.ordered_arbitrary(&mut ar_step, 100.0).unwrap();
        let x_step = m.initial_step_from_value(&x);
        results.push((x, x_cache, x_step));
    }
    for _ in 0..100_000 {
        let len = results.len();
        let (x, cache, step) = &mut results[fastrand::usize(0..len)];
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
            results.push(next);
        } else {
            panic!()
        }
    }

    for _ in 0..100_000 {
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
            panic!()
        }
    }
    println!("{}", results.len());
}
