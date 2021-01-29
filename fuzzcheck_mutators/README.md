# fuzzcheck_mutators

This crate contains implementations of the `Mutator` trait of [fuzzcheck].

Very few types are supported yet. `()`, `bool`, `u8`-`u64`, `i8-i64` `Vec`, 
and `Option` are supported, as well as arbitrary structs and enums through the
procedural macros provided by `fuzzcheck_mutators_derive`. But other types of
the standard library such as `String`, `HashMap/Set`, etc. do not have default 
mutators yet.

So you can fuzz-test types such as `Vec<(Option<u8>, bool)>` but not `&str`,
`HashSet`, etc.

[fuzzcheck]: https://crates.io/crates/fuzzcheck
