extern crate fuzzcheck_input;
use fuzzcheck::input::FuzzedInput;
use fuzzcheck_input::integer::*;
use fuzzcheck_input::vector::*;

type F = FuzzedVector<FuzzedU8>;

fn main() {
    let target = vec![0, 167, 200, 103, 56, 78, 2, 254];

    let mut x = vec![0, 167, 200, 103, 56, 78, 2, 127];
    let mut x_state = F::state_from_value(&x);

    //let mut results: Vec<Vec<u8>> = vec![];
    for i in 0..30_000_000 {
        let token = F::mutate(&mut x, &mut x_state, 100.0);
        //results.push(x.clone());
        if x == target {
            println!("success at {}", i);
        }
        F::unmutate(&mut x, &mut x_state, token);
    }
    //println!("{:?}", results);

    // results.clear();

    // for i in 1..2 {
    //     results.push(F::arbitrary(40, 100.0));
    // }

    // println!("{:?}", results);
}
