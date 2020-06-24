
extern crate fuzzcheck_traits;
extern crate fuzzcheck_mutators;
use fuzzcheck_mutators::integer::*;
use fuzzcheck_traits::Mutator;

// type F = I8Mutator;

fn main() {
    let nbr_tries = 1_000_000;
    let mut sum_counts: u32 = 0;

    for _ in 0 .. nbr_tries {
        // let x = fastrand::u32(..);
        let count = fastrand::u64(..);
        // let mut list = vec![];
        // loop {
            let new = U32Mutator::_arbitrary(0, u32::MAX, count);
            //println!("{}", count);
            sum_counts = sum_counts.wrapping_add(new);
            // let new = fastrand::u16(..);
            // count += 1;
            // if new == x {
            //     break
            // }
            // if let Err(idx) = list.binary_search(&new) {
            //     list.insert(idx, new);
            //     // list.sort();
            // }
            // if list.len() == 15_000 as usize {
            //     break
            // }
        // }
        // sum_counts += count;
    }
    println!("{}", (sum_counts as f64) / nbr_tries as f64);
    

    // let mut m = F::default();

    // // let mut results: Vec<<F as Mutator>::Value> = vec![];
    // // for i in 0 .. (u8::MAX as usize) + 1 {
    // //     let y =  m.arbitrary(i as usize, 0.0).0;
    // //     results.push(y);
    // // }

    // // // let x: u8 = (-1 as i8) as u8;
    // // // println!("{}", x)

    // // println!("{:?}", results);
    // // results.sort();
    // // //println!("{:?} {}", results, results.len());
    // // results.dedup();
    // // println!("{}", results.len());

    // let mut x = 12;//F::default();
    // let mut x_cache = m.cache_from_value(&x);
    // let mut step = m.mutation_step_from_value(&x);

    // // let low =  x.wrapping_sub(std::u8::MAX / 2);
    // // let high = x.wrapping_add(std::u8::MAX / 2 + 1);
    // // println!("{} {}", low, high);

    // let mut results: Vec<i8> = vec![x];
    // for _ in 0..265 {
    //     let token = m.mutate(&mut x, &mut x_cache, &mut step, 8.0);
    //     results.push(x);
    //     m.unmutate(&mut x, &mut x_cache, token);
    // }
    // //results.sort();
    // println!("{:?}", results);
    // // results.dedup();
    // println!("{:?}", results.len());
    // results.clear();

    // for i in 0..30 {
    //     results.push(F::arbitrary(i, 1.0));
    // }

    // println!("{:?}", results);
}
