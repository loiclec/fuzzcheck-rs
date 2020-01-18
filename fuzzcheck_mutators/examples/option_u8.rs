extern crate fuzzcheck_mutators;
use fuzzcheck::Mutator;
use fuzzcheck_mutators::integer::*;
use fuzzcheck_mutators::option::*;

type F = OptionMutator<U8Mutator>;

fn main() {
    let m = F::default();
    let mut x = Some(10);
    let mut x_cache = m.cache_from_value(&x);
    let mut x_step = m.mutation_step_from_value(&x);

    let mut results: Vec<Option<u8>> = vec![];
    for _ in 0..30 {
        let token = m.mutate(&mut x, &mut x_cache, &mut x_step, 8.0);
        results.push(x);
        m.unmutate(&mut x, &mut x_cache, token);
    }
    println!("{:?}", results);

    results.clear();

    for i in 0..30 {
        results.push(m.arbitrary(i, 1.0).0);
    }

    println!("{:?}", results);
}
