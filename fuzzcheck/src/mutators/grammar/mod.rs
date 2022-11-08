//! Grammar-based mutators and related utilties.
//!
//! This module provides a grammar-based `impl Mutator<AST>` which generates an abstract syntax
//! tree satisfying a grammar, created through [`grammar_based_ast_mutator`]. The resulting mutator can be
//! transformed into a `Mutator<(AST, String)>`, where the second element of the tuple is the string corresponding
//! to the abstract syntax tree, by calling [`.with_string()`](ASTMutator::with_string).
//!
//! To specify a grammar, you should use the following functions:
#![cfg_attr(
    feature = "regex_grammar",
    doc = "* [`regex`](crate::mutators::grammar::regex) to create a grammar from a regular expression **(only supported on crate feature `regex_grammar`)**"
)]
//! * [`literal`] for a grammar that matches a single character
//! * [`literal_ranges`] for a grammar matching a single character within a specified ranges
//! * [`literal_ranges`] for a grammar matching a single character within any of multiple ranges
//! * [`alternation`] for a grammar matching any of a list of grammar rules
//! * [`concatenation`] matching multiple grammar rules one after the other
//! * [`repetition`] matching a grammar rule multiple times
//! * [`recursive`] and [`recurse`] to create recursive grammar rules
#![cfg_attr(
    feature = "regex_grammar",
    doc = r###"
Examples:
```
use fuzzcheck::mutators::grammar::{alternation, concatenation, literal, literal_range, recurse, recursive, regex, repetition};
let rule = repetition(
    concatenation([
        alternation([
            regex("hello[0-9]"),
            regex("world[0-9]?")
        ]),
        literal(' '),
    ]),
    1..5
);
/* “rule” matches/generates:
    hello1
    hello2 hello6 world
    world8 hello5 world7 world world
    ...
*/
let rule = recursive(|rule| {
    alternation([
        concatenation([
            regex(r"\(|\["),
            recurse(rule),
            regex(r"\)|\]"),
        ]),
        literal_range('a' ..= 'z')
    ])
});
/* rule matches/generates:
    (([a)))
    (([[d))])
    z
    ...
 */
```
"###
)]
#![allow(clippy::type_complexity)]
#![allow(clippy::module_inception)]
#![allow(clippy::nonstandard_macro_braces)]

mod ast;
mod grammar;
mod mutators;

#[cfg(feature = "regex_grammar")]
mod regex;

#[doc(inline)]
pub use ast::AST;
#[cfg(feature = "regex_grammar")]
#[doc(inline)]
#[doc(cfg(feature = "regex_grammar"))]
pub use grammar::regex;
#[doc(inline)]
pub use grammar::{alternation, concatenation, literal, literal_range, literal_ranges, recurse, recursive, repetition};
#[doc(inline)]
pub use grammar::{Grammar, GrammarInner};
#[doc(inline)]
pub use mutators::grammar_based_ast_mutator;
#[doc(inline)]
pub use mutators::ASTMutator;
