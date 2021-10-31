use std::{
    ops::{Range, RangeBounds, RangeInclusive},
    rc::Rc,
    rc::Weak,
};

#[derive(Clone, Debug)]
/// A grammar which can be used for fuzzing.
///
/// Note that instead of constructing these yourself you probably want to use the provided macros
/// (see the documentation for each field for the appropriate macro to use).
pub enum Grammar {
    /// [crate::literal]
    Literal(Vec<RangeInclusive<char>>),
    /// [crate::alternation]
    Alternation(Vec<Rc<Grammar>>),
    /// [crate::concatenation]
    Concatenation(Vec<Rc<Grammar>>),
    /// [crate::repetition]
    Repetition(Rc<Grammar>, Range<usize>),
    /// [crate::recurse]
    Recurse(Weak<Grammar>),
    /// [crate::recursive]
    Recursive(Rc<Grammar>),
}

impl Grammar {
    #[no_coverage]
    /// You may want to use the [crate::literal] macro instead of invoking this function directly.
    pub fn literal_ranges(ranges: Vec<RangeInclusive<char>>) -> Rc<Grammar> {
        Rc::new(Grammar::Literal(ranges))
    }
    #[no_coverage]
    /// You may want to use the [crate::literal] macro instead of invoking this function directly.
    pub fn literal<R>(range: R) -> Rc<Grammar>
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
    #[no_coverage]
    /// You may want to use the [crate::alternation] macro instead of invoking this function directly.
    pub fn alternation(gs: impl IntoIterator<Item = Rc<Grammar>>) -> Rc<Grammar> {
        Rc::new(Grammar::Alternation(gs.into_iter().collect()))
    }
    #[no_coverage]
    /// You may want to use the [crate::concatenation] macro instead of invoking this function
    /// directly.
    pub fn concatenation(gs: impl IntoIterator<Item = Rc<Grammar>>) -> Rc<Grammar> {
        Rc::new(Grammar::Concatenation(gs.into_iter().collect()))
    }
    #[no_coverage]
    /// You may want to use the [crate::repetition] macro instead of invoking this function
    /// directly.
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

    #[no_coverage]
    /// You may want to use the [crate::recurse] macro instead of invoking this function directly.
    pub fn recurse(g: &Weak<Grammar>) -> Rc<Grammar> {
        Rc::new(Grammar::Recurse(g.clone()))
    }
    #[no_coverage]
    /// You may want to use the [crate::recursive] macro instead of invoking this function directly.
    pub fn recursive(data_fn: impl Fn(&Weak<Grammar>) -> Rc<Grammar>) -> Rc<Grammar> {
        Rc::new(Grammar::Recursive(Rc::new_cyclic(|g| {
            Rc::try_unwrap(data_fn(g)).unwrap()
        })))
    }
}
