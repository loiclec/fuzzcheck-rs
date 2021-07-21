use crate::boxed::BoxMutator;
use crate::fuzzcheck_traits::Mutator;
use crate::grammar::mutators::ASTMutator;
use crate::grammar::mutators::ASTSingleVariant;
use crate::tuples::Tuple1Mutator;
use crate::{alternation, either::Either, integer::CharWithinRangeMutator, vector::UnmutateVecToken};
use crate::{
    alternation::AlternationMutator, fixed_len_vector, fixed_len_vector::FixedLenVecMutator, vector::VecMutator,
};

use super::ast::{ASTMapping, ASTMappingKind, AST};

/**
    A type that that can propagate updates to a value (From) to
    another value (To) based on a Token.

    The token should be the UnmutateToken of a mutator.
*/
pub trait IncrementalMapping<From: Clone, To, M: Mutator<From>> {
    /// The `from_value` was already mutated. The `token` encodes how to unmutate `from_value`. Based on `token` and `from_value`,
    /// this function updates the `to_value`.
    fn mutate_value_from_token(&mut self, from_value: &From, to_value: &mut To, token: &M::UnmutateToken);
    /// The `from_value` has not yet been unmutated. The `token` encodes how to unmutate `from_value`. Based on `token` alone,
    /// this function updates the `to_value`.
    fn unmutate_value_from_token(&mut self, to_value: &mut To, token: &M::UnmutateToken);
}

impl<'a, M> IncrementalMapping<Vec<AST>, String, VecMutator<AST, M>> for ASTMappingSequence<'a>
where
    M: Mutator<AST>,
    ASTMapping: IncrementalMapping<AST, String, M>,
{
    fn mutate_value_from_token(
        &mut self,
        from_value: &Vec<AST>,
        to_value: &mut String,
        token: &UnmutateVecToken<AST, M>,
    ) {
        match token {
            UnmutateVecToken::Element(idx, t) => {
                let v = &from_value[*idx];
                let child = &mut self.children[*idx];
                let old_len = child.len;
                child.mutate_value_from_token(v, to_value, t);
                let diff_len = child.len.wrapping_sub(old_len);
                for child in &mut self.children[idx + 1..] {
                    child.start_index = child.start_index.wrapping_add(diff_len);
                }
                *self.len = self.len.wrapping_add(diff_len);
            }
            UnmutateVecToken::Remove(idx) => {
                let start_index = self
                    .children
                    .get(idx.wrapping_sub(1))
                    .map(|c| c.start_index + c.len)
                    .unwrap_or(*self.start_index);

                let v = &from_value[*idx];
                let (new_s, new_c) = v.generate_string_starting_at_idx(start_index);
                let len = new_c.len;

                self.children.insert(*idx, new_c);
                to_value.insert_str(start_index, &new_s);
                for child in &mut self.children[idx + 1..] {
                    child.start_index = child.start_index + len;
                }
                *self.len = *self.len + len;
            }
            UnmutateVecToken::RemoveMany(range) => {
                // I need to add the value v[idx] to s and c and update the following indices
                let vs = &from_value[range.clone()];
                let idx = range.start;

                let mut start_index = self
                    .children
                    .get(idx.wrapping_sub(1))
                    .map(|c| c.start_index + c.len)
                    .unwrap_or(*self.start_index);
                let original_start_index = start_index;
                let mut total_len = 0;

                let mut tmp_s = String::new();
                let mut tmp_cs = vec![];

                for v in vs {
                    let (new_s, new_c) = v.generate_string_starting_at_idx(start_index);
                    let len = new_c.len;
                    start_index += len;

                    tmp_cs.push(new_c);
                    tmp_s.push_str(&new_s);

                    total_len += len;
                }

                crate::vector::insert_many(self.children, idx, tmp_cs.into_iter());
                to_value.insert_str(original_start_index, &tmp_s);

                for child in &mut self.children[range.end..] {
                    child.start_index = child.start_index + total_len;
                }
                *self.len = *self.len + total_len;
            }
            UnmutateVecToken::Insert(idx, _) => {
                let (start_idx, len) = {
                    let c = &self.children[*idx];
                    (c.start_index, c.len)
                };
                to_value.replace_range(start_idx..start_idx + len, "");
                self.children.remove(*idx);

                for child in &mut self.children[*idx..] {
                    child.start_index = child.start_index - len;
                }
                *self.len = *self.len - len;
            }
            UnmutateVecToken::InsertMany(idx, xs) => {
                let (start_idx, len) = {
                    let cs = &self.children[*idx..*idx + xs.len()];
                    let start_index = cs.first().map(|c| c.start_index).unwrap_or(*self.start_index);
                    let mut len = 0;
                    for c in cs {
                        len += c.len;
                    }
                    (start_index, len)
                };
                to_value.replace_range(start_idx..start_idx + len, "");
                self.children.drain(*idx..*idx + xs.len());

                for child in &mut self.children[*idx..] {
                    child.start_index = child.start_index - len;
                }
                *self.len = *self.len - len;
            }
            UnmutateVecToken::Replace(_) => {
                let mut start_index = *self.start_index;
                let original_start_idx = start_index;
                self.children.clear();
                let mut new_s = String::new();
                for x in from_value {
                    let c = x.generate_string_in(&mut new_s, &mut start_index);
                    self.children.push(c);
                }
                to_value.replace_range(*self.start_index..*self.start_index + *self.len, &new_s);
                *self.len = start_index - original_start_idx;
            }
            UnmutateVecToken::Nothing => {}
        }
    }

    fn unmutate_value_from_token(&mut self, to_value: &mut String, token: &UnmutateVecToken<AST, M>) {
        match token {
            UnmutateVecToken::Element(idx, t) => {
                let child = &mut self.children[*idx];
                let old_len = child.len;
                child.unmutate_value_from_token(to_value, &t);
                let diff_len = child.len.wrapping_sub(old_len);
                for child in &mut self.children[idx + 1..] {
                    child.start_index = child.start_index.wrapping_add(diff_len);
                }
                *self.len = self.len.wrapping_add(diff_len);
            }
            UnmutateVecToken::Remove(idx) => {
                let (start_idx, len) = {
                    let c = &self.children[*idx];
                    (c.start_index, c.len)
                };
                to_value.replace_range(start_idx..start_idx + len, "");
                self.children.remove(*idx);

                for child in &mut self.children[*idx..] {
                    child.start_index = child.start_index - len;
                }
                *self.len = *self.len - len;
            }
            UnmutateVecToken::RemoveMany(range) => {
                let (start_idx, len) = {
                    let cs = &self.children[range.clone()];
                    let start_index = cs.first().map(|c| c.start_index).unwrap_or(*self.start_index);
                    let mut len = 0;
                    for c in cs {
                        len += c.len;
                    }
                    (start_index, len)
                };
                to_value.replace_range(start_idx..start_idx + len, "");
                self.children.drain(range.clone());

                for child in &mut self.children[range.start..] {
                    child.start_index = child.start_index - len;
                }
                *self.len = *self.len - len;
            }
            UnmutateVecToken::Insert(idx, x) => {
                let start_index = self
                    .children
                    .get(idx.wrapping_sub(1))
                    .map(|c| c.start_index + c.len)
                    .unwrap_or(*self.start_index);

                // I need to add the value v[idx] to s and c and update the following indices
                let (new_s, new_c) = x.generate_string_starting_at_idx(start_index);
                let len = new_c.len;
                // adjust len of whole mapping
                // and indices of things following them
                *self.len = *self.len + len;
                self.children.insert(*idx, new_c);
                to_value.insert_str(start_index, &new_s);

                for child in &mut self.children[idx + 1..] {
                    child.start_index = child.start_index + len;
                }
            }
            UnmutateVecToken::InsertMany(idx, xs) => {
                let mut start_index = self
                    .children
                    .get(idx.wrapping_sub(1))
                    .map(|c| c.start_index + c.len)
                    .unwrap_or(*self.start_index);
                let original_start_index = start_index;
                let mut total_len = 0;

                let mut tmp_s = String::new();
                let mut tmp_cs = vec![];
                for x in xs {
                    let (new_s, new_c) = x.generate_string_starting_at_idx(start_index);
                    let len = new_c.len;
                    start_index += len;
                    total_len += len;

                    tmp_cs.push(new_c);
                    tmp_s.push_str(&new_s);
                }

                crate::vector::insert_many(self.children, *idx, tmp_cs.into_iter());
                to_value.insert_str(original_start_index, &tmp_s);

                for child in &mut self.children[idx + xs.len()..] {
                    child.start_index = child.start_index + total_len;
                }
                *self.len = *self.len + total_len;
            }
            UnmutateVecToken::Replace(x) => {
                let mut start_index = *self.start_index;
                let original_start_idx = start_index;
                self.children.clear();
                let mut new_s = String::new();
                for x in x {
                    let c = x.generate_string_in(&mut new_s, &mut start_index);
                    self.children.push(c);
                }
                to_value.replace_range(*self.start_index..*self.start_index + *self.len, &new_s);
                *self.len = start_index - original_start_idx;
            }
            UnmutateVecToken::Nothing => {}
        }
    }
}

impl<'a, M> IncrementalMapping<Vec<AST>, String, FixedLenVecMutator<AST, M>> for ASTMappingSequence<'a>
where
    M: Mutator<AST>,
    ASTMapping: IncrementalMapping<AST, String, M>,
{
    fn mutate_value_from_token(
        &mut self,
        from_value: &Vec<AST>,
        to_value: &mut String,
        token: &fixed_len_vector::UnmutateVecToken<AST, M>,
    ) {
        match token {
            fixed_len_vector::UnmutateVecToken::Element(idx, t) => {
                let v = &from_value[*idx];
                let child = &mut self.children[*idx];
                let old_len = child.len;
                child.mutate_value_from_token(v, to_value, t);
                let diff_len = child.len.wrapping_sub(old_len);
                for child in &mut self.children[idx + 1..] {
                    child.start_index = child.start_index.wrapping_add(diff_len);
                }
                *self.len = self.len.wrapping_add(diff_len);
            }
            fixed_len_vector::UnmutateVecToken::Replace(_) => {
                let mut start_index = *self.start_index;
                let original_start_idx = start_index;
                self.children.clear();
                let mut new_s = String::new();
                for x in from_value {
                    let c = x.generate_string_in(&mut new_s, &mut start_index);
                    self.children.push(c);
                }
                to_value.replace_range(*self.start_index..*self.start_index + *self.len, &new_s);
                *self.len = start_index - original_start_idx;
            }
        }
    }

    fn unmutate_value_from_token(&mut self, to_value: &mut String, token: &fixed_len_vector::UnmutateVecToken<AST, M>) {
        match token {
            fixed_len_vector::UnmutateVecToken::Element(idx, t) => {
                let child = &mut self.children[*idx];
                let old_len = child.len;
                child.unmutate_value_from_token(to_value, &t);
                let diff_len = child.len.wrapping_sub(old_len);
                for child in &mut self.children[idx + 1..] {
                    child.start_index = child.start_index.wrapping_add(diff_len);
                }
                *self.len = self.len.wrapping_add(diff_len);
            }
            fixed_len_vector::UnmutateVecToken::Replace(x) => {
                let mut start_index = *self.start_index;
                let original_start_idx = start_index;
                self.children.clear();
                let mut new_s = String::new();
                for x in x {
                    let c = x.generate_string_in(&mut new_s, &mut start_index);
                    self.children.push(c);
                }
                to_value.replace_range(*self.start_index..*self.start_index + *self.len, &new_s);
                *self.len = start_index - original_start_idx;
            }
        }
    }
}

impl<'a, M1, M2> IncrementalMapping<Vec<AST>, String, Either<M1, M2>> for ASTMappingSequence<'a>
where
    M1: Mutator<Vec<AST>>,
    M2: Mutator<Vec<AST>>,
    Self: IncrementalMapping<Vec<AST>, String, M1> + IncrementalMapping<Vec<AST>, String, M2>,
{
    fn mutate_value_from_token(
        &mut self,
        from_value: &Vec<AST>,
        to_value: &mut String,
        token: &<Either<M1, M2> as Mutator<Vec<AST>>>::UnmutateToken,
    ) {
        match token {
            Either::Left(token) => {
                <Self as IncrementalMapping<_, String, M1>>::mutate_value_from_token(self, from_value, to_value, token)
            }
            Either::Right(token) => {
                <Self as IncrementalMapping<_, String, M2>>::mutate_value_from_token(self, from_value, to_value, token)
            }
        }
    }

    fn unmutate_value_from_token(
        &mut self,
        to_value: &mut String,
        token: &<Either<M1, M2> as Mutator<Vec<AST>>>::UnmutateToken,
    ) {
        match token {
            Either::Left(token) => {
                <Self as IncrementalMapping<_, String, M1>>::unmutate_value_from_token(self, to_value, token)
            }
            Either::Right(token) => {
                <Self as IncrementalMapping<_, String, M2>>::unmutate_value_from_token(self, to_value, token)
            }
        }
    }
}

impl<M> IncrementalMapping<AST, String, AlternationMutator<AST, M>> for ASTMapping
where
    M: Mutator<AST>,
    Self: IncrementalMapping<AST, String, M>,
{
    fn mutate_value_from_token(
        &mut self,
        from_value: &AST,
        to_value: &mut String,
        token: &alternation::UnmutateToken<AST, M::UnmutateToken>,
    ) {
        match token {
            alternation::UnmutateToken::Replace(_) => {
                let (new_s, new_c) = from_value.generate_string_starting_at_idx(self.start_index);
                to_value.replace_range(self.start_index..self.start_index + self.len, &new_s);
                self.len = new_c.len;
                self.content = new_c.content;
            }
            alternation::UnmutateToken::Inner(_, token) => {
                self.mutate_value_from_token(from_value, to_value, token);
            }
        }
    }

    fn unmutate_value_from_token(
        &mut self,
        to_value: &mut String,
        token: &alternation::UnmutateToken<AST, M::UnmutateToken>,
    ) {
        match token {
            alternation::UnmutateToken::Replace(x) => {
                let (new_s, new_c) = x.generate_string_starting_at_idx(self.start_index);
                to_value.replace_range(self.start_index..self.start_index + self.len, &new_s);
                self.len = new_c.len;
                self.content = new_c.content;
            }
            alternation::UnmutateToken::Inner(_, token) => {
                self.unmutate_value_from_token(to_value, token);
            }
        }
    }
}

impl<M1, M2> IncrementalMapping<AST, String, Either<M1, M2>> for ASTMapping
where
    M1: Mutator<AST>,
    M2: Mutator<AST>,
    Self: IncrementalMapping<AST, String, M1> + IncrementalMapping<AST, String, M2>,
{
    fn mutate_value_from_token(
        &mut self,
        from_value: &AST,
        to_value: &mut String,
        token: &<Either<M1, M2> as Mutator<AST>>::UnmutateToken,
    ) {
        match token {
            Either::Left(token) => {
                <Self as IncrementalMapping<AST, String, M1>>::mutate_value_from_token(
                    self, from_value, to_value, token,
                );
            }
            Either::Right(token) => {
                <Self as IncrementalMapping<AST, String, M2>>::mutate_value_from_token(
                    self, from_value, to_value, token,
                );
            }
        }
    }

    fn unmutate_value_from_token(
        &mut self,
        to_value: &mut String,
        token: &<Either<M1, M2> as Mutator<AST>>::UnmutateToken,
    ) {
        match token {
            Either::Left(token) => {
                <Self as IncrementalMapping<AST, String, M1>>::unmutate_value_from_token(self, to_value, token);
            }
            Either::Right(token) => {
                <Self as IncrementalMapping<AST, String, M2>>::unmutate_value_from_token(self, to_value, token)
            }
        }
    }
}

impl<'a> IncrementalMapping<char, String, CharWithinRangeMutator> for ASTMappingAtom<'a> {
    fn mutate_value_from_token(
        &mut self,
        from_value: &char,
        to_value: &mut String,
        _token: &<CharWithinRangeMutator as Mutator<char>>::UnmutateToken,
    ) {
        to_value.replace_range(
            *self.start_index..*self.start_index + *self.len,
            &from_value.to_string(),
        );
        *self.len = from_value.len_utf8();
    }

    fn unmutate_value_from_token(
        &mut self,
        to_value: &mut String,
        token: &<CharWithinRangeMutator as Mutator<char>>::UnmutateToken,
    ) {
        to_value.replace_range(*self.start_index..*self.start_index + *self.len, &token.to_string());
        *self.len = token.len_utf8();
    }
}

pub struct ASTMappingSequence<'a> {
    children: &'a mut Vec<ASTMapping>,
    start_index: &'a mut usize,
    len: &'a mut usize,
}
pub struct ASTMappingAtom<'a> {
    start_index: &'a mut usize,
    len: &'a mut usize,
}

type UnmutateTokenSingleVariant<M1, M2, M3> = <ASTSingleVariant<M1, M2, M3> as Mutator<AST>>::UnmutateToken;

impl<M1, M2, M3>
    IncrementalMapping<
        AST,
        String,
        ASTSingleVariant<
            Tuple1Mutator<char, M1>,
            Tuple1Mutator<Vec<AST>, M2>,
            Tuple1Mutator<Box<AST>, BoxMutator<AST, M3>>,
        >,
    > for ASTMapping
where
    M1: Mutator<char>,
    M2: Mutator<Vec<AST>>,
    M3: Mutator<AST>,
    ASTMapping: IncrementalMapping<AST, String, M3>,
    for<'b> ASTMappingAtom<'b>: IncrementalMapping<char, String, M1>,
    for<'b> ASTMappingSequence<'b>: IncrementalMapping<Vec<AST>, String, M2>,
{
    fn mutate_value_from_token(
        &mut self,
        from_value: &AST,
        to_value: &mut String,
        token: &UnmutateTokenSingleVariant<
            Tuple1Mutator<char, M1>,
            Tuple1Mutator<Vec<AST>, M2>,
            Tuple1Mutator<Box<AST>, BoxMutator<AST, M3>>,
        >,
    ) {
        match (token, from_value, &mut self.content) {
            (ASTSingleVariant::Token(token), AST::Token(from_value), ASTMappingKind::Token) => {
                let mut atom = ASTMappingAtom {
                    start_index: &mut self.start_index,
                    len: &mut self.len,
                };
                atom.mutate_value_from_token(from_value, to_value, token);
            }
            (ASTSingleVariant::Sequence(token), AST::Sequence(from_value), ASTMappingKind::Sequence(children)) => {
                let mut sequence = ASTMappingSequence {
                    children,
                    start_index: &mut self.start_index,
                    len: &mut self.len,
                };
                sequence.mutate_value_from_token(from_value, to_value, token);
            }
            (ASTSingleVariant::Box(token), AST::Box(from_value), ASTMappingKind::Box(mapping)) => {
                <ASTMapping as IncrementalMapping<AST, String, M3>>::mutate_value_from_token(
                    mapping, from_value, to_value, token,
                )
            }
            _ => panic!(),
        }
    }

    fn unmutate_value_from_token(
        &mut self,
        to_value: &mut String,
        token: &UnmutateTokenSingleVariant<
            Tuple1Mutator<char, M1>,
            Tuple1Mutator<Vec<AST>, M2>,
            Tuple1Mutator<Box<AST>, BoxMutator<AST, M3>>,
        >,
    ) {
        match (token, &mut self.content) {
            (ASTSingleVariant::Token(token), ASTMappingKind::Token) => {
                let mut atom = ASTMappingAtom {
                    start_index: &mut self.start_index,
                    len: &mut self.len,
                };
                atom.unmutate_value_from_token(to_value, token);
            }
            (ASTSingleVariant::Sequence(token), ASTMappingKind::Sequence(children)) => {
                let mut sequence = ASTMappingSequence {
                    children,
                    start_index: &mut self.start_index,
                    len: &mut self.len,
                };
                sequence.unmutate_value_from_token(to_value, token);
            }
            (ASTSingleVariant::Box(token), ASTMappingKind::Box(mapping)) => {
                <ASTMapping as IncrementalMapping<AST, String, M3>>::unmutate_value_from_token(mapping, to_value, token)
            }
            _ => panic!(),
        }
    }
}

impl IncrementalMapping<AST, String, ASTMutator> for ASTMapping {
    fn mutate_value_from_token(
        &mut self,
        from_value: &AST,
        to_value: &mut String,
        token: &<ASTMutator as Mutator<AST>>::UnmutateToken,
    ) {
        <Self as IncrementalMapping<
            AST,
            String,
            ASTSingleVariant<
                Tuple1Mutator<char, CharWithinRangeMutator>,
                Tuple1Mutator<Vec<AST>, Either<FixedLenVecMutator<AST, ASTMutator>, VecMutator<AST, ASTMutator>>>,
                Tuple1Mutator<Box<AST>, BoxMutator<AST, Either<ASTMutator, AlternationMutator<AST, ASTMutator>>>>,
            >,
        >>::mutate_value_from_token(self, from_value, to_value, token.inner.as_ref());
    }

    fn unmutate_value_from_token(
        &mut self,
        to_value: &mut String,
        token: &<ASTMutator as Mutator<AST>>::UnmutateToken,
    ) {
        <Self as IncrementalMapping<
            AST,
            String,
            ASTSingleVariant<
                Tuple1Mutator<char, CharWithinRangeMutator>,
                Tuple1Mutator<Vec<AST>, Either<FixedLenVecMutator<AST, ASTMutator>, VecMutator<AST, ASTMutator>>>,
                Tuple1Mutator<Box<AST>, BoxMutator<AST, Either<ASTMutator, AlternationMutator<AST, ASTMutator>>>>,
            >,
        >>::unmutate_value_from_token(self, to_value, token.inner.as_ref());
    }
}
