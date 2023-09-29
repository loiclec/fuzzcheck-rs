extern crate self as fuzzcheck;

#[cfg(feature = "serde_json_serializer")]
use serde::{Deserialize, Serialize};

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
#[cfg_attr(feature = "serde_json_serializer", derive(Serialize, Deserialize))]
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum AST {
    #[doc(hidden)]
    Token(char),
    #[doc(hidden)]
    Sequence(Vec<AST>),
}

impl AST {
    #[coverage(off)]
    pub fn generate_string_in(&self, string: &mut String) {
        match self {
            AST::Token(c) => {
                string.push(*c);
            }
            AST::Sequence(asts) => {
                for ast in asts {
                    ast.generate_string_in(string);
                }
            }
        }
    }

    /// Converts the AST to its `String` representation
    #[allow(clippy::inherent_to_string)]
    #[coverage(off)]
    pub fn to_string(&self) -> String {
        let mut s = String::with_capacity(64);
        self.generate_string_in(&mut s);
        s
    }
}
