use std::str::FromStr;

use proc_macro2::{Delimiter, Group, Ident, Literal, TokenStream, TokenTree};
use quote::ToTokens;
use syn::{parse2, AngleBracketedGenericArguments, Field, GenericArgument, Generics, Token, Variant};

pub struct TokenBuilder {
    pub groups: Vec<(Delimiter, TokenStream)>,
}

impl Default for TokenBuilder {
    fn default() -> Self {
        Self {
            groups: vec![(Delimiter::None, TokenStream::new())],
        }
    }
}

impl TokenBuilder {
    pub fn finish(mut self) -> TokenStream {
        if self.groups.len() != 1 {
            for (delimiter, stream) in self.groups.iter() {
                eprintln!("Delimiter: {:?}, Stream: {}", delimiter, stream);
            }
            panic!("Groups not empty, you missed a pop_group.")
        }
        self.groups.pop().unwrap().1
    }

    pub fn extend_tree<T: Into<TokenTree>>(&mut self, tt: T) {
        self.groups.last_mut().unwrap().1.extend(Some(tt.into()));
    }

    pub fn stream(&mut self, what: TokenStream) {
        for c in what.into_iter() {
            self.extend_tree(c);
        }
    }

    pub fn add(&mut self, what: &str) {
        fn is_delimiter(c: char) -> bool {
            matches!(c, '{' | '(' | '[' | '}' | ')' | ']')
        }

        let delimiter_matches = what.matches(is_delimiter);
        let mut between_delimiter_matches = what.split(is_delimiter);

        if let Some(first_match_no_delimiter) = between_delimiter_matches.next() {
            let ts = if first_match_no_delimiter.trim().is_empty() {
                TokenStream::new()
            } else {
                proc_macro2::TokenStream::from_str(first_match_no_delimiter).unwrap_or_else(|_| {
                    panic!(
                        "Could not parse the following string into a token stream: {}",
                        first_match_no_delimiter
                    )
                })
            };
            self.stream(ts);
        }

        for (delimiter, ts) in delimiter_matches.zip(between_delimiter_matches) {
            match delimiter {
                "{" => self.push_group(Delimiter::Brace),
                "(" => self.push_group(Delimiter::Parenthesis),
                "[" => self.push_group(Delimiter::Bracket),
                "}" => self.pop_group(Delimiter::Brace),
                ")" => self.pop_group(Delimiter::Parenthesis),
                "]" => self.pop_group(Delimiter::Bracket),
                _ => unreachable!(),
            }
            let ts = if ts.trim().is_empty() {
                TokenStream::new()
            } else {
                proc_macro2::TokenStream::from_str(ts)
                    .unwrap_or_else(|_| panic!("Could not parse the following string into a token stream: {}", ts))
            };

            self.stream(ts);
        }
    }

    pub fn push_group(&mut self, delim: Delimiter) {
        self.groups.push((delim, TokenStream::new()));
    }

    pub fn pop_group(&mut self, delim: Delimiter) {
        if self.groups.len() < 2 {
            panic!("pop_group stack is empty {}", self.groups.len());
        }
        let ts = self.groups.pop().unwrap();
        if ts.0 != delim {
            panic!("pop_group Delimiter mismatch, got {:?} expected {:?}", ts.0, delim);
        }
        self.extend_tree(TokenTree::from(Group::new(delim, ts.1)));
    }
}

macro_rules! join_ts {
    ($iter:expr) => {
        {
            #[allow(unused_mut)]
            let mut tb = $crate::token_builder::TokenBuilder::default();
            for part in $iter {
                 $crate::token_builder::ExtendTokenBuilder::add(&$parts, &mut tb);
            }
            tb.finish()
        }
    };
    ($iter:expr, separator: $sep:expr) => {
        {
            #[allow(unused_mut)]
            let mut tb = $crate::token_builder::TokenBuilder::default();
            let mut add_sep = false;
            for part in $iter {
                if add_sep {
                    $sep.add_to(&mut tb);
                }
                $crate::token_builder::ExtendTokenBuilder::add(&$parts, &mut tb);
                add_sep = true;
            }
            tb.finish()
        }
    };
    ($iter:expr, $part_pat:pat, $($parts:expr) *) => {
        {
            #[allow(unused_mut)]
            let mut tb = $crate::token_builder::TokenBuilder::default();
            for $part_pat in $iter {
		    	$(
                    $crate::token_builder::ExtendTokenBuilder::add(&$parts, &mut tb);
                )*
            }
            tb.finish()
        }
    };
    ($iter:expr, $part_pat:pat, $($parts:expr) *, separator: $sep:expr) => {
        {
            #[allow(unused_mut)]
            let mut tb = $crate::token_builder::TokenBuilder::default();
            let mut add_sep = false;
            for $part_pat in $iter {
                if add_sep {
                    tb.add($sep);
                }
                {
			    	$(
                        $crate::token_builder::ExtendTokenBuilder::add(&$parts, &mut tb);
                    )*
			    }
                add_sep = true;
            }
            tb.finish()
        }
    };
}

pub trait ExtendTokenBuilder {
    fn add(&self, tb: &mut TokenBuilder);
}

pub struct Quoted<'a, T>(pub &'a T);

impl<'a, T> ExtendTokenBuilder for Quoted<'a, T>
where
    T: ToTokens,
{
    fn add(&self, tb: &mut TokenBuilder) {
        self.0.to_tokens(&mut tb.groups.last_mut().unwrap().1);
    }
}
impl<'a> ExtendTokenBuilder for &'a str {
    fn add(&self, tb: &mut TokenBuilder) {
        tb.add(self);
    }
}
impl ExtendTokenBuilder for str {
    fn add(&self, tb: &mut TokenBuilder) {
        tb.add(self);
    }
}
impl ExtendTokenBuilder for String {
    fn add(&self, tb: &mut TokenBuilder) {
        tb.add(self);
    }
}
impl ExtendTokenBuilder for usize {
    fn add(&self, tb: &mut TokenBuilder) {
        tb.extend_tree(TokenTree::Literal(Literal::usize_unsuffixed(*self)));
    }
}
impl ExtendTokenBuilder for syn::Type {
    fn add(&self, tb: &mut TokenBuilder) {
        self.to_tokens(&mut tb.groups.last_mut().unwrap().1);
    }
}
impl<'a> ExtendTokenBuilder for &'a TokenStream {
    fn add(&self, tb: &mut TokenBuilder) {
        tb.stream((*self).clone());
    }
}
impl ExtendTokenBuilder for TokenStream {
    fn add(&self, tb: &mut TokenBuilder) {
        tb.stream(self.clone());
    }
}
impl ExtendTokenBuilder for TokenTree {
    fn add(&self, tb: &mut TokenBuilder) {
        tb.extend_tree(self.clone());
    }
}
impl ExtendTokenBuilder for Ident {
    fn add(&self, tb: &mut TokenBuilder) {
        tb.extend_tree(TokenTree::Ident(self.clone()))
    }
}
impl<'a> ExtendTokenBuilder for &'a Ident {
    fn add(&self, tb: &mut TokenBuilder) {
        tb.extend_tree(TokenTree::Ident((*self).clone()))
    }
}

macro_rules! ts {
    ($($parts:expr) *) => {
        #[allow(unused_mut)]
        {
	        let mut tb = $crate::TokenBuilder::default();
	        $(
	        	$crate::token_builder::ExtendTokenBuilder::add(&$parts, &mut tb);
	        )*
	        tb.finish()
	    }
    }
}

macro_rules! extend_ts {
    ($tb:expr, $($parts:expr) *) => {
    	{
	        $(
	        	$crate::token_builder::ExtendTokenBuilder::add(&$parts, $tb);
	        )*
	    }
    }
}
macro_rules! ident {
    ($($x:expr) *) => {{
        let mut s = String::new();
        $(
            s.push_str(&$x.to_string());
        )*
        Ident::new(&s, proc_macro2::Span::call_site())
    }};
}
pub(crate) use {extend_ts, ident, join_ts, ts};

// #[cfg(test)]
// mod tests {
//     use crate::token_builder_part_from_string as parse;

//     #[test]
//     fn test_macros() {
//         let ts = ts! {
//             parse(&format!("hello world {}", 4)) 7 + 9 { 8 }
//         };
//     }
// }

pub fn safe_field_ident(field: &Field, idx: usize) -> Ident {
    if let Some(ident) = &field.ident {
        ident.clone()
    } else {
        ident!("_" idx)
    }
}

pub fn access_field(field: &Field, idx: usize) -> TokenTree {
    if let Some(ident) = &field.ident {
        TokenTree::Ident(ident.clone())
    } else {
        TokenTree::Literal(Literal::usize_unsuffixed(idx))
    }
}

pub fn generics_arg_by_mutating_type_params(
    g: &Generics,
    f: impl Fn(Ident) -> TokenStream,
) -> AngleBracketedGenericArguments {
    let args = g
        .type_params()
        .into_iter()
        .map(|tp| parse2::<GenericArgument>(f(tp.ident.clone())).unwrap())
        .collect();

    AngleBracketedGenericArguments {
        colon2_token: None,
        lt_token: <Token![<]>::default(),
        args,
        gt_token: <Token![>]>::default(),
    }
}

pub fn pattern_match(variant: &Variant, enum_ident: &Ident, binding_append: Option<Ident>) -> TokenStream {
    let get_ident = |field: &Field, idx: usize| {
        if let Some(binding_append) = &binding_append {
            ident!(safe_field_ident(field, idx) binding_append)
        } else {
            safe_field_ident(field, idx)
        }
    };

    let mut tb = TokenBuilder::default();
    extend_ts!(&mut tb,
        enum_ident "::" variant.ident
    );
    match &variant.fields {
        syn::Fields::Named(f) => {
            extend_ts!(&mut tb,
                "{"
                    join_ts!(f.named.iter().enumerate(), (i, field),
                        access_field(field, i) ":" get_ident(field, i)
                    )
                "}"
            );
        }
        syn::Fields::Unnamed(f) => {
            extend_ts!(&mut tb,
                "("
                    join_ts!(f.unnamed.iter().enumerate(), (i, field),
                        get_ident(field, i)
                    )
                ")"
            );
        }
        syn::Fields::Unit => {}
    }
    tb.finish()
}
