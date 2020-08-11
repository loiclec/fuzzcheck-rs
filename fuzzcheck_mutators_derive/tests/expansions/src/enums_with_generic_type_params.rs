use fuzzcheck_mutators::fuzzcheck_derive_mutator;

#[fuzzcheck_derive_mutator(DefaultMutator)]
#[derive(Clone)]
pub enum X<T> {
    A(T),
    B(Vec<T>)
}

// #[fuzzcheck_derive_mutator(DefaultMutator)]
// #[derive(Clone)]
// pub enum Y<T,U,V,W> {
//     W,
//     X(W),
//     Y { t: Option<T>, u: U },
//     Z { v: (V, u8) }
// }

