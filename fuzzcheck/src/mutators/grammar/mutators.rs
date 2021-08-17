extern crate self as fuzzcheck;

use super::grammar::Grammar;
use super::parser::parse_from_grammar;
use crate::mutators::alternation::AlternationMutator;
use crate::mutators::boxed::BoxMutator;
use crate::mutators::char::CharWithinRangeMutator;
use crate::mutators::either::Either;
use crate::mutators::fixed_len_vector::FixedLenVecMutator;
use crate::mutators::grammar::ast::{ASTMap, AST};
use crate::mutators::incremental_map::IncrementalMapMutator;
use crate::mutators::recursive::{RecurToMutator, RecursiveMutator};
use crate::mutators::tuples::Tuple1Mutator;
use crate::mutators::vector::VecMutator;
use crate::Mutator;
use fuzzcheck_mutators_derive::make_single_variant_mutator;
use std::collections::HashMap;
use std::rc::{Rc, Weak};

make_single_variant_mutator! {
    pub enum AST {
        Token(char),
        Sequence(Vec<AST>),
        Box(Box<AST>),
    }
}

/**
TODO: something like this
make_mutator_wrapper! {
    name: ASTMutator,
    additional_wrapper: Box,
    wrapped_mutator:  Either<
    ASTSingleVariant<
        Tuple1Mutator<char, CharWithinRangeMutator>,
        Tuple1Mutator<Vec<AST>, Either<FixedLenVecMutator<AST, ASTMutator>, VecMutator<AST, ASTMutator>>>,
        Tuple1Mutator<
            Box<AST>,
            BoxMutator<
                AST,
                Either<Either<ASTMutator, RecurToMutator<ASTMutator>>, AlternationMutator<AST, ASTMutator>>,
            >,
        >,
    >,
    RecursiveMutator<ASTMutator>,
>
}
*/

type InnerASTMutator = Either<
    ASTSingleVariant<
        Tuple1Mutator<CharWithinRangeMutator>,
        Tuple1Mutator<Either<FixedLenVecMutator<AST, ASTMutator>, VecMutator<AST, ASTMutator>>>,
        Tuple1Mutator<
            BoxMutator<Either<Either<ASTMutator, RecurToMutator<ASTMutator>>, AlternationMutator<AST, ASTMutator>>>,
        >,
    >,
    RecursiveMutator<ASTMutator>,
>;

pub struct ASTMutator {
    pub inner: Box<InnerASTMutator>,
}
pub struct ASTMutatorCache {
    pub inner: Box<<InnerASTMutator as Mutator<AST>>::Cache>,
}
impl ASTMutatorCache {
    #[no_coverage]
    fn new(inner: <InnerASTMutator as Mutator<AST>>::Cache) -> Self {
        Self { inner: Box::new(inner) }
    }
}
pub struct ASTMutatorMutationStep {
    pub inner: Box<<InnerASTMutator as Mutator<AST>>::MutationStep>,
}
impl ASTMutatorMutationStep {
    #[no_coverage]
    fn new(inner: <InnerASTMutator as Mutator<AST>>::MutationStep) -> Self {
        Self { inner: Box::new(inner) }
    }
}
pub struct ASTMutatorArbitraryStep {
    pub inner: Box<<InnerASTMutator as Mutator<AST>>::ArbitraryStep>,
}
pub struct ASTMutatorUnmutateToken {
    pub inner: Box<<InnerASTMutator as Mutator<AST>>::UnmutateToken>,
}
impl ASTMutatorUnmutateToken {
    #[no_coverage]
    fn new(inner: <InnerASTMutator as Mutator<AST>>::UnmutateToken) -> Self {
        Self { inner: Box::new(inner) }
    }
}
impl Mutator<AST> for ASTMutator {
    type Cache = ASTMutatorCache;
    type MutationStep = ASTMutatorMutationStep;
    type ArbitraryStep = ASTMutatorArbitraryStep;
    type UnmutateToken = ASTMutatorUnmutateToken;

    #[no_coverage]
    fn default_arbitrary_step(&self) -> Self::ArbitraryStep {
        Self::ArbitraryStep {
            inner: Box::new(self.inner.default_arbitrary_step()),
        }
    }

    #[no_coverage]
    fn validate_value(&self, value: &AST) -> Option<(Self::Cache, Self::MutationStep)> {
        let (cache, step) = self.inner.validate_value(value)?;
        Some((Self::Cache::new(cache), Self::MutationStep::new(step)))
    }

    #[no_coverage]
    fn max_complexity(&self) -> f64 {
        self.inner.max_complexity()
    }

    #[no_coverage]
    fn min_complexity(&self) -> f64 {
        self.inner.min_complexity()
    }

    #[no_coverage]
    fn complexity(&self, value: &AST, cache: &Self::Cache) -> f64 {
        self.inner.complexity(value, &cache.inner)
    }

    #[no_coverage]
    fn ordered_arbitrary(&self, step: &mut Self::ArbitraryStep, max_cplx: f64) -> Option<(AST, f64)> {
        self.inner.ordered_arbitrary(&mut step.inner, max_cplx)
    }

    #[no_coverage]
    fn random_arbitrary(&self, max_cplx: f64) -> (AST, f64) {
        self.inner.random_arbitrary(max_cplx)
    }

    #[no_coverage]
    fn ordered_mutate(
        &self,
        value: &mut AST,
        cache: &mut Self::Cache,
        step: &mut Self::MutationStep,
        max_cplx: f64,
    ) -> Option<(Self::UnmutateToken, f64)> {
        let (token, cplx) = self
            .inner
            .ordered_mutate(value, &mut cache.inner, &mut step.inner, max_cplx)?;
        Some((Self::UnmutateToken::new(token), cplx))
    }

    #[no_coverage]
    fn random_mutate(&self, value: &mut AST, cache: &mut Self::Cache, max_cplx: f64) -> (Self::UnmutateToken, f64) {
        let (token, cplx) = self.inner.random_mutate(value, &mut cache.inner, max_cplx);
        (Self::UnmutateToken::new(token), cplx)
    }

    #[no_coverage]
    fn unmutate(&self, value: &mut AST, cache: &mut Self::Cache, t: Self::UnmutateToken) {
        self.inner.unmutate(value, &mut cache.inner, *t.inner)
    }
}

#[no_coverage]
pub fn grammar_based_string_mutator(grammar: Rc<Grammar>) -> impl Mutator<String> {
    let grammar_cloned = grammar.clone();
    let parse = move |string: &String| parse_from_grammar(string, grammar_cloned.clone());
    IncrementalMapMutator::<AST, String, ASTMutator, ASTMap, _>::new(
        #[no_coverage]
        parse,
        ASTMutator::from_grammar(grammar),
    )
}

impl ASTMutator {
    #[no_coverage]
    fn token(m: CharWithinRangeMutator) -> Self {
        Self {
            inner: Box::new(Either::Left(ASTSingleVariant::Token(Tuple1Mutator::new(m)))),
        }
    }
    #[no_coverage]
    fn sequence(m: Either<FixedLenVecMutator<AST, ASTMutator>, VecMutator<AST, ASTMutator>>) -> Self {
        Self {
            inner: Box::new(Either::Left(ASTSingleVariant::Sequence(Tuple1Mutator::new(m)))),
        }
    }
    #[no_coverage]
    fn alternation(m: AlternationMutator<AST, ASTMutator>) -> Self {
        Self {
            inner: Box::new(Either::Left(ASTSingleVariant::Box(Tuple1Mutator::new(
                BoxMutator::new(Either::Right(m)),
            )))),
        }
    }
    #[no_coverage]
    fn recur(m: RecurToMutator<ASTMutator>) -> Self {
        Self {
            inner: Box::new(Either::Left(ASTSingleVariant::Box(Tuple1Mutator::new(
                BoxMutator::new(Either::Left(Either::Right(m))),
            )))),
        }
    }
    #[no_coverage]
    fn recursive(m: impl FnMut(&Weak<Self>) -> Self) -> Self {
        Self {
            inner: Box::new(Either::Right(RecursiveMutator::new(m))),
        }
    }

    #[no_coverage]
    pub fn from_grammar(grammar: Rc<Grammar>) -> Self {
        let mut others = HashMap::new();
        Self::from_grammar_rec(grammar, &mut others)
    }

    #[no_coverage]
    pub fn from_grammar_rec(grammar: Rc<Grammar>, others: &mut HashMap<*const Grammar, Weak<ASTMutator>>) -> Self {
        match grammar.as_ref() {
            Grammar::Literal(l) => Self::token(CharWithinRangeMutator::new(l.clone())),
            Grammar::Alternation(gs) => Self::alternation(AlternationMutator::new(
                gs.iter().map(|g| Self::from_grammar_rec(g.clone(), others)).collect(),
            )),
            Grammar::Concatenation(gs) => {
                let mut ms = Vec::<ASTMutator>::new();
                for g in gs {
                    let m = Self::from_grammar_rec(g.clone(), others);
                    ms.push(m);
                }
                Self::sequence(Either::Left(FixedLenVecMutator::new(ms)))
            }
            Grammar::Repetition(g, range) => Self::sequence(Either::Right(VecMutator::new(
                Self::from_grammar_rec(g.clone(), others),
                range.start..=range.end - 1,
            ))),
            Grammar::Recurse(g) => {
                if let Some(m) = others.get(&g.as_ptr()) {
                    Self::recur(RecurToMutator::from(m))
                } else {
                    panic!()
                }
            }
            Grammar::Recursive(g) => Self::recursive(|m| {
                let weak_g = Rc::downgrade(g);
                others.insert(weak_g.as_ptr(), m.clone());
                Self::from_grammar_rec(g.clone(), others)
            }),
        }
    }
}
