use std::marker::PhantomData;

/// A serializer that uses [`serde`] and [`ron`] to serialize the test
/// inputs (of arbitrary type `T: Serializable + for<'e> Deserializable<'e>`)
/// to a "rusty object notation" file.
#[doc(cfg(feature = "serde_ron_serializer"))]
pub struct SerdeRonSerializer<S> {
    phantom: PhantomData<S>,
}

impl<S> Default for SerdeRonSerializer<S> {
    #[no_coverage]
    fn default() -> Self {
        Self { phantom: PhantomData }
    }
}

impl<S> crate::traits::Serializer for SerdeRonSerializer<S>
where
    S: serde::Serialize + for<'e> serde::Deserialize<'e>,
{
    type Value = S;

    #[no_coverage]
    fn extension(&self) -> &str {
        "ron"
    }
    #[no_coverage]
    fn from_data(&self, data: &[u8]) -> Option<S> {
        let utf8_encoded = std::str::from_utf8(data).ok()?;
        ron::from_str(utf8_encoded).ok()
    }
    #[no_coverage]
    fn to_data(&self, value: &Self::Value) -> Vec<u8> {
        ron::to_string(value).unwrap().into_bytes()
    }
}
