
extern crate self as fuzzcheck_mutators;

use crate::fuzzcheck_traits;

use fuzzcheck_mutators::{TupleStructure, VecMutator};
use fuzzcheck_traits::RecurToMutator;
use fuzzcheck_mutators::DefaultMutator;
use fuzzcheck_mutators::{AlternationMutator, CharWithinRangeMutator};

#[derive(Clone, DefaultMutator)]
pub enum ChoiceIdents {
    Struct,
    Enum,
    Fn,
    Mut,
    Union,
    A,
    B,
    X,
    Y,
    Foo,
    Bar,
    Pub,
    Crate,
    Derive,
    Other,
}

#[derive(Clone, DefaultMutator)]
pub struct LiteralSource;

#[derive(Clone, DefaultMutator)]
pub enum SpacingSource {
    Alone, Joint
}

#[derive(Clone)]
pub struct PunctSource {
    pub c: char,
    pub spacing: SpacingSource,
}

#[fuzzcheck_mutators::make_mutator(name: PunctSourceMutator, default: true)]
pub struct PunctSource {
    #[field_mutator(AlternationMutator<char, CharWithinRangeMutator> = { 
        AlternationMutator::new(
            vec![
                CharWithinRangeMutator::new('#' ..= '\''), 
                CharWithinRangeMutator::new('*' ..= '/'), 
                CharWithinRangeMutator::new(':' ..= '?'), 
                CharWithinRangeMutator::new('\\' ..= '\\'), 
                CharWithinRangeMutator::new('_' ..= '_'), 
                CharWithinRangeMutator::new('|' ..= '|')
                ]
            )
    })]
    c: char,
    spacing: SpacingSource,
}

#[derive(Clone, DefaultMutator)]
pub enum DelimiterSource {
    Parenthesis,
    Brace,
    Bracket,
    None,
}

#[derive(Clone)]
pub struct GroupSource {
    pub delimiter: DelimiterSource,
    pub stream: TokenStreamSource,
}

#[fuzzcheck_mutators::make_mutator(name: GroupSourceMutator, default: false)]
pub struct GroupSource {
    #[field_mutator(<DelimiterSource as DefaultMutator>::Mutator)]
    pub delimiter: DelimiterSource,
    pub stream: TokenStreamSource,
}

#[derive(Clone)]
pub enum TokenTreeSource {
    Group(GroupSource),
    Ident(ChoiceIdents),
    Punct(PunctSource),
    Literal(LiteralSource),
}

#[fuzzcheck_mutators::make_mutator(name: TokenTreeSourceMutator, default: false)]
pub enum TokenTreeSource {
    Group(GroupSource),
    Ident(#[field_mutator(<ChoiceIdents as DefaultMutator>::Mutator)] ChoiceIdents),
    Punct(#[field_mutator(<PunctSource as DefaultMutator>::Mutator )] PunctSource),
    Literal(#[field_mutator(<LiteralSource as DefaultMutator>::Mutator )] LiteralSource),
}

#[derive(Clone)]
pub struct TokenStreamSource {
    pub tokens: Vec<TokenTreeSource>,
}

fn dictionary() -> Vec<Vec<TokenTreeSource>> {
    vec![
        vec![TokenTreeSource::Punct(PunctSource {
            c: ':',
            spacing: SpacingSource::Joint
        }), TokenTreeSource::Punct(PunctSource {
            c: ':',
            spacing: SpacingSource::Alone
        })],
        vec![TokenTreeSource::Punct(PunctSource {
            c: '-',
            spacing: SpacingSource::Joint
        }), TokenTreeSource::Punct(PunctSource {
            c: '>',
            spacing: SpacingSource::Alone
        })],
        vec![TokenTreeSource::Punct(PunctSource {
            c: '<',
            spacing: SpacingSource::Alone
        }), TokenTreeSource::Punct(PunctSource {
            c: '>',
            spacing: SpacingSource::Alone
        })],
        vec![TokenTreeSource::Punct(PunctSource {
            c: '\'',
            spacing: SpacingSource::Joint
        }), TokenTreeSource::Punct(PunctSource {
            c: '_',
            spacing: SpacingSource::Alone
        })],
        vec![TokenTreeSource::Punct(PunctSource {
            c: '=',
            spacing: SpacingSource::Joint
        }), TokenTreeSource::Punct(PunctSource {
            c: '>',
            spacing: SpacingSource::Alone
        })],
        vec![TokenTreeSource::Punct(PunctSource {
            c: '.',
            spacing: SpacingSource::Joint
        }), TokenTreeSource::Punct(PunctSource {
            c: '.',
            spacing: SpacingSource::Joint
        }), TokenTreeSource::Punct(PunctSource {
            c: '.',
            spacing: SpacingSource::Alone
        })],
        vec![TokenTreeSource::Punct(PunctSource {
            c: '-',
            spacing: SpacingSource::Alone
        }), TokenTreeSource::Group(GroupSource {
            delimiter: DelimiterSource::Bracket,
            stream: TokenStreamSource { tokens: vec![] }
        })],
        vec![TokenTreeSource::Ident(ChoiceIdents::Foo), TokenTreeSource::Group(GroupSource {
            delimiter: DelimiterSource::Parenthesis,
            stream: TokenStreamSource { tokens: vec![] }
        })]
    ]
}

#[fuzzcheck_mutators::make_mutator(name: TokenStreamSourceMutator, default: true, recursive: true)]
pub struct TokenStreamSource {
    #[field_mutator(VecMutator<TokenTreeSource,
    TokenTreeSourceMutator<
        GroupSourceMutator<
            RecurToMutator< TokenStreamSourceMutator >
        >,
    >> = { 
        println!("making vec of token tree mutators");
        let x = VecMutator::new_with_dict(
        TokenTreeSourceMutator::new(
            GroupSourceMutator::new(crate::BasicEnumMutator::new::<DelimiterSource>(), self_.into()),
            ChoiceIdents::default_mutator(),PunctSource::default_mutator(),LiteralSource::default_mutator(),
        ), 0 ..= 100, dictionary()
    );
    println!("done making vec of token tree mutators");

        x
    }
)]
    pub tokens: Vec<TokenTreeSource>,
}

// fn foo<M: crate::fuzzcheck_traits::Mutator<TokenStreamSource>>(m: M) {

// }
// fn bar() {
//     let x = TokenStreamSource::default_mutator();
//     foo(x)
// }
