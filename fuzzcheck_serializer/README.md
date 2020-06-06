# fuzzcheck_serializer

This crate provides implementations of the `Serializer` trait defined by
[fuzzcheck].

There are currently two choices:

1. `ByteSerializer` serializes a `Vec<u8>` by directly writing the bits to 
a file. You can choose the file extension.
2. `SerdeSerializer` uses `serde` and `serde_json` to serialize any
serde-`Serializable` type to a json file.

A catch is that `SerdeSerializer` is not directly defined in this crate. Instead,
you must use the `define_serde_serializer!()` macro in the fuzz-target script to
define it. Like this:

```rust
#[macro_use]
extern crate fuzzcheck_serializer;

extern crate serde;
extern crate serde_json; // serde_json MUST be visible
extern crate fuzzcheck; // fuzzcheck MUST be visible

use serde::{Serialize, Deserialize}; // Serializable and Deserializable MUST be visible

// Makes the SerdeSerializer available to fuzzcheck
define_serde_serializer!();

fn main() {
    // Will mutate values of type Vec<u8>
    let mutator = VecMutator<U8Mutator>::default();

    // Test inputs will be encoded with serde_json 
    let serializer = SerdeSerializer::<Vec<u8>>::default();

    // Launch the fuzzing process on the test function
    let _ = fuzzcheck::launch(test, mutator, serializer);
}
```

[fuzzcheck]: https://crates.io/crates/fuzzcheck