# Serializer

We also need to know how to save the generated test cases, 
of type `Vec<Operation<T>>`, to the file system.

The easiest way to do that is to use `serde` and its derive macros
`Serialize` and `Deserialize`.

```rust ignore
use serde::{Serialize, Deserialize};

#[derive(Clone, DefaultMutator, Serialize, Deserialize)]
enum Operation<T: Ord> {
    Insert(T),
    Contains(T),
}
```
Then, we will provide a `fuzzcheck::SerdeSerializer` to our fuzz test so that
each interesting `Vec<Operation<T>>` is saved as a JSON file. 

There are other ways to serialize test cases. For example, for values of type
`Vec<u8>`, you can use a `fuzzcheck::ByteSerializer`, which will simply copy
the content of the vector to the file system. For values of type `String`, one
can use a `fuzzcheck::StringSerializer`.
