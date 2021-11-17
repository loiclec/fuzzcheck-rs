#![allow(unused_attributes)]
#![feature(no_coverage)]

use std::rc::{Rc, Weak};

use fuzzcheck::mutators::grammar::*;
use fuzzcheck::mutators::testing_utilities::test_mutator;
// use fuzzcheck::{DefaultMutator, Mutator};

#[no_coverage]
fn text() -> Rc<Grammar> {
    regex("([\u{0}-\u{7f}]|.)+|CDATA")
}
#[no_coverage]
fn whitespace() -> Rc<Grammar> {
    regex("[ \t\n\r]+")
}
#[no_coverage]
fn header(md: &Weak<Grammar>) -> Rc<Grammar> {
    concatenation([regex("#+"), recurse(md), regex("#*")])
}
#[no_coverage]
pub fn quote() -> Rc<Grammar> {
    regex(">+")
}
#[no_coverage]
pub fn list() -> Rc<Grammar> {
    regex("[-*+]|[0-9]*[.)]")
}
#[no_coverage]
pub fn emphasis(md: &Weak<Grammar>) -> Rc<Grammar> {
    concatenation([regex("[*_~`]+"), recurse(md), regex("[*_~`]+")])
}
#[no_coverage]
pub fn autolink(md: &Weak<Grammar>) -> Rc<Grammar> {
    concatenation([literal('<'), alternation([recurse(md), text(), web()]), literal('>')])
}
#[no_coverage]
pub fn reference(md: &Weak<Grammar>) -> Rc<Grammar> {
    concatenation([
        regex("!?\\["),
        recurse(md),
        literal(']'),
        repetition(concatenation([literal('('), recurse(md), literal(')')]), 0..=1),
    ])
}
#[no_coverage]
pub fn reference_definition(md: &Weak<Grammar>) -> Rc<Grammar> {
    concatenation([
        literal('['),
        recurse(md),
        literal(']'),
        repetition(whitespace(), 0..=1),
        literal(':'),
    ])
}
#[no_coverage]
pub fn thematic_break_or_setext_or_fence() -> Rc<Grammar> {
    alternation([
        regex("[* \t]{3,}"),
        regex("[- \t]{3,}"),
        regex("[= \t]{3,}"),
        regex("[~ \t]{3,}"),
        regex("[` \t]{3,}"),
    ])
}
#[no_coverage]
pub fn backslash() -> Rc<Grammar> {
    literal('\\')
}
#[no_coverage]
pub fn entity() -> Rc<Grammar> {
    concatenation([
        literal('&'),
        repetition(literal('#'), 0..=1),
        repetition(text(), 0..=1),
        repetition(literal(';'), 0..=1),
    ])
}
#[no_coverage]
pub fn task(whole: &Weak<Grammar>) -> Rc<Grammar> {
    concatenation([
        regex("-|\\+"),
        alternation([whitespace(), text()]),
        literal('['),
        alternation([regex(r"x|\^"), text(), recurse(whole)]),
        literal(']'),
    ])
}
#[no_coverage]
pub fn indented_block(whole: &Weak<Grammar>) -> Rc<Grammar> {
    concatenation([regex("[ \t]+"), recurse(whole)])
}
#[no_coverage]
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
#[no_coverage]
pub fn html_comment(whole: &Weak<Grammar>) -> Rc<Grammar> {
    concatenation([regex("<-+"), recurse(whole), regex("-+>")])
}
#[no_coverage]
fn quoted(whole: &Weak<Grammar>) -> Rc<Grammar> {
    concatenation([regex("[\"']"), alternation([text(), recurse(whole)]), regex("[\"']")])
}
#[no_coverage]
fn fenced_block(whole: &Weak<Grammar>) -> Rc<Grammar> {
    concatenation([regex("~{3,}|`{3,}"), recurse(whole), regex("~{3,}|`{3,}")])
}
#[no_coverage]
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
#[no_coverage]
fn web() -> Rc<Grammar> {
    concatenation([regex("(https?://)?(www.)?"), text(), literal('.'), text()])
}
#[no_coverage]
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
    test_mutator(mutator, 1000., 1000., false, 100, 100);
    // let mutator = grammar_based_string_mutator(markdown());
    // test_mutator(mutator, 1000., 1000., false, 100, 100);
    // let mutator = <Vec<SampleStruct<u8, u8>>>::default_mutator();
    // test_mutator(mutator, 1000., 1000., false, 100, 100);
}
