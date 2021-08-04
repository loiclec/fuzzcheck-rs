use crate::traits::MutatorWrapper;

pub struct Wrapper<T>(pub T);
impl<T> MutatorWrapper for Wrapper<T> {
    type Wrapped = T;
    #[no_coverage]
    fn wrapped_mutator(&self) -> &Self::Wrapped {
        &self.0
    }
}

#[macro_export]
macro_rules! recursive_mutator_wrapper {
    (name: $name:ident, wrapped: $t:ty, generics: < $($e:ident),* >  where $($rest:tt)*) => {
        pub struct $name < $($e),* > ($t) where $($rest)*;
        impl< $($e),* > $fuzzcheck_traits::MutatorWrapper for $name < $($e),* > where $($rest)* {
            type Wrapped = $name  < $($e),* > ;
            #[no_coverage] fn wrapped_mutator(&self) -> &Self::Wrapped {
                &self.0
            }
        }
    };
    ($name:ident, $t:ty) => {
        $crate::rec_mutator_wrapper!(name: $name, wrapped: $t, generics: < >  where );
    };
}
