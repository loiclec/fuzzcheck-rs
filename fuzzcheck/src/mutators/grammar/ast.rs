extern crate self as fuzzcheck;

#[cfg(feature = "serde_json_serializer")]
use serde::{ser::SerializeTuple, Deserialize, Serialize};

/// An abstract syntax tree.
///
#[cfg_attr(
    feature = "serde_json_serializer",
    doc = "It can be serialized with [`SerdeSerializer`](crate::SerdeSerializer) on crate feature `serde_json_serializer`"
)]
#[cfg_attr(
    not(feature = "serde_json_serializer"),
    doc = "It can be serialized with `SerdeSerializer` on crate feature `serde_json_serializer`"
)]
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum AST {
    #[doc(hidden)]
    Token(char),
    #[doc(hidden)]
    Sequence(Vec<AST>),
    #[doc(hidden)]
    Box(Box<AST>),
}

impl AST {
    #[no_coverage]
    fn generate_string_in(&self, string: &mut String) {
        match self {
            AST::Token(c) => {
                string.push(*c);
            }
            AST::Sequence(asts) => {
                for ast in asts {
                    ast.generate_string_in(string);
                }
            }
            AST::Box(ast) => {
                ast.generate_string_in(string);
            }
        }
    }

    /// Converts the AST to its `String` representation
    #[allow(clippy::inherent_to_string)]
    #[no_coverage]
    pub fn to_string(&self) -> String {
        let mut s = String::with_capacity(64);
        self.generate_string_in(&mut s);
        s
    }
}
/// A type that is exactly the same as AST so that I can derive most of the
/// Serialize/Deserialize implementation
#[cfg(feature = "serde_json_serializer")]
#[derive(Serialize, Deserialize)]
enum __AST {
    Token(char),
    Sequence(Vec<__AST>),
    Box(Box<__AST>),
}

#[cfg(feature = "serde_json_serializer")]
#[doc(cfg(feature = "serde_json_serializer"))]
impl Serialize for AST {
    #[no_coverage]
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let string = self.to_string();
        let mut ser = serializer.serialize_tuple(2)?;
        ser.serialize_element(&string)?;
        let ast = unsafe { std::mem::transmute::<&AST, &__AST>(self) };
        ser.serialize_element(ast)?;
        ser.end()
    }
}
#[cfg(feature = "serde_json_serializer")]
#[doc(cfg(feature = "serde_json_serializer"))]
impl<'de> Deserialize<'de> for AST {
    #[no_coverage]
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let ast = <(String, __AST)>::deserialize(deserializer).map(
            #[no_coverage]
            |x| x.1,
        )?;
        Ok(unsafe { std::mem::transmute::<__AST, AST>(ast) })
    }
}
