use std::marker::PhantomData;

/// A serializer that uses `serde` and `serde_json` to serialize the test
/// inputs (of arbitrary type `T: Serializable + for<'e> Deserializable<'e>`)
/// to a json file.
#[doc(cfg(feature = "serde_json_serializer"))]
pub struct SerdeSerializer<S> {
    phantom: PhantomData<S>,
}

impl<S> Default for SerdeSerializer<S> {
    #[coverage(off)]
    fn default() -> Self {
        Self { phantom: PhantomData }
    }
}

impl<S> crate::traits::Serializer for SerdeSerializer<S>
where
    S: serde::Serialize + for<'e> serde::Deserialize<'e>,
{
    type Value = S;

    #[coverage(off)]
    fn extension(&self) -> &str {
        "json"
    }
    #[coverage(off)]
    fn from_data(&self, data: &[u8]) -> Option<S> {
        serde_json::from_slice(data).ok()
    }
    #[coverage(off)]
    fn to_data(&self, value: &Self::Value) -> Vec<u8> {
        serde_json::to_vec(value).unwrap()
    }
}
