extern crate fuzzcheck_mutators;

extern crate fuzzcheck_mutators_derive;

use fuzzcheck_mutators::fuzzcheck_traits::Mutator;
use fuzzcheck_mutators::DefaultMutator;

#[derive(Clone, PartialEq, Eq, Debug, DefaultMutator)]
pub enum Y {
    M,
    N,
    O,
    Q,
    S,
    T,
    U,
    V,
    W,
}

#[derive(Clone, PartialEq, Eq, Debug, DefaultMutator)]
pub enum X {
    A,
    B(bool),
    C {},
    D(Y),
}

type XDefaultMutator = <X as DefaultMutator>::Mutator;
type ArbitraryStep = <XDefaultMutator as Mutator<X>>::ArbitraryStep;

fn main() {
    let complexity = 5.0;
    let mut m = X::default_mutator();
    let mut ar_step: _ = ArbitraryStep::default();

    for _ in 0..20 {
        if let Some(x) = m.ordered_arbitrary(&mut ar_step, complexity) as Option<(X, _)> {
            println!("{:?}", x.0);
        } else {
            println!("---");
        }
    }

    println!("\n===== MUTATE from A =====\n\n");

    let mut value = X::A;
    let mut cache = m.cache_from_value(&value);
    let mut step = m.initial_step_from_value(&value);
    for _ in 0..20 {
        // print!("{:?} ", value);
        if let Some(t) = m.ordered_mutate(&mut value, &mut cache, &mut step, complexity) {
            println!("{:?} ", value);
            m.unmutate(&mut value, &mut cache, t);
            // println!("{:?} ", value);
        } else {
            println!("---")
        }
    }

    println!("\n===== MUTATE from B(true) =====\n\n");

    let mut value = X::B(true);

    let mut cache = m.cache_from_value(&value);
    println!(
        "complexity: {:?}, {:.2}",
        <XDefaultMutator as Mutator<X>>::max_complexity(&m),
        m.complexity(&value, &cache)
    );
    let mut step = m.initial_step_from_value(&value);
    for _ in 0..20 {
        // print!("{:?} ", value);
        if let Some(t) = m.ordered_mutate(&mut value, &mut cache, &mut step, complexity) {
            println!("{:?} ", value);
            m.unmutate(&mut value, &mut cache, t);
            // println!("{:?} ", value);
        } else {
            println!("---")
        }
    }
}
