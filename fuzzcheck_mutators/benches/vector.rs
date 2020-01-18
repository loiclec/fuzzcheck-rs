#![feature(test)]

extern crate fuzzcheck_input;
extern crate test;
use fuzzcheck::input::Mutator;
use fuzzcheck_input::integer::*;
use fuzzcheck_input::vector::*;

type F = VecMutator<VecMutator<U8Mutator>>;

#[cfg(test)]
mod tests {
    use super::*;
    use test::Bencher;

    // this needs to be a better bench
    #[bench]
    fn bench_mutate_vector(b: &mut Bencher) {
        let m = F::default();

        b.iter(|| {
            let mut x = vec![
                vec![2, 89, 1, 0, 4, 2],
                vec![3],
                vec![4, 9, 0, 0, 0, 2, 5],
                vec![5],
                vec![2, 89, 1, 0, 4, 2],
                vec![3],
                vec![4, 9, 0, 0, 0, 2, 5],
                vec![5],
                vec![2, 89, 1, 0, 4, 2],
                vec![3],
                vec![4, 9, 0, 0, 0, 2, 5],
                vec![5],
                vec![2, 89, 1, 0, 4, 2],
                vec![3],
                vec![4, 9, 0, 0, 0, 2, 5],
                vec![5],
            ];
            let mut x_cache = m.cache_from_value(&x);
            let mut x_step = m.mutation_step_from_value(&x);
            let cplx = m.complexity(&x, &x_cache);
            assert!(cplx < 800.0);

            for i in 0..100 {
                // let _ = x.clone();
                let token = m.mutate(&mut x, &mut x_cache, &mut x_step, 1600.0);
                m.unmutate(&mut x, &mut x_cache, token);
                // let (x, y) = m.arbitrary(i, 500.0);
            }
            //i += 1;
            // let _ = x.clone();
            //let (x, y) = m.arbitrary(i, 100.0);
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
