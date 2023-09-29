use std::ops::{Range, RangeBounds, RangeInclusive};
use std::rc::{Rc, Weak};

#[cfg(feature = "regex_grammar")]
use crate::mutators::grammar::regex::grammar_from_regex;

#[derive(Clone, Debug)]
/// A grammar which can be used for fuzzing.
///
/// See [the module documentation](crate::mutators::grammar) for advice on how to create a grammar.
pub enum Grammar {
    Literal(Vec<RangeInclusive<char>>),
    Alternation(Vec<Rc<Grammar>>),
    Concatenation(Vec<Rc<Grammar>>),
    Repetition(Rc<Grammar>, Range<usize>),
    Recurse(Weak<Grammar>),
    Recursive(Rc<Grammar>),
}

#[cfg(feature = "regex_grammar")]
#[doc(cfg(feature = "regex_grammar"))]
#[coverage(off)]
pub fn regex(s: &str) -> Rc<Grammar> {
    grammar_from_regex(s)
}

#[coverage(off)]
/// Creates an [`Rc<Grammar>`] which outputs characters in the given range.
///
/// For example, to generate characters in the range 'a' to 'z' (inclusive), one
/// could use this code
///
/// ```
/// let a_to_z = literal_ranges('a'..='z');
/// ```
pub fn literal_ranges(ranges: Vec<RangeInclusive<char>>) -> Rc<Grammar> {
    Rc::new(Grammar::Literal(ranges))
}

#[coverage(off)]
/// Creates an [`Rc<Grammar>`] which matches a single character literal.
///
/// ```
/// let l = literal('l');
/// ```
pub fn literal(l: char) -> Rc<Grammar> {
    Rc::new(Grammar::Literal(vec![l..=l]))
}
#[coverage(off)]
pub fn literal_range<R>(range: R) -> Rc<Grammar>
where
    R: RangeBounds<char>,
{
    let start = match range.start_bound() {
        std::ops::Bound::Included(x) => *x,
        std::ops::Bound::Excluded(x) => unsafe { char::from_u32_unchecked((*x as u32) + 1) },
        std::ops::Bound::Unbounded => panic!("The range must have a lower bound"),
    };
    let end = match range.end_bound() {
        std::ops::Bound::Included(x) => *x,
        std::ops::Bound::Excluded(x) => unsafe { char::from_u32_unchecked(*x as u32 - 1) },
        std::ops::Bound::Unbounded => panic!("The range must have an upper bound"),
    };
    Rc::new(Grammar::Literal(vec![start..=end]))
}

/// Produces a grammar which will choose between the provided grammars.
#[coverage(off)]
pub fn alternation(gs: impl IntoIterator<Item = Rc<Grammar>>) -> Rc<Grammar> {
    Rc::new(Grammar::Alternation(gs.into_iter().collect()))
}

/// Produces a grammar which will concatenate the output of all the provided
/// grammars together, in order.
///
/// For example, the grammar
/// ```
/// concatenation([
///     regex("fuzz"),
///     regex("check")
/// ])
/// ```
/// would output "fuzzcheck".
#[coverage(off)]
pub fn concatenation(gs: impl IntoIterator<Item = Rc<Grammar>>) -> Rc<Grammar> {
    Rc::new(Grammar::Concatenation(gs.into_iter().collect()))
}

#[coverage(off)]
/// Repeats the provided grammar some number of times in the given range.
pub fn repetition<R>(gs: Rc<Grammar>, range: R) -> Rc<Grammar>
where
    R: RangeBounds<usize>,
{
    let start = match range.start_bound() {
        std::ops::Bound::Included(x) => *x,
        std::ops::Bound::Excluded(x) => *x + 1,
        std::ops::Bound::Unbounded => panic!("The range must have a lower bound"),
    };
    let end = match range.end_bound() {
        std::ops::Bound::Included(x) => x.saturating_add(1),
        std::ops::Bound::Excluded(x) => *x,
        std::ops::Bound::Unbounded => usize::MAX,
    };
    Rc::new(Grammar::Repetition(gs, start..end))
}

#[coverage(off)]
/// Used to indicate a point of recursion to Fuzzcheck. Should be combined with
/// [`recursive`].
///
/// See the module documentation ([`super`]) for an example on how to use it.
pub fn recurse(g: &Weak<Grammar>) -> Rc<Grammar> {
    Rc::new(Grammar::Recurse(g.clone()))
}

#[coverage(off)]
/// Creates a recursive grammar. This function should be combined with
/// [`recurse`] to make recursive calls.
///
/// See the module documentation ([`super`]) for an example on how to use it.
pub fn recursive(data_fn: impl Fn(&Weak<Grammar>) -> Rc<Grammar>) -> Rc<Grammar> {
    Rc::new(Grammar::Recursive(Rc::new_cyclic(|g| {
        Rc::try_unwrap(data_fn(g)).unwrap()
    })))
}
