use decent_synquote_alternative as synquote;
use proc_macro2::{Ident, Span, TokenStream};

use synquote::parser::*;
use synquote::token_builder::*;

use crate::Common;
use crate::MakeMutatorSettings;

/*
 TODO: Take maximum complexity into account! For now it is partly ignored. One shouldn't switch to
 an item whose minimum complexity is greater than the maximum allowed complexity
*/
#[allow(non_snake_case)]
fn make_enum_n_payload_structure(tb: &mut TokenBuilder, n: usize) {
    let cm = Common::new(n);
    let Ti = cm.Ti.as_ref();
    let n_plus_1_type_params = join_ts!(0..n + 1, i, Ti(i), separator: ",");
    let TupleKindi = cm.TupleKindi.as_ref();
    let SelfTupleKindAsRefTypes = |i: usize| ts!("<Self::" TupleKindi(i) "as" cm.RefTypes ">" );

    extend_ts!(tb,
        "#[derive(" cm.Clone ")]
        pub enum" cm.EitherNP1_ident "<" n_plus_1_type_params "> {"
            join_ts!(0 .. n+1, i,
                Ti(i)"(" Ti(i) "),"
            )
        "}
        
        pub trait" cm.EnumNPayloadStructure_ident "{"
            join_ts!(0..n, i,
                "type " TupleKindi(i) ":" cm.RefTypes ";"
                "type" Ti(i) ":" cm.TupleStructure "<Self::" TupleKindi(i) ">;"
            )
            "fn get_ref<'a>(&'a self) ->" cm.EitherNP1_ident "<" join_ts!(0..n, i, SelfTupleKindAsRefTypes(i)"::Ref<'a> ,") " usize>;"
            "fn get_mut<'a>(&'a mut self) ->" cm.EitherNP1_ident "<" join_ts!(0..n, i, SelfTupleKindAsRefTypes(i)"::Mut<'a>,") " usize>;"
            "fn new(t:" cm.EitherNP1_ident "<" join_ts!(0..n, i, "Self::" Ti(i) ",") " usize>) -> Self;"
        "}"
    )
}

#[allow(non_snake_case)]
fn make_enum_n_payload_mutator(tb: &mut TokenBuilder, n: usize) {
    let cm = Common::new(n);
    let T = cm.Ti.as_ref();
    let M = cm.Mi.as_ref();
    let TupleKind = cm.TupleKindi.as_ref();
    let TupleStructure = cm.TupleStructureTupleKindi.as_ref();
    let TupleMutator = cm.TupleMutatorTiTupleKindi.as_ref();
    let mutator_ = cm.mutator_i.as_ref();

    let type_params = join_ts!(0..n, i,
        T(i) "," M(i) "," TupleKind(i)
    , separator: ",");
    let where_clause = join_ts!(0..n, i, 
        T(i) ":" cm.Clone " +" TupleStructure(i) ","
        TupleKind(i) ":" cm.RefTypes ","
        M(i) ":" TupleMutator(i)
    , separator: ",");

    extend_ts!(tb, "
    pub struct" cm.EnumNPayloadMutator_ident "<" type_params "> where" where_clause "{"
    join_ts!(0..n, i, 
        "pub" mutator_(i) ":" M(i) ","
    )
        "rng: " cm.fastrand "::Rng ,
        _phantom: " cm.PhantomData "<(" join_ts!(0..n, i, T(i) "," TupleKind(i), separator: "," ) ")>"
    "}

    impl<" type_params "> " cm.EnumNPayloadMutator_ident "<" type_params "> where" where_clause "{
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


    impl<" type_params "> " cm.Default "for" cm.EnumNPayloadMutator_ident "<" type_params "> where" where_clause  ","
        join_ts!(0..n, i,
            M(i) ":" cm.Default
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

#[allow(non_snake_case)]
fn make_enum_mutator_helper_types(tb: &mut TokenBuilder, n: usize) {
    let cm = Common::new(n);
    let T = cm.Ti.as_ref();
    let type_params = join_ts!(0..n, i, T(i), separator: ",");

    extend_ts!(tb,
        "#[derive(" cm.Clone ")]
        pub struct" cm.EnumNPayloadArbitraryStep_ident "<" join_ts!(0..n, i, T(i) ":" cm.Default, separator: ",") "> {
            steps: " cm.Vec "<" cm.EitherNP1_ident "<" type_params ", usize > >,
            idx: usize,
        }
        impl<" join_ts!(0..n, i, T(i) ":" cm.Default, separator: ",") ">" cm.Default "for" cm.EnumNPayloadArbitraryStep_ident "<" type_params "> {
            fn default() -> Self {
                Self {
                    steps: vec![" join_ts!(0..n, i, cm.EitherNP1_ident "::" T(i) "(" T(i) "::default()) ," ) cm.EitherNP1_ident "::" T(n) "(0)],
                    idx: 0,
                }
            }
        }
        #[derive(" cm.Clone ")]
        pub struct " cm.EnumNPayloadMutationStep_ident "<" type_params ", AS> {
            inner: " cm.EitherNP1_ident "<" type_params ", ()>,
            arbitrary: AS,
        }
        "
    )
}

#[allow(non_snake_case)]
fn impl_mutator(tb: &mut TokenBuilder, n: usize) {
    let cm = Common::new(n);
    let T = cm.Ti.as_ref();
    let EitherT = cm.EitherNP1_identTi.as_ref();
    let M = cm.Mi.as_ref();
    let TupleKind = cm.TupleKindi.as_ref();
    let TupleStructure = cm.TupleStructureTupleKindi.as_ref();
    let TupleMutator = cm.TupleMutatorTiTupleKindi.as_ref();
    let mutator_ = cm.mutator_i.as_ref();

    extend_ts!(tb, "
    impl<T," join_ts!(0..n, i, T(i) "," M(i) "," TupleKind(i), separator: "," ) "> " cm.fuzzcheck_traits_Mutator "<T>
        for " cm.EnumNPayloadMutator_ident "<" join_ts!(0..n, i, T(i) "," M(i) "," TupleKind(i), separator: "," ) "> 
    where
        T: " cm.Clone "+ " cm.EnumNPayloadStructure_ident "<" join_ts!(0..n, i, TupleKind(i) "=" TupleKind(i) "," T(i) "=" T(i) "," ) ">" ","
    join_ts!(0..n, i,
        T(i) ":" cm.Clone " + " TupleStructure(i) ","
        M(i) ":" TupleMutator(i) ","
        TupleKind(i) ":" cm.RefTypes
    , separator: ",")
    "{
        type Cache = " cm.EitherNP1_ident "<" join_ts!(0..n, i, M(i) "::Cache ,") "()>;
        type MutationStep = " cm.EnumNPayloadMutationStep_ident "<" join_ts!(0..n, i, M(i) "::MutationStep ,") "Self::ArbitraryStep>;
        type ArbitraryStep = " cm.EnumNPayloadArbitraryStep_ident "<" join_ts!(0..n, i, M(i) "::ArbitraryStep", separator: ",") ">;
        type UnmutateToken = " cm.EitherNP1_ident "<" join_ts!(0..n, i, M(i) "::UnmutateToken ,") "(T, Self::Cache)>;
    
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
                    arbitrary: { 
                        let mut step: Self::ArbitraryStep = <_>::default() ;
                        step.steps.remove(" i ");
                        step
                    },
                }," 
            )
                EitherT(n) "(_) => Self::MutationStep {
                    inner:" EitherT(n) "(()),
                    arbitrary: <_>::default(),
                },
            }
        }
        fn max_complexity(&self) -> f64 {
            " cm.size_to_cplxity "(" cm.variant_count_T ")
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
            " cm.size_to_cplxity "(" cm.variant_count_T ")
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
            " cm.size_to_cplxity "(" cm.variant_count_T ")
                + match (value.get_ref(), cache) {"
                join_ts!(0..n, i,
                    "(" EitherT(i) "(value), " EitherT(i) "(cache)) => self." mutator_(i) ".complexity(value, cache),"
                )
                    "(" EitherT(n) "(_)," EitherT(n) "(_)) => 0.0,
                    _ => unreachable!(),
                }
        }
        fn ordered_arbitrary(&self, step: &mut Self::ArbitraryStep, max_cplx: f64) -> Option<(T, Self::Cache)> {
            if max_cplx < <Self as " cm.fuzzcheck_traits_Mutator "<T> >::min_complexity(self) { return " cm.None " }
            if step.steps.is_empty() {
                return " cm.None ";
            }
            let steps_len = step.steps.len();
            let inner_max_cplx = max_cplx - " cm.size_to_cplxity "(" cm.variant_count_T ");
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
                    if *x <= " cm.variant_count_T " - " n " {
                        " cm.Some "((T::new(" EitherT(n) "(*x)), " EitherT(n) "(())))
                    } else {
                        " cm.None "
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
        fn random_arbitrary(&self, max_cplx: f64) -> (T, Self::Cache) {
            let inner_max_cplx = max_cplx - " cm.size_to_cplxity "(" cm.variant_count_T ");
            let nbr_variants = if " cm.variant_count_T " > " n " { " n+1 " } else { " n " };
            match self.rng.usize(..nbr_variants) {"
            join_ts!(0..n, i,
                i "=> {
                    let (v, c) = self." mutator_(i) ".random_arbitrary(inner_max_cplx);
                    (T::new(" EitherT(i) "(v)), " EitherT(i) "(c))
                }"
            )
                n "=> {
                    let pick = self.rng.usize(.." cm.variant_count_T " - " n ");
                    (T::new(" EitherT(n) "(pick)), " EitherT(n) "(()))
                }
                _ => {
                    unreachable!()
                }
            }
        }
        fn ordered_mutate(
            &self,
            value: &mut T,
            cache: &mut Self::Cache,
            step: &mut Self::MutationStep,
            max_cplx: f64,
        ) -> " cm.Option "<Self::UnmutateToken> {
            if max_cplx < <Self as " cm.fuzzcheck_traits_Mutator "<T> >::min_complexity(self) { return " cm.None " }
            let inner_max_cplx = max_cplx - " cm.size_to_cplxity "(" cm.variant_count_T ");
            if self.rng.usize(..100) == 0 {
                let (new_value, new_cache) = self.random_arbitrary(max_cplx);
                let old_value = ::std::mem::replace(value, new_value);
                let old_cache = ::std::mem::replace(cache, new_cache);
                return" cm.Some "(" EitherT(n) "((old_value, old_cache)))
            }
            match (value.get_mut(), cache.borrow_mut(), &mut step.inner) {"
            join_ts!(0..n, i,
                "(" EitherT(i) "(inner_value)," EitherT(i) "(inner_cache)," EitherT(i) "(inner_step)) => {
                    if let " cm.Some "(token) = self
                        ." mutator_(i) "
                        .ordered_mutate(inner_value, inner_cache, inner_step, inner_max_cplx)
                    {
                        return " cm.Some "(" EitherT(i) "(token));
                    }
                }"
            )
            "   (" EitherT(n) "(_), " EitherT(n) "(_), " EitherT(n) "(_)) => {
                    // TODO: this could be slightly better, avoiding a repetition by mutating instead of using arbitrary
                }
                _ => unreachable!(),
            }
            if let " cm.Some "((new_value, new_cache)) = self.ordered_arbitrary(&mut step.arbitrary, max_cplx) {
                let old_value = ::std::mem::replace(value, new_value);
                let old_cache = ::std::mem::replace(cache, new_cache);
                " cm.Some "(" EitherT(n) "((old_value, old_cache)))
            } else {
                " cm.None "
            }
        }
        fn random_mutate(&self, value: &mut T, cache: &mut Self::Cache, max_cplx: f64) -> Self::UnmutateToken {
            let inner_max_cplx = max_cplx - " cm.size_to_cplxity "(" cm.variant_count_T ");
            if self.rng.usize(..100) == 0 {
                let (new_value, new_cache) = self.random_arbitrary(max_cplx);
                let old_value = ::std::mem::replace(value, new_value);
                let old_cache = ::std::mem::replace(cache, new_cache);
                return " EitherT(n) "((old_value, old_cache))
            }
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

pub fn make_basic_enum_mutator(tb: &mut TokenBuilder, n: usize) {
    make_enum_n_payload_structure(tb, n);
    make_enum_n_payload_mutator(tb, n);
    make_enum_mutator_helper_types(tb, n);
    impl_mutator(tb, n);
}

#[allow(non_snake_case)]
pub fn impl_enum_structure_trait(tb: &mut TokenBuilder, enu: &Enum) {
    let items_with_fields = enu
        .items
        .iter()
        .enumerate()
        .filter_map(|(i, item)| match &item.data {
            Some(EnumItemData::Struct(_, fields)) if fields.len() > 0 => Some((i, fields)),
            _ => None,
        })
        .collect::<Box<_>>();
    let items_without_fields = enu
        .items
        .iter()
        .enumerate()
        .filter_map(|(_, item)| match &item.data {
            Some(EnumItemData::Struct(_, fields)) if fields.len() > 0 => None,
            _ => Some(item),
        })
        .collect::<Box<_>>();
    let n = items_with_fields.len();
    let cm = Common::new(n);
    let TupleKind = cm.TupleKindi.as_ref();
    let Tuple = cm.Tuplei.as_ref();
    let T = cm.Ti.as_ref();

    let generics_no_eq = enu.generics.removing_eq_type();
    let generics_no_eq_nor_bounds = enu.generics.removing_bounds_and_eq_type();

    let mut where_clause = enu.where_clause.clone().unwrap_or_default();
    where_clause.add_clause_items(join_ts!(&enu.generics.type_params, tp,
        tp.type_ident ": 'static ,"
    ));

    let field_types = items_with_fields
        .iter()
        .map(|(_, fields)| join_ts!(fields.iter(), field, field.ty, separator: ","))
        .collect::<Box<_>>();
    let field_types_ref = items_with_fields
        .iter()
        .map(|(_, fields)| join_ts!(fields.iter(), field, "&'a" field.ty, separator: ","))
        .collect::<Box<_>>();
    let field_types_mut = items_with_fields
        .iter()
        .map(|(_, fields)| join_ts!(fields.iter(), field, "&'a mut" field.ty, separator: ","))
        .collect::<Box<_>>();

    let either_owned = ts!(
        cm.EitherNP1_path "<"
            join_ts!(0..items_with_fields.len(), i,
                "Self::" T(i) ","
            )
            "usize"
        ">"
    );
    let either_ref = ts!(
        cm.EitherNP1_path "<"
            join_ts!(0..n, i,
                "(" field_types_ref[i] ") ,"
            )
            "usize"
        ">"
    );
    let either_mut = ts!(
        cm.EitherNP1_path "<"
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
                cm.EitherNP1_path "::" match &item.data {
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
        impl" generics_no_eq cm.EnumNPayloadStructure_path 
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
                cm.EitherNP1_path "::" T(i) "(x) => {"
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
                cm.EitherNP1_path "::" T(n) "(x) => match x %" enu.items.len() - n "{"
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

#[derive(Clone)]
struct FieldMutator {
    i: usize,
    j: usize,
    field: StructField,
    kind: FieldMutatorKind,
}
#[derive(Clone)]
enum FieldMutatorKind {
    Generic,
    Prescribed(Ty, Option<TokenStream>),
}
impl FieldMutator {
    fn mutator_stream(&self, cm: &Common) -> TokenStream {
        match &self.kind {
            FieldMutatorKind::Generic => ts!(cm.Mi_j.as_ref()(self.i, self.j)),
            FieldMutatorKind::Prescribed(m, _) => ts!(m),
        }
    }
}

#[allow(non_snake_case)]
pub(crate) fn impl_default_mutator_for_enum(tb: &mut TokenBuilder, enu: &Enum, settings: &MakeMutatorSettings) {
    let items_with_fields = enu
        .items
        .iter()
        .enumerate()
        .filter_map(|(i, item)| match &item.data {
            Some(EnumItemData::Struct(_, fields)) if fields.len() > 0 => Some((i, fields)),
            _ => None,
        })
        .collect::<Box<_>>();

    let n = items_with_fields.len();
    let cm = Common::new(n);

    let generics_no_eq = enu.generics.removing_eq_type();
    let generics_no_eq_nor_bounds = enu.generics.removing_bounds_and_eq_type();

    let field_mutators = items_with_fields
        .iter()
        .map(|(i, fields)| {
            let i = *i;
            fields
                .iter()
                .enumerate()
                .map(|(j, field)| {
                    let mut mutator = None;
                    for attribute in field.attributes.iter() {
                        if let Some((m, init)) = super::read_field_default_mutator_attribute(attribute.clone()) {
                            mutator = Some((m, init));
                        }
                    }
                    if let Some(m) = mutator {
                        FieldMutator {
                            i,
                            j,
                            field: field.clone(),
                            kind: FieldMutatorKind::Prescribed(m.0, m.1),
                        }
                    } else {
                        FieldMutator {
                            i,
                            j,
                            field: field.clone(),
                            kind: FieldMutatorKind::Generic,
                        }
                    }
                })
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();

    let field_generic_mutators = field_mutators
        .iter()
        .flatten()
        .filter(|m| match m.kind {
            FieldMutatorKind::Generic => true,
            FieldMutatorKind::Prescribed(_, _) => false,
        })
        .collect::<Vec<_>>();
    let field_prescribed_mutators = field_mutators
        .iter()
        .flatten()
        .filter_map(|m| match &m.kind {
            FieldMutatorKind::Generic => None,
            FieldMutatorKind::Prescribed(mutator, init) => Some((m.clone(), mutator.clone(), init.clone())),
        })
        .collect::<Vec<_>>();

    let TupleNMutator = cm.TupleNMutator.as_ref();
    let TupleN = cm.Tuplei.as_ref();

    let EnumMutator = ident!(enu.ident "Mutator");
    let mut EnumMutator_generics = enu.generics.removing_eq_type();
    for field_mutator in field_generic_mutators.iter() {
        EnumMutator_generics.type_params.push(TypeParam {
            type_ident: field_mutator.mutator_stream(&cm),
            ..<_>::default()
        })
    }
    let mut EnumMutator_where_clause = enu.where_clause.clone().unwrap_or(WhereClause::default());
    EnumMutator_where_clause.add_clause_items(ts!(
        join_ts!(&enu.generics.type_params, ty_param,
            ty_param.type_ident ":" cm.Clone "+ 'static ,"
        )
        join_ts!(&field_generic_mutators, field_mutator,
            field_mutator.mutator_stream(&cm) ":" cm.fuzzcheck_mutator_traits_Mutator "<" field_mutator.field.ty "> ,"
        )
    ));
    /*
        Enum2PayloadMutator <
            (u8, u16, u32),
            Tuple2Mutator<
                u8, u16, u32
                M0_0, M0_1, CustomU32Mutator,
            >,
            String,
            Tuple1Mutator <
                String,
                StringMutator
                >
            >;
    */
    let InnerMutator = ts!(
        cm.EnumNPayloadMutator_path "<"
        join_ts!(field_mutators.iter(), item_field_mutators,
            "(" join_ts!(item_field_mutators.iter(), fm, fm.field.ty, separator: "," ) "),
            " TupleNMutator(item_field_mutators.len()) "<"
                join_ts!(item_field_mutators.iter(), fm,
                    fm.field.ty ","
                )
                join_ts!(item_field_mutators.iter(), fm,
                    fm.mutator_stream(&cm)
                , separator: ",")
            ">,"
            TupleN(item_field_mutators.len()) "<"
                join_ts!(item_field_mutators.iter(), fm,
                    fm.field.ty
                , separator: ",")
            ">"
        , separator: ",")
        ">"
    );
    let InnerMutator_as_Mutator =
        ts!("<" InnerMutator "as" cm.fuzzcheck_mutator_traits_Mutator "<" enu.ident generics_no_eq_nor_bounds "> >" );

    let mut Default_where_clause = EnumMutator_where_clause.clone();
    Default_where_clause.add_clause_items(ts!(InnerMutator ":" cm.Default));

    let mut DefaultMutator_Mutator_generics = enu.generics.removing_bounds_and_eq_type();
    for field_mutator in field_mutators.iter().flatten() {
        match &field_mutator.kind {
            FieldMutatorKind::Generic => DefaultMutator_Mutator_generics.type_params.push(TypeParam {
                type_ident: ts!("<" field_mutator.field.ty "as" cm.DefaultMutator ">::Mutator"),
                ..<_>::default()
            }),
            FieldMutatorKind::Prescribed(_, _) => {}
        }
    }

    let mut DefaultMutator_where_clause = enu.where_clause.clone().unwrap_or(WhereClause::default());
    DefaultMutator_where_clause.add_clause_items(ts!(
        join_ts!(&enu.generics.type_params, ty_param,
            ty_param.type_ident ":" cm.DefaultMutator "+ 'static ,"
        )
        join_ts!(field_prescribed_mutators.iter().filter(|(_, _, init)| init.is_none()), (_, mutator, _),
            mutator ":" cm.Default ","
        )
    ));

    extend_ts!(tb,
    enu.visibility "struct" EnumMutator EnumMutator_generics EnumMutator_where_clause
    "{
        pub mutator:" InnerMutator "
    }

    impl " EnumMutator_generics EnumMutator EnumMutator_generics.removing_bounds_and_eq_type() EnumMutator_where_clause "
    {
        pub fn new(" 
        join_ts!(field_mutators.iter().flatten(), field_mutator,
            ident!("mutator_" enu.items[field_mutator.i].ident "_" field_mutator.field.access()) ":" field_mutator.mutator_stream(&cm)
        , separator: ",") ") -> Self {
            Self {
                mutator: " cm.EnumNPayloadMutator_path "::new("
                    join_ts!(items_with_fields.iter(), (i, fields),
                        TupleNMutator(fields.len()) "::new("
                            join_ts!(fields.iter(), field,
                                ident!("mutator_" enu.items[*i].ident "_" field.access())
                            , separator: ",")
                        ")"
                    , separator: ",")
                ")
            }
        }
    }
    impl " EnumMutator_generics cm.Default "for" EnumMutator EnumMutator_generics.removing_bounds_and_eq_type() 
        Default_where_clause "
    {
        fn default() -> Self {
            Self { mutator : <_>::default() }
        }
    }
    impl " EnumMutator_generics cm.fuzzcheck_mutator_traits_Mutator "<" enu.ident generics_no_eq_nor_bounds "> 
        for " EnumMutator EnumMutator_generics.removing_bounds_and_eq_type() EnumMutator_where_clause "
    {
        type Cache = <" InnerMutator " as " cm.fuzzcheck_mutator_traits_Mutator "<" enu.ident generics_no_eq_nor_bounds ">" ">::Cache;
        type MutationStep = <" InnerMutator " as " cm.fuzzcheck_mutator_traits_Mutator "<" enu.ident generics_no_eq_nor_bounds ">" ">::MutationStep;
        type ArbitraryStep = <" InnerMutator " as " cm.fuzzcheck_mutator_traits_Mutator "<" enu.ident generics_no_eq_nor_bounds ">" ">::ArbitraryStep;
        type UnmutateToken = <" InnerMutator " as " cm.fuzzcheck_mutator_traits_Mutator "<" enu.ident generics_no_eq_nor_bounds ">" ">::UnmutateToken;
    
        fn cache_from_value(&self, value: &" enu.ident generics_no_eq_nor_bounds ") -> Self::Cache {
            " InnerMutator_as_Mutator "::cache_from_value(&self.mutator, value)
        }

        fn initial_step_from_value(&self, value: &" enu.ident generics_no_eq_nor_bounds ") -> Self::MutationStep {
            " InnerMutator_as_Mutator "::initial_step_from_value(&self.mutator, value)
        }

        fn max_complexity(&self) -> f64 {
            " InnerMutator_as_Mutator "::max_complexity(&self.mutator)
        }

        fn min_complexity(&self) -> f64 {
            " InnerMutator_as_Mutator "::min_complexity(&self.mutator)
        }

        fn complexity(&self, value: &" enu.ident generics_no_eq_nor_bounds ", cache: &Self::Cache) -> f64 {
            " InnerMutator_as_Mutator "::complexity(&self.mutator, value, cache)
        }

        fn ordered_arbitrary(&self, step: &mut Self::ArbitraryStep, max_cplx: f64) -> Option<(" enu.ident generics_no_eq_nor_bounds ", Self::Cache)> {
            " InnerMutator_as_Mutator "::ordered_arbitrary(&self.mutator, step, max_cplx)
        }

        fn random_arbitrary(&self, max_cplx: f64) -> (" enu.ident generics_no_eq_nor_bounds ", Self::Cache) {
            " InnerMutator_as_Mutator "::random_arbitrary(&self.mutator, max_cplx)
        }

        fn ordered_mutate(
            &self,
            value: &mut " enu.ident generics_no_eq_nor_bounds ",
            cache: &mut Self::Cache,
            step: &mut Self::MutationStep,
            max_cplx: f64,
        ) -> Option<Self::UnmutateToken> {
            " InnerMutator_as_Mutator "::ordered_mutate(
                &self.mutator,
                value,
                cache,
                step,
                max_cplx,
            )
        }

        fn random_mutate(&self, value: &mut " enu.ident generics_no_eq_nor_bounds ", cache: &mut Self::Cache, max_cplx: f64) -> Self::UnmutateToken {
            " InnerMutator_as_Mutator "::random_mutate(&self.mutator, value, cache, max_cplx)
        }

        fn unmutate(&self, value: &mut " enu.ident generics_no_eq_nor_bounds ", cache: &mut Self::Cache, t: Self::UnmutateToken) {
            " InnerMutator_as_Mutator "::unmutate(&self.mutator, value, cache, t)
        }
    }"
    if settings.default {
        ts!("impl" generics_no_eq cm.DefaultMutator "for" enu.ident generics_no_eq_nor_bounds DefaultMutator_where_clause "{
            type Mutator = " EnumMutator DefaultMutator_Mutator_generics ";
        
            fn default_mutator() -> Self::Mutator {
                Self::Mutator::new("
                    join_ts!(field_mutators.iter().flatten(), field_mutator,
                        match &field_mutator.kind {
                            FieldMutatorKind::Generic => {
                                ts!("<" field_mutator.field.ty "as" cm.DefaultMutator ">::default_mutator()")
                            }
                            FieldMutatorKind::Prescribed(_, Some(init)) => {
                                ts!(init)
                            }
                            FieldMutatorKind::Prescribed(mutator, None) => {
                                ts!("<" mutator "as" cm.Default ">::default()")
                            }
                        }
                    , separator: ",")
                ")
            }
        }")
    } else {
        ts!()
    }
        )
}

#[allow(non_snake_case)]
pub(crate) fn impl_basic_enum_structure(tb: &mut TokenBuilder, enu: &Enum, settings: &MakeMutatorSettings) {
    assert!(
        enu.items.len() > 0
            && enu
                .items
                .iter()
                .all(|item| !matches!(&item.data, Some(EnumItemData::Struct(_, fields)) if fields.len() > 0))
    );

    let BasicEnumStructure = ts!(settings.fuzzcheck_mutators_crate "::BasicEnumStructure");

    let items_init = enu
        .items
        .iter()
        .map(|item| match &item.data {
            Some(EnumItemData::Struct(kind, _)) => ts!(kind.open() kind.close()),
            _ => ts!(),
        })
        .collect::<Box<_>>();

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

#[allow(non_snake_case)]
pub(crate) fn impl_default_mutator_for_basic_enum(tb: &mut TokenBuilder, enu: &Enum, settings: &MakeMutatorSettings) {
    assert!(
        enu.items.len() > 0
            && enu
                .items
                .iter()
                .all(|item| !matches!(&item.data, Some(EnumItemData::Struct(_, fields)) if fields.len() > 0))
    );

    let cm = Common::new(0);

    let BasicEnumMutator = ts!(settings.fuzzcheck_mutators_crate "::BasicEnumMutator");

    extend_ts!(tb,
        "impl" cm.DefaultMutator "for " enu.ident " {
            type Mutator = " BasicEnumMutator ";
        
            fn default_mutator() -> Self::Mutator {
                Self::Mutator::default()
            }
        }"
    )
}

#[cfg(test)]
mod test {
    use crate::{decent_synquote_alternative::TokenBuilderExtend, MakeMutatorSettings};
    use decent_synquote_alternative::{parser::TokenParser, token_builder::TokenBuilder};
    use proc_macro2::TokenStream;

    use super::{
        impl_basic_enum_structure, impl_default_mutator_for_basic_enum, impl_default_mutator_for_enum,
        impl_enum_structure_trait, impl_mutator, make_enum_mutator_helper_types, make_enum_n_payload_mutator,
        make_enum_n_payload_structure,
    };

    #[test]
    fn test_impl_default_mutator_for_basic_enum() {
        let code = "
        enum X {
            A,
            B,
            C,
        }
        "
        .parse::<TokenStream>()
        .unwrap();
        let mut parser = TokenParser::new(code);
        let enu = parser.eat_enumeration().unwrap();

        let mut tb = TokenBuilder::new();
        impl_default_mutator_for_basic_enum(&mut tb, &enu, &<_>::default());
        let generated = tb.end().to_string();

        let expected = "
        impl fuzzcheck_mutators::DefaultMutator for X {
            type Mutator = fuzzcheck_mutators::BasicEnumMutator;
        
            fn default_mutator() -> Self::Mutator {
                Self::Mutator::default()
            }
        }
        "
        .parse::<TokenStream>()
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
        "
        .parse::<TokenStream>()
        .unwrap();
        let mut parser = TokenParser::new(code);
        let enu = parser.eat_enumeration().unwrap();

        let mut tb = TokenBuilder::new();
        impl_basic_enum_structure(&mut tb, &enu, &<_>::default());
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
        "
        .parse::<TokenStream>()
        .unwrap()
        .to_string();
        assert_eq!(generated, expected, "\n\n{}\n\n{}\n\n", generated, expected);
    }

    #[test]
    fn test_impl_default_mutator_for_enum_refactor() {
        let code = "
        pub enum Option<T> {
            Some(T),
            None,
        }        
        "
        .parse::<TokenStream>()
        .unwrap();
        let mut parser = TokenParser::new(code);
        let enu = parser.eat_enumeration().unwrap();

        let mut tb = TokenBuilder::new();
        let mut settings = MakeMutatorSettings::default();
        settings.fuzzcheck_mutators_crate = ts!("crate");
        impl_default_mutator_for_enum(&mut tb, &enu, &settings);
        let generated = tb.end().to_string();

        assert!(false, "\n\n{}\n\n", generated);
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
        "
        .parse::<TokenStream>()
        .unwrap();
        let mut parser = TokenParser::new(code);
        let enu = parser.eat_enumeration().unwrap();

        let mut tb = TokenBuilder::new();
        let mut settings = MakeMutatorSettings::default();
        settings.fuzzcheck_mutators_crate = ts!("crate");
        impl_default_mutator_for_enum(&mut tb, &enu, &settings);
        let generated = tb.end().to_string();

        let expected = "
        struct EMutator<T, M2_0, M2_1, M3_0>
        where
            T: ::std::clone::Clone + 'static,
            M2_0: fuzzcheck_mutators::fuzzcheck_traits::Mutator<(T, u8)> ,
            M2_1: fuzzcheck_mutators::fuzzcheck_traits::Mutator<u8> ,
            M3_0: fuzzcheck_mutators::fuzzcheck_traits::Mutator<u16>
        {
            pub mutator: fuzzcheck_mutators::Enum2PayloadMutator<
                ((T, u8), u8),
                fuzzcheck_mutators::Tuple2Mutator<(T, u8), u8, M2_0, M2_1>,
                fuzzcheck_mutators::Tuple2<(T, u8), u8> ,
                (u16),
                fuzzcheck_mutators::Tuple1Mutator<u16, M3_0>,
                fuzzcheck_mutators::Tuple1<u16>
            >
        }
        impl<T, M2_0, M2_1, M3_0> EMutator<T, M2_0, M2_1, M3_0>
        where
            T: ::std::clone::Clone + 'static,
            M2_0: fuzzcheck_mutators::fuzzcheck_traits::Mutator<(T, u8)> ,
            M2_1: fuzzcheck_mutators::fuzzcheck_traits::Mutator<u8> ,
            M3_0: fuzzcheck_mutators::fuzzcheck_traits::Mutator<u16>
        {
            pub fn new(mutator_Left_0: M2_0, mutator_Left_1: M2_1, mutator_Right_0: M3_0) -> Self {
                Self {
                    mutator: fuzzcheck_mutators::Enum2PayloadMutator::new(
                        fuzzcheck_mutators::Tuple2Mutator::new(mutator_Left_0, mutator_Left_1),
                        fuzzcheck_mutators::Tuple1Mutator::new(mutator_Right_0)
                    )
                }
            }
        }
        impl<T, M2_0, M2_1, M3_0> ::std::default::Default for EMutator<T, M2_0, M2_1, M3_0>
        where
            T: ::std::clone::Clone + 'static,
            M2_0: fuzzcheck_mutators::fuzzcheck_traits::Mutator<(T, u8)> ,
            M2_1: fuzzcheck_mutators::fuzzcheck_traits::Mutator<u8> ,
            M3_0: fuzzcheck_mutators::fuzzcheck_traits::Mutator<u16> ,
            fuzzcheck_mutators::Enum2PayloadMutator<
                ((T, u8), u8),
                fuzzcheck_mutators::Tuple2Mutator<(T, u8), u8, M2_0, M2_1>,
                fuzzcheck_mutators::Tuple2<(T, u8), u8> ,
                (u16),
                fuzzcheck_mutators::Tuple1Mutator<u16, M3_0>,
                fuzzcheck_mutators::Tuple1<u16>
            > : ::std::default::Default
        {
            fn default() -> Self {
                Self {
                    mutator: <_>::default()
                }
            }
        }
        impl<T, M2_0, M2_1, M3_0> fuzzcheck_mutators::fuzzcheck_traits::Mutator<E<T> > for EMutator<T, M2_0, M2_1, M3_0>
        where
            T: ::std::clone::Clone + 'static,
            M2_0: fuzzcheck_mutators::fuzzcheck_traits::Mutator<(T, u8)> ,
            M2_1: fuzzcheck_mutators::fuzzcheck_traits::Mutator<u8> ,
            M3_0: fuzzcheck_mutators::fuzzcheck_traits::Mutator<u16>
        {
            type Cache = <fuzzcheck_mutators::Enum2PayloadMutator<
                ((T, u8), u8),
                fuzzcheck_mutators::Tuple2Mutator<(T, u8), u8, M2_0, M2_1>,
                fuzzcheck_mutators::Tuple2<(T, u8), u8> ,
                (u16),
                fuzzcheck_mutators::Tuple1Mutator<u16, M3_0>,
                fuzzcheck_mutators::Tuple1<u16>
            > as fuzzcheck_mutators::fuzzcheck_traits::Mutator<E<T> > >::Cache;
            type MutationStep = <fuzzcheck_mutators::Enum2PayloadMutator<
                ((T, u8), u8),
                fuzzcheck_mutators::Tuple2Mutator<(T, u8), u8, M2_0, M2_1>,
                fuzzcheck_mutators::Tuple2<(T, u8), u8> ,
                (u16),
                fuzzcheck_mutators::Tuple1Mutator<u16, M3_0>,
                fuzzcheck_mutators::Tuple1<u16>
            > as fuzzcheck_mutators::fuzzcheck_traits::Mutator<E<T> > >::MutationStep;
            type ArbitraryStep = <fuzzcheck_mutators::Enum2PayloadMutator<
                ((T, u8), u8),
                fuzzcheck_mutators::Tuple2Mutator<(T, u8), u8, M2_0, M2_1>,
                fuzzcheck_mutators::Tuple2<(T, u8), u8> ,
                (u16),
                fuzzcheck_mutators::Tuple1Mutator<u16, M3_0>,
                fuzzcheck_mutators::Tuple1<u16>
            > as fuzzcheck_mutators::fuzzcheck_traits::Mutator<E<T> > >::ArbitraryStep;
            type UnmutateToken = <fuzzcheck_mutators::Enum2PayloadMutator<
                ((T, u8), u8),
                fuzzcheck_mutators::Tuple2Mutator<(T, u8), u8, M2_0, M2_1>,
                fuzzcheck_mutators::Tuple2<(T, u8), u8> ,
                (u16),
                fuzzcheck_mutators::Tuple1Mutator<u16, M3_0>,
                fuzzcheck_mutators::Tuple1<u16>
            > as fuzzcheck_mutators::fuzzcheck_traits::Mutator<E<T> > >::UnmutateToken;
            fn cache_from_value(&self, value: &E<T>) -> Self::Cache {
                <fuzzcheck_mutators::Enum2PayloadMutator<
                    ((T, u8), u8),
                    fuzzcheck_mutators::Tuple2Mutator<(T, u8), u8, M2_0, M2_1>,
                    fuzzcheck_mutators::Tuple2<(T, u8), u8> ,
                    (u16),
                    fuzzcheck_mutators::Tuple1Mutator<u16, M3_0>,
                    fuzzcheck_mutators::Tuple1<u16>
                > as fuzzcheck_mutators::fuzzcheck_traits::Mutator<E<T> > > ::cache_from_value(&self.mutator, value)
            }
            fn initial_step_from_value(&self, value: &E<T>) -> Self::MutationStep {
                <fuzzcheck_mutators::Enum2PayloadMutator<
                    ((T, u8), u8),
                    fuzzcheck_mutators::Tuple2Mutator<(T, u8), u8, M2_0, M2_1>,
                    fuzzcheck_mutators::Tuple2<(T, u8), u8> ,
                    (u16),
                    fuzzcheck_mutators::Tuple1Mutator<u16, M3_0>,
                    fuzzcheck_mutators::Tuple1<u16>
                > as fuzzcheck_mutators::fuzzcheck_traits::Mutator<E<T> > > ::initial_step_from_value(&self.mutator, value)
            }
            fn max_complexity(&self) -> f64 {
                <fuzzcheck_mutators::Enum2PayloadMutator<
                    ((T, u8), u8),
                    fuzzcheck_mutators::Tuple2Mutator<(T, u8), u8, M2_0, M2_1>,
                    fuzzcheck_mutators::Tuple2<(T, u8), u8> ,
                    (u16),
                    fuzzcheck_mutators::Tuple1Mutator<u16, M3_0>,
                    fuzzcheck_mutators::Tuple1<u16>
                > as fuzzcheck_mutators::fuzzcheck_traits::Mutator<E<T> > > ::max_complexity(&self.mutator)
            }
            fn min_complexity(&self) -> f64 {
                <fuzzcheck_mutators::Enum2PayloadMutator<
                    ((T, u8), u8),
                    fuzzcheck_mutators::Tuple2Mutator<(T, u8), u8, M2_0, M2_1>,
                    fuzzcheck_mutators::Tuple2<(T, u8), u8> ,
                    (u16),
                    fuzzcheck_mutators::Tuple1Mutator<u16, M3_0>,
                    fuzzcheck_mutators::Tuple1<u16>
                > as fuzzcheck_mutators::fuzzcheck_traits::Mutator<E<T> > > ::min_complexity(&self.mutator)
            }
            fn complexity(&self, value: &E<T> , cache: &Self::Cache) -> f64 {
                <fuzzcheck_mutators::Enum2PayloadMutator<
                    ((T, u8), u8),
                    fuzzcheck_mutators::Tuple2Mutator<(T, u8), u8, M2_0, M2_1>,
                    fuzzcheck_mutators::Tuple2<(T, u8), u8> ,
                    (u16),
                    fuzzcheck_mutators::Tuple1Mutator<u16, M3_0>,
                    fuzzcheck_mutators::Tuple1<u16>
                > as fuzzcheck_mutators::fuzzcheck_traits::Mutator<E<T> > > ::complexity(&self.mutator, value, cache)
            }
            fn ordered_arbitrary(&self, step: &mut Self::ArbitraryStep, max_cplx: f64) -> Option<(E<T> , Self::Cache)> {
                <fuzzcheck_mutators::Enum2PayloadMutator<
                    ((T, u8), u8),
                    fuzzcheck_mutators::Tuple2Mutator<(T, u8), u8, M2_0, M2_1>,
                    fuzzcheck_mutators::Tuple2<(T, u8), u8> ,
                    (u16),
                    fuzzcheck_mutators::Tuple1Mutator<u16, M3_0>,
                    fuzzcheck_mutators::Tuple1<u16>
                > as fuzzcheck_mutators::fuzzcheck_traits::Mutator<E<T> > > ::ordered_arbitrary(&self.mutator, step, max_cplx)
            }
            fn random_arbitrary(&self, max_cplx: f64) -> (E<T> , Self::Cache) {
                <fuzzcheck_mutators::Enum2PayloadMutator<
                    ((T, u8), u8),
                    fuzzcheck_mutators::Tuple2Mutator<(T, u8), u8, M2_0, M2_1>,
                    fuzzcheck_mutators::Tuple2<(T, u8), u8> ,
                    (u16),
                    fuzzcheck_mutators::Tuple1Mutator<u16, M3_0>,
                    fuzzcheck_mutators::Tuple1<u16>
                > as fuzzcheck_mutators::fuzzcheck_traits::Mutator<E<T> > > ::random_arbitrary(&self.mutator, max_cplx)
            }
            fn ordered_mutate(
                &self,
                value: &mut E<T> ,
                cache: &mut Self::Cache,
                step: &mut Self::MutationStep,
                max_cplx: f64 ,
            ) -> Option<Self::UnmutateToken> {
                <fuzzcheck_mutators::Enum2PayloadMutator<
                    ((T, u8), u8),
                    fuzzcheck_mutators::Tuple2Mutator<(T, u8), u8, M2_0, M2_1>,
                    fuzzcheck_mutators::Tuple2<(T, u8), u8> ,
                    (u16),
                    fuzzcheck_mutators::Tuple1Mutator<u16, M3_0>,
                    fuzzcheck_mutators::Tuple1<u16>
                > as fuzzcheck_mutators::fuzzcheck_traits::Mutator<E<T> > > ::ordered_mutate(
                    &self.mutator, value, cache, step, max_cplx ,
                )
            }
            fn random_mutate(&self, value: &mut E<T> , cache: &mut Self::Cache, max_cplx: f64) -> Self::UnmutateToken {
                <fuzzcheck_mutators::Enum2PayloadMutator<
                    ((T, u8), u8),
                    fuzzcheck_mutators::Tuple2Mutator<(T, u8), u8, M2_0, M2_1>,
                    fuzzcheck_mutators::Tuple2<(T, u8), u8> ,
                    (u16),
                    fuzzcheck_mutators::Tuple1Mutator<u16, M3_0>,
                    fuzzcheck_mutators::Tuple1<u16>
                > as fuzzcheck_mutators::fuzzcheck_traits::Mutator<E<T> > > ::random_mutate(&self.mutator, value, cache, max_cplx)
            }
            fn unmutate(&self, value: &mut E<T> , cache: &mut Self::Cache, t: Self::UnmutateToken) {
                <fuzzcheck_mutators::Enum2PayloadMutator<
                    ((T, u8), u8),
                    fuzzcheck_mutators::Tuple2Mutator<(T, u8), u8, M2_0, M2_1>,
                    fuzzcheck_mutators::Tuple2<(T, u8), u8> ,
                    (u16),
                    fuzzcheck_mutators::Tuple1Mutator<u16, M3_0>,
                    fuzzcheck_mutators::Tuple1<u16>
                > as fuzzcheck_mutators::fuzzcheck_traits::Mutator<E<T> > > ::unmutate(&self.mutator, value, cache, t)
            }
        }
        impl<T> fuzzcheck_mutators::DefaultMutator for E<T>
        where
            T: fuzzcheck_mutators::DefaultMutator + 'static
        {
            type Mutator = EMutator<
                T,
                <(T, u8) as fuzzcheck_mutators::DefaultMutator>::Mutator,
                <u8 as fuzzcheck_mutators::DefaultMutator>::Mutator,
                <u16 as fuzzcheck_mutators::DefaultMutator>::Mutator
            > ;
            fn default_mutator() -> Self::Mutator {
                Self::Mutator::new(
                    <(T, u8) as fuzzcheck_mutators::DefaultMutator>::default_mutator(),
                    <u8 as fuzzcheck_mutators::DefaultMutator>::default_mutator(),
                    <u16 as fuzzcheck_mutators::DefaultMutator>::default_mutator()
                )
            }
        }
        "
        .parse::<TokenStream>()
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
        "
        .parse::<TokenStream>()
        .unwrap();
        let mut parser = TokenParser::new(code);
        let enu = parser.eat_enumeration().unwrap();

        let mut tb = TokenBuilder::new();
        impl_enum_structure_trait(&mut tb, &enu);
        let generated = tb.end().to_string();

        let expected = "
        # [allow (non_shorthand_field_patterns)] 
        impl<T, U: Clone> fuzzcheck_mutators::Enum1PayloadStructure for E<T, U> where Vec<T>: Default, T: 'static, U: 'static {
            type TupleKind0 = fuzzcheck_mutators::Tuple3<T, u8, U> ;
            type T0 = (T, u8, U);

            fn get_ref<'a>(&'a self) -> fuzzcheck_mutators::Either2<(&'a T, &'a u8, &'a U), usize> {
                match self {
                    E::Foo{} => { fuzzcheck_mutators::Either2::T1(0) }
                    E::Bar() => { fuzzcheck_mutators::Either2::T1(1) }
                    E::Left(_0, _1, _2) => { fuzzcheck_mutators::Either2::T0((_0, _1, _2)) }
                    E::Baz => { fuzzcheck_mutators::Either2::T1(2) }
                }
            }
            fn get_mut<'a>(&'a mut self) -> fuzzcheck_mutators::Either2<(&'a mut T, &'a mut u8, &'a mut U), usize> {
                match self {
                    E::Foo{} => { fuzzcheck_mutators::Either2::T1(0) }
                    E::Bar() => { fuzzcheck_mutators::Either2::T1(1) }
                    E::Left(_0, _1, _2) => { fuzzcheck_mutators::Either2::T0((_0, _1, _2)) }
                    E::Baz => { fuzzcheck_mutators::Either2::T1(2) }
                }
            }
            fn new(t: fuzzcheck_mutators::Either2<Self::T0, usize>) -> Self {
                match t {
                    fuzzcheck_mutators::Either2::T0(x) => { E::Left{0: x.0, 1: x.1, 2: x.2} }
                    fuzzcheck_mutators::Either2::T1(x) => match x % 3 {
                        0 => Self::Foo{},
                        1 => Self::Bar{},
                        2 => Self::Baz,
                        _ => unreachable!(),
                    }
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
    fn test_impl_enum_structure_trait_2() {
        let code = "
        pub enum E<T> {
            Left { x: T, _y: u8 },
            Right(u16),
        }
        "
        .parse::<TokenStream>()
        .unwrap();
        let mut parser = TokenParser::new(code);
        let enu = parser.eat_enumeration().unwrap();

        let mut tb = TokenBuilder::new();
        impl_enum_structure_trait(&mut tb, &enu);
        let generated = tb.end().to_string();

        let expected = "
        # [allow (non_shorthand_field_patterns)] 
        impl<T> fuzzcheck_mutators::Enum2PayloadStructure for E<T> where T: 'static {
            type TupleKind0 = fuzzcheck_mutators::Tuple2<T, u8> ;
            type T0 = (T, u8);
            type TupleKind1 = fuzzcheck_mutators::Tuple1<u16> ;
            type T1 = (u16);

            fn get_ref<'a>(&'a self) -> fuzzcheck_mutators::Either3<(&'a T, &'a u8), (&'a u16), usize> {
                match self {
                    E::Left { x: x, _y: _y } => { fuzzcheck_mutators::Either3::T0((x, _y)) }
                    E::Right(_0) => { fuzzcheck_mutators::Either3::T1((_0)) }
                }
            }
            fn get_mut<'a>(&'a mut self) -> fuzzcheck_mutators::Either3<(&'a mut T, &'a mut u8), (&'a mut u16), usize> {
                match self {
                    E::Left { x: x, _y: _y } => { fuzzcheck_mutators::Either3::T0((x, _y)) }
                    E::Right(_0) => { fuzzcheck_mutators::Either3::T1((_0)) }
                }
            }
            fn new(t: fuzzcheck_mutators::Either3<Self::T0, Self::T1, usize>) -> Self {
                match t {
                    fuzzcheck_mutators::Either3::T0(x) => { E::Left{x: x.0, _y: x.1} }
                    fuzzcheck_mutators::Either3::T1(x) => { E::Right { 0: x } }
                    fuzzcheck_mutators::Either3::T2(x) => match x % 0 {
                        _ => unreachable!(),
                    }
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
    fn test_impl_enum_structure_trait() {
        let code = "
        pub enum E<T> {
            Foo,
            Bar,
            Left(T, u8),
            Right(u16),
        }
        "
        .parse::<TokenStream>()
        .unwrap();
        let mut parser = TokenParser::new(code);
        let enu = parser.eat_enumeration().unwrap();

        let mut tb = TokenBuilder::new();
        impl_enum_structure_trait(&mut tb, &enu);
        let generated = tb.end().to_string();

        let expected = "
        # [allow (non_shorthand_field_patterns)] 
        impl<T> fuzzcheck_mutators::Enum2PayloadStructure for E<T> where T: 'static {
            type TupleKind0 = fuzzcheck_mutators::Tuple2<T, u8> ;
            type T0 = (T, u8);
            type TupleKind1 = fuzzcheck_mutators::Tuple1<u16> ;
            type T1 = (u16);

            fn get_ref<'a>(&'a self) -> fuzzcheck_mutators::Either3<(&'a T, &'a u8), (&'a u16), usize> {
                match self {
                    E::Foo => { fuzzcheck_mutators::Either3::T2(0) }
                    E::Bar => { fuzzcheck_mutators::Either3::T2(1) }
                    E::Left(_0, _1) => { fuzzcheck_mutators::Either3::T0((_0, _1)) }
                    E::Right(_0) => { fuzzcheck_mutators::Either3::T1((_0)) }
                }
            }
            fn get_mut<'a>(&'a mut self) -> fuzzcheck_mutators::Either3<(&'a mut T, &'a mut u8), (&'a mut u16), usize> {
                match self {
                    E::Foo => { fuzzcheck_mutators::Either3::T2(0) }
                    E::Bar => { fuzzcheck_mutators::Either3::T2(1) }
                    E::Left(_0, _1) => { fuzzcheck_mutators::Either3::T0((_0, _1)) }
                    E::Right(_0) => { fuzzcheck_mutators::Either3::T1((_0)) }
                }
            }
            fn new(t: fuzzcheck_mutators::Either3<Self::T0, Self::T1, usize>) -> Self {
                match t {
                    fuzzcheck_mutators::Either3::T0(x) => { E::Left{0: x.0, 1: x.1} }
                    fuzzcheck_mutators::Either3::T1(x) => { E::Right { 0: x } }
                    fuzzcheck_mutators::Either3::T2(x) => match x % 2 {
                        0 => Self::Foo,
                        1 => Self::Bar,
                        _ => unreachable!(),
                    }
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
    fn test_impl_mutator() {
        let mut tb = TokenBuilder::new();
        impl_mutator(&mut tb, 2);
        let generated = tb.end().to_string();
        let expected = "
impl<T, T0, M0, TupleKind0, T1, M1, TupleKind1> ::fuzzcheck_traits::Mutator<T>
    for Enum2PayloadMutator<T0, M0, TupleKind0, T1, M1, TupleKind1>
where
    T: ::std::clone::Clone + Enum2PayloadStructure<TupleKind0 = TupleKind0, T0 = T0, TupleKind1 = TupleKind1, T1 = T1 , > ,
    T0: ::std::clone::Clone + fuzzcheck_mutators::TupleStructure<TupleKind0> ,
    M0: fuzzcheck_mutators::TupleMutator<T0, TupleKind0> ,
    TupleKind0: fuzzcheck_mutators::RefTypes,
    T1: ::std::clone::Clone + fuzzcheck_mutators::TupleStructure<TupleKind1> ,
    M1: fuzzcheck_mutators::TupleMutator<T1, TupleKind1> ,
    TupleKind1: fuzzcheck_mutators::RefTypes
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
                arbitrary: { 
                    let mut step: Self::ArbitraryStep = <_>::default() ;
                    step.steps.remove(0);
                    step
                },
            },
            Either3::T1(x) => Self::MutationStep {
                inner: Either3::T1(self.mutator_1.initial_step_from_value(x)),
                arbitrary: { 
                    let mut step: Self::ArbitraryStep = <_>::default() ;
                    step.steps.remove(1);
                    step
                },
            },
            Either3::T2(_) => Self::MutationStep {
                inner: Either3::T2(()),
                arbitrary: <_>::default(),
            },
        }
    }
    fn max_complexity(&self) -> f64 {
        fuzzcheck_mutators::size_to_cplxity(::std::mem::variant_count::<T>())
            + [self.mutator_0.max_complexity(), self.mutator_1.max_complexity()]
                .iter()
                .max_by(|x, y| x.partial_cmp(y).unwrap_or(Ordering::Equal))
                .unwrap()
    }
    fn min_complexity(&self) -> f64 {
        fuzzcheck_mutators::size_to_cplxity(::std::mem::variant_count::<T>())
            + [self.mutator_0.min_complexity(), self.mutator_1.min_complexity()]
                .iter()
                .min_by(|x, y| x.partial_cmp(y).unwrap_or(Ordering::Equal))
                .unwrap()
    }
    fn complexity(&self, value: &T, cache: &Self::Cache) -> f64 {
        fuzzcheck_mutators::size_to_cplxity(::std::mem::variant_count::<T>())
            + match (value.get_ref(), cache) {
                (Either3::T0(value), Either3::T0(cache)) => self.mutator_0.complexity(value, cache),
                (Either3::T1(value), Either3::T1(cache)) => self.mutator_1.complexity(value, cache),
                (Either3::T2(_), Either3::T2(_)) => 0.0,
                _ => unreachable!(),
            }
    }
    fn ordered_arbitrary(&self, step: &mut Self::ArbitraryStep, max_cplx: f64) -> Option<(T, Self::Cache)> {
        if max_cplx < <Self as ::fuzzcheck_traits::Mutator<T> >::min_complexity(self) { return ::std::option::Option::None }
        if step.steps.is_empty() {
            return ::std::option::Option::None;
        }
        let steps_len = step.steps.len();
        let inner_max_cplx = max_cplx - fuzzcheck_mutators::size_to_cplxity(::std::mem::variant_count::<T>());
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
                    ::std::option::Option::Some((T::new(Either3::T2(*x)), Either3::T2(())))
                } else {
                    ::std::option::Option::None
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
    fn random_arbitrary(&self, max_cplx: f64) -> (T, Self::Cache) {
        let inner_max_cplx = max_cplx - fuzzcheck_mutators::size_to_cplxity(::std::mem::variant_count::<T>());
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
        &self,
        value: &mut T,
        cache: &mut Self::Cache,
        step: &mut Self::MutationStep,
        max_cplx: f64,
    ) -> ::std::option::Option<Self::UnmutateToken> {
        if max_cplx < <Self as ::fuzzcheck_traits::Mutator<T> >::min_complexity(self) { return ::std::option::Option::None }
        let inner_max_cplx = max_cplx - fuzzcheck_mutators::size_to_cplxity(::std::mem::variant_count::<T>());

        if self.rng.usize(..100) == 0 {
            let (new_value, new_cache) = self.random_arbitrary(max_cplx);
            let old_value = ::std::mem::replace(value, new_value);
            let old_cache = ::std::mem::replace(cache, new_cache);
            return ::std::option::Option::Some(Either3::T2((old_value, old_cache)))
        }

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
    fn random_mutate(&self, value: &mut T, cache: &mut Self::Cache, max_cplx: f64) -> Self::UnmutateToken {
        let inner_max_cplx = max_cplx - fuzzcheck_mutators::size_to_cplxity(::std::mem::variant_count::<T>());
        if self.rng.usize(..100) == 0 {
            let (new_value, new_cache) = self.random_arbitrary(max_cplx);
            let old_value = ::std::mem::replace(value, new_value);
            let old_cache = ::std::mem::replace(cache, new_cache);
            return Either3::T2((old_value, old_cache))
        }

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
        make_enum_n_payload_mutator(&mut tb, 2);
        let generated = tb.end().to_string();
        let expected = "
pub struct Enum2PayloadMutator<T0, M0, TupleKind0, T1, M1, TupleKind1>
where
    T0: ::std::clone::Clone + fuzzcheck_mutators::TupleStructure<TupleKind0> ,
    TupleKind0: fuzzcheck_mutators::RefTypes,
    M0: fuzzcheck_mutators::TupleMutator<T0, TupleKind0> ,
    T1: ::std::clone::Clone + fuzzcheck_mutators::TupleStructure<TupleKind1> ,
    TupleKind1: fuzzcheck_mutators::RefTypes,
    M1: fuzzcheck_mutators::TupleMutator<T1, TupleKind1>
{
    pub mutator_0: M0,
    pub mutator_1: M1,
    rng: fuzzcheck_mutators::fastrand::Rng,
    _phantom: ::std::marker::PhantomData<(T0, TupleKind0, T1, TupleKind1)>
}

impl<T0, M0, TupleKind0, T1, M1, TupleKind1> Enum2PayloadMutator<T0, M0, TupleKind0, T1, M1, TupleKind1>
where
    T0: ::std::clone::Clone + fuzzcheck_mutators::TupleStructure<TupleKind0> ,
    TupleKind0: fuzzcheck_mutators::RefTypes,
    M0: fuzzcheck_mutators::TupleMutator<T0, TupleKind0> ,
    T1: ::std::clone::Clone + fuzzcheck_mutators::TupleStructure<TupleKind1> ,
    TupleKind1: fuzzcheck_mutators::RefTypes,
    M1: fuzzcheck_mutators::TupleMutator<T1, TupleKind1>
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
    T0: ::std::clone::Clone + fuzzcheck_mutators::TupleStructure<TupleKind0> ,
    TupleKind0: fuzzcheck_mutators::RefTypes,
    M0: fuzzcheck_mutators::TupleMutator<T0, TupleKind0> ,
    T1: ::std::clone::Clone + fuzzcheck_mutators::TupleStructure<TupleKind1> ,
    TupleKind1: fuzzcheck_mutators::RefTypes,
    M1: fuzzcheck_mutators::TupleMutator<T1, TupleKind1> ,
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
        make_enum_n_payload_structure(&mut tb, 2);
        let generated = tb.end().to_string();
        let expected = "
        #[derive(::std::clone::Clone)]
        pub enum Either3<T0, T1, T2> {
            T0(T0),
            T1(T1),
            T2(T2),
        }
        pub trait Enum2PayloadStructure {
            type TupleKind0: fuzzcheck_mutators::RefTypes;
            type T0: fuzzcheck_mutators::TupleStructure<Self::TupleKind0>;
            type TupleKind1: fuzzcheck_mutators::RefTypes;
            type T1: fuzzcheck_mutators::TupleStructure<Self::TupleKind1>;
        
            fn get_ref<'a>(
                &'a self
            ) -> Either3< <Self::TupleKind0 as fuzzcheck_mutators::RefTypes> ::Ref<'a> , <Self::TupleKind1 as fuzzcheck_mutators::RefTypes> ::Ref<'a> , usize>;
            fn get_mut<'a>(
                &'a mut self
            ) -> Either3< <Self::TupleKind0 as fuzzcheck_mutators::RefTypes> ::Mut<'a>, <Self::TupleKind1 as fuzzcheck_mutators::RefTypes> ::Mut<'a>, usize>;
            fn new(t: Either3<Self::T0, Self::T1, usize>) -> Self;
        }
        "
        .parse::<TokenStream>()
        .unwrap()
        .to_string();
        assert_eq!(generated, expected, "\n\n{}\n\n{}\n\n", generated, expected);
    }
}
