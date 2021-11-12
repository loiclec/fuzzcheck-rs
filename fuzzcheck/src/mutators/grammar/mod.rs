#![allow(clippy::type_complexity)]
#![allow(clippy::module_inception)]
#![allow(clippy::nonstandard_macro_braces)]

mod ast;
mod grammar;
mod incremental_map_conformance;
mod list;
mod mutators;
mod parser;
mod regex;

#[doc(inline)]
pub use ast::AST;
#[doc(inline)]
pub use grammar::Grammar;
#[doc(inline)]
pub use mutators::grammar_based_ast_mutator;
#[doc(inline)]
pub use mutators::grammar_based_string_mutator;
#[doc(inline)]
pub use regex::grammar_from_regex;
