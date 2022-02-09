#![feature(no_coverage)]
#![feature(trivial_bounds)]

mod alternation_char_mutators;
mod char_mutators;
mod constrained_integer;
mod derived_mutually_recursive_structs;
mod derived_recursive_struct;
mod expansions;
#[cfg(feature = "regex_grammar")]
mod grammar_based_mutators;
mod option;
mod vector;
