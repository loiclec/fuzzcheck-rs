//! This crate contains types implementing the Serializer trait of fuzzcheck.
//! There are currently two implementations:
//!
//! * SerdeSerializer uses the `serde` and `serde_json` crate to serialize
//! the test inputs (of arbitrary Serializable type) to a `.json` file.
//! It is available under the “serde” feature
//!
//! * [ByteSerializer] encodes and decodes values of type `Vec<u8>` by simply
//! copy/pasting the bytes from/to the files. The extension is customizable.
//!

mod serde_serializer;
use std::{marker::PhantomData, str::FromStr};

pub use serde_serializer::SerdeSerializer;

use crate::Serializer;

/**
A Serializer for Vec<u8> that simply copies the bytes from/to the files.

```ignore
#[no_coverage] fn parse(data: &[u8]) -> bool {
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
    #[no_coverage]
    pub fn new(ext: &'static str) -> Self {
        Self { ext }
    }
}

impl crate::traits::Serializer for ByteSerializer {
    type Value = Vec<u8>;
    #[no_coverage]
    fn is_utf8(&self) -> bool {
        false
    }
    #[no_coverage]
    fn extension(&self) -> &str {
        self.ext
    }
    #[no_coverage]
    fn from_data(&self, data: &[u8]) -> Option<Self::Value> {
        Some(data.into())
    }
    #[no_coverage]
    fn to_data(&self, value: &Self::Value) -> Vec<u8> {
        value.clone()
    }
}

pub struct StringSerializer<StringType>
where
    StringType: ToString + FromStr,
{
    pub extension: &'static str,
    _phantom: PhantomData<StringType>,
}
impl<StringType> StringSerializer<StringType>
where
    StringType: ToString + FromStr,
{
    pub fn new(extension: &'static str) -> Self {
        Self {
            extension,
            _phantom: PhantomData,
        }
    }
}
impl<StringType> Serializer for StringSerializer<StringType>
where
    StringType: ToString + FromStr,
{
    type Value = StringType;
    fn is_utf8(&self) -> bool {
        true
    }

    fn extension(&self) -> &str {
        self.extension
    }

    fn from_data(&self, data: &[u8]) -> Option<Self::Value> {
        let string = String::from_utf8(data.to_vec()).ok()?;
        let value = Self::Value::from_str(&string).ok()?;
        Some(value)
    }

    fn to_data(&self, value: &Self::Value) -> Vec<u8> {
        value.to_string().into_bytes()
    }
}
