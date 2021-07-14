extern crate self as fuzzcheck_mutators;

use super::make_mutator;
use crate::alternation;
use crate::vector::UnmutateVecToken;
use crate::BoxMutator;
use crate::{CharWithinRangeMutator, VecMutator};
use alternation::AlternationMutator;
use fuzzcheck_traits::Mutator;

use regex_syntax::hir::Hir;
use regex_syntax::hir::HirKind;

pub enum Grammar {
    Literal(char),
    // etc.
}

/**
    A grammar-conforming string
*/
#[derive(Clone, Debug)]
pub enum GrammarString {
    /// e.g. a character/token literal, possibly chosen from a class like [a-zA-Z0-9] or [abc] or \w or a or ... (etc.)
    Atom(char),
    /// obtained from a repetition , e.g. a+ or a? or a* or a{7} or [a-z]+ or etc.
    /// or:  obtained from a concatenation, e.g. abcd or (ab)c or (a|b)(c+)d+e?9.
    Sequence(Vec<GrammarString>),
    /// e.g.  obtained from an OR pattern, e.g. a|b|c|d
    /// or: from a group with parentheses e.g. (abc)
    Box(Box<GrammarString>),
}

#[derive(Debug)]
pub struct GrammarStringMapping {
    pub start_index: usize,
    pub len: usize,
    pub content: GrammarStringMappingKind,
}
#[derive(Debug)]
pub enum GrammarStringMappingKind {
    Atom,
    Sequence(Vec<GrammarStringMapping>),
    Box(Box<GrammarStringMapping>),
}

#[make_mutator(name: GrammarStringMutator, recursive: true, default: false)]
pub enum GrammarString {
    Atom(#[field_mutator(AlternationMutator<char, CharWithinRangeMutator>)] char),
    Sequence(#[field_mutator(VecMutator<GrammarString, GrammarStringMutator>)] Vec<GrammarString>),
    Box(#[field_mutator(BoxMutator<GrammarString, GrammarStringMutator>)] Box<GrammarString>),
}

impl GrammarString {
    fn generate_string_in(&self, s: &mut String, start_index: &mut usize) -> GrammarStringMapping {
        match self {
            GrammarString::Atom(c) => {
                let len = c.len_utf8();
                let orig_start_index = *start_index;
                s.push(*c);
                *start_index += len;
                GrammarStringMapping {
                    start_index: orig_start_index,
                    len,
                    content: GrammarStringMappingKind::Atom,
                }
            }
            GrammarString::Sequence(sfs) => {
                let original_start_idx = *start_index;
                let mut cs = vec![];
                for sf in sfs {
                    let c = sf.generate_string_in(s, start_index);
                    cs.push(c);
                }
                GrammarStringMapping {
                    start_index: original_start_idx,
                    len: *start_index - original_start_idx,
                    content: GrammarStringMappingKind::Sequence(cs),
                }
            }
            GrammarString::Box(sf) => {
                let original_start_idx = *start_index;
                let c = sf.generate_string_in(s, start_index);
                GrammarStringMapping {
                    start_index: original_start_idx,
                    len: *start_index - original_start_idx,
                    content: GrammarStringMappingKind::Box(Box::new(c)),
                }
            }
        }
    }
    fn generate_string(&self) -> (String, GrammarStringMapping) {
        let mut s = String::new();
        let mut start_index = 0;
        let c = self.generate_string_in(&mut s, &mut start_index);
        (s, c)
    }
}

type GSMUnmutateToken = <GrammarStringMutator as Mutator<GrammarString>>::UnmutateToken;
type GSMCache = <GrammarStringMutator as Mutator<GrammarString>>::Cache;
type GSMMutationStep = <GrammarStringMutator as Mutator<GrammarString>>::MutationStep;
type GSMArbitraryStep = <GrammarStringMutator as Mutator<GrammarString>>::ArbitraryStep;

impl GrammarStringMapping {
    pub fn update_string_from_unmutate_token(
        &mut self,
        token: &GSMUnmutateToken,
        string: &mut String,
        grammar_string: &GrammarString,
    ) {
        match token.inner.as_ref() {
            alternation::UnmutateToken::Replace(x) => {
                let (new_s, new_c) = grammar_string.generate_string();
                string.replace_range(self.start_index..self.start_index + self.len, &new_s);
                self.len = new_c.len;
                self.content = new_c.content;
            }
            alternation::UnmutateToken::Inner(_idx, t) => {
                // let m = &m.mutator.mutator.mutators[*idx];
                match (t, grammar_string, &mut self.content) {
                    (GrammarStringSingleVariant::Atom(_), GrammarString::Atom(v), GrammarStringMappingKind::Atom) => {
                        string.replace_range(self.start_index..self.start_index + self.len, &v.to_string());
                        self.len = v.len_utf8();
                    }
                    (
                        GrammarStringSingleVariant::Sequence(t),
                        GrammarString::Sequence(v),
                        GrammarStringMappingKind::Sequence(children),
                    ) => {
                        match t {
                            UnmutateVecToken::Element(idx, t) => {
                                let v = &v[*idx];
                                let child = &mut children[*idx];
                                let old_len = child.len;
                                child.update_string_from_unmutate_token(t, string, v);
                                let diff_len = child.len.wrapping_sub(old_len);
                                for child in &mut children[idx + 1..] {
                                    child.start_index = child.start_index.wrapping_add(diff_len);
                                }
                                self.len = self.len.wrapping_add(diff_len);
                            }
                            UnmutateVecToken::Remove(idx) => {
                                let v = &v[*idx];
                                let (new_s, mut new_c) = v.generate_string();
                                let len = new_c.len;
                                let start_index = children
                                    .get(idx.wrapping_sub(1))
                                    .map(|c| c.start_index + c.len)
                                    .unwrap_or(self.start_index);
                                new_c.start_index = start_index;
                                children.insert(*idx, new_c);
                                string.insert_str(start_index, &new_s);
                                for child in &mut children[idx + 1..] {
                                    child.start_index = child.start_index + len;
                                }
                                self.len = self.len + len;
                            }
                            UnmutateVecToken::RemoveMany(range) => {
                                // I need to add the value v[idx] to s and c and update the following indices
                                let vs = &v[range.clone()];
                                let idx = range.start;

                                let mut start_index = children
                                    .get(idx.wrapping_sub(1))
                                    .map(|c| c.start_index + c.len)
                                    .unwrap_or(self.start_index);
                                let original_start_index = start_index;
                                let mut total_len = 0;

                                let new_s_and_c = vs.iter().map(|v| v.generate_string());
                                let mut tmp_s = String::new();
                                let mut tmp_cs = vec![];

                                for (new_s, mut new_c) in new_s_and_c {
                                    let len = new_c.len;
                                    new_c.start_index = start_index;
                                    start_index += len;

                                    tmp_cs.push(new_c);
                                    tmp_s.push_str(&new_s);

                                    total_len += len;
                                }

                                crate::vector::insert_many(children, idx, tmp_cs.into_iter());
                                string.insert_str(original_start_index, &tmp_s);

                                for child in &mut children[range.end..] {
                                    child.start_index = child.start_index + total_len;
                                }
                                self.len = self.len + total_len;
                            }
                            UnmutateVecToken::Insert(idx, _) => {
                                let (start_idx, len) = {
                                    let c = &children[*idx];
                                    (c.start_index, c.len)
                                };
                                string.replace_range(start_idx..start_idx + len, "");
                                children.remove(*idx);

                                for child in &mut children[*idx..] {
                                    child.start_index = child.start_index - len;
                                }
                                self.len = self.len - len;
                            }
                            UnmutateVecToken::InsertMany(idx, xs) => {
                                let (start_idx, len) = {
                                    let cs = &children[*idx..*idx + xs.len()];
                                    let start_index = cs.first().map(|c| c.start_index).unwrap_or(self.start_index);
                                    let mut len = 0;
                                    for c in cs {
                                        len += c.len;
                                    }
                                    (start_index, len)
                                };
                                string.replace_range(start_idx..start_idx + len, "");
                                children.drain(*idx..*idx + xs.len());

                                for child in &mut children[*idx..] {
                                    child.start_index = child.start_index - len;
                                }
                                self.len = self.len - len;
                            }
                            UnmutateVecToken::Replace(_) => {
                                let mut start_index = self.start_index;
                                let original_start_idx = start_index;
                                children.clear();
                                let mut new_s = String::new();
                                for x in v {
                                    let c = x.generate_string_in(&mut new_s, &mut start_index);
                                    children.push(c);
                                }
                                string.replace_range(self.start_index..self.start_index + self.len, &new_s);
                                self.len = start_index - original_start_idx;
                            }
                            UnmutateVecToken::Nothing => {}
                        }
                    }
                    _ => unreachable!(),
                }
            }
        }
    }

    pub fn unmutate_string_from_unmutate_token(&mut self, token: &GSMUnmutateToken, string: &mut String) {
        match token.inner.as_ref() {
            alternation::UnmutateToken::Replace(x) => {
                let (new_s, new_c) = x.generate_string();
                string.replace_range(self.start_index..self.start_index + self.len, &new_s);
                self.len = new_c.len;
                self.content = new_c.content;
            }
            alternation::UnmutateToken::Inner(_idx, t) => {
                // let m = &m.mutator.mutator.mutators[*idx];
                match (t, &mut self.content) {
                    (GrammarStringSingleVariant::Atom(l), GrammarStringMappingKind::Atom) => {
                        let l = match l {
                            alternation::UnmutateToken::Replace(l) => l,
                            alternation::UnmutateToken::Inner(_, l) => l,
                        };
                        string.replace_range(self.start_index..self.start_index + self.len, &l.to_string());
                        self.len = l.len_utf8();
                    }
                    (GrammarStringSingleVariant::Sequence(t), GrammarStringMappingKind::Sequence(children)) => {
                        match t {
                            UnmutateVecToken::Element(idx, t) => {
                                let child = &mut children[*idx];
                                let old_len = child.len;
                                child.unmutate_string_from_unmutate_token(&t, string);
                                let diff_len = child.len.wrapping_sub(old_len);
                                for child in &mut children[idx + 1..] {
                                    child.start_index = child.start_index.wrapping_add(diff_len);
                                }
                                self.len = self.len.wrapping_add(diff_len);
                            }
                            UnmutateVecToken::Remove(idx) => {
                                let (start_idx, len) = {
                                    let c = &children[*idx];
                                    (c.start_index, c.len)
                                };
                                string.replace_range(start_idx..start_idx + len, "");
                                children.remove(*idx);

                                for child in &mut children[*idx..] {
                                    child.start_index = child.start_index - len;
                                }
                                self.len = self.len - len;
                            }
                            UnmutateVecToken::RemoveMany(range) => {
                                let (start_idx, len) = {
                                    let cs = &children[range.clone()];
                                    let start_index = cs.first().map(|c| c.start_index).unwrap_or(self.start_index);
                                    let mut len = 0;
                                    for c in cs {
                                        len += c.len;
                                    }
                                    (start_index, len)
                                };
                                string.replace_range(start_idx..start_idx + len, "");
                                children.drain(range.clone());

                                for child in &mut children[range.start..] {
                                    child.start_index = child.start_index - len;
                                }
                                self.len = self.len - len;
                            }
                            UnmutateVecToken::Insert(idx, x) => {
                                // I need to add the value v[idx] to s and c and update the following indices
                                let (new_s, mut new_c) = x.generate_string();
                                let len = new_c.len;
                                let start_index = children
                                    .get(idx.wrapping_sub(1))
                                    .map(|c| c.start_index + c.len)
                                    .unwrap_or(self.start_index);
                                new_c.start_index = start_index;
                                children.insert(*idx, new_c);
                                string.insert_str(start_index, &new_s);
                                for child in &mut children[idx + 1..] {
                                    child.start_index = child.start_index + len;
                                }
                                self.len = self.len + len;
                            }
                            UnmutateVecToken::InsertMany(idx, xs) => {
                                let mut start_index = children
                                    .get(idx.wrapping_sub(1))
                                    .map(|c| c.start_index + c.len)
                                    .unwrap_or(self.start_index);
                                let original_start_index = start_index;
                                let mut total_len = 0;

                                let new_s_and_c = xs.iter().map(|v| v.generate_string());
                                let mut tmp_s = String::new();
                                let mut tmp_cs = vec![];

                                for (new_s, mut new_c) in new_s_and_c {
                                    let len = new_c.len;
                                    new_c.start_index = start_index;
                                    start_index += len;

                                    tmp_cs.push(new_c);
                                    tmp_s.push_str(&new_s);

                                    total_len += len;
                                }

                                crate::vector::insert_many(children, *idx, tmp_cs.into_iter());
                                string.insert_str(original_start_index, &tmp_s);

                                for child in &mut children[idx + xs.len()..] {
                                    child.start_index = child.start_index + total_len;
                                }
                                self.len = self.len + total_len;
                            }
                            UnmutateVecToken::Replace(x) => {
                                let mut start_index = self.start_index;
                                let original_start_idx = start_index;
                                children.clear();
                                let mut new_s = String::new();
                                for x in x {
                                    let c = x.generate_string_in(&mut new_s, &mut start_index);
                                    children.push(c);
                                }
                                string.replace_range(self.start_index..self.start_index + self.len, &new_s);
                                self.len = start_index - original_start_idx;
                            }
                            UnmutateVecToken::Nothing => {}
                        }
                    }
                    _ => unreachable!(),
                }
            }
        }
    }
}

pub struct GrammarBasedStringMutator {
    grammar: regex_syntax::hir::Hir,
    grammar_string_mutator: GrammarStringMutator,
}
pub struct Cache {
    grammar_string: GrammarString,
    grammar_string_mutator_cache: GSMCache,
    mapping: GrammarStringMapping,
}
impl GrammarBasedStringMutator {
    fn cache_from_grammar_string(&self, gs: GrammarString) -> (Cache, GSMMutationStep, String) {
        let (grammar_string_mutator_cache, mutation_step) = self.grammar_string_mutator.validate_value(&gs).unwrap();
        let (string, mapping) = gs.generate_string();
        (
            Cache {
                grammar_string: gs,
                grammar_string_mutator_cache,
                mapping,
            },
            mutation_step,
            string,
        )
    }
}
impl Mutator<String> for GrammarBasedStringMutator {
    type Cache = Cache;
    type MutationStep = GSMMutationStep;
    type ArbitraryStep = GSMArbitraryStep;
    type UnmutateToken = GSMUnmutateToken;

    fn default_arbitrary_step(&self) -> Self::ArbitraryStep {
        self.grammar_string_mutator.default_arbitrary_step()
    }

    fn validate_value(&self, value: &String) -> Option<(Self::Cache, Self::MutationStep)> {
        // let grammar_string = ??;
        // let (cache, string) = self.cache_from_grammar_string(gs)
    }

    fn max_complexity(&self) -> f64 {
        todo!()
    }

    fn min_complexity(&self) -> f64 {
        todo!()
    }

    fn complexity(&self, value: &String, cache: &Self::Cache) -> f64 {
        todo!()
    }

    fn ordered_arbitrary(&self, step: &mut Self::ArbitraryStep, max_cplx: f64) -> Option<(String, f64)> {
        todo!()
    }

    fn random_arbitrary(&self, max_cplx: f64) -> (String, f64) {
        todo!()
    }

    fn ordered_mutate(
        &self,
        value: &mut String,
        cache: &mut Self::Cache,
        step: &mut Self::MutationStep,
        max_cplx: f64,
    ) -> Option<(Self::UnmutateToken, f64)> {
        todo!()
    }

    fn random_mutate(&self, value: &mut String, cache: &mut Self::Cache, max_cplx: f64) -> (Self::UnmutateToken, f64) {
        todo!()
    }

    fn unmutate(&self, value: &mut String, cache: &mut Self::Cache, t: Self::UnmutateToken) {
        todo!()
    }
}
