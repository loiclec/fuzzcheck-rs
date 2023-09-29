#![cfg(feature = "serde_json_serializer")]
#![allow(unused_attributes)]
#![feature(coverage_attribute)]

use std::rc::{Rc, Weak};

use fuzzcheck::mutators::grammar::*;
use fuzzcheck::mutators::testing_utilities::test_mutator;
// use fuzzcheck::{DefaultMutator, Mutator};

#[coverage(off)]
fn text() -> Rc<Grammar> {
    regex("([\u{0}-\u{7f}]|.)+|CDATA")
}
#[coverage(off)]
fn whitespace() -> Rc<Grammar> {
    regex("[ \t\n\r]+")
}
#[coverage(off)]
fn header(md: &Weak<Grammar>) -> Rc<Grammar> {
    concatenation([regex("#+"), recurse(md), regex("#*")])
}
#[coverage(off)]
pub fn quote() -> Rc<Grammar> {
    regex(">+")
}
#[coverage(off)]
pub fn list() -> Rc<Grammar> {
    regex("[-*+]|[0-9]*[.)]")
}
#[coverage(off)]
pub fn emphasis(md: &Weak<Grammar>) -> Rc<Grammar> {
    concatenation([regex("[*_~`]+"), recurse(md), regex("[*_~`]+")])
}
#[coverage(off)]
pub fn autolink(md: &Weak<Grammar>) -> Rc<Grammar> {
    concatenation([literal('<'), alternation([recurse(md), text(), web()]), literal('>')])
}
#[coverage(off)]
pub fn reference(md: &Weak<Grammar>) -> Rc<Grammar> {
    concatenation([
        regex("!?\\["),
        recurse(md),
        literal(']'),
        repetition(concatenation([literal('('), recurse(md), literal(')')]), 0..=1),
    ])
}
#[coverage(off)]
pub fn reference_definition(md: &Weak<Grammar>) -> Rc<Grammar> {
    concatenation([
        literal('['),
        recurse(md),
        literal(']'),
        repetition(whitespace(), 0..=1),
        literal(':'),
    ])
}
#[coverage(off)]
pub fn thematic_break_or_setext_or_fence() -> Rc<Grammar> {
    alternation([
        regex("[* \t]{3,}"),
        regex("[- \t]{3,}"),
        regex("[= \t]{3,}"),
        regex("[~ \t]{3,}"),
        regex("[` \t]{3,}"),
    ])
}
#[coverage(off)]
pub fn backslash() -> Rc<Grammar> {
    literal('\\')
}
#[coverage(off)]
pub fn entity() -> Rc<Grammar> {
    concatenation([
        literal('&'),
        repetition(literal('#'), 0..=1),
        repetition(text(), 0..=1),
        repetition(literal(';'), 0..=1),
    ])
}
#[coverage(off)]
pub fn task(whole: &Weak<Grammar>) -> Rc<Grammar> {
    concatenation([
        regex("-|\\+"),
        alternation([whitespace(), text()]),
        literal('['),
        alternation([regex(r"x|\^"), text(), recurse(whole)]),
        literal(']'),
    ])
}
#[coverage(off)]
pub fn indented_block(whole: &Weak<Grammar>) -> Rc<Grammar> {
    concatenation([regex("[ \t]+"), recurse(whole)])
}
#[coverage(off)]
pub fn html() -> Rc<Grammar> {
    concatenation([
        regex("</?"),
        text(),
        repetition(
            concatenation([
                regex("[ \t]?"),
                text(),
                literal('='),
                literal('"'),
                text(),
                literal('"'),
            ]),
            0..,
        ),
        literal('>'),
    ])
}
#[coverage(off)]
pub fn html_comment(whole: &Weak<Grammar>) -> Rc<Grammar> {
    concatenation([regex("<-+"), recurse(whole), regex("-+>")])
}
#[coverage(off)]
fn quoted(whole: &Weak<Grammar>) -> Rc<Grammar> {
    concatenation([regex("[\"']"), alternation([text(), recurse(whole)]), regex("[\"']")])
}
#[coverage(off)]
fn fenced_block(whole: &Weak<Grammar>) -> Rc<Grammar> {
    concatenation([regex("~{3,}|`{3,}"), recurse(whole), regex("~{3,}|`{3,}")])
}
#[coverage(off)]
fn table(whole: &Weak<Grammar>) -> Rc<Grammar> {
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
#[coverage(off)]
fn web() -> Rc<Grammar> {
    concatenation([regex("(https?://)?(www.)?"), text(), literal('.'), text()])
}
#[coverage(off)]
fn markdown() -> Rc<Grammar> {
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
