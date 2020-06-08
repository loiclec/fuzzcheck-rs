extern crate fuzzcheck_mutators;
use fuzzcheck::Mutator;
use fuzzcheck_mutators::integer::*;
use fuzzcheck_mutators::tuples::*;

#[derive(Clone, Debug)]
struct S {
    a: u8,
    b: u8,
}
impl TupleMap for S {
    type A = u8;
    type B = u8;
    type V = Self;

    fn get_a(v: &Self) -> &u8 {
        &v.a
    }
    fn get_b(v: &Self) -> &u8 {
        &v.b
    }

    fn get_a_mut(v: &mut Self) -> &mut u8 {
        &mut v.a
    }
    fn get_b_mut(v: &mut Self) -> &mut u8 {
        &mut v.b
    }

    fn new(a: u8, b: u8) -> Self {
        Self { a, b }
    }
}

type F = Tuple2Mutator<S, U8Mutator, U8Mutator>;

fn main() {
    let mut m = F::default();
    let mut x = S { a: 10, b: 10 };
    let mut x_cache = m.cache_from_value(&x);
    let mut x_step = m.mutation_step_from_value(&x);

    let mut results: Vec<S> = vec![];
    for _ in 0..30 {
        let token = m.mutate(&mut x, &mut x_cache, &mut x_step, 8.0);
        results.push(x.clone());
        m.unmutate(&mut x, &mut x_cache, token);
    }
    println!("{:?}", results);

    results.clear();

    for i in 0..30 {
        let el = m.arbitrary(i, 1.0);
        results.push(el.0);
    }

    println!("{:?}", results);
}
