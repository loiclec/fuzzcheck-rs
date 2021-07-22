#![allow(clippy::type_complexity)]
#![allow(clippy::module_inception)]
#![allow(clippy::nonstandard_macro_braces)]

mod ast;
mod grammar;
mod list;
mod mapping;
mod mutators;
mod parser;

pub use ast::{ASTMapping, ASTMappingKind, AST};
pub use grammar::Grammar;
pub use mutators::{
    ASTMutator, ASTMutatorArbitraryStep, ASTMutatorCache, ASTMutatorMutationStep, ASTMutatorUnmutateToken,
    GrammarBasedStringMutator,
};
pub use parser::parse_from_grammar;

/**
    Creates a grammar corresponding to a single character within the specfied range or ranges.
    ```
    # use fuzzcheck_mutators::grammar::Grammar;
    # use fuzzcheck_mutators::{concatenation, literal};
    let a = literal!('a'); // a single character
    let a_to_z = literal!('a' ..= 'z'); // a character within a range
    let digit_or_space = literal! { ('0'..='9'), (' ') }; // either a digit or a space
    ```
*/
#[macro_export]
macro_rules! literal {
    ($l:literal) => {
        Grammar::literal($l..=$l)
    };
    ($l:expr) => {
        Grammar::literal($l)
    };
    ( $(($x:expr)),* ) => {
        Grammar::alternation(vec![
            $(literal!($x)),*
        ])
    };
}
/**
    Creates a grammar corresponding to a sequence of rules.

    ```
    # use fuzzcheck_mutators::grammar::Grammar;
    # use fuzzcheck_mutators::{concatenation, literal};
    // will match only the string "abcd"
    let abcd = concatenation! {
        literal!('a'),
        literal!('b'),
        literal!('c'),
        literal!('d')
    };
    ```
*/
#[macro_export]
macro_rules! concatenation {
    ($($gsm:expr),*) => {
        Grammar::concatenation([
            $($gsm),*
        ])
    }
    ;
}
#[macro_export]
macro_rules! alternation {
    ($($gsm:expr),*) => {
        Grammar::alternation([
            $($gsm),*
        ])
    }
    ;
}
#[macro_export]
macro_rules! repetition {
    ($g:expr, $range:expr) => {
        Grammar::repetition($g, $range)
    };
}

#[macro_export]
macro_rules! recursive {
    ($g:pat in $e:expr) => {
        Grammar::recursive(|$g| $e)
    };
}
#[macro_export]
macro_rules! recurse {
    ($g:ident) => {
        Grammar::recurse($g)
    };
}
