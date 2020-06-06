# fuzzcheck_mutators

This crate contains implementations of the `Mutator` trait of [fuzzcheck].

Very few types are supported yet. `()`, `bool`, `u8`, `Vec`, and `Option` are
supported, and I have worked a bit on generic mutators that could work with 
structs and enums, but there is still a lot of work to be done.

So you can fuzz-test types such as `Vec<(Option<u8>, bool)>` but not `&str`,
`HashSet`, `i32`, etc.

[fuzzcheck]: https://crates.io/crates/fuzzcheck
