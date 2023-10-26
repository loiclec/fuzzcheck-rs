//! Types implementing the [Serializer] trait.
//!
//! There are currently three implementations:
//!
//! * SerdeSerializer uses the `serde` and `serde_json` crate to serialize
//! the test inputs (of arbitrary Serializable type) to a `.json` file.
//!
//! * [ByteSerializer] encodes and decodes values of type `Vec<u8>` by simply
//! copy/pasting the bytes from/to the files. The extension is customizable.
//!
//! * [StringSerializer] encodes and decodes values of any type implementing
//! `FromStr` and `ToString` into utf-8 encoded text files.

#[cfg(feature = "serde_ron_serializer")]
mod serde_ron_serializer;
#[cfg(feature = "serde_json_serializer")]
mod serde_serializer;

use std::marker::PhantomData;
use std::str::FromStr;

#[cfg(feature = "serde_ron_serializer")]
pub use serde_ron_serializer::SerdeRonSerializer;
#[cfg(feature = "serde_json_serializer")]
pub use serde_serializer::SerdeSerializer;

use crate::Serializer;

/**
A Serializer for `Vec<u8>` that simply copies the bytes from/to the files.
*/
pub struct ByteSerializer {
    ext: &'static str,
}

impl ByteSerializer {
    /// Create a byte serializer. The only argument is the name of the extension
    /// that the created files should have. For example:
    /// ```
    /// use fuzzcheck::ByteSerializer;
    ///
    /// let ser = ByteSerializer::new("png");
    /// ````
    #[coverage(off)]
    pub fn new(ext: &'static str) -> Self {
        Self { ext }
    }
}

impl crate::traits::Serializer for ByteSerializer {
    type Value = Vec<u8>;

    #[coverage(off)]
    fn extension(&self) -> &str {
        self.ext
    }
    #[coverage(off)]
    fn from_data(&self, data: &[u8]) -> Option<Self::Value> {
        Some(data.into())
    }
    #[coverage(off)]
    fn to_data(&self, value: &Self::Value) -> Vec<u8> {
        value.clone()
    }
}

/**
A serializer that encodes and decodes values of any type implementing
`FromStr` and `ToString` into utf-8 encoded text files.
 */
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
    /// Create a string serializer. The only argument is the name of the extension
    /// that the created files should have. For example:
    /// ```
    /// use fuzzcheck::StringSerializer;
    ///
    /// let ser = StringSerializer::<String>::new("txt");
    /// ````
    #[coverage(off)]
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

    #[coverage(off)]
    fn extension(&self) -> &str {
        self.extension
    }
    #[coverage(off)]
    fn from_data(&self, data: &[u8]) -> Option<Self::Value> {
        let string = String::from_utf8(data.to_vec()).ok()?;
        let value = Self::Value::from_str(&string).ok()?;
        Some(value)
    }
    #[coverage(off)]
    fn to_data(&self, value: &Self::Value) -> Vec<u8> {
        value.to_string().into_bytes()
    }
}
