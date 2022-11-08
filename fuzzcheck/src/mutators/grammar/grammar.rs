use std::ops::{Add, BitOr, Range, RangeBounds, RangeInclusive};
use std::rc::{Rc, Weak};

#[cfg(feature = "regex_grammar")]
use crate::mutators::grammar::regex::grammar_from_regex;

#[derive(Clone, Debug)]
/// A grammar which can be used for fuzzing.
///
/// See [the module documentation](crate::mutators::grammar) for advice on how to create a grammar.
pub enum GrammarInner {
    Literal(Vec<RangeInclusive<char>>),
    Alternation(Vec<Grammar>),
    Concatenation(Vec<Grammar>),
    Repetition(Grammar, Range<usize>),
    Recurse(Weak<GrammarInner>),
    Recursive(Grammar),
}

#[derive(Debug, Clone)]
/// A [`Grammar`] can be transformed into an [`ASTMutator`] (which generates
/// [`String`]s corresponding to the grammar in question) or combined with other
/// grammars to produce a more complicated grammar.
///
/// For examples on how to use this struct, see the crate documentation
/// ([`super`]).
///
/// [`ASTMutator`]: crate::mutators::grammar::ASTMutator
pub struct Grammar(pub(crate) Rc<GrammarInner>);

impl From<Rc<GrammarInner>> for Grammar {
    fn from(inner: Rc<GrammarInner>) -> Self {
        Grammar(inner)
    }
}

impl AsRef<GrammarInner> for Grammar {
    fn as_ref(&self) -> &GrammarInner {
        &self.0
    }
}

impl Add for Grammar {
    type Output = Grammar;

    /// Calls [`concatenation`] on the two provided grammars.
    fn add(self, rhs: Self) -> Self::Output {
        concatenation([self, rhs])
    }
}

impl BitOr for Grammar {
    type Output = Grammar;

    /// Calls [`alternation`] on the two provided grammars.
    fn bitor(self, rhs: Self) -> Self::Output {
        alternation([self, rhs])
    }
}

#[cfg(feature = "regex_grammar")]
#[doc(cfg(feature = "regex_grammar"))]
#[no_coverage]
pub fn regex(s: &str) -> Grammar {
    grammar_from_regex(s)
}

#[no_coverage]
/// Creates a [`Grammar`] which outputs characters in the given range.
///
/// For example, to generate characters in the range 'a' to 'z' (inclusive), one
/// could use this code
///
/// ```
/// # use fuzzcheck::mutators::grammar::literal_ranges;
/// let a_to_z = literal_ranges(vec!['a'..='b', 'q'..='s', 't'..='w']);
/// ```
pub fn literal_ranges(ranges: Vec<RangeInclusive<char>>) -> Grammar {
    Rc::new(GrammarInner::Literal(ranges)).into()
}

#[no_coverage]
/// Creates a [`Grammar`] which matches a single character literal.
///
/// ```
/// # use fuzzcheck::mutators::grammar::literal;
/// let l = literal('l');
/// ```
pub fn literal(l: char) -> Grammar {
    Rc::new(GrammarInner::Literal(vec![l..=l])).into()
}

#[no_coverage]
pub fn literal_range<R>(range: R) -> Grammar
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
    Rc::new(GrammarInner::Literal(vec![start..=end])).into()
}

/// Produces a grammar which will choose between the provided grammars.
///
/// For example, this grammar
/// ```
/// # use fuzzcheck::mutators::grammar::{Grammar, alternation, regex};
/// let fuzz_or_check: Grammar = alternation([
///     regex("fuzz"),
///     regex("check")
/// ]);
/// ```
/// would output either "fuzz" or "check".
///
/// It is also possible to use the `|` operator to write alternation grammars.
/// For example, the [`Grammar`] above could be equivalently written as
///
/// ```
/// # use fuzzcheck::mutators::grammar::{Grammar, regex};
/// let fuzz_or_check: Grammar = regex("fuzz") | regex("check");
/// ```
#[no_coverage]
pub fn alternation(gs: impl IntoIterator<Item = Grammar>) -> Grammar {
    Rc::new(GrammarInner::Alternation(gs.into_iter().collect())).into()
}

/// Produces a grammar which will concatenate the output of all the provided
/// grammars together, in order.
///
/// For example, the grammar
/// ```
/// # use fuzzcheck::mutators::grammar::{concatenation, Grammar, regex};
/// let fuzzcheck: Grammar = concatenation([
///     regex("fuzz"),
///     regex("check")
/// ]);
/// ```
/// would output "fuzzcheck".
///
/// It is also possible to use the `+` operator to concatenate separate grammars
/// together. For example, the grammar above could be equivalently written as
///
/// ```
/// # use fuzzcheck::mutators::grammar::{Grammar, regex};
/// let fuzzcheck: Grammar = regex("fuzz") + regex("check");
/// ```
#[no_coverage]
pub fn concatenation(gs: impl IntoIterator<Item = Grammar>) -> Grammar {
    Rc::new(GrammarInner::Concatenation(gs.into_iter().collect())).into()
}

#[no_coverage]
/// Repeats the provided [`Grammar`] some number of times in the given range.
pub fn repetition<R>(gs: Grammar, range: R) -> Grammar
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
    Rc::new(GrammarInner::Repetition(gs, start..end)).into()
}

#[no_coverage]
/// Used to indicate a point of recursion to Fuzzcheck. Should be combined with
/// [`recursive`].
///
/// See the module documentation ([`super`]) for an example on how to use it.
pub fn recurse(g: &Weak<GrammarInner>) -> Grammar {
    Rc::new(GrammarInner::Recurse(g.clone())).into()
}

#[no_coverage]
/// Creates a recursive [`Grammar`]. This function should be combined with
/// [`recurse`] to make recursive calls.
///
/// See the module documentation ([`super`]) for an example on how to use it.
pub fn recursive(data_fn: impl Fn(&Weak<GrammarInner>) -> Grammar) -> Grammar {
    Rc::new(GrammarInner::Recursive(
        Rc::new_cyclic(|g| Rc::try_unwrap(data_fn(g).0).unwrap()).into(),
    ))
    .into()
}
