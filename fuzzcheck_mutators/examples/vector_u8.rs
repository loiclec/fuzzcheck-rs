extern crate fuzzcheck_mutators;

use fuzzcheck_traits::Mutator;

use fuzzcheck_mutators::integer::*;
use fuzzcheck_mutators::vector::*;

fn main() {
    type M = VecMutator<U8Mutator>;
    let mut m = M::default();

    let x = vec![];
    let x_cache = m.cache_from_value(&x);
    let x_step = m.mutation_step_from_value(&x);

    let mut results = vec![(x, x_cache, x_step)];

    for i in 0..100 {
        let (x, x_cache) = m.arbitrary(i, 4096.0);
        let x_step = m.mutation_step_from_value(&x);
        results.push((x, x_cache, x_step));
    }

    for _ in 0..10_000_000 {
        let len = results.len();
        let (x, cache, step) = &mut results[fastrand::usize(0..len)];
        let prev_x = x.clone();
        let token = m.mutate(x, cache, step, 4096.0);
        m.unmutate(x, cache, token);
        assert!(x.clone() == prev_x);
    }
    println!("{}", results.len());
}
