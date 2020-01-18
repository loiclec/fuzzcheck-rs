// #![feature(test)]

// extern crate fuzzcheck_input;
// extern crate test;
// use fuzzcheck::mutator::InputGenerator;
// use fuzzcheck_mutatorsinteger::*;

// #[cfg(test)]
// mod tests {
//     use super::*;
//     use test::Bencher;

//     #[bench]
//     fn bench_mutate_integer(b: &mut Bencher) {
//         let mut generator = U8Generator::default();
//         let mut i = 0;
//         b.iter(|| {
//             let mut x = 70;
//             let mut cplx = 0.0;
//             generator.mutate(&mut x, i, &mut cplx, 0.0);
//             i += 1;
//         });
//     }
// }
