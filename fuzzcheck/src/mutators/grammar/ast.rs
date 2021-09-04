extern crate self as fuzzcheck;

#[cfg(feature = "serde_json_serializer")]
use serde::{Serialize, Deserialize};

use crate::Serializer;

/// An abstract syntax tree.
#[cfg_attr(feature = "serde_json_serializer", derive(Serialize, Deserialize))]
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AST {
    Token(char),
    Sequence(Vec<AST>),
    Box(Box<AST>),
}

#[cfg(feature = "serde_json_serializer")]
#[derive(Serialize, Deserialize)]
struct SerializedAST {
    ast: AST,
    string: String,
}

#[cfg(feature = "serde_json_serializer")]
pub struct ASTSerializer;
#[cfg(feature = "serde_json_serializer")]
impl Serializer for ASTSerializer {
    type Value = AST;

    #[no_coverage]
    fn is_utf8(&self) -> bool {
        true
    }

    #[no_coverage]
    fn extension(&self) -> &str {
        ".json"
    }

    #[no_coverage]
    fn from_data(&self, data: &[u8]) -> Option<Self::Value> {
        serde_json::from_slice::<SerializedAST>(data).ok().map(|x| x.ast)
    }

    #[no_coverage]
    fn to_data(&self, value: &Self::Value) -> Vec<u8> {
        let (string, _) = value.generate_string();
        let ser_ast = SerializedAST {
            ast: value.clone(),
            string
        };
        serde_json::to_vec_pretty(&ser_ast).unwrap()
    }
}

/// Like an abstract syntax tree, but augmented with the string indices that correspond to each node
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ASTMap {
    pub start_index: usize,
    pub len: usize,
    pub content: ASTMappingKind,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ASTMappingKind {
    Token,
    Sequence(Vec<ASTMap>),
    Box(Box<ASTMap>),
}

impl AST {
    #[no_coverage]
    pub fn generate_string_in(&self, s: &mut String, start_index: &mut usize) -> ASTMap {
        match self {
            AST::Token(c) => {
                let len = c.len_utf8();
                let orig_start_index = *start_index;
                s.push(*c);
                *start_index += len;
                ASTMap {
                    start_index: orig_start_index,
                    len,
                    content: ASTMappingKind::Token,
                }
            }
            AST::Sequence(asts) => {
                let original_start_idx = *start_index;
                let mut cs = vec![];
                for ast in asts {
                    let c = ast.generate_string_in(s, start_index);
                    cs.push(c);
                }
                ASTMap {
                    start_index: original_start_idx,
                    len: *start_index - original_start_idx,
                    content: ASTMappingKind::Sequence(cs),
                }
            }
            AST::Box(ast) => {
                let mapping = ast.generate_string_in(s, start_index);
                ASTMap {
                    start_index: mapping.start_index,
                    len: mapping.len,
                    content: ASTMappingKind::Box(Box::new(mapping)),
                }
            }
        }
    }
    #[no_coverage]
    pub fn generate_string(&self) -> (String, ASTMap) {
        let mut s = String::new();
        let mut start_index = 0;
        let c = self.generate_string_in(&mut s, &mut start_index);
        (s, c)
    }
    #[no_coverage]
    pub fn generate_string_starting_at_idx(&self, idx: usize) -> (String, ASTMap) {
        let mut s = String::new();
        let mut start_index = idx;
        let c = self.generate_string_in(&mut s, &mut start_index);
        (s, c)
    }
}

impl From<&AST> for ASTMap {
    #[no_coverage]
    fn from(ast: &AST) -> Self {
        ast.generate_string().1
    }
}
impl From<AST> for String {
    #[no_coverage]
    fn from(ast: AST) -> Self {
        ast.generate_string().0
    }
}
