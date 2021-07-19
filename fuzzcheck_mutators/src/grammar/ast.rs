extern crate self as fuzzcheck_mutators;

/// An abstract syntax tree.
#[derive(Clone, Debug)]
pub enum AST {
    Token(char),
    Sequence(Vec<AST>),
    Box(Box<AST>),
}

impl AST {
    pub fn generate_string_in(&self, s: &mut String, start_index: &mut usize) -> ASTMapping {
        match self {
            AST::Token(c) => {
                let len = c.len_utf8();
                let orig_start_index = *start_index;
                s.push(*c);
                *start_index += len;
                ASTMapping {
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
                ASTMapping {
                    start_index: original_start_idx,
                    len: *start_index - original_start_idx,
                    content: ASTMappingKind::Sequence(cs),
                }
            }
            AST::Box(ast) => {
                let mapping = ast.generate_string_in(s, start_index);
                ASTMapping {
                    start_index: mapping.start_index,
                    len: mapping.len,
                    content: ASTMappingKind::Box(Box::new(mapping)),
                }
            }
        }
    }
    pub fn generate_string(&self) -> (String, ASTMapping) {
        let mut s = String::new();
        let mut start_index = 0;
        let c = self.generate_string_in(&mut s, &mut start_index);
        (s, c)
    }
    pub fn generate_string_starting_at_idx(&self, idx: usize) -> (String, ASTMapping) {
        let mut s = String::new();
        let mut start_index = idx;
        let c = self.generate_string_in(&mut s, &mut start_index);
        (s, c)
    }
}

/// Like an abstract syntax tree, but augmented with the string indices that correspond to each node
#[derive(Debug)]
pub struct ASTMapping {
    pub start_index: usize,
    pub len: usize,
    pub content: ASTMappingKind,
}
#[derive(Debug)]
pub enum ASTMappingKind {
    Token,
    Sequence(Vec<ASTMapping>),
    Box(Box<ASTMapping>),
}
