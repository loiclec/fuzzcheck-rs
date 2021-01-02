extern crate serde;
extern crate serde_json;

use std::marker::PhantomData;

/// `SerdeSerializer<T>` uses `serde` and `serde_json` to serialize the test
/// inputs (of arbitrary type `T: Serializable+ for<'e> Deserializable<'e>`)
/// to a json file.
pub struct SerdeSerializer<S> {
    phantom: PhantomData<S>,
}

impl<S> Default for SerdeSerializer<S> {
    fn default() -> Self {
        Self { phantom: PhantomData }
    }
}

impl<S> fuzzcheck_traits::Serializer for SerdeSerializer<S>
where
    S: serde::Serialize + for<'e> serde::Deserialize<'e>,
{
    type Value = S;
    fn is_utf8(&self) -> bool {
        true
    }
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
