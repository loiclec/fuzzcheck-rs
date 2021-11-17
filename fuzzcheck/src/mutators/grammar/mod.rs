//! Grammar-based mutators and related utilties.
//!
//! This module provides two mutators a grammar-based `impl Mutator<AST>` which generates an abstract syntax
//! tree satisfying a grammar, created though [`grammar_based_ast_mutator`]. You can then obtain a string
//! from the [`AST`] by calling [`ast.to_string()`](AST::to_string).
//!
//!
//! To specify a grammar, you should use the following functions:
//! * [`regex`](crate::mutators::grammar::regex) to create a grammar from a regular expression
//! * [`literal`] for a grammar that matches a single character
//! * [`literal_ranges`] for a grammar matching a single character within a specified ranges
//! * [`literal_ranges`] for a grammar matching a single character within any of multiple ranges
//! * [`alternation`] for a grammar matching any of a list of grammar rules
//! * [`concatenation`] matching multiple grammar rules one after the other
//! * [`repetition`] matching a grammar rule multiple times
//! * [`recursive`] and [`recurse`] to create recursive grammar rules
//!
//! Examples:
//! ```
//! use fuzzcheck::mutators::grammar::{alternation, concatenation, literal, literal_range, recurse, recursive, regex, repetition};
//!
//! let rule = repetition(
//!     concatenation([
//!         alternation([
//!             regex("hello[0-9]"),
//!             regex("world[0-9]?")
//!         ]),
//!         literal(' '),
//!     ]),
//!     1..5
//! );
//! /* “rule” matches/generates:
//!     hello1
//!     hello2 hello6 world
//!     world8 hello5 world7 world world
//!     ...
//! */
//!
//! let rule = recursive(|rule| {
//!     alternation([
//!         concatenation([
//!             regex(r"\(|\["),
//!             recurse(rule),
//!             regex(r"\)|\]"),
//!         ]),
//!         literal_range('a' ..= 'z')
//!     ])
//! });
//! /* rule matches/generates:
//!     (([a)))
//!     (([[d))])
//!     z
//!     ...
//!  */
//! ```
#![allow(clippy::type_complexity)]
#![allow(clippy::module_inception)]
#![allow(clippy::nonstandard_macro_braces)]

mod ast;
mod grammar;
// mod incremental_map_conformance;
// mod list;
mod mutators;
// mod parser;
mod regex;

#[doc(inline)]
pub use ast::AST;
#[doc(inline)]
pub use grammar::Grammar;
#[doc(inline)]
pub use grammar::{
    alternation, concatenation, literal, literal_range, literal_ranges, recurse, recursive, regex, repetition,
};
#[doc(inline)]
pub use mutators::grammar_based_ast_mutator;

#[doc(inline)]
// pub use mutators::grammar_based_string_mutator;
pub use mutators::GrammarBasedASTMutator;
// pub use mutators::GrammarBasedStringMutator;
