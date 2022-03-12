# Creating a grammar-based mutator

## String Mutators

First, I should note that fuzzcheck does not yet have a general-purpose
`Mutator<String>`. However, we can still use two approaches to generate
strings. The first approach is to use a `Vec<u8>` and then obtain a string from the
slice of bytes using `String::from_utf8_lossy(..)`. The second is to use
*grammar-based mutators* to generate syntax trees and their associated strings.
The shape of the generated trees are described by a *grammar*.

For example, we can write:
```rust ignore
use fuzzcheck::mutators::grammar;

let rule1 = grammar::regex("[a-z](0|ad)[!?]{1,10}")
```
which is a grammar that matches/generates the following strings:
```
gad!
b0???!
w0?
```
but not the following:
```
yad     (missing the last rule: a repetition of ! and ?)
Gad!    (G is not in 'a' ..= 'z')
b0ad?!  (what follows the first letter can be either 0 or ad , but not both)
```

We can also write recursive grammars as follows:

```rust ignore
use fuzzcheck::mutators::grammar::{regex, literal, alternation, concatenation, recursive, recurse};

let grammar = recursive(|g| {
    alternation([
        regex("[a-zA-Z]"),
        concatenation([
            literal('('),
            recurse(g),
            literal(')')
        ])
    ])
);
```
which matches the following strings:
```
F
s
(a)
((((H))))
((((((((((((((((((((((((((((((((((((((((n))))))))))))))))))))))))))))))))))))))))
```
but not those:
```
(a        (mismatched parentheses)
((()))    (no letter inside the parentheses)
```

It can be useful to build a grammar piece by piece instead of within a single declaration.

```rust ignore
use fuzzcheck::mutators::grammar::{regex, literal, alternation, concatenation, repetition, recursive, recurse};
use fuzzcheck::mutators::grammar::Grammar;
use std::rc::Rc;
use std::rc::Weak;

fn whitespace() -> Rc<Grammar> {
    regex("[ \t]+")
}
fn simple_short_ascii_word() -> Rc<Grammar> {
    regex("[a-zA-Z]{1,10}")
}
fn reference() -> Rc<Grammar> {
    concatenation([
        literal('['),
        simple_short_ascii_word(),
        literal(']')
    ])
}
fn recursing_rule(whole: &Weak<Grammar>) -> Rc<Grammar> {
    concatenation([
        literal('<'),
        recurse!(whole),
        literal('>')
    ])
}
fn final_grammar() -> Rc<Grammar> {
    recursive!(|whole_grammar| {
        alternation([
            simple_short_ascii_word(),
            reference(),
            recursing_rule(whole_grammar)
        ])
    })
}
```
The above grammar matches the following strings:
```
hello
[world]
<planto>
<<<[bing]>>>
```

## Creating a String mutator from a grammar

Once you have a grammar, of type `Rc<Grammar>`, you can obtain
a mutator that generates syntax trees matching the grammar as well as the
associated string:

```rust ignore
use fuzzcheck::mutators::grammar::grammar_based_ast_mutator;
let grammar = regex(/* .. */);
let mutator = grammar_based_ast_mutator(grammar) // : impl Mutator<AST>
    .with_string(); // : impl Mutator<(AST, String)>
```

Unfortunately, the argument of the test function will need to be `&(AST, String)` instead of
`&str`. (A previous version of fuzzcheck also had a grammar-based *string* mutator,
which implemented `Mutator<String>`. However, I have removed it from the latest version
while I fix some bugs with it).

The test function given to fuzzcheck will then need to have the following signature:
```rust ignore
fn test((_, x): &(AST, String)) { 
    // ...
}
// instead of
fn test(x: &str) { }
```

In the next section, we build a grammar such that it generates interesting markdown strings.
