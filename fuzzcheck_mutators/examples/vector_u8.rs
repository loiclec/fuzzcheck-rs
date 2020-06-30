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

    for i in 0 .. 100 {
        let (x, x_cache) = m.arbitrary(i, 4096.0);
        let x_step = m.mutation_step_from_value(&x);
        results.push((x, x_cache, x_step));
    }

    for _ in 0..10_000_000 {
        let len = results.len();
        let (x, cache, step) = &mut results[fastrand::usize(0..len)];
        let prev_x = x.clone();

        // println!("{:?}", x);
        let token = m.mutate(x, cache, step, 4096.0);

        let next = (x.clone(), cache.clone(), m.mutation_step_from_value(x));

        // println!("{:?}", x);
        m.unmutate(x, cache, token);
        // println!("{:?}", x);
        assert!(x.clone() == prev_x);

        // results.push(next);
    }
    println!("{}", results.len());
    // for (x, _, _) in results.iter() {
    //     println!("{:?}", x.len());
    // }

    // results.clear();

    // for i in 0..20 {
    //     results.push(m.arbitrary(i, 100.0).0);
    // }

    // println!("{:?}", results);
}
