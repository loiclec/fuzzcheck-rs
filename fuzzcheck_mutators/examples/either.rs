use fuzzcheck::Mutator;

extern crate fuzzcheck_mutators;
use fuzzcheck_mutators::either::*;
use fuzzcheck_mutators::integer::*;

#[derive(Clone, Debug)]
enum E {
    A(u8),
    B(u8),
}
impl EitherMap for E {
    type A = u8;
    type B = u8;
    type V = Self;

    fn left(a: u8) -> Self {
        E::A(a)
    }
    fn right(b: u8) -> Self {
        E::B(b)
    }

    fn get_either(v: &Self) -> Either<&u8, &u8> {
        match v {
            E::A(a) => Either::Left(a),
            E::B(b) => Either::Right(b),
        }
    }
    fn get_either_mut(v: &mut Self) -> Either<&mut u8, &mut u8> {
        match v {
            E::A(a) => Either::Left(a),
            E::B(b) => Either::Right(b),
        }
    }
}

type F = EitherMutator<E, U8Mutator, U8Mutator>;

fn main() {
    let mut m = F::default();
    let mut x = E::A(10);
    let mut x_cache = m.cache_from_value(&x);
    let mut x_step = m.mutation_step_from_value(&x);

    let mut results: Vec<E> = vec![];
    for _ in 0..130 {
        let token = m.mutate(&mut x, &mut x_cache, &mut x_step, 8.0);
        results.push(x.clone());
        m.unmutate(&mut x, &mut x_cache, token);
    }
    println!("{:?}", results);

    results.clear();

    for i in 0..30 {
        results.push(m.arbitrary(i, 1.0).0);
    }

    println!("{:?}", results);
}
