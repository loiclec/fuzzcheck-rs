use std::marker::PhantomData;

pub enum Either2<T0, T1> {
    T0(T0),
    T1(T1),
}
