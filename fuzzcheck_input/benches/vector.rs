#![feature(test)]

extern crate fuzzcheck_input;
extern crate test;
use fuzzcheck::input::FuzzedInput;
use fuzzcheck_input::integer::*;
use fuzzcheck_input::vector::*;

type F = FuzzedVector<FuzzedU8>;

#[cfg(test)]
mod tests {
    use super::*;
    use test::Bencher;

    // this needs to be a better bench
    #[bench]
    fn bench_mutate_vector(b: &mut Bencher) {
        let mut x = vec![2, 3, 4, 5];
        let mut x_state = F::state_from_value(&x);
        let mut i = 0;
        b.iter(|| {
            let token = F::mutate(&mut x, &mut x_state, 10.0);
            F::unmutate(&mut x, &mut x_state, token);
            i += 1;
        });
    }

    // #[bench]
    // fn bench_new_vector(b: &mut Bencher) {
    //     let u8_generator = U8Generator::default();
    //     let mut generator = VectorGenerator::new(u8_generator);
    //     let mut i = 0;
    //     b.iter(|| {
    //         let _ = generator.new_input(i, 10.0);
    //         i += 1;
    //     });
    // }
}
