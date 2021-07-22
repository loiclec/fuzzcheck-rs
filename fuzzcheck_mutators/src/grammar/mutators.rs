extern crate self as fuzzcheck_mutators;

use std::collections::HashMap;
use std::rc::{Rc, Weak};

use fuzzcheck_mutators_derive::make_single_variant_mutator;

use crate::either::Either;
use crate::fuzzcheck_traits::Mutator;

use crate::recursive::{RecurToMutator, RecursiveMutator};
use crate::{alternation::AlternationMutator, boxed::BoxMutator, tuples::Tuple1Mutator};
use crate::{fixed_len_vector::FixedLenVecMutator, integer::CharWithinRangeMutator, vector::VecMutator};

use super::grammar::Grammar;
use super::mapping::IncrementalMapping;
use crate::grammar::ast::{ASTMapping, AST};

make_single_variant_mutator! {
    pub enum AST {
        Token(char),
        Sequence(Vec<AST>),
        Box(Box<AST>),
    }
}

type InnerASTMutator = Either<
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
>;

pub struct ASTMutator {
    pub inner: Box<InnerASTMutator>,
}
pub struct ASTMutatorCache {
    pub inner: Box<<InnerASTMutator as Mutator<AST>>::Cache>,
}
impl ASTMutatorCache {
    fn new(inner: <InnerASTMutator as Mutator<AST>>::Cache) -> Self {
        Self { inner: Box::new(inner) }
    }
}
pub struct ASTMutatorMutationStep {
    pub inner: Box<<InnerASTMutator as Mutator<AST>>::MutationStep>,
}
impl ASTMutatorMutationStep {
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
    fn new(inner: <InnerASTMutator as Mutator<AST>>::UnmutateToken) -> Self {
        Self { inner: Box::new(inner) }
    }
}
impl Mutator<AST> for ASTMutator {
    type Cache = ASTMutatorCache;
    type MutationStep = ASTMutatorMutationStep;
    type ArbitraryStep = ASTMutatorArbitraryStep;
    type UnmutateToken = ASTMutatorUnmutateToken;

    fn default_arbitrary_step(&self) -> Self::ArbitraryStep {
        Self::ArbitraryStep {
            inner: Box::new(self.inner.default_arbitrary_step()),
        }
    }

    fn validate_value(&self, value: &AST) -> Option<(Self::Cache, Self::MutationStep)> {
        let (cache, step) = self.inner.validate_value(value)?;
        Some((Self::Cache::new(cache), Self::MutationStep::new(step)))
    }

    fn max_complexity(&self) -> f64 {
        self.inner.max_complexity()
    }

    fn min_complexity(&self) -> f64 {
        self.inner.min_complexity()
    }

    fn complexity(&self, value: &AST, cache: &Self::Cache) -> f64 {
        self.inner.complexity(value, &cache.inner)
    }

    fn ordered_arbitrary(&self, step: &mut Self::ArbitraryStep, max_cplx: f64) -> Option<(AST, f64)> {
        self.inner.ordered_arbitrary(&mut step.inner, max_cplx)
    }

    fn random_arbitrary(&self, max_cplx: f64) -> (AST, f64) {
        self.inner.random_arbitrary(max_cplx)
    }

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

    fn random_mutate(&self, value: &mut AST, cache: &mut Self::Cache, max_cplx: f64) -> (Self::UnmutateToken, f64) {
        let (token, cplx) = self.inner.random_mutate(value, &mut cache.inner, max_cplx);
        (Self::UnmutateToken::new(token), cplx)
    }

    fn unmutate(&self, value: &mut AST, cache: &mut Self::Cache, t: Self::UnmutateToken) {
        self.inner.unmutate(value, &mut cache.inner, *t.inner)
    }
}

pub struct GrammarBasedStringMutator {
    grammar: Rc<Grammar>,
    ast_mutator: ASTMutator,
}
impl GrammarBasedStringMutator {
    pub fn new(grammar: Rc<Grammar>) -> Self {
        Self {
            grammar: grammar.clone(),
            ast_mutator: ASTMutator::from_grammar(grammar),
        }
    }
}
pub struct Cache {
    ast: AST,
    ast_mutator_cache: <ASTMutator as Mutator<AST>>::Cache,
    mapping: ASTMapping,
}

impl Mutator<String> for GrammarBasedStringMutator {
    type Cache = Cache;
    type MutationStep = <ASTMutator as Mutator<AST>>::MutationStep;
    type ArbitraryStep = <ASTMutator as Mutator<AST>>::ArbitraryStep;
    type UnmutateToken = <ASTMutator as Mutator<AST>>::UnmutateToken;

    fn default_arbitrary_step(&self) -> Self::ArbitraryStep {
        self.ast_mutator.default_arbitrary_step()
    }

    fn validate_value(&self, value: &String) -> Option<(Self::Cache, Self::MutationStep)> {
        let ast = crate::grammar::parser::parse_from_grammar(value, self.grammar.clone())?;
        let (ast_mutator_cache, mutation_step) = self.ast_mutator.validate_value(&ast).unwrap();
        let (_, mapping) = ast.generate_string();
        let cache = Cache {
            ast,
            ast_mutator_cache,
            mapping,
        };
        Some((cache, mutation_step))
    }

    fn max_complexity(&self) -> f64 {
        self.ast_mutator.max_complexity()
    }

    fn min_complexity(&self) -> f64 {
        self.ast_mutator.min_complexity()
    }

    fn complexity(&self, _value: &String, cache: &Self::Cache) -> f64 {
        self.ast_mutator.complexity(&cache.ast, &cache.ast_mutator_cache)
    }

    fn ordered_arbitrary(&self, step: &mut Self::ArbitraryStep, max_cplx: f64) -> Option<(String, f64)> {
        let (value, cplx) = self.ast_mutator.ordered_arbitrary(step, max_cplx)?;
        let (x, _) = value.generate_string();
        Some((x, cplx))
    }

    fn random_arbitrary(&self, max_cplx: f64) -> (String, f64) {
        let (value, cplx) = self.ast_mutator.random_arbitrary(max_cplx);
        let (x, _) = value.generate_string();
        (x, cplx)
    }

    fn ordered_mutate(
        &self,
        value: &mut String,
        cache: &mut Self::Cache,
        step: &mut Self::MutationStep,
        max_cplx: f64,
    ) -> Option<(Self::UnmutateToken, f64)> {
        let (token, cplx) =
            self.ast_mutator
                .ordered_mutate(&mut cache.ast, &mut cache.ast_mutator_cache, step, max_cplx)?;
        <ASTMapping as IncrementalMapping<AST, String, ASTMutator>>::mutate_value_from_token(
            &mut cache.mapping,
            &cache.ast,
            value,
            &token,
        );
        Some((token, cplx))
    }

    fn random_mutate(&self, value: &mut String, cache: &mut Self::Cache, max_cplx: f64) -> (Self::UnmutateToken, f64) {
        let (token, cplx) = self
            .ast_mutator
            .random_mutate(&mut cache.ast, &mut cache.ast_mutator_cache, max_cplx);
        <ASTMapping as IncrementalMapping<AST, String, ASTMutator>>::mutate_value_from_token(
            &mut cache.mapping,
            &cache.ast,
            value,
            &token,
        );
        (token, cplx)
    }

    fn unmutate(&self, value: &mut String, cache: &mut Self::Cache, t: Self::UnmutateToken) {
        <ASTMapping as IncrementalMapping<AST, String, ASTMutator>>::unmutate_value_from_token(
            &mut cache.mapping,
            value,
            &t,
        );

        self.ast_mutator
            .unmutate(&mut cache.ast, &mut cache.ast_mutator_cache, t);
    }
}

impl ASTMutator {
    fn token(m: CharWithinRangeMutator) -> Self {
        Self {
            inner: Box::new(Either::Left(ASTSingleVariant::Token(Tuple1Mutator::new(m)))),
        }
    }
    fn sequence(m: Either<FixedLenVecMutator<AST, ASTMutator>, VecMutator<AST, ASTMutator>>) -> Self {
        Self {
            inner: Box::new(Either::Left(ASTSingleVariant::Sequence(Tuple1Mutator::new(m)))),
        }
    }
    fn alternation(m: AlternationMutator<AST, ASTMutator>) -> Self {
        Self {
            inner: Box::new(Either::Left(ASTSingleVariant::Box(Tuple1Mutator::new(
                BoxMutator::new(Either::Right(m)),
            )))),
        }
    }
    // fn boxed(m: ASTMutator) -> Self {
    //     Self {
    //         inner: Box::new(Either::Left(ASTSingleVariant::Box(Tuple1Mutator::new(
    //             BoxMutator::new(Either::Left(Either::Left(m))),
    //         )))),
    //     }
    // }
    fn recur(m: RecurToMutator<ASTMutator>) -> Self {
        Self {
            inner: Box::new(Either::Left(ASTSingleVariant::Box(Tuple1Mutator::new(
                BoxMutator::new(Either::Left(Either::Right(m))),
            )))),
        }
    }
    fn recursive(m: impl FnMut(&Weak<Self>) -> Self) -> Self {
        Self {
            inner: Box::new(Either::Right(RecursiveMutator::new(m))),
        }
    }

    pub fn from_grammar(grammar: Rc<Grammar>) -> Self {
        let mut others = HashMap::new();
        Self::from_grammar_rec(grammar, &mut others)
    }

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
                return Self::sequence(Either::Left(FixedLenVecMutator::new(ms)));
            }
            Grammar::Repetition(g, range) => Self::sequence(Either::Right(VecMutator::new(
                Self::from_grammar_rec(g.clone(), others),
                range.start..=range.end - 1,
            ))),
            Grammar::Recurse(g) => {
                if let Some(m) = others.get(&g.as_ptr()) {
                    return Self::recur(RecurToMutator::from(m));
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
