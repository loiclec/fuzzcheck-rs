use std::any::TypeId;

use fuzzcheck::mutators::integer::U8Mutator;
use fuzzcheck::mutators::option::OptionMutator;
use fuzzcheck::subvalue_provider::{CrossoverSubValueProvider, Generation, SubValueProviderId};
use fuzzcheck::{DefaultMutator, Mutator, SubValueProvider};

#[test]
fn test_crossover_option() {
    let crossover_value = vec![None, Some(83638), Some(1), Some(2), Some(3), Some(19373246372)];
    let crossover_mutator = <Vec<Option<usize>>>::default_mutator();
    let crossover_cache = crossover_mutator.validate_value(&crossover_value).unwrap();

    let subvalue_provider = CrossoverSubValueProvider::new(
        SubValueProviderId {
            idx: 0,
            generation: Generation(0),
        },
        &crossover_value,
        &crossover_cache,
        &crossover_mutator,
    );

    let mut index = 0;
    for _ in 0..100 {
        let x = subvalue_provider
            .get_subvalue(TypeId::of::<usize>(), 100.0, &mut index)
            .and_then(|(x, cplx)| x.downcast_ref::<usize>().map(|x| (x, cplx)));
        if let Some((x, cplx)) = x {
            println!("{x}  {cplx:.2}");
        } else {
            break;
        }
    }
    let mutator = <Option<usize>>::default_mutator();
    let mut value = Option::Some(0);
    let mut cache = mutator.validate_value(&value).unwrap();
    let mut step = mutator.default_mutation_step(&value, &cache);

    let mut found = false;
    for i in 0..100000 {
        let (token, _) = mutator
            .ordered_mutate(&mut value, &mut cache, &mut step, &subvalue_provider, 10000.)
            .unwrap();
        if value == Some(19373246372) {
            found = true;
            println!("found after {i} iterations");
            break;
        }
        mutator.unmutate(&mut value, &mut cache, token);
    }
    assert!(found);
}

#[test]
fn test_option() {
    let m = OptionMutator::new(U8Mutator::default());
    fuzzcheck::mutators::testing_utilities::test_mutator(m, 100.0, 100.0, false, true, 500, 500);
}
