#![allow(clippy::type_complexity)]
#![allow(clippy::module_inception)]
#![allow(clippy::nonstandard_macro_braces)]

mod ast;
mod grammar;
mod incremental_map_conformance;
mod list;
mod mutators;
mod parser;

pub use grammar::Grammar;
pub use mutators::grammar_based_string_mutator;

/**
    Creates a grammar corresponding to a single character within the specfied range or ranges.
    ```
    use fuzzcheck::{concatenation, literal};
    let a = literal!('a'); // a single character
    let a_to_z = literal!('a' ..= 'z'); // a character within a range
    let digit_or_space = literal! { ('0'..='9'), (' ') }; // either a digit or a space
    ```
*/
#[macro_export]
macro_rules! literal {
    ($l:literal) => {
        $crate::mutators::grammar::Grammar::literal($l..=$l)
    };
    ($l:expr) => {
        $crate::mutators::grammar::Grammar::literal($l)
    };
    ( $(($x:expr)),* ) => {
        $crate::mutators::grammar::Grammar::alternation(vec![
            $(literal!($x)),*
        ])
    };
}
/**
    Creates a grammar corresponding to a sequence of rules.

    ```
    use fuzzcheck::{concatenation, literal};
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
        $crate::mutators::grammar::Grammar::concatenation([
            $($gsm),*
        ])
    }
    ;
}

/**
    Creates a grammar corresponding to an alternation of rules.

    ```
    use fuzzcheck::{alternation, literal};
    // will match only the strings "a", "b", and "c"
    let abc = alternation! {
        literal!('a'),
        literal!('b'),
        literal!('c')
    };
    ```
*/
#[macro_export]
macro_rules! alternation {
    ($($gsm:expr),*) => {
        $crate::mutators::grammar::Grammar::alternation([
            $($gsm),*
        ])
    }
    ;
}

/**
    Creates a grammar corresponding to the repetition of a rule.

    ```
    use fuzzcheck::{repetition, literal};
    // will match only the strings "", "a", and "aa"
    let g = repetition! {
        literal!('a'),
        0 ..= 2
    };
    ```
*/
#[macro_export]
macro_rules! repetition {
    ($g:expr, $range:expr) => {
        $crate::mutators::grammar::Grammar::repetition($g, $range)
    };
}

/**
    Creates a recursive grammar.

    ```
    use fuzzcheck::{recursive, recurse, literal, alternation, concatenation};
    // will match the strings "a", "(a)", "((a))", "(((a)))", and so on
    let g = recursive! { g in
        alternation! {
            concatenation! {
                literal!('('),
                recurse!(g),
                literal!(')')
            },
            literal!('a')
        }
    };
    ```
*/
#[macro_export]
macro_rules! recursive {
    ($g:pat in $e:expr) => {
        $crate::mutators::grammar::Grammar::recursive(|$g| $e)
    };
}

/**
    Creates a point of recursion in a grammar. See: [recursive!](crate::recursive!)
*/
#[macro_export]
macro_rules! recurse {
    ($g:ident) => {
        $crate::mutators::grammar::Grammar::recurse($g)
    };
}
