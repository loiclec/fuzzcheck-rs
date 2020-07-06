// Copyright (c) 2019 makepad

// makepad/render/microserde/derive/src/macro_lib.rs
// commit 1c753ca

#![allow(dead_code)]

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

fn token_tree_is_punct(tt: &TokenTree, what: char) -> bool {
    if let TokenTree::Punct(p) = tt {
        p.as_char() == what
    } else {
        false
    }
}

fn token_tree_is_ident(tt: &TokenTree, what: &str) -> bool {
    if let TokenTree::Ident(i) = tt {
        i.to_string() == what
    } else {
        false
    }
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
        if let Some(backtracked) = &mut self.backtracked {
            backtracked.backtrack(ts)
        } else {
            self.backtracked = Some(Box::new(TokenParser::new(ts)));
        }
    }

    pub fn peek(&mut self) -> Option<&TokenTree> {
        if let Some(backtracked) = &mut self.backtracked {
            backtracked.peek()
        } else {
            self.current.as_ref()
        }
    }

    pub fn advance(&mut self) {
        if let Some(backtracked) = &mut self.backtracked {
            backtracked.advance()
        } else {
            let last = self.iter_stack.last_mut().unwrap();
            let value = last.next();

            if let Some(tok) = value {
                self.current = Some(tok);
            } else {
                self.current = None;
            }
        }
    }

    pub fn advance_if(&mut self, cond: impl FnOnce(&TokenTree) -> bool) -> Option<TokenTree> {
        let value = self.peek();
        if let Some(tok) = value {
            if cond(tok) {
                self.advance();
                self.current.clone()
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

    pub fn eat_ident(&mut self, what: &str) -> Option<TokenTree> {
        self.advance_if(|tt| token_tree_is_ident(tt, what))
    }

    pub fn is_punct(&mut self, what: char) -> bool {
        // check if our punct is multichar.
        if let Some(tt) = self.peek() {
            token_tree_is_punct(tt, what)
        } else {
            return false;
        }
    }

    pub fn eat_punct(&mut self, what: char) -> Option<TokenTree> {
        self.advance_if(|tt| token_tree_is_punct(tt, what))
    }

    pub fn eat_any_ident(&mut self) -> Option<TokenTree> {
        self.advance_if(|tt| matches!(tt, TokenTree::Ident(_)))
    }

    pub fn eat_literal(&mut self) -> Option<TokenTree> {
        self.advance_if(|tt| matches!(tt, TokenTree::Literal(_)))
    }

    // TODO: rewrite that
    pub fn eat_where_clause(&mut self) -> Option<TokenStream> {
        let mut tb = TokenBuilder::new();
        if let Some(ident) = self.eat_ident("where") {
            tb.extend(ident);
            // ok now we parse an ident
            loop {
                if let Some(ident) = self.eat_any_ident() {
                    tb.extend(ident);
                    if let Some(generic) = self.eat_group_angle_bracket() {
                        tb.stream(generic);
                    }

                    if let Some(colon) = self.eat_punct(':') {
                        tb.extend(colon);
                    } else {
                        return None;
                    }

                    loop {
                        if let Some(ident) = self.eat_any_ident() {
                            tb.extend(ident);
                            if let Some(generic) = self.eat_group_angle_bracket() {
                                tb.stream(generic);
                            }

                            // check if we have upnext
                            // {, + or ,
                            if let Some(plus) = self.eat_punct('+') {
                                tb.extend(plus);
                                continue;
                            }
                            if let Some(colon) = self.eat_punct(',') {
                                // next one
                                tb.extend(colon);
                                break;
                            }
                            if self.is_brace() || self.is_punct(';') {
                                // upnext is a brace.. we're done
                                return Some(tb.end());
                            }
                        } else {
                            return None; // unexpected
                        }
                    }
                } else {
                    if self.is_brace() || self.is_punct(';') {
                        // upnext is a brace.. we're done
                        return Some(tb.end());
                    } else {
                        return None;
                    }
                    //
                    // return None // unexpected
                }
            }
        }
        return None;
    }

    // TODO: rewrite that
    pub fn eat_struct_field(&mut self) -> Option<(TokenTree, TokenStream)> {
        // lets parse an ident
        if let Some(field) = self.eat_any_ident() {
            if self.eat_punct(':').is_some() {
                if let Some(ty) = self.eat_type() {
                    return Some((field, ty));
                }
            }
        }
        return None;
    }

    // TODO: rewrite that
    pub fn eat_all_struct_fields(&mut self) -> Option<Vec<(TokenTree, TokenStream)>> {
        if self.open_brace() {
            let mut fields = Vec::new();
            while !self.eat_eot() {
                if let Some((field, ty)) = self.eat_struct_field() {
                    fields.push((field, ty));
                    self.eat_punct(',');
                } else {
                    return None;
                }
            }
            return Some(fields);
        }
        return None;
    }

    pub fn eat_group_angle_bracket(&mut self) -> Option<TokenStream> {
        let mut tb = TokenBuilder::new();
        // if we have a <, keep running and keep a < stack

        if let Some(ob) = self.eat_punct('<') {
            tb.extend(ob);
            let mut stack = 1;
            // keep eating things till we are at stack 0 for a ">"
            while stack > 0 {
                if let Some(ob) = self.eat_punct('<') {
                    tb.extend(ob);
                    stack += 1;
                }
                if let Some(cb) = self.eat_punct('>') {
                    tb.extend(cb);
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

    // TODO: rewrite
    pub fn eat_all_types(&mut self) -> Option<(TokenStream, Vec<TokenStream>)> {
        if self.open_paren() {
            let mut whole = TokenBuilder::new();
            whole.push_group(Delimiter::Parenthesis);

            let mut separated = Vec::new();
            while !self.eat_eot() {
                if let Some(tt) = self.eat_type() {
                    whole.stream(tt.clone());
                    separated.push(tt);
                    if let Some(comma) = self.eat_punct(',') {
                        whole.extend(comma);
                    }
                } else {
                    self.backtrack(whole.end());
                    return None;
                }
            }

            whole.pop_group(Delimiter::Parenthesis);
            Some((whole.end(), separated))
        } else {
            None
        }
    }

    pub fn eat_lifetime(&mut self) -> Option<TokenStream> {
        let mut tb = TokenBuilder::new();
        if let Some(ap) = self.eat_punct('\'') {
            tb.extend(ap);
            if let Some(lifetime) = self.eat_any_ident() {
                tb.extend(lifetime);
            } else if let Some(anon) = self.eat_punct('_') {
                tb.extend(anon);
            } else {
                self.backtrack(tb.end());
                return None;
            }
            Some(tb.end())
        } else {
            None
        }
    }

    pub fn eat_double_colon(&mut self) -> Option<TokenStream> {
        if let Some(c1) = self.eat_punct(':') {
            let mut tb = TokenBuilder::new();
            tb.extend(c1);
            if let Some(c2) = self.eat_punct(':') {
                tb.extend(c2);
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
            tb.extend(c1);
            if let Some(c2) = self.eat_punct('>') {
                tb.extend(c2);
                Some(tb.end())
            } else {
                self.backtrack(tb.end());
                None
            }
        } else {
            None
        }
    }

    pub fn eat_triple_dot(&mut self) -> Option<TokenStream> {
        if let Some(c1) = self.eat_punct('.') {
            let mut tb = TokenBuilder::new();
            tb.extend(c1);
            if let Some(c2) = self.eat_punct('.') {
                tb.extend(c2);
                if let Some(c3) = self.eat_punct('.') {
                    tb.extend(c3);
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

    pub fn eat_type_path_segment(&mut self) -> Option<TokenStream> {
        let mut tb = TokenBuilder::new();
        if let Some(ident) = self.eat_any_ident() {
            tb.extend(ident);
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
            tb.extend(star);
            if let Some(ident) = self.eat_ident("cont").or_else(|| self.eat_ident("mut")) {
                tb.extend(ident);
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
            tb.extend(for_ident);
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

    pub fn eat_function_qualifiers(&mut self) -> TokenStream {
        let mut tb = TokenBuilder::new();

        if let Some(async_const) = self.eat_ident("const").or_else(|| self.eat_ident("async")) {
            tb.extend(async_const);
        }

        if let Some(unsafe_ident) = self.eat_ident("unsafe") {
            tb.extend(unsafe_ident);
        }

        if let Some(extern_ident) = self.eat_ident("extern") {
            tb.extend(extern_ident);
            if let Some(abi) = self.eat_literal() {
                tb.extend(abi);
            }
        }

        tb.end()
    }

    pub fn eat_outer_attribute(&mut self) -> Option<TokenStream> {
        let mut tb = TokenBuilder::new();
        let nbr_sign = self.eat_punct('#')?;
        if self.open_bracket() {
            tb.extend(nbr_sign);
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

    // pub fn eat_maybe_named_function_parameters_variadic(&mut self) -> Option<TokenStream> {
    //     let mut tb = TokenBuilder::new();
    //     while let Some(mnp) = self.eat_maybe_named_param() {
    //         let comma = self.eat_punct(',')?;
    //         tb.stream(mnp);
    //         tb.extend(comma);
    //     }
    //     let mnp = self.eat_maybe_named_param()?;
    //     let comma = self.eat_punct(',')?;

    //     tb.stream(mnp);
    //     tb.extend(comma);

    //     while let Some(attr) = self.eat_outer_attribute() {
    //         tb.stream(attr);
    //     }

    //     let triple_dots = self.eat_triple_dot()?;
    //     tb.stream(triple_dots);

    //     Some(tb.end())
    // }

    // pub fn eat_maybe_named_param(&mut self) -> Option<TokenStream> {
    //     let mut tb = TokenBuilder::new();
    //     while let Some(attr) = self.eat_outer_attribute() {
    //         tb.stream(attr);
    //     }
    //     if let Some(ident_or_anon) = self.eat_any_ident().or_else(|| self.eat_punct('_')) {
    //         let colon = self.eat_punct(':')?;
    //         tb.extend(ident_or_anon);
    //         tb.extend(colon);
    //     }
    //     let ty = self.eat_type()?;
    //     tb.stream(ty);

    //     Some(tb.end())
    // }

    // pub fn eat_maybe_named_function_parameters(&mut self) -> Option<TokenStream> {
    //     let mnp1 = self.eat_maybe_named_param()?;
    //     let mut tb = TokenBuilder::new();
    //     tb.stream(mnp1);
    //     while let Some(comma) = self.eat_punct(',') {
    //         let mnp_i = self.eat_maybe_named_param()?;
    //         tb.extend(comma);
    //         tb.stream(mnp_i);
    //     }
    //     if let Some(comma) = self.eat_punct(',') {
    //         tb.extend(comma);
    //     }

    //     Some(tb.end())
    // }

    // pub fn eat_function_parameters_maybe_named_variadic(&mut self) -> Option<TokenStream> {
    //     self.eat_maybe_named_function_parameters().or_else(|| self.eat_maybe_named_function_parameters_variadic())
    // }

    pub fn eat_bare_function_type(&mut self) -> Option<TokenStream> {
        let mut tb = TokenBuilder::new();

        if let Some(for_lt) = self.eat_for_lifetimes() {
            tb.stream(for_lt);
        }
        let fq = self.eat_function_qualifiers();
        tb.stream(fq);

        if let Some(fn_ident) = self.eat_ident("fn") {
            tb.extend(fn_ident);

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

    pub fn eat_simple_path(&mut self) -> Option<TokenStream> {
        let mut tb = TokenBuilder::new();
        if let Some(db) = self.eat_double_colon() {
            tb.stream(db);
        }
        if let Some(sps) = self.eat_any_ident() { // simple path segment, except $crate
            tb.extend(sps);
            while let Some(db) = self.eat_double_colon() {
                if let Some(sps) = self.eat_any_ident() { // simple path segment, except $crate
                    tb.stream(db);
                    tb.extend(sps);
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
                    tb.extend(q);
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
            tb.extend(dyn_ident);
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
                tb.extend(impl_ident);
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

        if let Some((tys, _)) = self.eat_all_types() {
            // parenthesized_type
            tb.stream(tys);
            return Some(tb.end());
        } else if let Some(ittob) = self.eat_impl_trait_type_one_bound() {
            // impl trait one bound
            tb.stream(ittob);
            return Some(tb.end());
        } else if let Some(itotob) = self.eat_impl_trait_type_one_bound() {
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
            tb.extend(never);
            return Some(tb.end());
        } else if let Some(raw_ptr) = self.eat_raw_pointer_type() {
            // raw pointer type
            tb.stream(raw_ptr);
            return Some(tb.end());
        } else if let Some(amp) = self.eat_punct('&') {
            // reference type
            tb.extend(amp);
            if let Some(lt) = self.eat_lifetime() {
                tb.stream(lt);
            }
            if let Some(mut_ident) = self.eat_ident("mut") {
                tb.extend(mut_ident);
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
            tb.extend(punct);
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
                tb.extend(plus);
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
            tb.extend(impl_ident);
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
            tb.extend(dyn_ident);
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
            tb.extend(pub_ident);
            if let Some(tt) = self.eat_group(Delimiter::Bracket) {
                tb.extend(tt);
            }
            Some(tb.end())
        } else {
            None
        }
    }
}
