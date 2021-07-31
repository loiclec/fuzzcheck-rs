extern crate serde;
extern crate serde_json;

use std::marker::PhantomData;

#[cfg(feature = "fuzzcheck_traits_through_fuzzcheck")]
use fuzzcheck::fuzzcheck_traits;

/// `SerdeSerializer<T>` uses `serde` and `serde_json` to serialize the test
/// inputs (of arbitrary type `T: Serializable+ for<'e> Deserializable<'e>`)
/// to a json file.
pub struct SerdeSerializer<S> {
    phantom: PhantomData<S>,
}

impl<S> Default for SerdeSerializer<S> {
    #[no_coverage]
    fn default() -> Self {
        Self { phantom: PhantomData }
    }
}

impl<S> fuzzcheck_traits::Serializer for SerdeSerializer<S>
where
    S: serde::Serialize + for<'e> serde::Deserialize<'e>,
{
    type Value = S;
    #[no_coverage]
    fn is_utf8(&self) -> bool {
        true
    }
    #[no_coverage]
    fn extension(&self) -> &str {
        "json"
    }
    #[no_coverage]
    fn from_data(&self, data: &[u8]) -> Option<S> {
        serde_json::from_slice(data).ok()
    }
    #[no_coverage]
    fn to_data(&self, value: &Self::Value) -> Vec<u8> {
        serde_json::to_vec(value).unwrap()
    }
}
