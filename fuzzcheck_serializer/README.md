# fuzzcheck_serializer

This crate provides implementations of the `Serializer` trait defined by
[fuzzcheck].

There are currently two choices:

1. `ByteSerializer` serializes a `Vec<u8>` by directly writing the bits to 
a file. You can choose the file extension.
2. `SerdeSerializer` uses `serde` and `serde_json` to serialize any
serde-`Serializable` type to a json file. Accessible through the `serde-json`
feature.
3. `JsonSerializer` is a lightweight alternative to `SerdeSerializer` that uses
the `json` and `decent-serde-json-alternative` crates. Accessible through the
`serde-json-alternative` feature.

[fuzzcheck]: https://crates.io/crates/fuzzcheck