extern crate fuzzcheck_input;
use fuzzcheck::input::FuzzedInput;
use fuzzcheck_input::integer::*;
use fuzzcheck_input::option::*;

type F = FuzzedOption<FuzzedU8>;

fn main() {
    let mut x = Some(10);
    let mut x_state = F::state_from_value(&x);

    let mut results: Vec<Option<u8>> = vec![];
    for _ in 0..30 {
        let token = F::mutate(&mut x, &mut x_state, 8.0);
        results.push(x);
        F::unmutate(&mut x, &mut x_state, token);
    }
    println!("{:?}", results);

    results.clear();

    for i in 0..30 {
        results.push(F::arbitrary(i, 1.0));
    }

    println!("{:?}", results);
}
