extern crate fuzzcheck_mutators;
use fuzzcheck::Mutator;
use fuzzcheck_mutators::integer::*;
use fuzzcheck_mutators::vector::*;

extern crate rand;
use rand::rngs::SmallRng;
use rand::SeedableRng;

fn main() {
    type M = VecMutator<U8Mutator>;
    let mut m = M::new(SmallRng::seed_from_u64(0), U8Mutator::default());

    let mut x = vec![0, 167, 200, 103, 56, 78, 2, 127];
    let mut x_cache = m.cache_from_value(&x);
    let mut x_step = m.mutation_step_from_value(&x);

    let mut results: Vec<Vec<u8>> = vec![];
    for _ in 0..100 {
        let prev_x = x.clone();
        // println!("{:?}", x);
        let token = m.mutate(&mut x, &mut x_cache, &mut x_step, 1000.0);
        results.push(x.clone());
        // println!("{:?}", x);
        m.unmutate(&mut x, &mut x_cache, token);
        // println!("{:?}", x);
        assert!(x == prev_x);
    }
    for x in results.iter() {
        println!("{:?}", x);
    }

    // results.clear();

    // for i in 0..20 {
    //     results.push(m.arbitrary(i, 100.0).0);
    // }

    // println!("{:?}", results);
}
