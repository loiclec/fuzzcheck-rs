
use decent_synquote_alternative as synquote;
use proc_macro2::{Ident, Span, TokenStream};

use synquote::parser::*;
use synquote::token_builder::*;

fn make_enum_n_payload_structure(tb: &mut TokenBuilder, n: usize, fuzzcheck_mutators_crate: TokenStream) {
    let clone = ts!("::std::clone::Clone");
    let T = |i: usize| ident!("T" i);
    let n_plus_1_type_params = join_ts!(0..n + 1, i, T(i), separator: ",");
    let EitherNP1 = ident!("Either" n+1);
    let EnumNPayloadStructure = ident!("Enum" n "PayloadStructure");

    let TupleKind = |i: usize| ident!("TupleKind" i);

    let RefTypes = ts!(fuzzcheck_mutators_crate "::RefTypes");
    let TupleStructure = ts!(fuzzcheck_mutators_crate "::TupleStructure");

    let SelfTupleKindAsRefTypes = |i: usize| ts!("<Self::" TupleKind(i) "as" RefTypes ">" );

    extend_ts!(tb,
        "#[derive(" clone ")]
        pub enum" EitherNP1 "<" n_plus_1_type_params "> {"
            join_ts!(0 .. n+1, i,
                T(i)"(" T(i) "),"
            )
        "}
        
        pub trait" EnumNPayloadStructure "{"
            join_ts!(0..n, i,
                "type " TupleKind(i) ":" RefTypes ";"
                "type" T(i) ":" TupleStructure "<Self::" TupleKind(i) ">;"
            )
            "fn get_ref<'a>(&'a self) ->" EitherNP1 "<" join_ts!(0..n, i, SelfTupleKindAsRefTypes(i)"::Ref<'a> ,") " usize>;"
            "fn get_mut<'a>(&'a mut self) ->" EitherNP1 "<" join_ts!(0..n, i, SelfTupleKindAsRefTypes(i)"::Mut<'a>,") " usize>;"
            "fn new(t:" EitherNP1 "<" join_ts!(0..n, i, "Self::" T(i) ",") " usize>) -> Self;"
        "}"
    )
}

fn make_enum_n_payload_mutator(tb: &mut TokenBuilder, n: usize, fuzzcheck_mutators_crate: TokenStream) {
    let clone = ts!("::std::clone::Clone");
    let EnumNPayloadMutator = ident!("Enum" n "PayloadMutator");
    let T = |i: usize| ident!("T" i);
    let M = |i: usize| ident!("M" i);
    let TupleKind = |i: usize| ident!("TupleKind" i);
    let TupleStructure = |i: usize| ts!(fuzzcheck_mutators_crate "::TupleStructure<" TupleKind(i) ">");
    let RefTypes = ts!(fuzzcheck_mutators_crate "::RefTypes");
    let TupleMutator = |i: usize| ts!(fuzzcheck_mutators_crate "::TupleMutator<" T(i) "," TupleKind(i) ">");
    let mutator_ = |i: usize| ident!("mutator_" i);
    let fastrand = ts!(fuzzcheck_mutators_crate "::fastrand");
    let PhantomData = ts!("::std::marker::PhantomData");
    let Default = ts!("::std::default::Default");

    let type_params = join_ts!(0..n, i,
        T(i) "," M(i) "," TupleKind(i)
    , separator: ",") ;
    let where_clause = join_ts!(0..n, i, 
        T(i) ":" clone " +" TupleStructure(i) ","
        TupleKind(i) ":" RefTypes ","
        M(i) ":" TupleMutator(i)
    , separator: ",");

    extend_ts!(tb, "
    pub struct" EnumNPayloadMutator "<" type_params "> where" where_clause "{"
    join_ts!(0..n, i, 
        "pub" mutator_(i) ":" M(i) ","
    )
        "rng: " fastrand "::Rng ,
        _phantom: " PhantomData "<(" join_ts!(0..n, i, T(i) "," TupleKind(i), separator: "," ) ")>"
    "}

    impl<" type_params "> " EnumNPayloadMutator "<" type_params "> where" where_clause "{
        pub fn new(" join_ts!(0..n, i, mutator_(i) ":" M(i), separator: "," ) ") -> Self {
            Self {"
                join_ts!(0..n, i, 
                    mutator_(i) ","
                )
                "rng: <_>::default(),
                _phantom: <_>::default()
            }
        }
    }


    impl<" type_params "> " Default "for" EnumNPayloadMutator "<" type_params "> where" where_clause  ","
        join_ts!(0..n, i,
            M(i) ":" Default
        , separator: ",") 
    "{
        fn default() -> Self {
            Self {"
                join_ts!(0..n, i, 
                    mutator_(i) ": <_>::default(),"
                )
                "rng: <_>::default(),
                _phantom: <_>::default()
            }
        }
    }
    ")
}

fn make_enum_mutator_helper_types(tb: &mut TokenBuilder, n: usize) {
    let clone = ts!("::std::clone::Clone");
    let vec = ts!("::std::vec::Vec");
    let Default = ts!("::std::default::Default");
    let EitherNP1 = ident!("Either" n+1);
    let EnumNPayloadArbitraryStep = ident!("Enum" n "PayloadArbitraryStep");
    let EnumNPayloadMutationStep = ident!("Enum" n "PayloadMutationStep");
    let T = |i: usize| ident!("T" i);
    let type_params = join_ts!(0..n, i, T(i), separator: ",");

    extend_ts!(tb,
        "#[derive(" clone ")]
        pub struct" EnumNPayloadArbitraryStep "<" join_ts!(0..n, i, T(i) ":" Default, separator: ",") "> {
            steps: " vec "<" EitherNP1 "<" type_params ", usize > >,
            idx: usize,
        }
        impl<" join_ts!(0..n, i, T(i) ":" Default, separator: ",") ">" Default "for" EnumNPayloadArbitraryStep "<" type_params "> {
            fn default() -> Self {
                Self {
                    steps: vec![" join_ts!(0..n, i, EitherNP1 "::" T(i) "(" T(i) "::default()) ," ) EitherNP1 "::" T(n) "(0)],
                    idx: 0,
                }
            }
        }
        #[derive(" clone ")]
        pub struct " EnumNPayloadMutationStep "<" type_params ", AS> {
            inner: " EitherNP1 "<" type_params ", ()>,
            arbitrary: AS,
        }
        "
    )
}

fn impl_mutator(tb: &mut TokenBuilder, n: usize, fuzzcheck_mutators_crate: TokenStream) {
    let clone = ts!("::std::clone::Clone");
    let Option = ts!("::std::option::Option");
    let some = ts!(Option "::Some");
    let none = ts!(Option "::None");

    let EnumNPayloadStructure = ident!("Enum" n "PayloadStructure");
    let EnumNPayloadMutator = ident!("Enum" n "PayloadMutator");
    let EnumNPayloadArbitraryStep = ident!("Enum" n "PayloadArbitraryStep");
    let EnumNPayloadMutationStep = ident!("Enum" n "PayloadMutationStep");

    let Either = ident!("Either" n+1);
    let T = |i: usize| ident!("T" i);
    let EitherT = |i: usize| ts!(Either "::" T(i));
    let M = |i: usize| ident!("M" i);
    let TupleKind = |i: usize| ident!("TupleKind" i);
    let TupleStructure = |i: usize| ts!(fuzzcheck_mutators_crate "::TupleStructure<" TupleKind(i) ">");
    let RefTypes = ts!(fuzzcheck_mutators_crate "::RefTypes");
    let TupleMutator = |i: usize| ts!(fuzzcheck_mutators_crate "::TupleMutator<" T(i) "," TupleKind(i) ">");
    let mutator_ = |i: usize| ident!("mutator_" i);
    let Mutator = ts!("::fuzzcheck_traits::Mutator");
    let variant_count = ts!("::std::mem::variant_count::<T>()");
    let size_to_cplxity = ts!(fuzzcheck_mutators_crate "::size_to_cplxity");

    extend_ts!(tb, "
    impl<T," join_ts!(0..n, i, T(i) "," M(i) "," TupleKind(i), separator: "," ) "> " Mutator "<T>
        for " EnumNPayloadMutator "<" join_ts!(0..n, i, T(i) "," M(i) "," TupleKind(i), separator: "," ) "> 
    where
        T: " clone "+ " EnumNPayloadStructure "<" join_ts!(0..n, i, TupleKind(i) "=" TupleKind(i) "," T(i) "=" T(i) "," ) ">" ","
    join_ts!(0..n, i,
        T(i) ":" clone " + " TupleStructure(i) ","
        M(i) ":" TupleMutator(i) ","
        TupleKind(i) ":" RefTypes
    , separator: ",")
    "{
        type Cache = " Either "<" join_ts!(0..n, i, M(i) "::Cache ,") "()>;
        type MutationStep = " EnumNPayloadMutationStep "<" join_ts!(0..n, i, M(i) "::MutationStep ,") "Self::ArbitraryStep>;
        type ArbitraryStep = " EnumNPayloadArbitraryStep "<" join_ts!(0..n, i, M(i) "::ArbitraryStep", separator: ",") ">;
        type UnmutateToken = " Either "<" join_ts!(0..n, i, M(i) "::UnmutateToken ,") "(T, Self::Cache)>;
    
        fn cache_from_value(&self, value: &T) -> Self::Cache {
            let x = value.get_ref();
            match x {"
            join_ts!(0..n, i,
                EitherT(i) "(x) =>" EitherT(i) "(self." mutator_(i) ".cache_from_value(x)),"
            )
            EitherT(n) "(_) =>" EitherT(n) "(()),"
            "}
        }
        fn initial_step_from_value(&self, value: &T) -> Self::MutationStep {
            match value.get_ref() {"
            join_ts!(0..n, i,
                EitherT(i) "(x) => Self::MutationStep {
                    inner:" EitherT(i) "(self." mutator_(i) ".initial_step_from_value(x)),
                    arbitrary: <_>::default(),
                }," 
            )
                EitherT(n) "(_) => Self::MutationStep {
                    inner:" EitherT(n) "(()),
                    arbitrary: <_>::default(),
                },
            }
        }
        fn max_complexity(&self) -> f64 {
            " size_to_cplxity "(" variant_count ")
                + ["
                join_ts!(0..n, i,
                    "self." mutator_(i) ".max_complexity()"
                , separator: ",")
                "]
                    .iter()
                    .max_by(|x, y| x.partial_cmp(y).unwrap_or(Ordering::Equal))
                    .unwrap()
        }
        fn min_complexity(&self) -> f64 {
            " size_to_cplxity "(" variant_count ")
                + ["
                join_ts!(0..n, i,
                    "self." mutator_(i) ".min_complexity()"
                , separator: ",")
                "]
                    .iter()
                    .min_by(|x, y| x.partial_cmp(y).unwrap_or(Ordering::Equal))
                    .unwrap()
        }
        fn complexity(&self, value: &T, cache: &Self::Cache) -> f64 {
            " size_to_cplxity "(" variant_count ")
                + match (value.get_ref(), cache) {"
                join_ts!(0..n, i,
                    "(" EitherT(i) "(value), " EitherT(i) "(cache)) => self." mutator_(i) ".complexity(value, cache),"
                )
                    "(" EitherT(n) "(_)," EitherT(n) "(_)) => 0.0,
                    _ => unreachable!(),
                }
        }
        fn ordered_arbitrary(&mut self, step: &mut Self::ArbitraryStep, max_cplx: f64) -> Option<(T, Self::Cache)> {
            if step.steps.is_empty() {
                return None;
            }
            let steps_len = step.steps.len();
            let inner_max_cplx = max_cplx - " size_to_cplxity "(" variant_count ");
            let substep = &mut step.steps[step.idx % steps_len];
    
            let result = match substep {"
            join_ts!(0..n, i,
                EitherT(i) "(substep) => self
                    ." mutator_(i)
                    ".ordered_arbitrary(substep, inner_max_cplx)
                    .map(|(value, cache)| 
                        (T::new(" EitherT(i) "(value)), " EitherT(i) "(cache))
                    ),"
            )
                EitherT(n) "(x) => {
                    *x += 1;
                    if *x <= " variant_count " - " n " {
                        Some((T::new(" EitherT(n) "(*x)), " EitherT(n) "(())))
                    } else {
                        None
                    }
                }"
            "};
            if let Some(result) = result {
                step.idx += 1;
                Some(result)
            } else {
                step.steps.remove(step.idx % steps_len);
                self.ordered_arbitrary(step, max_cplx)
            }
        }
        fn random_arbitrary(&mut self, max_cplx: f64) -> (T, Self::Cache) {
            let inner_max_cplx = max_cplx - " size_to_cplxity "(" variant_count ");
            let nbr_variants = if " variant_count " > " n " { " n+1 " } else { " n " };
            match self.rng.usize(..nbr_variants) {"
            join_ts!(0..n, i,
                i "=> {
                    let (v, c) = self." mutator_(i) ".random_arbitrary(inner_max_cplx);
                    (T::new(" EitherT(i) "(v)), " EitherT(i) "(c))
                }"
            )
                n "=> {
                    let pick = self.rng.usize(.." variant_count " - " n ");
                    (T::new(" EitherT(n) "(pick)), " EitherT(n) "(()))
                }
                _ => {
                    unreachable!()
                }
            }
        }
        fn ordered_mutate(
            &mut self,
            value: &mut T,
            cache: &mut Self::Cache,
            step: &mut Self::MutationStep,
            max_cplx: f64,
        ) -> " Option "<Self::UnmutateToken> {
            let inner_max_cplx = max_cplx - " size_to_cplxity "(" variant_count ");
            match (value.get_mut(), cache.borrow_mut(), &mut step.inner) {"
            join_ts!(0..n, i,
                "(" EitherT(i) "(inner_value)," EitherT(i) "(inner_cache)," EitherT(i) "(inner_step)) => {
                    if let " some "(token) = self
                        ." mutator_(i) "
                        .ordered_mutate(inner_value, inner_cache, inner_step, inner_max_cplx)
                    {
                        return " some "(" EitherT(i) "(token));
                    }
                }"
            )
            "   (" EitherT(n) "(_), " EitherT(n) "(_), " EitherT(n) "(_)) => {}
                _ => unreachable!(),
            }
            if let " some "((new_value, new_cache)) = self.ordered_arbitrary(&mut step.arbitrary, max_cplx) {
                let old_value = ::std::mem::replace(value, new_value);
                let old_cache = ::std::mem::replace(cache, new_cache);
                " some "(" EitherT(n) "((old_value, old_cache)))
            } else {
                " none "
            }
        }
        fn random_mutate(&mut self, value: &mut T, cache: &mut Self::Cache, max_cplx: f64) -> Self::UnmutateToken {
            let inner_max_cplx = max_cplx - " size_to_cplxity "(" variant_count ");
            match (value.get_mut(), cache.borrow_mut()) {"
            join_ts!(0..n, i,
                "(" EitherT(i) "(inner_value), " EitherT(i) "(inner_cache)) => {
                    return " EitherT(i) "(self." mutator_(i) ".random_mutate(inner_value, inner_cache, inner_max_cplx))
                }"
            )
            "   (" EitherT(n) "(_), " EitherT(n) "(_)) => {}
                _ => unreachable!(),
            }
            let (new_value, new_cache) = self.random_arbitrary(max_cplx);
            let old_value = ::std::mem::replace(value, new_value);
            let old_cache = ::std::mem::replace(cache, new_cache);
            " EitherT(n) "((old_value, old_cache))
        }
        fn unmutate(&self, value: &mut T, cache: &mut Self::Cache, t: Self::UnmutateToken) {
            let (old_value, old_cache) = match (value.get_mut(), cache.borrow_mut(), t) {"
            join_ts!(0..n, i,
                "(" EitherT(i) "(inner_value), " EitherT(i) "(inner_cache), " EitherT(i) "(token)) => {
                    self." mutator_(i) ".unmutate(inner_value, inner_cache, token);
                    return;
                }"
            )
            "  (_, _, " EitherT(n) "((old_value, old_cache))) => (old_value, old_cache),
                _ => unreachable!(),
            };
            let _ = ::std::mem::replace(value, old_value);
            let _ = ::std::mem::replace(cache, old_cache);
        }
    }"
    )
}

pub fn make_basic_enum_mutator(tb: &mut TokenBuilder, n: usize, fuzzcheck_mutators_crate: TokenStream) {
    make_enum_n_payload_structure(tb, n, fuzzcheck_mutators_crate.clone());
    make_enum_n_payload_mutator(tb, n, fuzzcheck_mutators_crate.clone());
    make_enum_mutator_helper_types(tb, n);
    impl_mutator(tb, n, fuzzcheck_mutators_crate);
}

pub fn impl_enum_structure_trait(tb: &mut TokenBuilder, enu: &Enum, fuzzcheck_mutators_crate: TokenStream) {
    
    let items_with_fields = enu.items.iter().enumerate().filter_map(|(i, item)| {
        match &item.data {
            Some(EnumItemData::Struct(_, fields)) if fields.len() > 0 => {
                Some((i, fields))
            }
            _ => None
        }
    }).collect::<Box<_>>();
    let items_without_fields = enu.items.iter().enumerate().filter_map(|(_, item)| {
        match &item.data {
            Some(EnumItemData::Struct(_, fields)) if fields.len() > 0 => {
                None
            }
            _ => Some(item)
        }
    }).collect::<Box<_>>();
    let n = items_with_fields.len();

    let EnumNPayloadStructure = ts!(fuzzcheck_mutators_crate "::" ident!("Enum" n "PayloadStructure"));

    let generics_no_eq = enu.generics.removing_eq_type();
    let generics_no_eq_nor_bounds = enu.generics.removing_bounds_and_eq_type();

    let mut where_clause = enu.where_clause.clone().unwrap_or_default();
    for tp in &enu.generics.type_params {
        where_clause.items.push(WhereClauseItem {
            for_lifetimes: None,
            lhs: tp.type_ident.clone(),
            rhs: ts!("'static"),
        });
    }
    let TupleKind = |i: usize| ident!("TupleKind" i);
    let Tuple = |i: usize| ts!(fuzzcheck_mutators_crate "::" ident!("Tuple" i));
    let T = |i: usize| ident!("T" i);

    let field_types = items_with_fields.iter().map(|(_, fields)| {
        join_ts!(fields.iter(), field, field.ty, separator: ",")
    }).collect::<Box<_>>();
    let field_types_ref = items_with_fields.iter().map(|(_, fields)| {
        join_ts!(fields.iter(), field, "&'a" field.ty, separator: ",")
    }).collect::<Box<_>>();
    let field_types_mut = items_with_fields.iter().map(|(_, fields)| {
        join_ts!(fields.iter(), field, "&'a mut" field.ty, separator: ",")
    }).collect::<Box<_>>();

    let EitherN = ts!(fuzzcheck_mutators_crate "::" ident!("Either" n+1));

    let either_owned = ts!(
        EitherN "<" 
            join_ts!(0..items_with_fields.len(), i, 
                "Self::" T(i) ","
            )
            "usize"
        ">"
    );
    let either_ref = ts!(
        EitherN "<" 
            join_ts!(0..n, i, 
                "(" field_types_ref[i] ") ,"
            )
            "usize"
        ">"
    );
    let either_mut = ts!(
        EitherN "<" 
            join_ts!(0..n, i, 
                "(" field_types_mut[i] ") ,"
            )
            "usize"
        ">"
    );

    let match_get_ref_or_mut = {
        {
            let mut count_no_data: isize = -1;
            let mut count_data: isize = -1;
            join_ts!(&enu.items, item, 
                item.pattern_match(&enu.ident, None) "=> {"
                EitherN "::" match &item.data {
                    Some(EnumItemData::Struct(_, fields)) if fields.len() > 0 => {
                        count_data += 1;
                        ts!(
                            T(count_data as usize) "((" 
                                join_ts!(fields.iter(), f, 
                                    f.safe_ident()
                                , separator: ",")
                            "))"
                        )
                    }
                    _ => {
                        count_no_data += 1;
                        ts!(T(n) "(" { count_no_data as usize } ")")
                    }
                }
                "}"
            )
        }
    };

    extend_ts!(tb,
        "
        #[allow(non_shorthand_field_patterns)]
        impl" generics_no_eq EnumNPayloadStructure 
            "for" enu.ident generics_no_eq_nor_bounds where_clause 
        "{"
        join_ts!(0..n, i,
            "type" TupleKind(i) "=" Tuple(items_with_fields[i].1.len()) "<" field_types[i] "> ;"
            "type" T(i) "=" "(" field_types[i] ");"
        )
        "
        fn get_ref<'a>(&'a self) -> " either_ref "{"
            "match self {"
                match_get_ref_or_mut
            "}
        }
        fn get_mut<'a>(&'a mut self) -> " either_mut "{"
            "match self {"
                match_get_ref_or_mut
            "}
        }
        fn new(t: " either_owned ") -> Self {
            match t {"
            join_ts!(0..n, i, 
                EitherN "::" T(i) "(x) => {"
                    enu.ident "::" enu.items[items_with_fields[i].0].ident "{"
                        if items_with_fields[i].1.len() == 1 {
                            ts!(items_with_fields[i].1[0].access() ": x")
                        } else {
                            join_ts!(items_with_fields[i].1.iter().enumerate(), (i, field), 
                                field.access() ": x." i 
                            , separator: ",")
                        }
                    "}"
                "}"
            )
                EitherN "::" T(n) "(x) => match x %" enu.items.len() - n "{"
                    join_ts!(0..enu.items.len() - n, i, 
                        i "=> Self::" items_without_fields[i].ident match items_without_fields[i].data {
                            Some(EnumItemData::Struct(_, _)) => {
                                ts!("{}")
                            }
                            _ => {
                                ts!()
                            }
                        } ","
                    )
                    "_ => unreachable!() ,"
                "}"
            "}
        }
    }"
    )
}

pub fn impl_default_mutator_for_enum(tb: &mut TokenBuilder, enu: &Enum, fuzzcheck_mutators_crate: TokenStream) {
    let items_with_fields = enu.items.iter().enumerate().filter_map(|(i, item)| {
        match &item.data {
            Some(EnumItemData::Struct(_, fields)) if fields.len() > 0 => {
                Some((i, fields))
            }
            _ => None
        }
    }).collect::<Box<_>>();

    let n = items_with_fields.len();

    let EnumNPayloadMutator = ts!(fuzzcheck_mutators_crate "::" ident!("Enum" n "PayloadMutator"));

    let TupleNMutator = |n: usize| ts!(fuzzcheck_mutators_crate "::" ident!("Tuple" n "Mutator"));
    let TupleN = |n: usize| ts!(fuzzcheck_mutators_crate "::" ident!("Tuple" n));


    let generics_no_eq = enu.generics.removing_eq_type();
    let generics_no_eq_nor_bounds = enu.generics.removing_bounds_and_eq_type();
    let DefaultMutator = ts!(fuzzcheck_mutators_crate "::DefaultMutator");
    let mut where_clause = enu.where_clause.clone().unwrap_or(WhereClause::default());
    for ty_param in enu.generics.type_params.iter() {
        where_clause.items.push(WhereClauseItem {
            for_lifetimes: None,
            lhs: ty_param.type_ident.clone(),
            rhs: ts!(DefaultMutator "+ 'static"),
        });
    }

    extend_ts!(tb,
    "impl" generics_no_eq DefaultMutator "for" enu.ident generics_no_eq_nor_bounds where_clause "{
        type Mutator = " EnumNPayloadMutator "<"
        join_ts!(items_with_fields.iter(), (_, fields),
            "(" join_ts!(fields.iter(), field, field.ty, separator: "," ) "),
            " TupleNMutator(fields.len()) "<"
                join_ts!(fields.iter(), field, 
                    field.ty ","
                )
                join_ts!(fields.iter(), field, 
                    "<" field.ty " as " DefaultMutator ">::Mutator"
                , separator: ",")
            ">,"
            TupleN(fields.len()) "<" 
                join_ts!(fields.iter(), field, 
                    field.ty
                , separator: ",")
            ">"
        , separator: ",")
        ">;
    
        fn default_mutator() -> Self::Mutator {
            Self::Mutator::new("
            join_ts!(items_with_fields.iter(), (_, fields),
                TupleNMutator(fields.len()) "::new(" 
                    join_ts!(fields.iter(), field, 
                        "<" field.ty ">::default_mutator()"
                    , separator: ",")
                ")"
            , separator: ",")
            ")
        }
    }"
    )
}

pub fn impl_wrapped_tuple_1_structure(tb: &mut TokenBuilder, enu: &Enum, fuzzcheck_mutators_crate: TokenStream) {
    assert!(enu.items.len() == 1 && matches!(&enu.items[0].data, Some(EnumItemData::Struct(_, fields)) if fields.len() == 1));
    if let Some(EnumItemData::Struct(_, fields)) = &enu.items[0].data {
        let item = &enu.items[0];
        let field = fields[0].clone();
        let field_type = field.ty.clone();

        let generics_no_eq = enu.generics.removing_eq_type();
        let generics_no_eq_nor_bounds = enu.generics.removing_bounds_and_eq_type();
        
        let TupleStructure = ts!(fuzzcheck_mutators_crate "::TupleStructure");
        let WrappedTuple1 = ts!(fuzzcheck_mutators_crate "::WrappedTuple1");
    
        let mut where_clause = enu.where_clause.clone().unwrap_or(WhereClause::default());
        for tp in enu.generics.type_params.iter() {
            where_clause.items.push(
                WhereClauseItem {
                    for_lifetimes: None,
                    lhs: tp.type_ident.clone(),
                    rhs: ts!("'static"),
                }
            );    
        }
    
        extend_ts!(tb,
            "impl " generics_no_eq TupleStructure "<" WrappedTuple1 "<" field_type "> > 
                for " enu.ident generics_no_eq_nor_bounds where_clause " 
            {
                fn get_ref<'a>(&'a self) -> &'a " field_type " {
                    match self {
                        Self:: " item.ident " { " field.access() ": x } => { x }
                    }
                }
            
                fn get_mut<'a>(&'a mut self) -> &'a mut " field_type " {
                    match self {
                        Self:: " item.ident " { " field.access() ": x } => { x }
                    }
                }
            
                fn new(t: " field_type ") -> Self {
                    Self:: " item.ident " { " field.access() ": t }
                }
            }"
        );
    } else {
        unreachable!()
    }
}

pub fn impl_default_mutator_for_enum_wrapped_tuple(tb: &mut TokenBuilder, enu: &Enum, fuzzcheck_mutators_crate: TokenStream) {
    assert!(enu.items.len() == 1 && matches!(&enu.items[0].data, Some(EnumItemData::Struct(_, fields)) if fields.len() == 1));
    if let Some(EnumItemData::Struct(_, fields)) = &enu.items[0].data {
        let field = fields[0].clone();

        let generics_no_eq = enu.generics.removing_eq_type();
        let generics_no_eq_nor_bounds = enu.generics.removing_bounds_and_eq_type();
     
        let mut where_clause = enu.where_clause.clone().unwrap_or(WhereClause::default());
        for tp in enu.generics.type_params.iter() {
            where_clause.items.push(
                WhereClauseItem {
                    for_lifetimes: None,
                    lhs: tp.type_ident.clone(),
                    rhs: ts!("'static"),
                }
            );    
        }
    
        let DefaultMutator = ts!(fuzzcheck_mutators_crate "::DefaultMutator");
        let WrappedMutator = ts!(fuzzcheck_mutators_crate "::WrappedMutator");
    
        extend_ts!(tb, 
        "impl " generics_no_eq DefaultMutator "for" enu.ident generics_no_eq_nor_bounds where_clause "{
            type Mutator = " WrappedMutator "<" field.ty ", <" field.ty "as" DefaultMutator ">::Mutator>;
        
            fn default_mutator() -> Self::Mutator {
                Self::Mutator::new(<" field.ty ">::default_mutator())
            }
        }
        ")   
    } else {
        unreachable!()
    }
}

pub fn impl_basic_enum_structure(tb: &mut TokenBuilder, enu: &Enum, fuzzcheck_mutators_crate: TokenStream) {
    assert!(
        enu.items.len() > 0 
        && enu.items.iter().all(|item| 
            !matches!(&item.data, Some(EnumItemData::Struct(_, fields)) if fields.len() > 0)
        )
    );

    let BasicEnumStructure = ts!(fuzzcheck_mutators_crate "::BasicEnumStructure");

    let items_init = enu.items.iter().map(|item| {
        match &item.data {
            Some(EnumItemData::Struct(kind, _)) => ts!(kind.open() kind.close()),
            _ => ts!()
        }
    }).collect::<Box<_>>();

    extend_ts!(tb,
        "impl" BasicEnumStructure "for" enu.ident "{
            fn from_item_index(item_index: usize) -> Self {
                match item_index {"
                join_ts!(enu.items.iter().enumerate(), (i, item),
                    i "=>" enu.ident "::" item.ident items_init[i] ","
                )
                "
                    _ => unreachable!()
                }
            }
        
            fn get_item_index(&self) -> usize {
                match self {"
                join_ts!(enu.items.iter().enumerate(), (i, item),
                    enu.ident "::" item.ident items_init[i] "=>" i ","
                )
                "}
            }
        }"
    )
}


pub fn impl_default_mutator_for_basic_enum(tb: &mut TokenBuilder, enu: &Enum, fuzzcheck_mutators_crate: TokenStream) {
    assert!(
        enu.items.len() > 0 
        && enu.items.iter().all(|item| 
            !matches!(&item.data, Some(EnumItemData::Struct(_, fields)) if fields.len() > 0)
        )
    );

    let DefaultMutator  = ts!(fuzzcheck_mutators_crate "::DefaultMutator");
    let BasicEnumMutator = ts!(fuzzcheck_mutators_crate "::BasicEnumMutator");
    
    extend_ts!(tb,
        "impl" DefaultMutator "for " enu.ident " {
            type Mutator = " BasicEnumMutator ";
        
            fn default_mutator() -> Self::Mutator {
                Self::Mutator::default()
            }
        }"
    )
}


#[cfg(test)]
mod test {
    use crate::{decent_synquote_alternative::TokenBuilderExtend};
    use decent_synquote_alternative::{parser::TokenParser, token_builder::TokenBuilder};
    use proc_macro2::TokenStream;

    use super::{impl_basic_enum_structure, impl_default_mutator_for_basic_enum, impl_default_mutator_for_enum, impl_enum_structure_trait, impl_mutator, impl_wrapped_tuple_1_structure, make_enum_mutator_helper_types, make_enum_n_payload_mutator, make_enum_n_payload_structure};

    #[test]
    fn test_impl_default_mutator_for_basic_enum() {
        let code = "
        enum X {
            A,
            B,
            C,
        }
        ".parse::<TokenStream>().unwrap();
        let mut parser = TokenParser::new(code);
        let enu = parser.eat_enumeration().unwrap();
        
        let mut tb = TokenBuilder::new();
        impl_default_mutator_for_basic_enum(&mut tb, &enu, ts!("fuzzcheck_mutators"));
        let generated = tb.end().to_string();

        let expected = "
        impl fuzzcheck_mutators::DefaultMutator for X {
            type Mutator = fuzzcheck_mutators::BasicEnumMutator;
        
            fn default_mutator() -> Self::Mutator {
                Self::Mutator::default()
            }
        }
        ".parse::<TokenStream>()
        .unwrap()
        .to_string();
        assert_eq!(generated, expected, "\n\n{}\n\n{}\n\n", generated, expected);
    }

    #[test]
    fn test_impl_basic_enum_structure() {
        let code = "
        enum X {
            A,
            B { },
            C ( ),
        }
        ".parse::<TokenStream>().unwrap();
        let mut parser = TokenParser::new(code);
        let enu = parser.eat_enumeration().unwrap();
        
        let mut tb = TokenBuilder::new();
        impl_basic_enum_structure(&mut tb, &enu, ts!("fuzzcheck_mutators"));
        let generated = tb.end().to_string();

        let expected = "
        impl fuzzcheck_mutators::BasicEnumStructure for X {
            fn from_item_index(item_index: usize) -> Self {
                match item_index {
                    0 => X::A,
                    1 => X::B { },
                    2 => X::C ( ),
                    _ => unreachable!()
                }
            }
        
            fn get_item_index(&self) -> usize {
                match self {
                    X::A => 0,
                    X::B { } => 1,
                    X::C ( ) => 2,
                }
            }
        }
        ".parse::<TokenStream>()
        .unwrap()
        .to_string();
        assert_eq!(generated, expected, "\n\n{}\n\n{}\n\n", generated, expected);
    }

    #[test]
    fn test_impl_wrapped_tuple_1_structure() {
        let code = "
        pub enum A<T: Clone> {
            X(Option<T>),
        }
        ".parse::<TokenStream>().unwrap();
        let mut parser = TokenParser::new(code);
        let enu = parser.eat_enumeration().unwrap();
        
        let mut tb = TokenBuilder::new();
        impl_wrapped_tuple_1_structure(&mut tb, &enu, ts!("fuzzcheck_mutators"));
        let generated = tb.end().to_string();

        let expected = "
        impl<T: Clone> fuzzcheck_mutators::TupleStructure<fuzzcheck_mutators::WrappedTuple1<Option<T> > > for A<T> where T: 'static {
            fn get_ref<'a>(&'a self) -> &'a Option<T> {
                match self {
                    Self::X {0: x} => { x }
                }
            }
        
            fn get_mut<'a>(&'a mut self) -> &'a mut Option<T> {
                match self {
                    Self::X {0: x} => { x }
                }
            }
        
            fn new(t: Option<T>) -> Self {
                Self::X {0: t}
            }
        } 
        ".parse::<TokenStream>()
        .unwrap()
        .to_string();
        assert_eq!(generated, expected, "\n\n{}\n\n{}\n\n", generated, expected);
    }

    #[test]
    fn test_impl_default_mutator_for_enum() {
        let code = "
        enum E<T> {
            Foo,
            Bar,
            Left((T, u8), u8),
            Right(u16),
        }
        ".parse::<TokenStream>().unwrap();
        let mut parser = TokenParser::new(code);
        let enu = parser.eat_enumeration().unwrap();
        
        let mut tb = TokenBuilder::new();
        impl_default_mutator_for_enum(&mut tb, &enu, ts!("crate"));
        let generated = tb.end().to_string();

        let expected = "
impl<T> crate::DefaultMutator for E<T> where T: crate::DefaultMutator + 'static {
    type Mutator = crate::Enum2PayloadMutator<
        ((T, u8), u8),
        crate::Tuple2Mutator<(T, u8), u8, <(T, u8) as crate::DefaultMutator>::Mutator, <u8 as crate::DefaultMutator>::Mutator>,
        crate::Tuple2<(T, u8), u8> ,
        (u16),
        crate::Tuple1Mutator<u16, <u16 as crate::DefaultMutator>::Mutator>,
        crate::Tuple1<u16>
    >;

    fn default_mutator() -> Self::Mutator {
        Self::Mutator::new(
            crate::Tuple2Mutator::new(<(T, u8)>::default_mutator(), <u8>::default_mutator()),
            crate::Tuple1Mutator::new(<u16>::default_mutator())
        )
    }
}
        ".parse::<TokenStream>()
        .unwrap()
        .to_string();
        assert_eq!(generated, expected, "\n\n{}\n\n{}\n\n", generated, expected);
    }

    #[test]
    fn test_impl_enum_structure_trait_3() {
        let code = "
        pub enum E<T, U: Clone> where Vec<T>: Default {
            Foo{},
            Bar(),
            Left(T, u8, U),
            Baz,
        }
        ".parse::<TokenStream>().unwrap();
        let mut parser = TokenParser::new(code);
        let enu = parser.eat_enumeration().unwrap();
        
        let mut tb = TokenBuilder::new();
        impl_enum_structure_trait(&mut tb, &enu, ts!("crate"));
        let generated = tb.end().to_string();

        let expected = "
        # [allow (non_shorthand_field_patterns)] 
        impl<T, U: Clone> crate::Enum1PayloadStructure for E<T, U> where Vec<T>: Default, T: 'static, U: 'static {
            type TupleKind0 = crate::Tuple3<T, u8, U> ;
            type T0 = (T, u8, U);

            fn get_ref<'a>(&'a self) -> crate::Either2<(&'a T, &'a u8, &'a U), usize> {
                match self {
                    E::Foo{} => { crate::Either2::T1(0) }
                    E::Bar() => { crate::Either2::T1(1) }
                    E::Left(_0, _1, _2) => { crate::Either2::T0((_0, _1, _2)) }
                    E::Baz => { crate::Either2::T1(2) }
                }
            }
            fn get_mut<'a>(&'a mut self) -> crate::Either2<(&'a mut T, &'a mut u8, &'a mut U), usize> {
                match self {
                    E::Foo{} => { crate::Either2::T1(0) }
                    E::Bar() => { crate::Either2::T1(1) }
                    E::Left(_0, _1, _2) => { crate::Either2::T0((_0, _1, _2)) }
                    E::Baz => { crate::Either2::T1(2) }
                }
            }
            fn new(t: crate::Either2<Self::T0, usize>) -> Self {
                match t {
                    crate::Either2::T0(x) => { E::Left{0: x.0, 1: x.1, 2: x.2} }
                    crate::Either2::T1(x) => match x % 3 {
                        0 => Self::Foo{},
                        1 => Self::Bar{},
                        2 => Self::Baz,
                        _ => unreachable!(),
                    }
                }
            }
        }
        ".parse::<TokenStream>()
        .unwrap()
        .to_string();
        assert_eq!(generated, expected, "\n\n{}\n\n{}\n\n", generated, expected);
    }

    #[test]
    fn test_impl_enum_structure_trait_2() {
        let code = "
        pub enum E<T> {
            Left { x: T, _y: u8 },
            Right(u16),
        }
        ".parse::<TokenStream>().unwrap();
        let mut parser = TokenParser::new(code);
        let enu = parser.eat_enumeration().unwrap();
        
        let mut tb = TokenBuilder::new();
        impl_enum_structure_trait(&mut tb, &enu, ts!("crate"));
        let generated = tb.end().to_string();

        let expected = "
        # [allow (non_shorthand_field_patterns)] 
        impl<T> crate::Enum2PayloadStructure for E<T> where T: 'static {
            type TupleKind0 = crate::Tuple2<T, u8> ;
            type T0 = (T, u8);
            type TupleKind1 = crate::Tuple1<u16> ;
            type T1 = (u16);

            fn get_ref<'a>(&'a self) -> crate::Either3<(&'a T, &'a u8), (&'a u16), usize> {
                match self {
                    E::Left { x: x, _y: _y } => { crate::Either3::T0((x, _y)) }
                    E::Right(_0) => { crate::Either3::T1((_0)) }
                }
            }
            fn get_mut<'a>(&'a mut self) -> crate::Either3<(&'a mut T, &'a mut u8), (&'a mut u16), usize> {
                match self {
                    E::Left { x: x, _y: _y } => { crate::Either3::T0((x, _y)) }
                    E::Right(_0) => { crate::Either3::T1((_0)) }
                }
            }
            fn new(t: crate::Either3<Self::T0, Self::T1, usize>) -> Self {
                match t {
                    crate::Either3::T0(x) => { E::Left{x: x.0, _y: x.1} }
                    crate::Either3::T1(x) => { E::Right { 0: x } }
                    crate::Either3::T2(x) => match x % 0 {
                        _ => unreachable!(),
                    }
                }
            }
        }
        ".parse::<TokenStream>()
        .unwrap()
        .to_string();
        assert_eq!(generated, expected, "\n\n{}\n\n{}\n\n", generated, expected);
    }

    #[test]
    fn test_impl_enum_structure_trait() {
        let code = "
        pub enum E<T> {
            Foo,
            Bar,
            Left(T, u8),
            Right(u16),
        }
        ".parse::<TokenStream>().unwrap();
        let mut parser = TokenParser::new(code);
        let enu = parser.eat_enumeration().unwrap();
        
        let mut tb = TokenBuilder::new();
        impl_enum_structure_trait(&mut tb, &enu, ts!("crate"));
        let generated = tb.end().to_string();

        let expected = "
        # [allow (non_shorthand_field_patterns)] 
        impl<T> crate::Enum2PayloadStructure for E<T> where T: 'static {
            type TupleKind0 = crate::Tuple2<T, u8> ;
            type T0 = (T, u8);
            type TupleKind1 = crate::Tuple1<u16> ;
            type T1 = (u16);

            fn get_ref<'a>(&'a self) -> crate::Either3<(&'a T, &'a u8), (&'a u16), usize> {
                match self {
                    E::Foo => { crate::Either3::T2(0) }
                    E::Bar => { crate::Either3::T2(1) }
                    E::Left(_0, _1) => { crate::Either3::T0((_0, _1)) }
                    E::Right(_0) => { crate::Either3::T1((_0)) }
                }
            }
            fn get_mut<'a>(&'a mut self) -> crate::Either3<(&'a mut T, &'a mut u8), (&'a mut u16), usize> {
                match self {
                    E::Foo => { crate::Either3::T2(0) }
                    E::Bar => { crate::Either3::T2(1) }
                    E::Left(_0, _1) => { crate::Either3::T0((_0, _1)) }
                    E::Right(_0) => { crate::Either3::T1((_0)) }
                }
            }
            fn new(t: crate::Either3<Self::T0, Self::T1, usize>) -> Self {
                match t {
                    crate::Either3::T0(x) => { E::Left{0: x.0, 1: x.1} }
                    crate::Either3::T1(x) => { E::Right { 0: x } }
                    crate::Either3::T2(x) => match x % 2 {
                        0 => Self::Foo,
                        1 => Self::Bar,
                        _ => unreachable!(),
                    }
                }
            }
        }
        ".parse::<TokenStream>()
        .unwrap()
        .to_string();
        assert_eq!(generated, expected, "\n\n{}\n\n{}\n\n", generated, expected);
    }

    #[test]
    fn test_impl_mutator() {
        let mut tb = TokenBuilder::new();
        impl_mutator(&mut tb, 2, ts!("crate"));
        let generated = tb.end().to_string();
        let expected = "
impl<T, T0, M0, TupleKind0, T1, M1, TupleKind1> ::fuzzcheck_traits::Mutator<T>
    for Enum2PayloadMutator<T0, M0, TupleKind0, T1, M1, TupleKind1>
where
    T: ::std::clone::Clone + Enum2PayloadStructure<TupleKind0 = TupleKind0, T0 = T0, TupleKind1 = TupleKind1, T1 = T1 , > ,
    T0: ::std::clone::Clone + crate::TupleStructure<TupleKind0> ,
    M0: crate::TupleMutator<T0, TupleKind0> ,
    TupleKind0: crate::RefTypes,
    T1: ::std::clone::Clone + crate::TupleStructure<TupleKind1> ,
    M1: crate::TupleMutator<T1, TupleKind1> ,
    TupleKind1: crate::RefTypes
{
    type Cache = Either3<M0::Cache, M1::Cache, ()>;
    type MutationStep = Enum2PayloadMutationStep<M0::MutationStep, M1::MutationStep, Self::ArbitraryStep>;
    type ArbitraryStep = Enum2PayloadArbitraryStep<M0::ArbitraryStep, M1::ArbitraryStep>;
    type UnmutateToken = Either3<M0::UnmutateToken, M1::UnmutateToken, (T, Self::Cache)>;

      fn cache_from_value(&self, value: &T) -> Self::Cache {
        let x = value.get_ref();
        match x {
            Either3::T0(x) => Either3::T0(self.mutator_0.cache_from_value(x)),
            Either3::T1(x) => Either3::T1(self.mutator_1.cache_from_value(x)),
            Either3::T2(_) => Either3::T2(()),
        }
    }

    fn initial_step_from_value(&self, value: &T) -> Self::MutationStep {
        match value.get_ref() {
            Either3::T0(x) => Self::MutationStep {
                inner: Either3::T0(self.mutator_0.initial_step_from_value(x)),
                arbitrary: <_>::default(),
            },
            Either3::T1(x) => Self::MutationStep {
                inner: Either3::T1(self.mutator_1.initial_step_from_value(x)),
                arbitrary: <_>::default(),
            },
            Either3::T2(_) => Self::MutationStep {
                inner: Either3::T2(()),
                arbitrary: <_>::default(),
            },
        }
    }
    fn max_complexity(&self) -> f64 {
        crate::size_to_cplxity(::std::mem::variant_count::<T>())
            + [self.mutator_0.max_complexity(), self.mutator_1.max_complexity()]
                .iter()
                .max_by(|x, y| x.partial_cmp(y).unwrap_or(Ordering::Equal))
                .unwrap()
    }
    fn min_complexity(&self) -> f64 {
        crate::size_to_cplxity(::std::mem::variant_count::<T>())
            + [self.mutator_0.min_complexity(), self.mutator_1.min_complexity()]
                .iter()
                .min_by(|x, y| x.partial_cmp(y).unwrap_or(Ordering::Equal))
                .unwrap()
    }
    fn complexity(&self, value: &T, cache: &Self::Cache) -> f64 {
        crate::size_to_cplxity(::std::mem::variant_count::<T>())
            + match (value.get_ref(), cache) {
                (Either3::T0(value), Either3::T0(cache)) => self.mutator_0.complexity(value, cache),
                (Either3::T1(value), Either3::T1(cache)) => self.mutator_1.complexity(value, cache),
                (Either3::T2(_), Either3::T2(_)) => 0.0,
                _ => unreachable!(),
            }
    }
    fn ordered_arbitrary(&mut self, step: &mut Self::ArbitraryStep, max_cplx: f64) -> Option<(T, Self::Cache)> {
        if step.steps.is_empty() {
            return None;
        }
        let steps_len = step.steps.len();
        let inner_max_cplx = max_cplx - crate::size_to_cplxity(::std::mem::variant_count::<T>());
        let substep = &mut step.steps[step.idx % steps_len];

        let result = match substep {
            Either3::T0(substep) => self
                .mutator_0
                .ordered_arbitrary(substep, inner_max_cplx)
                .map(|(value, cache)| (T::new(Either3::T0(value)), Either3::T0(cache))),
            Either3::T1(substep) => self
                .mutator_1
                .ordered_arbitrary(substep, inner_max_cplx)
                .map(|(value, cache)| (T::new(Either3::T1(value)), Either3::T1(cache))),
            Either3::T2(x) => {
                *x += 1;
                if *x <= ::std::mem::variant_count::<T>() - 2 {
                    Some((T::new(Either3::T2(*x)), Either3::T2(())))
                } else {
                    None
                }
            }
        };
        if let Some(result) = result {
            step.idx += 1;
            Some(result)
        } else {
            step.steps.remove(step.idx % steps_len);
            self.ordered_arbitrary(step, max_cplx)
        }
    }
    fn random_arbitrary(&mut self, max_cplx: f64) -> (T, Self::Cache) {
        let inner_max_cplx = max_cplx - crate::size_to_cplxity(::std::mem::variant_count::<T>());
        let nbr_variants = if ::std::mem::variant_count::<T>() > 2 { 3 } else { 2 };
        match self.rng.usize(..nbr_variants) {
            0 => {
                let (v, c) = self.mutator_0.random_arbitrary(inner_max_cplx);
                (T::new(Either3::T0(v)), Either3::T0(c))
            }
            1 => {
                let (v, c) = self.mutator_1.random_arbitrary(inner_max_cplx);
                (T::new(Either3::T1(v)), Either3::T1(c))
            }
            2 => {
                let pick = self.rng.usize(.. ::std::mem::variant_count::<T>() - 2);
                (T::new(Either3::T2(pick)), Either3::T2(()))
            }
            _ => {
                unreachable!()
            }
        }
    }
    fn ordered_mutate(
        &mut self,
        value: &mut T,
        cache: &mut Self::Cache,
        step: &mut Self::MutationStep,
        max_cplx: f64,
    ) -> ::std::option::Option<Self::UnmutateToken> {
        let inner_max_cplx = max_cplx - crate::size_to_cplxity(::std::mem::variant_count::<T>());
        match (value.get_mut(), cache.borrow_mut(), &mut step.inner) {
            (Either3::T0(inner_value), Either3::T0(inner_cache), Either3::T0(inner_step)) => {
                if let ::std::option::Option::Some(token) = self
                    .mutator_0
                    .ordered_mutate(inner_value, inner_cache, inner_step, inner_max_cplx)
                {
                    return ::std::option::Option::Some(Either3::T0(token));
                }
            }
            (Either3::T1(inner_value), Either3::T1(inner_cache), Either3::T1(inner_step)) => {
                if let ::std::option::Option::Some(token) = self
                    .mutator_1
                    .ordered_mutate(inner_value, inner_cache, inner_step, inner_max_cplx)
                {
                    return ::std::option::Option::Some(Either3::T1(token));
                }
            }
            (Either3::T2(_), Either3::T2(_), Either3::T2(_)) => {}
            _ => unreachable!(),
        }
        if let ::std::option::Option::Some((new_value, new_cache)) = self.ordered_arbitrary(&mut step.arbitrary, max_cplx) {
            let old_value = ::std::mem::replace(value, new_value);
            let old_cache = ::std::mem::replace(cache, new_cache);
            ::std::option::Option::Some(Either3::T2((old_value, old_cache)))
        } else {
            ::std::option::Option::None
        }
    }
    fn random_mutate(&mut self, value: &mut T, cache: &mut Self::Cache, max_cplx: f64) -> Self::UnmutateToken {
        let inner_max_cplx = max_cplx - crate::size_to_cplxity(::std::mem::variant_count::<T>());
        match (value.get_mut(), cache.borrow_mut()) {
            (Either3::T0(inner_value), Either3::T0(inner_cache)) => {
                return Either3::T0(self.mutator_0.random_mutate(inner_value, inner_cache, inner_max_cplx))
            }
            (Either3::T1(inner_value), Either3::T1(inner_cache)) => {
                return Either3::T1(self.mutator_1.random_mutate(inner_value, inner_cache, inner_max_cplx))
            }
            (Either3::T2(_), Either3::T2(_)) => {}
            _ => unreachable!(),
        }
        let (new_value, new_cache) = self.random_arbitrary(max_cplx);
        let old_value = ::std::mem::replace(value, new_value);
        let old_cache = ::std::mem::replace(cache, new_cache);
        Either3::T2((old_value, old_cache))
    }
    fn unmutate(&self, value: &mut T, cache: &mut Self::Cache, t: Self::UnmutateToken) {
        let (old_value, old_cache) = match (value.get_mut(), cache.borrow_mut(), t) {
            (Either3::T0(inner_value), Either3::T0(inner_cache), Either3::T0(token)) => {
                self.mutator_0.unmutate(inner_value, inner_cache, token);
                return;
            }
            (Either3::T1(inner_value), Either3::T1(inner_cache), Either3::T1(token)) => {
                self.mutator_1.unmutate(inner_value, inner_cache, token);
                return;
            }
            (_, _, Either3::T2((old_value, old_cache))) => (old_value, old_cache),
            _ => unreachable!(),
        };
        let _ = ::std::mem::replace(value, old_value);
        let _ = ::std::mem::replace(cache, old_cache);
    }
}
        "
        .parse::<TokenStream>()
        .unwrap()
        .to_string();
        assert_eq!(generated, expected, "\n\n{}\n\n{}\n\n", generated, expected);
    }

    #[test]
    fn test_make_enum_mutator_helper_types() {
        let mut tb = TokenBuilder::new();
        make_enum_mutator_helper_types(&mut tb, 2);
        let generated = tb.end().to_string();
        let expected = "
        #[derive(::std::clone::Clone)]
        pub struct Enum2PayloadArbitraryStep<T0: ::std::default::Default, T1: ::std::default::Default> {
            steps: ::std::vec::Vec<Either3<T0, T1, usize> >,
            idx: usize,
        }
        impl<T0: ::std::default::Default, T1: ::std::default::Default> ::std::default::Default for Enum2PayloadArbitraryStep<T0, T1> {
            fn default() -> Self {
                Self {
                    steps: vec![Either3::T0(T0::default()), Either3::T1(T1::default()), Either3::T2(0)],
                    idx: 0,
                }
            }
        }
        
        #[derive(::std::clone::Clone)]
        pub struct Enum2PayloadMutationStep<T0, T1, AS> {
            inner: Either3<T0, T1, ()>,
            arbitrary: AS,
        }
        "
        .parse::<TokenStream>()
        .unwrap()
        .to_string();
        assert_eq!(generated, expected, "\n\n{}\n\n{}\n\n", generated, expected);
    }

    #[test]
    fn test_make_enum_n_payload_mutator() {
        let mut tb = TokenBuilder::new();
        make_enum_n_payload_mutator(&mut tb, 2, ts!("crate"));
        let generated = tb.end().to_string();
        let expected = "
pub struct Enum2PayloadMutator<T0, M0, TupleKind0, T1, M1, TupleKind1>
where
    T0: ::std::clone::Clone + crate::TupleStructure<TupleKind0> ,
    TupleKind0: crate::RefTypes,
    M0: crate::TupleMutator<T0, TupleKind0> ,
    T1: ::std::clone::Clone + crate::TupleStructure<TupleKind1> ,
    TupleKind1: crate::RefTypes,
    M1: crate::TupleMutator<T1, TupleKind1>
{
    pub mutator_0: M0,
    pub mutator_1: M1,
    rng: crate::fastrand::Rng,
    _phantom: ::std::marker::PhantomData<(T0, TupleKind0, T1, TupleKind1)>
}

impl<T0, M0, TupleKind0, T1, M1, TupleKind1> Enum2PayloadMutator<T0, M0, TupleKind0, T1, M1, TupleKind1>
where
    T0: ::std::clone::Clone + crate::TupleStructure<TupleKind0> ,
    TupleKind0: crate::RefTypes,
    M0: crate::TupleMutator<T0, TupleKind0> ,
    T1: ::std::clone::Clone + crate::TupleStructure<TupleKind1> ,
    TupleKind1: crate::RefTypes,
    M1: crate::TupleMutator<T1, TupleKind1>
{
    pub fn new(mutator_0: M0, mutator_1: M1) -> Self {
        Self {
            mutator_0,
            mutator_1,
            rng: <_>::default(),
            _phantom: <_>::default()
        }
    }
}

impl<T0, M0, TupleKind0, T1, M1, TupleKind1> ::std::default::Default for Enum2PayloadMutator<T0, M0, TupleKind0, T1, M1, TupleKind1>
where
    T0: ::std::clone::Clone + crate::TupleStructure<TupleKind0> ,
    TupleKind0: crate::RefTypes,
    M0: crate::TupleMutator<T0, TupleKind0> ,
    T1: ::std::clone::Clone + crate::TupleStructure<TupleKind1> ,
    TupleKind1: crate::RefTypes,
    M1: crate::TupleMutator<T1, TupleKind1> ,
    M0: ::std::default::Default,
    M1: ::std::default::Default
{
    fn default() -> Self {
        Self {
            mutator_0: <_>::default(),
            mutator_1: <_>::default(),
            rng: <_>::default(),
            _phantom: <_>::default()
        }
    }
}
        "
        .parse::<TokenStream>()
        .unwrap()
        .to_string();
        assert_eq!(generated, expected, "\n\n{}\n\n{}\n\n", generated, expected);
    }

    #[test]
    fn test_make_enum_n_payload_structure() {
        let mut tb = TokenBuilder::new();
        make_enum_n_payload_structure(&mut tb, 2, ts!("crate"));
        let generated = tb.end().to_string();
        let expected = "
        #[derive(::std::clone::Clone)]
        pub enum Either3<T0, T1, T2> {
            T0(T0),
            T1(T1),
            T2(T2),
        }
        pub trait Enum2PayloadStructure {
            type TupleKind0: crate::RefTypes;
            type T0: crate::TupleStructure<Self::TupleKind0>;
            type TupleKind1: crate::RefTypes;
            type T1: crate::TupleStructure<Self::TupleKind1>;
        
            fn get_ref<'a>(
                &'a self
            ) -> Either3< <Self::TupleKind0 as crate::RefTypes> ::Ref<'a> , <Self::TupleKind1 as crate::RefTypes> ::Ref<'a> , usize>;
            fn get_mut<'a>(
                &'a mut self
            ) -> Either3< <Self::TupleKind0 as crate::RefTypes> ::Mut<'a>, <Self::TupleKind1 as crate::RefTypes> ::Mut<'a>, usize>;
            fn new(t: Either3<Self::T0, Self::T1, usize>) -> Self;
        }
        "
        .parse::<TokenStream>()
        .unwrap()
        .to_string();
        assert_eq!(generated, expected, "\n\n{}\n\n{}\n\n", generated, expected);
    }
}
