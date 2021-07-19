mod ast;
mod grammar;
mod list;
mod mapping;
mod mutators;
mod parser;

pub use ast::{ASTMapping, ASTMappingKind, AST};
pub use grammar::{Grammar, InnerGrammar};
pub use mutators::{
    ASTMutator, ASTMutatorArbitraryStep, ASTMutatorCache, ASTMutatorMutationStep, ASTMutatorUnmutateToken,
    GrammarBasedStringMutator,
};
pub use parser::parse_from_grammar;

/**
    Creates a grammar corresponding to a single character within the specfied range or ranges.
    ```
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
