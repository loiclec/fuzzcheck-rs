# A markdown grammar

In this section we write a grammar with the goal of producing interesting markdown strings.

Note that this is different from formalizing the markdown syntax. We don't need
to make it unambiguous, or to have one rule per syntactic construct. The goal is merely to produce
“interesting” strings.

In fact, when testing the parser of a language, it is a good idea to make the grammar used 
for fuzz-testing **different** than the actual grammar of the language. The goal of fuzzing, 
after all, is to verify one’s assumption about the kinds of inputs that our code might handle. 
If we over-specify our mutators such that the only values they produce are the ones that we 
anticipated, then we won't discover many bugs. With that being said, let's write a markdown
grammar.

## Scope

Since this is just a tutorial, we will limit the scope of what we fuzz test. In particular, we
won't test any extension to the markdown syntax, such as strikethroughs and tables.

## Basic text

The most basic markdown document is just a series of letters with no special meaning:
```
Hi! My name is Loïc and I am running out of creativity to write good examples in this tutorial.
```
The grammar for this will just be a repetition of characters, which can also express using a regular expression.
```rust ignore
fn text() -> Rc<Grammar> {
    regex(".+")
}
```
But note that when using such a general pattern as `.+`, we will almost always produce characters with no meaning.
It is better to split the rule into two parts: ascii characters, and the other ones. The mutator will then
treat both cases with equal importance.
```rust ignore
fn text() -> Rc<Grammar> {
    regex("([\u{0}-\u{7f}]|.)+")
}
```

## Whitespace

Whenever we need whitespace, we will use the following:
```rust ignore
fn whitespace() -> Rc<Grammar> {
    regex("[ \t\n\r]+")
}
```

## Header

To generate strings of the form:
```
# Header
#### Header ######
```

We can use the following:
```rust ignore
fn header() -> Rc<Grammar> {
    concatenation([
        regex("#+"),
        text(),
        regex("#*")
    ])
}
```

However, we can do better. We don't want to assume that the parser will treat the inside of the header as
just text. What if a specific markdown string inside a header causes a bug?
It is almost always better to be more general. So instead, we replace the `text()` part of the grammar
with a recursion to the whole markdown grammar.

```rust ignore
fn header(md: &Weak<Grammar>) -> Rc<Grammar> {
    concatenation([
        regex("#+"),
        recurse(md),
        regex("#*")
    ])
}
```

## Others

It is not the most useful thing to dwell on the details of every rule. Instead, I will summarize here the markdown features
for which I wrote a grammar rule:
* headers: `## header`
* emphases: `**hello**` and `_world_`
* quotes: `> md`
* lists: `1. md` or `- md` or ..
* references: `[hello](world)`, `![ref]`, `[ref]: definition`
* autolinks: `<https://google.come>`
* thematic breaks: `---`, `*****`
* setext header (indirectly): `----`, `===`

For a comprehensive markdown grammar, there should be many more rules, but we will stop here. 
The full code is available at the bottom of the page.

## Putting it all together

We can now write the whole markdown grammar as a repetition of the subgrammars.

```rust ignore
fn markdown() -> Rc<Grammar> {
    recursive(|md| {
        repetition(
            alternation([
                text(),
                header(md),
                emphasis(md),
                quote(),
                list(),
                reference(md),
                reference_definition(md),
                autolink(md),
                thematic_break_or_setext()
            ]),
            0..
        )
    })
}
```

## Full Code

```rust ignore

#[cfg(all(fuzzing, test))]
mod tests {
    use crate::{html, Parser};
    use fuzzcheck::mutators::grammar::*;
    use std::rc::{Rc, Weak};

    fn text() -> Rc<Grammar> {
        regex("([\u{0}-\u{7f}]|.)+")
    }

    fn whitespace() -> Rc<Grammar> {
        regex("[ \t\n\r]+")
    }

    fn header(md: &Weak<Grammar>) -> Rc<Grammar> {
        concatenation([regex("#+"), recurse(md), regex("#*")])
    }

    pub fn quote() -> Rc<Grammar> {
        regex(">+")
    }

    pub fn list() -> Rc<Grammar> {
        regex("[-*+]+|[0-9]+[.)]?")
    }

    pub fn emphasis(md: &Weak<Grammar>) -> Rc<Grammar> {
        concatenation([regex("[*_~`]+"), recurse(md), regex("[*_~`]+")])
    }

    pub fn autolink(md: &Weak<Grammar>) -> Rc<Grammar> {
        concatenation([literal('<'), alternation([recurse(md), text()]), literal('>')])
    }

    pub fn reference(md: &Weak<Grammar>) -> Rc<Grammar> {
        concatenation([
            regex("!?\\["),
            recurse(md),
            literal(']'),
            repetition(concatenation([literal('('), recurse(md), literal(')')]), 0..=1),
        ])
    }

    pub fn reference_definition(md: &Weak<Grammar>) -> Rc<Grammar> {
        concatenation([
            literal('['),
            recurse(md),
            literal(']'),
            repetition(whitespace(), 0..=1),
            literal(':'),
        ])
    }

    pub fn thematic_break_or_setext_or_fence() -> Rc<Grammar> {
        alternation([
            regex("[* \t]{3,}"),
            regex("[- \t]{3,}"),
            regex("[= \t]{3,}"),
            regex("[~ \t]{3,}"),
            regex("[` \t]{3,}"),
        ])
    }
    fn markdown() -> Rc<Grammar> {
        recursive(|md| {
            repetition(
                alternation([
                    text(),
                    header(md),
                    emphasis(md),
                    quote(),
                    list(),
                    reference(md),
                    reference_definition(md),
                    autolink(md),
                    thematic_break_or_setext()
                ]),
                0..
            )
        })
    } 
    fn push_html_does_not_crash(md_string: &str) {
        let parser = Parser::new(md_string);
        let mut html_output = String::new();
        html::push_html(&mut html_output, parser);
    }
}
```