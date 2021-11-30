extern crate fuzzcheck;
use fuzzcheck::{mutators::integer::U8Mutator, mutators::vector::VecMutator, DefaultMutator, Mutator};

use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn criterion_benchmark(c: &mut Criterion) {
    c.bench_function(
        "Vec<Vec<u8>> ordered_mutate and unmutate for short typical vector",
        |b| {
            let m = VecMutator::new(VecMutator::new(U8Mutator::default(), 0..=usize::MAX), 0..=usize::MAX);
            // let m = VecMutator::new(U8Mutator::default(), 0..=100);
            let mut vector = vec![
                vec![1, 2],
                vec![],
                vec![8, 78, 32],
                vec![1, 1, 1, 1, 1, 1],
                vec![100, 200],
                vec![],
                vec![],
            ];
            let mut cache = m.validate_value(&vector).unwrap();
            let mut step = m.default_mutation_step(&vector, &cache);
            b.iter(move || {
                let (t, _cplx) = m.ordered_mutate(&mut vector, &mut cache, &mut step, 1000.0).unwrap();
                m.unmutate(&mut vector, &mut cache, t);
            })
        },
    );
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
