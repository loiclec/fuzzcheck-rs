use fuzzcheck_mutators_derive::make_mutator;
extern crate self as fuzzcheck;

make_mutator! {
    name: OptionMutator,
    default: true,
    type: pub enum Option<T> {
        Some(T),
        None,
    }
}

// // #[cfg(test)]
// // mod tests {
// //     use super::*;
// //     use crate::{DefaultMutator, Mutator};
// //     extern crate self as fuzzcheck;
// //     #[derive(Debug, Clone, DefaultMutator)]
// //     enum Op {
// //         Insert([u8; 4], u8),
// //         Remove([u8; 4]),
// //         Get([u8; 4]),
// //         Range(bool, bool),
// //     }
// //     #[test]
// //     fn test_all_paths() {
// //         let m = <Op>::default_mutator();
// //         let x = Op::Insert([0, 1, 2, 3], 1);
// //         let c = m.validate_value(&x).unwrap();
// //         m.all_paths(&x, &c);
// //     }
// // }
