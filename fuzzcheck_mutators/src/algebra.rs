use fuzzcheck_traits::Mutator;

use crate::{NeverMutator, RefTypes, TupleMutator, TupleStructure};

/* TODO
 * add a way to get a single-variant mutator with no restriction on the generated variant
 * write an AlternationMutator that uses the same logic as the current generated enum mutator
 *
*/

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
    // function pointers unconditionally implement auto-traits,
    // this ensures that EqualProof<A, B> always implements NotEqual by default,
    // because all of its fields implement NotEqual
    _phantom: fn(A, B),
}
impl<A> !NotEqual for EqualProof<A, A> {}
// now, it seems like I can use EqualProof<A, B> : NotEqual in impl bounds to prove that A != B
// but does it really work reliably?

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

pub fn make_mutator_supertype<T, M1, M2>(x: M1, y: M2) -> [<M1 as CommonMutatorSuperType<T, M2>>::Output; 2]
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

#[macro_export]
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
