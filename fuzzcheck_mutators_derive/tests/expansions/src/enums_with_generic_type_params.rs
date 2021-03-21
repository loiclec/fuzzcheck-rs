use fuzzcheck_mutators::DefaultMutator;

#[derive(Clone, DefaultMutator)]
pub enum X<T> {
    A(T),
    B(Vec<T>),
}

// #[derive(Clone, DefaultMutator)]
// pub enum Y<T, U, V, W> {
//     W,
//     X(W),
//     Y { t: Option<T>, u: U },
//     Z { v: (V, u8) },
// }

#[cfg(test)]
mod test {
    use super::*;
    use fuzzcheck_mutators::fuzzcheck_traits::Mutator;
    #[test]
    fn test_compile() {
        let m = X::<Vec<u8>>::default_mutator();
        let (value, _cache): (X<Vec<u8>>, _) = m.random_arbitrary(10.0);

        match value {
            X::A(_x) => {}
            X::B(_y) => {}
        }

        // let m = Y::<u8, bool, (), (u8, X<bool>)>::default_mutator();
        // let (value, _cache): (Y<u8, bool, (), (u8, X<bool>)>, _) = m.random_arbitrary(10.0);
        // match value {
        //     Y::W => {}
        //     Y::X(_) => {}
        //     Y::Y { t: _, u: _ } => {}
        //     Y::Z { v: _ } => {}
        // }
    }
}
