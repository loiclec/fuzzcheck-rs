#![cfg(feature = "serde_json_serializer")]
#![allow(unused_attributes)]
#![feature(no_coverage)]

use std::rc::Weak;

use fuzzcheck::mutators::grammar::*;
use fuzzcheck::mutators::testing_utilities::test_mutator;

#[no_coverage]
fn text() -> Grammar {
    regex("([\u{0}-\u{7f}]|.)+|CDATA")
}

#[no_coverage]
fn whitespace() -> Grammar {
    regex("[ \t\n\r]+")
}

#[no_coverage]
fn header(md: &Weak<GrammarInner>) -> Grammar {
    regex("#+") + recurse(md) + regex("#*")
}

#[no_coverage]
pub fn quote() -> Grammar {
    regex(">+")
}

#[no_coverage]
pub fn list() -> Grammar {
    regex("[-*+]|[0-9]*[.)]")
}

#[no_coverage]
pub fn emphasis(md: &Weak<GrammarInner>) -> Grammar {
    regex("[*_~`]+") + recurse(md) + regex("[*_~`]+")
}

#[no_coverage]
pub fn autolink(md: &Weak<GrammarInner>) -> Grammar {
    concatenation([literal('<'), alternation([recurse(md), text(), web()]), literal('>')])
}

#[no_coverage]
pub fn reference(md: &Weak<GrammarInner>) -> Grammar {
    concatenation([
        regex("!?\\["),
        recurse(md),
        literal(']'),
        repetition(literal('(') + recurse(md) + literal(')'), 0..=1),
    ])
}
#[no_coverage]
pub fn reference_definition(md: &Weak<GrammarInner>) -> Grammar {
    literal('[') + recurse(md) + literal(']') + repetition(whitespace(), 0..=1) + literal(':')
}
#[no_coverage]
pub fn thematic_break_or_setext_or_fence() -> Grammar {
    regex("[* \t]{3,}") | regex("[- \t]{3,}") | regex("[= \t]{3,}") | regex("[~ \t]{3,}") | regex("[` \t]{3,}")
}
#[no_coverage]
pub fn backslash() -> Grammar {
    literal('\\')
}

#[no_coverage]
pub fn entity() -> Grammar {
    literal('&') + repetition(literal('#'), 0..=1) + repetition(text(), 0..=1) + repetition(literal(';'), 0..=1)
}

#[no_coverage]
pub fn task(whole: &Weak<GrammarInner>) -> Grammar {
    concatenation([
        regex("-|\\+"),
        alternation([whitespace(), text()]),
        literal('['),
        alternation([regex(r"x|\^"), text(), recurse(whole)]),
        literal(']'),
    ])
}

#[no_coverage]
pub fn indented_block(whole: &Weak<GrammarInner>) -> Grammar {
    regex("[ \t]+") + recurse(whole)
}

#[no_coverage]
pub fn html() -> Grammar {
    regex("</?")
        + text()
        + repetition(
            concatenation([
                regex("[ \t]?"),
                text(),
                literal('='),
                literal('"'),
                text(),
                literal('"'),
            ]),
            0..,
        )
        + literal('>')
}

#[no_coverage]
pub fn html_comment(whole: &Weak<GrammarInner>) -> Grammar {
    regex("<-+") + recurse(whole) + regex("-+>")
}

#[no_coverage]
fn quoted(whole: &Weak<GrammarInner>) -> Grammar {
    regex("[\"']") + (text() | recurse(whole)) + regex("[\"']")
}

#[no_coverage]
fn fenced_block(whole: &Weak<GrammarInner>) -> Grammar {
    concatenation([regex("~{3,}|`{3,}"), recurse(whole), regex("~{3,}|`{3,}")])
}

#[no_coverage]
fn table(whole: &Weak<GrammarInner>) -> Grammar {
    repetition(
        // row
        concatenation([
            repetition(
                // column
                concatenation([
                    repetition(alternation([text(), recurse(whole), regex(":*-*:*")]), 0..=1),
                    literal('|'),
                    alternation([text(), recurse(whole), regex(":*-*:*")]),
                ]),
                1..10,
            ),
            literal_ranges(vec!['\r'..='\r', '\n'..='\n']),
        ]),
        1..10,
    )
}

#[no_coverage]
fn web() -> Grammar {
    concatenation([regex("(https?://)?(www.)?"), text(), literal('.'), text()])
}

#[no_coverage]
fn markdown() -> Grammar {
    recursive(|md| {
        repetition(
            alternation([
                whitespace(),
                text(),
                backslash(),
                entity(),
                task(md),
                header(md),
                emphasis(md),
                quote(),
                list(),
                web(),
                reference(md),
                reference_definition(md),
                autolink(md),
                thematic_break_or_setext_or_fence(),
                indented_block(md),
                html(),
                html_comment(md),
                quoted(md),
                fenced_block(md),
                table(md),
            ]),
            0..,
        )
    })
}

#[test]
fn test_grammar_based_ast_mutator() {
    let mutator = grammar_based_ast_mutator(markdown());
    test_mutator(mutator, 500., 500., false, true, 60, 100);
}
