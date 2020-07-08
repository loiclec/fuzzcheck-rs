// Copyright (c) 2019 makepad

// makepad/render/microserde/derive/src/macro_lib.rs
// commit 1c753ca

use proc_macro::token_stream::IntoIter;
use proc_macro::{Delimiter, Group, Ident, Literal, Punct, Spacing, Span, TokenStream, TokenTree};

// little macro utility lib

pub fn error_span(err: &str, span: Span) -> TokenStream {
    let mut tb = TokenBuilder::new();
    tb.ident_with_span("compile_error", span)
        .add("! (")
        .string(err)
        .add(")");
    tb.end()
}

pub fn error(err: &str) -> TokenStream {
    let mut tb = TokenBuilder::new();
    tb.add("compile_error ! (").string(err).add(")");
    tb.end()
}

pub struct TokenBuilder {
    pub groups: Vec<(Delimiter, TokenStream)>,
}

impl TokenBuilder {
    pub fn new() -> Self {
        Self {
            groups: vec![(Delimiter::None, TokenStream::new())],
        }
    }

    pub fn end_clone(&self) -> TokenStream {
        if self.groups.len() != 1 {
            panic!("Groups not empty, you missed a pop_group")
        }
        self.groups.last().unwrap().1.clone()
    }

    pub fn end(mut self) -> TokenStream {
        if self.groups.len() != 1 {
            panic!("Groups not empty, you missed a pop_group")
        }
        self.groups.pop().unwrap().1
    }

    pub fn eprint(&self) {
        eprintln!("{}", self.groups.last().unwrap().1.to_string());
    }

    pub fn extend(&mut self, tt: TokenTree) -> &mut Self {
        self.groups.last_mut().unwrap().1.extend(Some(tt));
        self
    }

    pub fn extend_ident(&mut self, id: Ident) -> &mut Self {
        self.extend(TokenTree::Ident(id))
    }

    pub fn extend_punct(&mut self, p: Punct) -> &mut Self {
        self.extend(TokenTree::Punct(p))
    }

    pub fn extend_literal(&mut self, l: Literal) -> &mut Self {
        self.extend(TokenTree::Literal(l))
    }

    pub fn stream(&mut self, what: TokenStream) -> &mut Self {
        for c in what.into_iter() {
            self.extend(c);
        }
        self
    }

    pub fn add(&mut self, what: &str) -> &mut Self {
        for part in what.split(" ") {
            match part {
                "{" => self.push_group(Delimiter::Brace),
                "(" => self.push_group(Delimiter::Parenthesis),
                "[" => self.push_group(Delimiter::Bracket),
                "}" => self.pop_group(Delimiter::Brace),
                ")" => self.pop_group(Delimiter::Parenthesis),
                "]" => self.pop_group(Delimiter::Bracket),
                "?" | ";" | "&" | "^" | ":" | "::" | "," | "!" | "." | "<<" | ">>" | "->" | "=>" | "<" | ">" | "<="
                | ">=" | "=" | "==" | "!=" | "+" | "+=" | "-" | "-=" | "*" | "*=" | "/" | "/=" => {
                    self.punct(part)
                },
                _ => {
                    if part.len() == 0 {
                        continue;
                    }
                    match part.chars().next().unwrap() {
                        '0'..='9' => self.unsuf_usize(part.parse().expect(&format!("Can't parse number \"{}\"", what))),
                        _ => self.ident(part),
                    }
                }
            };
        }
        self
    }

    pub fn ident(&mut self, id: &str) -> &mut Self {
        self.extend(TokenTree::from(Ident::new(id, Span::call_site())))
    }

    pub fn ident_with_span(&mut self, id: &str, span: Span) -> &mut Self {
        self.extend(TokenTree::from(Ident::new(id, span)))
    }

    pub fn punct(&mut self, s: &str) -> &mut Self {
        let mut last = None;
        for c in s.chars() {
            if let Some(last) = last {
                self.extend(TokenTree::from(Punct::new(
                    last,
                    Spacing::Joint,
                )));
            }
            last = Some(c);
        }
        if let Some(last) = last {
            self.extend(TokenTree::from(Punct::new(
                last,
                Spacing::Alone,
            )));
        }
        self
    }

    pub fn lifetime_ident(&mut self, ident: &str) -> &mut Self {
        self.extend(TokenTree::Punct(Punct::new('\'', Spacing::Joint)));
        self.ident(ident);
        self
    }
    pub fn lifetime_anon(&mut self) -> &mut Self {
        self.extend(TokenTree::Punct(Punct::new('\'', Spacing::Joint)));
        self.punct("_");
        self
    }

    pub fn string(&mut self, val: &str) -> &mut Self {
        self.extend(TokenTree::from(Literal::string(val)))
    }
    pub fn unsuf_usize(&mut self, val: usize) -> &mut Self {
        self.extend(TokenTree::from(Literal::usize_unsuffixed(val)))
    }
    pub fn suf_u16(&mut self, val: u16) -> &mut Self {
        self.extend(TokenTree::from(Literal::u16_suffixed(val)))
    }
    pub fn chr(&mut self, val: char) -> &mut Self {
        self.extend(TokenTree::from(Literal::character(val)))
    }
    pub fn _lit(&mut self, lit: Literal) -> &mut Self {
        self.extend(TokenTree::from(lit))
    }

    pub fn push_group(&mut self, delim: Delimiter) -> &mut Self {
        self.groups.push((delim, TokenStream::new()));
        self
    }

    pub fn stack_as_string(&self) -> String {
        let mut ret = String::new();
        for i in (0..self.groups.len() - 1).rev() {
            ret.push_str(&format!("Level {}: {}", i, self.groups[i].1.to_string()));
        }
        ret
    }

    pub fn pop_group(&mut self, delim: Delimiter) -> &mut Self {
        if self.groups.len() < 2 {
            eprintln!("Stack dump for error:\n{}", self.stack_as_string());
            panic!("pop_group stack is empty {}", self.groups.len());
        }
        let ts = self.groups.pop().unwrap();
        if ts.0 != delim {
            eprintln!("Stack dump for error:\n{}", self.stack_as_string());
            panic!("pop_group Delimiter mismatch, got {:?} expected {:?}", ts.0, delim);
        }
        self.extend(TokenTree::from(Group::new(delim, ts.1)));
        self
    }
}

pub struct TokenParser {
    backtracked: Option<Box<TokenParser>>,
    iter_stack: Vec<IntoIter>,
    current: Option<TokenTree>,
}

fn token_tree_as_punct(tt: &TokenTree, what: char) -> Option<Punct> {
    if let TokenTree::Punct(p) = tt {
        if p.as_char() == what {
            Some(p.clone())
        } else {
            None
        }
    } else {
        None
    }
}

fn token_tree_as_ident(tt: &TokenTree, what: &str) -> Option<Ident> {
    if let TokenTree::Ident(i) = tt {
        if i.to_string() == what {
            Some(i.clone())
        } else {
            None
        }
    } else {
        None
    }
}

pub enum StructKind {
    Struct,
    Tuple
}

pub struct Struct {
    pub whole: TokenStream,
    pub kind: StructKind,
    pub data: StructData
}

pub struct StructData {
    pub ident: Ident,
    pub generics: Option<Generics>,
    pub where_clause: Option<TokenStream>,
    pub struct_fields: Option<StructFields>,
}
pub struct StructField {
    pub whole: TokenStream,
    pub attributes: Vec<TokenStream>,
    pub visibility: Option<TokenStream>,
    pub identifier: Option<Ident>,
    pub ty: TokenStream,
}
pub struct StructFields {
    pub whole: TokenStream,
    pub fields: Vec<StructField>,
}
pub struct LifetimeParam {
    pub whole: TokenStream,
    pub ident: TokenStream,
    pub bounds: Option<TokenStream>,
}
pub struct TypeParam {
    pub whole: TokenStream,
    pub attributes: Vec<TokenStream>,
    pub ident: Ident,
    pub bounds: Option<TokenStream>,
    pub equal_ty: Option<TokenStream>,
}
pub struct Generics {
    pub whole: TokenStream,
    pub lifetime_params: Vec<LifetimeParam>,
    pub type_params: Vec<TypeParam>,
}

impl TokenParser {
    pub fn new(start: TokenStream) -> Self {
        let mut ret = Self {
            backtracked: None,
            iter_stack: vec![start.into_iter()],
            current: None,
        };
        ret.advance();
        ret
    }

    pub fn backtrack(&mut self, ts: TokenStream) {
        if !ts.is_empty() {
            if let Some(backtracked) = &mut self.backtracked {
                backtracked.backtrack(ts)
            } else {
                self.backtracked = Some(Box::new(TokenParser::new(ts)));
            }
        }
    }

    // TODO: remove as_ref
    pub fn peek(&mut self) -> Option<&TokenTree> {
        if let Some(backtracked) = &mut self.backtracked {
            backtracked.peek()
        } else {
            self.current.as_ref()
        }
    }

    pub fn advance(&mut self) {
        if let Some(backtracked) = &mut self.backtracked {
            backtracked.advance();
            if backtracked.peek().is_none() {
                self.backtracked = None;
            }
            return
        }
        let last = self.iter_stack.last_mut().unwrap();
        let value = last.next();

        if let Some(tok) = value {
            self.current = Some(tok);
        } else {
            self.current = None;
        }
    }

    pub fn advance_if<T>(&mut self, cond: impl FnOnce(&TokenTree) -> Option<T>) -> Option<T> {
        let value = self.peek();
        if let Some(tok) = value {
            if let Some(value) = cond(tok) {
                self.advance();
                Some(value)
            } else {
                None
            }
        } else {
            None
        }
    }

    pub fn unexpected(&self) -> TokenStream {
        error("Unexpected token")
    }

    pub fn is_delim(&mut self, delim: Delimiter) -> bool {
        if let Some(TokenTree::Group(group)) = self.peek() {
            group.delimiter() == delim
        } else {
            false
        }
    }

    pub fn is_brace(&mut self) -> bool {
        self.is_delim(Delimiter::Brace)
    }

    pub fn is_paren(&mut self) -> bool {
        self.is_delim(Delimiter::Parenthesis)
    }

    pub fn is_bracket(&mut self) -> bool {
        self.is_delim(Delimiter::Bracket)
    }

    pub fn open_delim(&mut self, delim: Delimiter) -> bool {
        let iter = if let Some(TokenTree::Group(group)) = self.peek() {
            if group.delimiter() == delim {
                Some(group.stream().into_iter())
            } else {
                None
            }
        } else {
            None
        };

        if let Some(iter) = iter {
            self.iter_stack.push(iter);
            self.advance();
            true
        } else {
            false
        }

    }

    pub fn open_brace(&mut self) -> bool {
        self.open_delim(Delimiter::Brace)
    }

    pub fn open_paren(&mut self) -> bool {
        self.open_delim(Delimiter::Parenthesis)
    }

    pub fn open_bracket(&mut self) -> bool {
        self.open_delim(Delimiter::Bracket)
    }

    pub fn is_eot(&mut self) -> bool {
        if self.current.is_none() && self.iter_stack.len() != 0 {
            return true;
        } else {
            return false;
        }
    }

    pub fn eat_eot(&mut self) -> bool {
        // current is None
        if self.is_eot() {
            self.iter_stack.pop();
            if self.iter_stack.len() != 0 {
                self.advance()
            }
            return true;
        }
        return false;
    }

    pub fn eat_ident(&mut self, what: &str) -> Option<Ident> {
        self.advance_if(|tt| token_tree_as_ident(tt, what))
    }

    pub fn is_punct(&mut self, what: char) -> bool {
        // check if our punct is multichar.
        if let Some(tt) = self.peek() {
            token_tree_as_punct(tt, what).is_some()
        } else {
            return false;
        }
    }

    pub fn eat_punct(&mut self, what: char) -> Option<Punct> {
        self.advance_if(|tt| token_tree_as_punct(tt, what))
    }

    pub fn eat_any_ident(&mut self) -> Option<Ident> {
        self.advance_if(|tt| 
            if let TokenTree::Ident(id) = tt {
                Some(id.clone())
            } else {
                None
            }
        )
    }

    pub fn eat_literal(&mut self) -> Option<Literal> {
        self.advance_if(|tt| 
            if let TokenTree::Literal(l) = tt {
                Some(l.clone())
            } else {
                None
            }
        )
    }

    pub fn eat_type_bound_where_clause_item(&mut self) -> Option<TokenStream> {
        let mut tb = TokenBuilder::new();
        if let Some(for_lt) = self.eat_for_lifetimes() {
            tb.stream(for_lt);
        }
        if let Some(ty) = self.eat_type() {
            tb.stream(ty);
            if let Some(colon) = self.eat_punct(':') {
                tb.extend_punct(colon);
                if let Some(typbs) = self.eat_type_param_bounds() {
                    tb.stream(typbs);
                }
                Some(tb.end())
            } else {
                self.backtrack(tb.end());
                None
            }
        } else {
            self.backtrack(tb.end());
            None
        }
    }

    pub fn eat_lifetime_bounds(&mut self) -> Option<TokenStream> {
        let mut tb = TokenBuilder::new();
        while let Some(lt) = self.eat_lifetime() {
            tb.stream(lt);
            if let Some(plus) = self.eat_punct('+') {
                tb.extend_punct(plus);
                continue
            } else {
                break
            }
        }
        Some(tb.end())
    }

    pub fn eat_lifetime_where_clause_item(&mut self) -> Option<TokenStream> {
        if let Some(lt) = self.eat_lifetime() {
            let mut tb = TokenBuilder::new();
            tb.stream(lt);
            if let Some(colon) = self.eat_punct(':') {
                tb.extend_punct(colon);
            } else {
                self.backtrack(tb.end());
                return None
            }
            if let Some(lt_bounds) = self.eat_lifetime_bounds() {
                tb.stream(lt_bounds);
                Some(tb.end())
            } else {
                self.backtrack(tb.end());
                return None
            }
        } else {
            None
        }
    }

    pub fn eat_where_clause_item(&mut self) -> Option<TokenStream> {
        self.eat_lifetime_where_clause_item().or_else(|| self.eat_type_bound_where_clause_item())
    }

    pub fn eat_where_clause(&mut self) -> Option<TokenStream> {
        if let Some(where_ident) = self.eat_ident("where") {
            let mut tb = TokenBuilder::new();
            tb.extend_ident(where_ident);

            while let Some(clause_item) = self.eat_where_clause_item() {
                tb.stream(clause_item);
                if let Some(comma) = self.eat_punct(',') {
                    tb.extend_punct(comma);
                    continue
                } else {
                    break
                }
            }
            Some(tb.end())
        } else {
            None
        }
    }

    pub fn eat_tuple_struct(&mut self) -> Option<Struct> {
        if let Some(struct_ident) = self.eat_ident("struct") {
            let mut tb = TokenBuilder::new();
            tb.extend_ident(struct_ident);

            if let Some(ident) = self.eat_any_ident() {
                tb.extend_ident(ident.clone());
                let generics = self.eat_generics();
                if let Some(generics) = &generics {
                    tb.stream(generics.whole.clone());
                }
                if self.open_paren() {
                    tb.push_group(Delimiter::Parenthesis);
                    let struct_fields = self.eat_tuple_fields();
                    if let Some(struct_fields) = &struct_fields {
                        tb.stream(struct_fields.whole.clone());
                    }
                    if self.eat_eot() {
                        tb.pop_group(Delimiter::Parenthesis);
        
                        let where_clause = self.eat_where_clause();
                        if let Some(where_clause) = where_clause.clone() {
                            tb.stream(where_clause);
                        }
                        if let Some(semi_colon) = self.eat_punct(';') {
                            tb.extend_punct(semi_colon);
                            Some(Struct {
                                whole: tb.end(),
                                kind: StructKind::Tuple,
                                data: StructData {
                                    ident,
                                    generics,
                                    where_clause,
                                    struct_fields,
                                }
                            })
                        } else { // TODO: check backtracking correctness
                            // this would crash
                            self.backtrack(tb.end());
                            None
                        }
                    } else {
                        self.backtrack(tb.end());
                        None
                    }
                } else {
                    self.backtrack(tb.end());
                    None
                }
            } else {
                self.backtrack(tb.end());
                None
            }
        } else {
            None
        }
    }

    pub fn eat_struct_struct(&mut self) -> Option<Struct> {
        if let Some(struct_ident) = self.eat_ident("struct") {
            let mut tb = TokenBuilder::new();
            tb.extend_ident(struct_ident);

            if let Some(ident) = self.eat_any_ident() {
                tb.extend_ident(ident.clone());
                let generics = self.eat_generics();
                if let Some(generics) = &generics {
                    tb.stream(generics.whole.clone());
                }
                let where_clause = self.eat_where_clause();
                if let Some(where_clause) = where_clause.clone() {
                    tb.stream(where_clause);
                }
                if self.open_brace() {
                    tb.push_group(Delimiter::Brace);

                    let struct_fields = self.eat_struct_fields();
                    tb.stream(struct_fields.whole.clone());
                    if self.eat_eot() {
                        tb.pop_group(Delimiter::Brace);
                        let whole = tb.end();
                        Some(Struct {
                            whole,
                            kind: StructKind::Struct,
                            data: StructData {
                                ident,
                                generics,
                                where_clause,
                                struct_fields: Some(struct_fields),
                            }
                        })
                    } else {
                        tb.pop_group(Delimiter::Brace);
                        self.backtrack(tb.end());
                        None
                    }
                } else if let Some(semi_colon) = self.eat_punct(';') {
                    tb.extend_punct(semi_colon);
                    Some(Struct {
                        whole: tb.end(),
                        kind: StructKind::Struct,
                        data: StructData {
                            ident,
                            generics,
                            where_clause,
                            struct_fields: None,
                        }
                    })
                } else {
                    self.backtrack(tb.end());
                    None
                }
            } else {
                self.backtrack(tb.end());
                None
            }
        } else {
            None
        }
    }

    pub fn eat_struct(&mut self) -> Option<Struct> {
        self.eat_struct_struct().or_else(|| self.eat_tuple_struct())
    }

    pub fn eat_lifetime_param(&mut self) -> Option<LifetimeParam> {
        let mut tb = TokenBuilder::new();
        let mut attributes = Vec::new();
        while let Some(outer_attribute) = self.eat_outer_attribute() {
            attributes.push(outer_attribute.clone());
            tb.stream(outer_attribute);
        }
        if let Some(ident) = self.eat_lifetime_or_label() {
            tb.stream(ident.clone());
            if let Some(colon) = self.eat_punct(':') {
                tb.extend_punct(colon);
                if let Some(bounds) = self.eat_lifetime_bounds() {
                    tb.stream(bounds.clone());
                    Some(LifetimeParam {
                        whole: tb.end(),
                        ident,
                        bounds: Some(bounds),
                    })
                } else {
                    self.backtrack(tb.end());
                    None
                }
            } else {
                Some(LifetimeParam {
                    whole: tb.end(),
                    ident,
                    bounds: None,
                })
            }
        } else {
            self.backtrack(tb.end());
            return None;
        }
    }

    pub fn eat_type_param(&mut self) -> Option<TypeParam> {
        let mut tb = TokenBuilder::new();
        let mut attributes = Vec::new();
        while let Some(attr) = self.eat_outer_attribute() {
            tb.stream(attr.clone());
            attributes.push(attr);
        }
        if let Some(ident) = self.eat_any_ident() {
            tb.extend_ident(ident.clone());
            let bounds = if let Some(colon) = self.eat_punct(':') {
                tb.extend_punct(colon);
                if let Some(bounds) = self.eat_type_param_bounds() {
                    tb.stream(bounds.clone());
                    Some(bounds)
                } else {
                    self.backtrack(tb.end());
                    return None
                }
            } else {
                None
            };
            let equal_ty = if let Some(equal) = self.eat_punct('=') {
                tb.extend_punct(equal);
                if let Some(ty) = self.eat_type() {
                    tb.stream(ty.clone());
                    Some(ty)
                } else {
                    self.backtrack(tb.end());
                    return None;
                }
            } else {
                None
            };
            Some(TypeParam {
                whole: tb.end(),
                attributes,
                ident,
                bounds,
                equal_ty,
            })
        } else {
            self.backtrack(tb.end());
            None
        }
    }

    pub fn eat_generics(&mut self) ->  Option<Generics> {
        let mut tb = TokenBuilder::new();
        if let Some(open) = self.eat_punct('<') {
            tb.extend_punct(open);
            let mut lifetime_params = Vec::new();
            while let Some(lt_param) = self.eat_lifetime_param() {
                tb.stream(lt_param.whole.clone());
                lifetime_params.push(lt_param);
                if let Some(comma) = self.eat_punct(',') {
                    tb.extend_punct(comma);
                }
            }
            let mut type_params = Vec::new();
            while let Some(ty_param) = self.eat_type_param() {
                tb.stream(ty_param.whole.clone());
                type_params.push(ty_param);
                if let Some(comma) = self.eat_punct(',') {
                    tb.extend_punct(comma);
                } else {
                    break
                }
            }
            
            if let Some(close) = self.eat_punct('>') {
                tb.extend_punct(close);
                Some(Generics {
                    whole: tb.end(),
                    lifetime_params,
                    type_params,
                })
            } else {
                self.backtrack(tb.end());
                None
            }
        } else {
            None
        }
    }

    pub fn eat_tuple_fields(&mut self) -> Option<StructFields> {
        if let Some(field) = self.eat_tuple_field() {
            let mut tb = TokenBuilder::new();
            let mut fields = Vec::new();
            tb.stream(field.whole.clone());
            fields.push(field);
            while let Some(comma) = self.eat_punct(',') {
                tb.extend_punct(comma);
                if let Some(field) = self.eat_tuple_field() {
                    tb.stream(field.whole.clone());
                    fields.push(field);
                } else {
                    break
                }
            }
            Some(StructFields {
                whole: tb.end(),
                fields: fields,
            })
        } else {
            None
        }
    }

    pub fn eat_tuple_field(&mut self) -> Option<StructField> {
        let mut tb = TokenBuilder::new();
        let mut attributes = Vec::new();
        while let Some(attribute) = self.eat_outer_attribute() {
            tb.stream(attribute.clone());
            attributes.push(attribute);
        }
        let visibility = self.eat_visibility();
        if let Some(visibility) = visibility.clone() {
            tb.stream(visibility);
        }
        if let Some(ty) = self.eat_type() {
            tb.stream(ty.clone());
            Some(StructField {
                whole: tb.end(),
                attributes,
                visibility,
                identifier: None,
                ty,
            })
        } else {
            self.backtrack(tb.end());
            None
        }
    }

    pub fn eat_struct_fields(&mut self) -> StructFields {
        let mut tb = TokenBuilder::new();
        let mut vec = Vec::new();
        while let Some(field) = self.eat_struct_field() {
            tb.stream(field.whole.clone());
            vec.push(field);
            if let Some(comma) = self.eat_punct(',') {
                tb.extend_punct(comma);
            } else {
                break
            }
        }
        StructFields {
            whole: tb.end(),
            fields: vec,
        }
    }

    pub fn eat_struct_field(&mut self) -> Option<StructField> {
        let mut tb = TokenBuilder::new();
        let mut attributes = vec![];
        while let Some(outer_attribute) = self.eat_outer_attribute() {
            attributes.push(outer_attribute.clone());
            tb.stream(outer_attribute);
        }
        let visibility = self.eat_visibility();
        if let Some(visibility) = visibility.clone() {
            tb.stream(visibility);
        }
        if let Some(identifier) = self.eat_any_ident() {
            tb.extend_ident(identifier.clone());
            if let Some(colon) = self.eat_punct(':') {
                tb.extend_punct(colon);
            }
            if let Some(ty) = self.eat_type() {
                tb.stream(ty.clone());
                Some(StructField {
                    whole: tb.end(),
                    attributes,
                    visibility,
                    identifier: Some(identifier),
                    ty,  
                })
            } else {
                self.backtrack(tb.end());
                None
            }
        } else {
            self.backtrack(tb.end());
            None
        }
    }

    pub fn eat_group_angle_bracket(&mut self) -> Option<TokenStream> {
        let mut tb = TokenBuilder::new();
        // if we have a <, keep running and keep a < stack

        if let Some(ob) = self.eat_punct('<') {
            tb.extend_punct(ob);
            let mut stack = 1;
            // keep eating things till we are at stack 0 for a ">"
            while stack > 0 {
                if let Some(ob) = self.eat_punct('<') {
                    tb.extend_punct(ob);
                    stack += 1;
                }
                if let Some(cb) = self.eat_punct('>') {
                    tb.extend_punct(cb);
                    stack -= 1;
                } else if self.eat_eot() {
                    // shits broken
                    return None;
                } else {
                    // store info here in generics struct
                    if let Some(current) = self.peek() {
                        tb.extend(current.clone());
                    }
                    self.advance();
                }
            }
            return Some(tb.end());
        } else {
            return None;
        }
    }

    // same as lifetime_token because I don't distinguish between ident and keywords
    pub fn eat_lifetime_or_label(&mut self) -> Option<TokenStream> {
        let mut tb = TokenBuilder::new();
        if let Some(ap) = self.eat_punct('\'') {
            tb.extend_punct(ap);
            if let Some(lifetime) = self.eat_any_ident() {
                tb.extend_ident(lifetime);
            } else {
                self.backtrack(tb.end());
                return None;
            }
            Some(tb.end())
        } else {
            None
        }
    }

    pub fn eat_lifetime(&mut self) -> Option<TokenStream> {
        self.eat_lifetime_or_label().or_else(|| {
            let mut tb = TokenBuilder::new();
            if let Some(ap) = self.eat_punct('\'') {
                tb.extend_punct(ap);
                if let Some(anon) = self.eat_punct('_') {
                    tb.extend_punct(anon);
                } else {
                    self.backtrack(tb.end());
                    return None;
                }
                Some(tb.end())
            } else {
                None
            }
        })
    }

    pub fn eat_double_colon(&mut self) -> Option<TokenStream> {
        if let Some(c1) = self.eat_punct(':') {
            let mut tb = TokenBuilder::new();
            tb.extend_punct(c1);
            if let Some(c2) = self.eat_punct(':') {
                tb.extend_punct(c2);
                Some(tb.end())
            } else {
                self.backtrack(tb.end());
                None
            }
        } else {
            None
        }
    }

    pub fn eat_fn_arrow(&mut self) -> Option<TokenStream> {
        if let Some(c1) = self.eat_punct('-') {
            let mut tb = TokenBuilder::new();
            tb.extend_punct(c1);
            if let Some(c2) = self.eat_punct('>') {
                tb.extend_punct(c2);
                Some(tb.end())
            } else {
                self.backtrack(tb.end());
                None
            }
        } else {
            None
        }
    }

    pub fn eat_type_path_segment(&mut self) -> Option<TokenStream> {
        let mut tb = TokenBuilder::new();
        if let Some(ident) = self.eat_any_ident() {
            tb.extend_ident(ident);
            if let Some(colons) = self.eat_double_colon() {
                tb.stream(colons);
            }
            if let Some(generic) = self.eat_group_angle_bracket() {
                tb.stream(generic);
            } else if let Some(fn_args) = self.eat_group(Delimiter::Parenthesis) {
                tb.extend(fn_args);
                if let Some(arrow) = self.eat_fn_arrow() {
                    tb.stream(arrow);
                    if let Some(ty) = self.eat_type() {
                        tb.stream(ty);
                    } else {
                        self.backtrack(tb.end());
                        return None;
                    }
                }
            }
            Some(tb.end())
        } else {
            None
        }
    }

    pub fn eat_type_path(&mut self) -> Option<TokenStream> {
        let mut tb = TokenBuilder::new();
        if let Some(colons) = self.eat_double_colon() {
            tb.stream(colons);
        }
        loop {
            if let Some(segment) = self.eat_type_path_segment() {
                tb.stream(segment);
                if let Some(colons) = self.eat_double_colon() {
                    tb.stream(colons);
                    continue;
                } else {
                    break Some(tb.end());
                }
            } else {
                self.backtrack(tb.end());
                return None
            }
        }
    }

    pub fn eat_raw_pointer_type(&mut self) -> Option<TokenStream> {
        if let Some(star) = self.eat_punct('*') {
            let mut tb = TokenBuilder::new();
            tb.extend_punct(star);
            if let Some(ident) = self.eat_ident("cont").or_else(|| self.eat_ident("mut")) {
                tb.extend_ident(ident);
                if let Some(ty) = self.eat_type_no_bounds() {
                    tb.stream(ty);
                    Some(tb.end())
                } else {
                    self.backtrack(tb.end());
                    return None
                }
            } else {    
                self.backtrack(tb.end());
                return None
            }
        } else {
            None
        }
    }

    pub fn eat_qualified_path_in_type(&mut self) -> Option<TokenStream> {
        // qualified path type
        let mut tb = TokenBuilder::new();
        if let Some(qtp) = self.eat_group_angle_bracket() {
            tb.stream(qtp);

            if let Some(colons) = self.eat_double_colon() {
                tb.stream(colons);
                if let Some(tps) = self.eat_type_path_segment() {
                    tb.stream(tps);
        
                    while let Some(colons) = self.eat_double_colon() {
                        tb.stream(colons);
                        if let Some(tps) = self.eat_type_path_segment() {
                            tb.stream(tps);
                        } else {
                            self.backtrack(tb.end());
                            return None
                        }
                    }
                    Some(tb.end())
                } else {
                    self.backtrack(tb.end());
                    None
                }
            } else {
                self.backtrack(tb.end());
                None
            }
        } else {
            None
        }
    }

    pub fn eat_for_lifetimes(&mut self) -> Option<TokenStream> {
        let mut tb = TokenBuilder::new();

        if let Some(for_ident) = self.eat_ident("for") {
            tb.extend_ident(for_ident);
            if let Some(lifetime_params) = self.eat_group_angle_bracket() {
                tb.stream(lifetime_params);
                Some(tb.end())
            } else {
                self.backtrack(tb.end());
                None
            }
        } else {
            None
        }
    }

    pub fn eat_outer_attribute(&mut self) -> Option<TokenStream> {
        let mut tb = TokenBuilder::new();
        let nbr_sign = self.eat_punct('#')?;
        if self.open_bracket() {
            tb.extend_punct(nbr_sign);
            if let Some(content) = self.eat_any_group() {
                tb.extend(content);
                Some(tb.end())   
            } else {
                self.backtrack(tb.end());
                None
            }
        } else {
            None
        }
    }

    pub fn eat_simple_path(&mut self) -> Option<TokenStream> {
        let mut tb = TokenBuilder::new();
        if let Some(db) = self.eat_double_colon() {
            tb.stream(db);
        }
        if let Some(sps) = self.eat_any_ident() { // simple path segment, except $crate
            tb.extend_ident(sps);
            while let Some(db) = self.eat_double_colon() {
                if let Some(sps) = self.eat_any_ident() { // simple path segment, except $crate
                    tb.stream(db);
                    tb.extend_ident(sps);
                } else {
                    self.backtrack(tb.end());
                    return None
                }
            }
            Some(tb.end())
        } else {
            self.backtrack(tb.end());
            None
        }
    }

    pub fn eat_macro_invocation(&mut self) -> Option<TokenStream> {
        if let Some(sp) = self.eat_simple_path() {
            let mut tb = TokenBuilder::new();
            tb.stream(sp);
            if let Some(tree) = self.eat_any_group() {
                tb.extend(tree);
                Some(tb.end())
            } else {
                self.backtrack(tb.end());
                None
            }
        } else {
            None
        }
    }

    // TODO: check for backtracking correctness
    pub fn eat_trait_bound(&mut self) -> Option<TokenStream> {
        if let Some(g) = self.eat_group(Delimiter::Parenthesis) {
            let mut tb = TokenBuilder::new();
            tb.extend(g);
            Some(tb.end())
        } else {
            let q = self.eat_punct('?');
            let for_lt = self.eat_for_lifetimes();

            let mut tb = TokenBuilder::new();

            if let Some(tp) = self.eat_type_path() {
                if let Some(q) = q {
                    tb.extend_punct(q);
                }
                if let Some(for_lt) = for_lt {
                    tb.stream(for_lt);
                }
                tb.stream(tp);
    
                Some(tb.end())
            } else {
                self.backtrack(tb.end());
                return None
            }
        }
    }

    pub fn eat_trait_object_type_one_bound(&mut self) -> Option<TokenStream> {
        let mut tb = TokenBuilder::new();
        if let Some(dyn_ident) = self.eat_ident("dyn") {
            tb.extend_ident(dyn_ident);
        }
        if let Some(trait_bound) = self.eat_trait_bound() {
            tb.stream(trait_bound);
            Some(tb.end())
        } else {
            self.backtrack(tb.end());
            None
        }
    }

    pub fn eat_impl_trait_type_one_bound(&mut self) -> Option<TokenStream> {
        if let Some(impl_ident) = self.eat_ident("impl") {
            let mut tb = TokenBuilder::new();
            if let Some(trait_bound) = self.eat_trait_bound() {
                tb.extend_ident(impl_ident);
                tb.stream(trait_bound);
                Some(tb.end())
            } else {
                self.backtrack(tb.end());
                None
            }
        } else {
            None
        }
    }

    pub fn eat_type_no_bounds(&mut self) -> Option<TokenStream> {
        let mut tb = TokenBuilder::new();

        if let Some(tys) = self.eat_group(Delimiter::Parenthesis) {
            // parenthesized_type
            tb.extend(tys);
            return Some(tb.end());
        } else if let Some(ittob) = self.eat_impl_trait_type_one_bound() {
            // impl trait one bound
            tb.stream(ittob);
            return Some(tb.end());
        } else if let Some(itotob) = self.eat_trait_object_type_one_bound() {
            tb.stream(itotob);
            return Some(tb.end());
        } else if let Some(typath) = self.eat_type_path() {
            // type path
            tb.stream(typath);
            return Some(tb.end());
        } else if let Some(tuple) = self.eat_group(Delimiter::Parenthesis) {
            // tuple type
            tb.extend(tuple);
            return Some(tb.end());
        } else if let Some(never) = self.eat_punct('!') {
            // never type
            tb.extend_punct(never);
            return Some(tb.end());
        } else if let Some(raw_ptr) = self.eat_raw_pointer_type() {
            // raw pointer type
            tb.stream(raw_ptr);
            return Some(tb.end());
        } else if let Some(amp) = self.eat_punct('&') {
            // reference type
            tb.extend_punct(amp);
            if let Some(lt) = self.eat_lifetime() {
                tb.stream(lt);
            }
            if let Some(mut_ident) = self.eat_ident("mut") {
                tb.extend_ident(mut_ident);
            }
            let ty = self.eat_type_no_bounds()?;
            tb.stream(ty);
            return Some(tb.end());
        } else if let Some(arr_or_slice) = self.eat_group(Delimiter::Bracket) {
            // array type + slice type
            tb.extend(arr_or_slice);
            return Some(tb.end());
        } else if let Some(punct) = self.eat_punct('_') {
            // inferred type
            tb.extend_punct(punct);
            return Some(tb.end());
        } else if let Some(qpit) = self.eat_qualified_path_in_type() {
            // qualified path in type
            tb.stream(qpit);
            return Some(tb.end());
        } else if let Some(m) = self.eat_macro_invocation() {
            tb.stream(m);
            return Some(tb.end());
        } else {
            return None;
        }
    }

    pub fn eat_type_param_bounds(&mut self) -> Option<TokenStream> {
        if let Some(tpb) = self.eat_type_param_bound() {
            let mut tb = TokenBuilder::new();
            tb.stream(tpb);
            while let Some(plus) = self.eat_punct('+') {
                tb.extend_punct(plus);
                if let Some(bound) = self.eat_type_param_bound() {
                    tb.stream(bound);
                }
            }
            Some(tb.end())
        } else {
            None
        }
    }

    pub fn eat_type_param_bound(&mut self) -> Option<TokenStream> {
        self.eat_lifetime().or_else(|| self.eat_trait_bound())
    }

    pub fn eat_impl_trait_type(&mut self) -> Option<TokenStream> {
        if let Some(impl_ident) = self.eat_ident("impl") {
            let mut tb = TokenBuilder::new();
            tb.extend_ident(impl_ident);
            if let Some(tpbs) = self.eat_type_param_bounds() {
                tb.stream(tpbs);
                Some(tb.end())
            } else {
                self.backtrack(tb.end());
                None
            }
        } else {
            None
        }
    }

    pub fn eat_trait_object_type(&mut self) -> Option<TokenStream> {
        let mut tb = TokenBuilder::new();
        if let Some(dyn_ident) = self.eat_ident("dyn") {
            tb.extend_ident(dyn_ident);
        }
        if let Some(tpbs) = self.eat_type_param_bounds() {
            tb.stream(tpbs);
            Some(tb.end())
        } else {
            self.backtrack(tb.end());
            None
        }
    }

    pub fn eat_type(&mut self) -> Option<TokenStream> {
        self.eat_type_no_bounds()
            .or_else(|| self.eat_impl_trait_type())
            .or_else(|| self.eat_trait_object_type())
    }

    pub fn eat_group(&mut self, delim: Delimiter) -> Option<TokenTree> {
        if let Some(TokenTree::Group(group)) = self.peek() {
            if group.delimiter() == delim {
                let ret = Some(TokenTree::Group(group.clone()));
                self.advance();
                ret
            } else {
                None
            }
        } else {
            None
        }
    }

    pub fn eat_any_group(&mut self) -> Option<TokenTree> {
        if let Some(TokenTree::Group(group)) = self.peek() {
            let ret = Some(TokenTree::Group(group.clone()));
            self.advance();
            return ret;
        }
        return None;
    }

    pub fn eat_visibility(&mut self) -> Option<TokenStream> {
        let mut tb = TokenBuilder::new();
        if let Some(pub_ident) = self.eat_ident("pub") {
            tb.extend_ident(pub_ident);
            if let Some(tt) = self.eat_group(Delimiter::Bracket) {
                tb.extend(tt);
            }
            Some(tb.end())
        } else {
            None
        }
    }
}


/*
    pub fn eat_triple_dot(&mut self) -> Option<TokenStream> {
        if let Some(c1) = self.eat_punct('.') {
            let mut tb = TokenBuilder::new();
            tb.extend_punct(c1);
            if let Some(c2) = self.eat_punct('.') {
                tb.extend_punct(c2);
                if let Some(c3) = self.eat_punct('.') {
                    tb.extend_punct(c3);
                    Some(tb.end())
                } else {
                    self.backtrack(tb.end());
                    None
                }
            } else {
                self.backtrack(tb.end());
                None
            }
        } else {
            None
        }
    }

    pub fn eat_function_qualifiers(&mut self) -> TokenStream {
        let mut tb = TokenBuilder::new();

        if let Some(async_const) = self.eat_ident("const").or_else(|| self.eat_ident("async")) {
            tb.extend_ident(async_const);
        }

        if let Some(unsafe_ident) = self.eat_ident("unsafe") {
            tb.extend_ident(unsafe_ident);
        }

        if let Some(extern_ident) = self.eat_ident("extern") {
            tb.extend_ident(extern_ident);
            if let Some(abi) = self.eat_literal() {
                tb.extend_literal(abi);
            }
        }

        tb.end()
    }

    pub fn eat_bare_function_type(&mut self) -> Option<TokenStream> {
        let mut tb = TokenBuilder::new();

        if let Some(for_lt) = self.eat_for_lifetimes() {
            tb.stream(for_lt);
        }
        let fq = self.eat_function_qualifiers();
        tb.stream(fq);

        if let Some(fn_ident) = self.eat_ident("fn") {
            tb.extend_ident(fn_ident);

            if let Some(arrow) = self.eat_fn_arrow() {
                tb.stream(arrow);
                if let Some(type_no_bounds) = self.eat_type_no_bounds() {
                    tb.stream(type_no_bounds);
                } else {
                    self.backtrack(tb.end());
                    return None
                }
            }

            Some(tb.end())
        } else {
            self.backtrack(tb.end());
            return None
        }
    }


    pub fn eat_maybe_named_function_parameters_variadic(&mut self) -> Option<TokenStream> {
        let mut tb = TokenBuilder::new();
        while let Some(mnp) = self.eat_maybe_named_param() {
            let comma = self.eat_punct(',')?;
            tb.stream(mnp);
            tb.extend_punct(comma);
        }
        let mnp = self.eat_maybe_named_param()?;
        let comma = self.eat_punct(',')?;

        tb.stream(mnp);
        tb.extend_punct(comma);

        while let Some(attr) = self.eat_outer_attribute() {
            tb.stream(attr);
        }

        let triple_dots = self.eat_triple_dot()?;
        tb.stream(triple_dots);

        Some(tb.end())
    }

    pub fn eat_maybe_named_param(&mut self) -> Option<TokenStream> {
        let mut tb = TokenBuilder::new();
        while let Some(attr) = self.eat_outer_attribute() {
            tb.stream(attr);
        }
        if let Some(ident_or_anon) = self.eat_any_ident().or_else(|| self.eat_punct('_')) {
            let colon = self.eat_punct(':')?;
            tb.extend_ident(ident_or_anon);
            tb.extend_punct(colon);
        }
        let ty = self.eat_type()?;
        tb.stream(ty);

        Some(tb.end())
    }

    pub fn eat_maybe_named_function_parameters(&mut self) -> Option<TokenStream> {
        let mnp1 = self.eat_maybe_named_param()?;
        let mut tb = TokenBuilder::new();
        tb.stream(mnp1);
        while let Some(comma) = self.eat_punct(',') {
            let mnp_i = self.eat_maybe_named_param()?;
            tb.extend_punct(comma);
            tb.stream(mnp_i);
        }
        if let Some(comma) = self.eat_punct(',') {
            tb.extend_punct(comma);
        }

        Some(tb.end())
    }

    pub fn eat_function_parameters_maybe_named_variadic(&mut self) -> Option<TokenStream> {
        self.eat_maybe_named_function_parameters().or_else(|| self.eat_maybe_named_function_parameters_variadic())
    }
*/