// extern crate fuzzcheck_input;
// use fuzzcheck::input::FuzzedInput;
// use fuzzcheck_input::integer::*;

// type F = FuzzedU8;

fn main() {
    // let mut results: Vec<u8> = vec![];
    // for i in 0 .. 256 {
    //     let y = arbitrary_u8(0, 255, i);
    //     results.push(y);
    // }
    // results.sort();
    // println!("{:?}", results);

    // let mut x = 127;//F::default();
    // let mut x_state = F::state_from_value(&x);

    // let low =  x.wrapping_sub(std::u8::MAX / 2);
    // let high = x.wrapping_add(std::u8::MAX / 2 + 1);
    // println!("{} {}", low, high);

    // let mut results: Vec<u8> = vec![x];
    // for _ in 0..300 {
    //     let token = F::mutate(&mut x, &mut x_state, 8.0);
    //     results.push(x);
    //     F::unmutate(&mut x, &mut x_state, token);
    // }
    // results.sort();
    // println!("{:?}", results);

    // results.clear();

    // for i in 0..30 {
    //     results.push(F::arbitrary(i, 1.0));
    // }

    // println!("{:?}", results);
}
