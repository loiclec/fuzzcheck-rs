use std::marker::PhantomData;

use fuzzcheck_traits::Mutator;

use crate::U32WithinRangeMutator;
use crate::{NeverMutator, RefTypes, Tuple1, Tuple1Mutator, TupleMutator, TupleStructure};

pub trait MutatorSuperType<T, SubM>: Mutator<T>
where
    T: Clone,
    SubM: Mutator<T>,
{
    fn upcast(m: SubM) -> Self;
}

impl<T, M> MutatorSuperType<T, NeverMutator> for M
where
    T: Clone,
    M: Mutator<T>,
    EqualProof<NeverMutator, M>: NotEqual,
{
    fn upcast(_m: NeverMutator) -> Self {
        unreachable!()
    }
}
impl<T, M> MutatorSuperType<T, M> for M
where
    T: Clone,
    M: Mutator<T>,
{
    fn upcast(m: M) -> Self {
        m
    }
}

pub trait TupleMutatorSuperType<T, TupleKind, SubM>: TupleMutator<T, TupleKind>
where
    T: Clone,
    TupleKind: RefTypes,
    T: TupleStructure<TupleKind>,
    SubM: TupleMutator<T, TupleKind>,
{
    fn upcast(m: SubM) -> Self;
}

impl<T, TupleKind, M> TupleMutatorSuperType<T, TupleKind, NeverMutator> for M
where
    T: Clone,
    TupleKind: RefTypes,
    T: TupleStructure<TupleKind>,
    M: TupleMutator<T, TupleKind>,
    EqualProof<NeverMutator, M>: NotEqual,
{
    fn upcast(_m: NeverMutator) -> Self {
        unreachable!()
    }
}
impl<T, TupleKind, M> TupleMutatorSuperType<T, TupleKind, M> for M
where
    T: Clone,
    TupleKind: RefTypes,
    T: TupleStructure<TupleKind>,
    M: TupleMutator<T, TupleKind>,
{
    fn upcast(m: M) -> Self {
        m
    }
}

fn upcast_mutators<T, M1, M2>(x: M1, y: M2) -> <M1 as CommonMutatorSuperType<T, M2>>::Output
where
    T: Clone,
    M1: Mutator<T>,
    M2: Mutator<T>,
    M1: CommonMutatorSuperType<T, M2>,
    // M2: MutatorSubtype<T, <M1 as CommonMutatorSuperType<T, M2>>::Output>,
{
    <<M1 as CommonMutatorSuperType<T, M2>>::Output as MutatorSuperType<T, M1>>::upcast(x)
    // <<M1 as CommonMutatorSuperType<T, M2>>::Output as MutatorSuperType<T, M2>>::upcast(y)
}

pub trait CommonMutatorSuperType<T, A>: Mutator<T>
where
    A: Mutator<T>,
    T: Clone,
{
    type Output: Mutator<T> + MutatorSuperType<T, A> + MutatorSuperType<T, Self>;
}

pub trait CommonTupleMutatorSuperType<T, TupleKind, A>: TupleMutator<T, TupleKind>
where
    T: Clone,
    TupleKind: RefTypes,
    A: TupleMutator<T, TupleKind>,
    T: TupleStructure<TupleKind>,
{
    type Output: TupleMutator<T, TupleKind>
        + TupleMutatorSuperType<T, TupleKind, A>
        + TupleMutatorSuperType<T, TupleKind, Self>;
}

// need #![feature(auto_traits)] and #![feature(negative_impls)]
pub auto trait NotEqual {}

pub struct EqualProof<A, B> {
    // functions unconditionally implement auto-traits,
    // this ensures that EqualProof<A, B> always implements NotEqual by default,
    // because all of its fields implement NotEqual
    _phantom: fn(A, B),
}
impl<A> !NotEqual for EqualProof<A, A> {}
// now, it seems like I can use EqualProof<A, B> : NotEqual in impl bounds to prove that A != B
// but does it really work?

impl<T, A> CommonMutatorSuperType<T, A> for A
where
    T: Clone,
    A: Mutator<T>,
{
    type Output = A;
}
impl<T, A> CommonMutatorSuperType<T, NeverMutator> for A
where
    T: Clone,
    EqualProof<NeverMutator, A>: NotEqual,
    A: Mutator<T>,
{
    type Output = A;
}
impl<T, A> CommonMutatorSuperType<T, A> for NeverMutator
where
    T: Clone,
    EqualProof<NeverMutator, A>: NotEqual,
    A: Mutator<T>,
{
    type Output = A;
}

impl<T, TupleKind, A> CommonTupleMutatorSuperType<T, TupleKind, A> for A
where
    T: Clone,
    TupleKind: RefTypes,
    T: TupleStructure<TupleKind>,
    A: TupleMutator<T, TupleKind>,
{
    type Output = A;
}
impl<T, TupleKind, A> CommonTupleMutatorSuperType<T, TupleKind, NeverMutator> for A
where
    T: Clone,
    TupleKind: RefTypes,
    T: TupleStructure<TupleKind>,
    EqualProof<NeverMutator, A>: NotEqual,
    A: TupleMutator<T, TupleKind>,
{
    type Output = A;
}
impl<T, TupleKind, A> CommonTupleMutatorSuperType<T, TupleKind, A> for NeverMutator
where
    T: Clone,
    TupleKind: RefTypes,
    T: TupleStructure<TupleKind>,
    EqualProof<NeverMutator, A>: NotEqual,
    A: TupleMutator<T, TupleKind>,
{
    type Output = A;
}

#[derive(Clone)]
enum EitherX {
    Left(u32),
    Right(u16),
}

use crate as fuzzcheck_mutators;
crate::make_single_variant_mutator! {
    enum EitherX {
        Left(u32),
        Right(u16),
    }
}

// TODO: traits for combining mutators together!
// e.g. two single variant mutators into one many-variants mutator
fn make_mutator_supertype<T, M1, M2>(x: M1, y: M2) -> [<M1 as CommonMutatorSuperType<T, M2>>::Output; 2]
where
    T: Clone,
    M1: Mutator<T>,
    M2: Mutator<T>,
    M1: CommonMutatorSuperType<T, M2>,
{
    [
        <M1 as CommonMutatorSuperType<T, M2>>::Output::upcast(x),
        <M1 as CommonMutatorSuperType<T, M2>>::Output::upcast(y),
    ]
}

macro_rules! make_mutator_supertype {
    ($mfirst: expr, $($m: expr),*) => {
        {
            let combined = $mfirst;
            $(
                let combined = make_mutator_supertype(combined, $m);
            )*
            combined
        }
    };
    ($mfirst: expr, $($m: expr),* ,) => {
        make_mutator_supertype!($mfirst, $($m),*)
    };
}

fn foo<T, M1, M2>(x: M1, y: M2) -> [<M1 as CommonMutatorSuperType<T, M2>>::Output; 2]
where
    T: Clone,
    M1: Mutator<T>,
    M2: Mutator<T>,
    M1: CommonMutatorSuperType<T, M2>,
{
    make_mutator_supertype![x, y,]
}

use super::U16Mutator;
use super::U32Mutator;

type T = EitherX;
type X1 = EitherXSingleVariant<Tuple1Mutator<u32, U32Mutator>, NeverMutator>;
type X2 = EitherXSingleVariant<NeverMutator, Tuple1Mutator<u16, U16Mutator>>;
type X3 = EitherXSingleVariant<Tuple1Mutator<u32, U32Mutator>, Tuple1Mutator<u16, U16Mutator>>;
type Y1 = EitherXSingleVariant<Tuple1Mutator<u32, U32WithinRangeMutator>, NeverMutator>;

fn upcast(x: X1) -> <X1 as CommonMutatorSuperType<T, X2>>::Output {
    <X1 as CommonMutatorSuperType<T, X2>>::Output::upcast(x)
    // match x {
    //     EitherXSingleVariant::Left(m) => EitherXSingleVariant::Left(m),
    //     EitherXSingleVariant::Right(m) => unreachable!(),
    // }
}

fn bar(x: &X1, y: X2) -> <X1 as CommonMutatorSuperType<T, X2>>::Output {
    <X1 as CommonMutatorSuperType<T, X2>>::Output::upcast(y)
}
fn bar2(x: X1, y: X3) -> <X1 as CommonMutatorSuperType<T, X3>>::Output {
    <X1 as CommonMutatorSuperType<T, X3>>::Output::upcast(x)
    //<X1 as CommonMutatorSuperType<T, X3>>::Output::upcast(y)
}
fn bar3(x: &X1, y: X1) -> <X2 as CommonMutatorSuperType<T, X2>>::Output {
    panic!()
}
// fn bar0(x: X1, y: X1) -> <X1 as CommonMutatorSuperType<T, X1>>::Output {
//     panic!()
// }

fn baz(x: &Tuple1Mutator<u32, U32Mutator>) -> bool {
    false
}

fn baz2(x: &Tuple1Mutator<u16, U16Mutator>) -> bool {
    false
}

fn combine(x: X1, y: X2) -> [<X1 as CommonMutatorSuperType<T, X2>>::Output; 2] {
    [
        match x {
            EitherXSingleVariant::Left(m) => EitherXSingleVariant::Left(m),
            EitherXSingleVariant::Right(m) => {
                unreachable!()
            }
        },
        match y {
            EitherXSingleVariant::Left(m) => {
                unreachable!()
            }
            EitherXSingleVariant::Right(m) => EitherXSingleVariant::Right(m),
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn foo() {
        let x: X1 = EitherXSingleVariantMutator::Left(Tuple1Mutator::new(U32Mutator::default()));
        let y: X2 = EitherXSingleVariantMutator::Right(Tuple1Mutator::new(U16Mutator::default()));

        let z = combine(x, y);

        assert!(match &z[0] {
            EitherXSingleVariant::Left(x) => baz(x),
            EitherXSingleVariant::Right(y) => baz2(y),
        })
    }
}
