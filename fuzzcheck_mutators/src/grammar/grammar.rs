use std::{
    ops::{Range, RangeBounds},
    rc::Rc,
    rc::Weak,
};

#[derive(Clone, Debug)]
pub struct Grammar {
    pub grammar: Rc<InnerGrammar>,
}
impl Grammar {
    pub fn new(inner: InnerGrammar) -> Self {
        Self {
            grammar: Rc::new(inner),
        }
    }
}

#[derive(Debug)]
pub enum InnerGrammar {
    Literal(Range<char>),
    Alternation(Vec<Grammar>),
    Concatenation(Vec<Grammar>),
    Repetition(Grammar, Range<usize>),
    Shared(Rc<Grammar>),
    Recurse(Weak<Grammar>),
}

impl Grammar {
    pub fn literal<R>(range: R) -> Grammar
    where
        R: RangeBounds<char>,
    {
        let start = match range.start_bound() {
            std::ops::Bound::Included(x) => *x,
            std::ops::Bound::Excluded(x) => unsafe { char::from_u32_unchecked((*x as u32) + 1) },
            std::ops::Bound::Unbounded => panic!("The range must have a lower bound"),
        };
        let end = match range.end_bound() {
            std::ops::Bound::Included(x) => unsafe { char::from_u32_unchecked((*x as u32) + 1) },
            std::ops::Bound::Excluded(x) => *x,
            std::ops::Bound::Unbounded => panic!("The range must have an upper bound"),
        };
        Grammar::new(InnerGrammar::Literal(start..end))
    }
    pub fn alternation(gs: impl IntoIterator<Item = Grammar>) -> Grammar {
        Grammar::new(InnerGrammar::Alternation(gs.into_iter().collect()))
    }
    pub fn concatenation(gs: impl IntoIterator<Item = Grammar>) -> Grammar {
        Grammar::new(InnerGrammar::Concatenation(gs.into_iter().collect()))
    }
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
            std::ops::Bound::Included(x) => *x + 1,
            std::ops::Bound::Excluded(x) => *x,
            std::ops::Bound::Unbounded => usize::MAX,
        };
        Grammar::new(InnerGrammar::Repetition(gs, start..end))
    }

    pub fn shared(g: &Rc<Grammar>) -> Grammar {
        Grammar::new(InnerGrammar::Shared(g.clone()))
    }
    pub fn recurse(g: &Weak<Grammar>) -> Grammar {
        Grammar::new(InnerGrammar::Recurse(g.clone()))
    }
}
