// TODO: explain why it is a macro
// TODO: pass the path to serde to macro?
// e.g. define_serde_serializer!(other_crate::serde)

/// Defines a struct called `SerdeSerializer<T>` that implements the
/// `Serializer` trait using serde.
///
/// ## Example
///
/// ```ignore
/// define_serde_serializer!();
///
/// let serializer = Serializer::<T>::default();
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

        impl<S> fuzzcheck::Serializer for SerdeSerializer<S>
        where
            S: Serialize + for<'e> Deserialize<'e>,
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

extern crate fuzzcheck;

pub struct ByteSerializer {
    ext: &'static str,
}

impl ByteSerializer {
    pub fn new(ext: &'static str) -> Self {
        Self { ext }
    }
}

impl fuzzcheck::Serializer for ByteSerializer {
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
