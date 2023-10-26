#![feature(coverage_attribute)]
// #![feature(trivial_bounds)]
mod alternation_char_mutators;
mod char_mutators;
mod const_generics;
mod constrained_integer;
mod derived_mutually_recursive_structs;
mod derived_recursive_struct;
mod derived_recursive_struct_fully_custom;
mod derived_struct;
mod enum_with_ignored_variant;
mod expansions;
#[cfg(feature = "regex_grammar")]
mod grammar_based_mutators;
mod option;
mod vector;
