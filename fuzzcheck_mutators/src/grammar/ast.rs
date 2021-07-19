extern crate self as fuzzcheck_mutators;

use super::grammar::InnerGrammar;
use crate::either::Either;
use crate::fuzzcheck_traits::Mutator;

use crate::{make_mutator, CharWithinRangeMutator, FixedLenVecMutator, VecMutator};
use crate::{AlternationMutator, BoxMutator, Tuple1Mutator};

use super::grammar::Grammar;
use super::mapping::IncrementalMapping;

/// An abstract syntax tree.
#[derive(Clone, Debug)]
pub enum AST {
    Token(char),
    Sequence(Vec<AST>),
    Box(Box<AST>),
}

// we don't use ASTMutator__, but we do use ASTSingleVariant and its Mutator conformance
#[make_mutator(name: ASTMutator__, recursive: false, default: false)]
pub enum AST {
    Token(char),
    Sequence(Vec<AST>),
    Box(Box<AST>),
}

type InnerASTMutator = ASTSingleVariant<
    Tuple1Mutator<char, CharWithinRangeMutator>,
    Tuple1Mutator<Vec<AST>, Either<FixedLenVecMutator<AST, ASTMutator>, VecMutator<AST, ASTMutator>>>,
    Tuple1Mutator<Box<AST>, BoxMutator<AST, Either<ASTMutator, AlternationMutator<AST, ASTMutator>>>>,
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

impl AST {
    pub fn generate_string_in(&self, s: &mut String, start_index: &mut usize) -> ASTMapping {
        match self {
            AST::Token(c) => {
                let len = c.len_utf8();
                let orig_start_index = *start_index;
                s.push(*c);
                *start_index += len;
                ASTMapping {
                    start_index: orig_start_index,
                    len,
                    content: ASTMappingKind::Token,
                }
            }
            AST::Sequence(asts) => {
                let original_start_idx = *start_index;
                let mut cs = vec![];
                for ast in asts {
                    let c = ast.generate_string_in(s, start_index);
                    cs.push(c);
                }
                ASTMapping {
                    start_index: original_start_idx,
                    len: *start_index - original_start_idx,
                    content: ASTMappingKind::Sequence(cs),
                }
            }
            AST::Box(ast) => {
                let mapping = ast.generate_string_in(s, start_index);
                ASTMapping {
                    start_index: mapping.start_index,
                    len: mapping.len,
                    content: ASTMappingKind::Box(Box::new(mapping)),
                }
            }
        }
    }
    pub fn generate_string(&self) -> (String, ASTMapping) {
        let mut s = String::new();
        let mut start_index = 0;
        let c = self.generate_string_in(&mut s, &mut start_index);
        (s, c)
    }
    pub fn generate_string_starting_at_idx(&self, idx: usize) -> (String, ASTMapping) {
        let mut s = String::new();
        let mut start_index = idx;
        let c = self.generate_string_in(&mut s, &mut start_index);
        (s, c)
    }
}

/// Like an abstract syntax tree, but augmented with the string indices that correspond to each node
#[derive(Debug)]
pub struct ASTMapping {
    pub start_index: usize,
    pub len: usize,
    pub content: ASTMappingKind,
}
#[derive(Debug)]
pub enum ASTMappingKind {
    Token,
    Sequence(Vec<ASTMapping>),
    Box(Box<ASTMapping>),
}

pub struct GrammarBasedStringMutator {
    grammar: Grammar,
    ast_mutator: ASTMutator,
}
impl GrammarBasedStringMutator {
    pub fn new(grammar: Grammar) -> Self {
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
    pub fn token(m: CharWithinRangeMutator) -> Self {
        Self {
            inner: Box::new(ASTSingleVariant::Token(Tuple1Mutator::new(m))),
        }
    }
    pub fn sequence(m: Either<FixedLenVecMutator<AST, ASTMutator>, VecMutator<AST, ASTMutator>>) -> Self {
        Self {
            inner: Box::new(ASTSingleVariant::Sequence(Tuple1Mutator::new(m))),
        }
    }
    pub fn boxed(m: Either<ASTMutator, AlternationMutator<AST, ASTMutator>>) -> Self {
        Self {
            inner: Box::new(ASTSingleVariant::Box(Tuple1Mutator::new(BoxMutator::new(m)))),
        }
    }

    pub fn from_grammar(grammar: Grammar) -> Self {
        match grammar.grammar.as_ref() {
            InnerGrammar::Literal(l) => Self::token(CharWithinRangeMutator::new(l.clone())),
            InnerGrammar::Alternation(gs) => Self::boxed(Either::Right(AlternationMutator::new(
                gs.iter().map(|g| Self::from_grammar(g.clone())).collect(),
            ))),
            InnerGrammar::Concatenation(gs) => Self::sequence(Either::Left(FixedLenVecMutator::new(
                gs.iter().map(|g| Self::from_grammar(g.clone())).collect(),
            ))),
            InnerGrammar::Repetition(g, range) => Self::sequence(Either::Right(VecMutator::new(
                Self::from_grammar(g.clone()),
                range.start..=range.end - 1,
            ))),
            InnerGrammar::Shared(g) => Self::from_grammar(g.as_ref().clone()),
            InnerGrammar::Recurse(_) => todo!(),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::grammar::ast::GrammarBasedStringMutator;
    use crate::grammar::grammar::Grammar;
    use crate::{alternation, concatenation, literal};
    use fuzzcheck_traits::Mutator;

    #[test]
    fn test_a10() {
        let grammar = concatenation! {
            literal!('a'..='z', '0'..='9'),
            literal!('a' ..= 'z'),
            alternation! {
                concatenation! {
                    literal!('a'..='z', '0'..='9'),
                    literal!('a' ..= 'z')
                },
                literal!('a' ..= 'c')
            }
        };

        let mutator = GrammarBasedStringMutator::new(grammar);

        for _ in 0..10 {
            let (mut value, _cplx) = mutator.random_arbitrary(1000.0);
            println!("{}", value);
            if let Some((mut cache, mut step)) = mutator.validate_value(&value) {
                println!("{:?}", value);
                let original = value.clone();
                for _ in 0..10 {
                    if let Some((t, _cplx)) = mutator.ordered_mutate(&mut value, &mut cache, &mut step, 1000.) {
                        println!("{:?}", value);
                        let _x = mutator.validate_value(&value).unwrap();
                        mutator.unmutate(&mut value, &mut cache, t);
                        assert_eq!(original, value);
                    } else {
                        panic!("exhausted");
                    }
                }
            } else {
                panic!();
            }
        }
    }
}
