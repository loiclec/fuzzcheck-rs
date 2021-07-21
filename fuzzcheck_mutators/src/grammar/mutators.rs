extern crate self as fuzzcheck_mutators;

use super::grammar::InnerGrammar;
use crate::either::Either;
use crate::fuzzcheck_traits::Mutator;

use crate::{alternation::AlternationMutator, boxed::BoxMutator, tuples::Tuple1Mutator};
use crate::{fixed_len_vector::FixedLenVecMutator, integer::CharWithinRangeMutator, make_mutator, vector::VecMutator};

use super::grammar::Grammar;
use super::mapping::IncrementalMapping;
use crate::grammar::ast::{ASTMapping, AST};

// we don't use ASTMutator__, but we do use ASTSingleVariant and its Mutator conformance
make_mutator! {
    name: ASTMutator__,
    recursive: false,
    default: false,
    type: pub enum AST {
        Token(char),
        Sequence(Vec<AST>),
        Box(Box<AST>),
    }
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
    fn token(m: CharWithinRangeMutator) -> Self {
        Self {
            inner: Box::new(ASTSingleVariant::Token(Tuple1Mutator::new(m))),
        }
    }
    fn sequence(m: Either<FixedLenVecMutator<AST, ASTMutator>, VecMutator<AST, ASTMutator>>) -> Self {
        Self {
            inner: Box::new(ASTSingleVariant::Sequence(Tuple1Mutator::new(m))),
        }
    }
    fn boxed(m: Either<ASTMutator, AlternationMutator<AST, ASTMutator>>) -> Self {
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
    use crate::grammar::grammar::Grammar;
    use crate::grammar::mutators::GrammarBasedStringMutator;
    use crate::{concatenation, literal, repetition};
    use fuzzcheck_traits::Mutator;

    #[test]
    fn test_a10() {
        // let grammar = concatenation! {
        //     alternation! {
        //         literal!('a' ..= 'z'),
        //         literal!('0' ..= '9')
        //     },
        //     literal!('a' ..= 'z'),
        //     alternation! {
        //         concatenation! {
        //             alternation! {
        //                 literal!('a'..='z'),
        //                 literal!('0'..='9')
        //             },
        //             literal!('a' ..= 'z')
        //         },
        //         literal!('a' ..= 'c')
        //     }
        // };
        // let grammar = repetition!(literal!('a'..='z'), 5..10);
        let grammar = concatenation! {
            literal!('a' ..= 'z'),
            repetition! {
                literal!(('a'..='z'), ('0'..='9')),
                5..=10
            },
            repetition! {
                literal!('0'..='9'),
                2 ..= 6
            },
            literal!('z')
        };

        let mutator = GrammarBasedStringMutator::new(grammar);

        let mut value = "a25y3c03z".to_owned();
        let (mut cache, mut step) = mutator.validate_value(&value).unwrap();
        for _ in 0..10 {
            let (t, cplx) = mutator
                .ordered_mutate(&mut value, &mut cache, &mut step, 1000.)
                .unwrap();
            println!("{} {}", value, cplx);

            mutator.unmutate(&mut value, &mut cache, t);
            // println!("{}", value);
        }

        // for _ in 0..10 {
        //     let (mut value, _cplx) = mutator.random_arbitrary(1000.0);
        //     println!("{}", value);
        //     if let Some((mut cache, mut step)) = mutator.validate_value(&value) {
        //         println!("{:?}", value);
        //         let original_value = value.clone();
        //         let original_ast = cache.ast.clone();
        //         let original_mapping = cache.mapping.clone();
        //         for _ in 0..10_000 {
        //             if let Some((t, _cplx)) = mutator.ordered_mutate(&mut value, &mut cache, &mut step, 1000.) {
        //                 // println!("{:?}", cache.ast);
        //                 println!("{:?}", value);
        //                 assert!(mutator.validate_value(&value).is_some());
        //                 mutator.unmutate(&mut value, &mut cache, t);
        //                 assert_eq!(original_value, value);
        //                 assert_eq!(original_ast, cache.ast);
        //                 assert_eq!(original_mapping, cache.mapping);
        //             } else {
        //                 panic!("exhausted");
        //             }
        //         }
        //     } else {
        //         println!("value is empty? {}", value.is_empty());
        //         panic!("could not parse {}", value);
        //     }
        // }
    }
}
