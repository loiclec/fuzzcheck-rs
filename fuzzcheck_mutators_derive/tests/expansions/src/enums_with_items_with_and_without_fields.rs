use fuzzcheck_mutators::fuzzcheck_derive_mutator;

#[fuzzcheck_derive_mutator(DefaultMutator)]
#[derive(Clone)]
pub enum X {
    A(u8),
    B,
}

// #[fuzzcheck_derive_mutator]
// #[derive(Clone)]
// pub enum Y {
//     V { v: Vec<Option<bool>>, w: (), x: ::std::collections::HashMap<u8, bool>, y: u8, z: bool },
//     W(bool, bool, bool, bool),
//     X(bool),
//     Y { y: Option<u8> },
//     Z (),
// }

// #[fuzzcheck_derive_mutator(DefaultMutator)]
// #[derive(Clone)]
// pub enum X {
//     A(u8),
//     B(u16),
//     C,
//     D(bool)
// }
