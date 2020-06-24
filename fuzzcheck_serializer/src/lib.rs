//! This crate contains types implementing the Serializer trait of fuzzcheck.
//! There are currently two implementations:
//!
//! * SerdeSerializer uses the `serde` and `serde_json` crate to serialize
//! the test inputs (of arbitrary Serializable type) to a `.json` file.
//! However, SerdeSerializer is not defined in this crate. Instead, it can
//! be made available through the [define_serde_serializer] macro.
//!
//! * [ByteSerializer] encodes and decodes values of type `Vec<u8>` by simply
//! copy/pasting the bytes from/to the files. The extension is customizable.
//!

extern crate fuzzcheck_traits;

/// Defines a struct called `SerdeSerializer<T>` that implements the
/// `fuzzcheck::Serializer` trait using serde.
///
/// `SerdeSerializer<T>` uses `serde` and `serde_json` to serialize the test
/// inputs (of arbitrary type `T: Serializable+ for<'e> Deserializable<'e>`)
/// to a json file.
///
/// This macro only works if `serde`, `serde_json`, and `fuzzcheck` are
/// accessible.
///
/// ## Example
///
/// ```ignore
/// extern crate serde;
/// extern crate serde_json;
/// extern crate fuzzcheck;
///
/// define_serde_serializer!();
///
/// let serializer = SerdeSerializer::<T>::default();
/// ```
#[macro_export]
macro_rules! define_serde_serializer {
    () => {
        pub struct SerdeSerializer<S> {
            phantom: std::marker::PhantomData<S>,
        }

        impl<S> Default for SerdeSerializer<S> {
            fn default() -> Self {
                Self {
                    phantom: std::marker::PhantomData,
                }
            }
        }

        impl<S> fuzzcheck_traits::Serializer for SerdeSerializer<S>
        where
            S: serde::Serialize + for<'e> serde::Deserialize<'e>,
        {
            type Value = S;
            fn extension(&self) -> &str {
                "json"
            }
            fn from_data(&self, data: &[u8]) -> Option<S> {
                serde_json::from_slice(data).ok()
            }
            fn to_data(&self, value: &Self::Value) -> Vec<u8> {
                serde_json::to_vec(value).unwrap()
            }
        }
    };
}

/**
A Serializer for Vec<u8> that simply copies the bytes from/to the files.

```ignore
fn parse(data: &[u8]) -> bool {
    // ...
}
let mutator = VecMutator<U8Mutator>::default();

// pass the desired extension of the file as argument
let serializer = ByteSerializer::new("png");

fuzzcheck::launch(parse, mutator, serializer)
```
*/
pub struct ByteSerializer {
    ext: &'static str,
}

impl ByteSerializer {
    pub fn new(ext: &'static str) -> Self {
        Self { ext }
    }
}

impl fuzzcheck_traits::Serializer for ByteSerializer {
    type Value = Vec<u8>;
    fn extension(&self) -> &str {
        self.ext
    }
    fn from_data(&self, data: &[u8]) -> Option<Self::Value> {
        Some(data.into())
    }
    fn to_data(&self, value: &Self::Value) -> Vec<u8> {
        value.clone()
    }
}
