use std::{any::Any, collections::HashMap};

use decent_synquote_alternative as synquote;
use proc_macro2::{Ident, Span, TokenStream};

use synquote::parser::*;
use synquote::token_builder::*;

use crate::Common;

pub fn make_single_variant_mutator(tb: &mut TokenBuilder, enu: &Enum) {
    let cm = Common::new(0);

    let EnumSingleVariant = ident!(enu.ident "SingleVariant");

    let EnumSingleVariantMutator = ident!(enu.ident "SingleVariantMutator");
    let Tuplei = cm.Tuplei.as_ref();

    // item_fields: vector holding the item field types
    // item_mutators: the token stream of the tuple mutator for the item fields
    // item_pattern_match_bindings: the bindings made when pattern matching the item
    let (item_fields, item_mutators, item_pattern_match_bindings): (
        HashMap<Ident, Vec<TokenStream>>,
        HashMap<Ident, TokenStream>,
        HashMap<Ident, Vec<Ident>>,
    ) = {
        let mut item_fields = HashMap::new();
        let mut map = HashMap::new();
        let mut bindings = HashMap::new();
        for item in &enu.items {
            match item.get_struct_data() {
                Some((_, fields)) if !fields.is_empty() => {
                    item_fields.insert(item.ident.clone(), fields.iter().map(|x| ts!(x.ty)).collect());
                    let field_tys = join_ts!(fields.iter(), field, field.ty, separator: ",");
                    map.insert(
                        item.ident.clone(),
                        ts!(
                            cm.TupleMutator "< (" field_tys ") ," Tuplei(fields.len()) "<" field_tys "> >"
                        ),
                    );
                    bindings.insert(
                        item.ident.clone(),
                        fields.iter().map(|field| field.safe_ident()).collect(),
                    );
                }
                _ => {
                    item_fields.insert(item.ident.clone(), vec![]);
                    map.insert(
                        item.ident.clone(),
                        ts!(
                            cm.fuzzcheck_mutator_traits_Mutator "<()>"
                        ),
                    );
                    bindings.insert(item.ident.clone(), vec![]);
                }
            }
        }
        (item_fields, map, bindings)
    };

    let single_variant_generics_for_prefix = |prefix: &Ident| Generics {
        lifetime_params: vec![],
        type_params: enu
            .items
            .iter()
            .map(|item| TypeParam {
                type_ident: ts!(ident!(prefix item.ident)),
                ..<_>::default()
            })
            .collect(),
    };
    let single_variant_generics = single_variant_generics_for_prefix(&ident!("M"));
    let enum_generics_no_eq = enu.generics.removing_eq_type();
    let enum_generics_no_bounds = enu.generics.removing_bounds_and_eq_type();

    let mut enum_where_clause_plus_cond = enu.where_clause.clone().unwrap_or_default();
    enum_where_clause_plus_cond.add_clause_items(join_ts!(&enu.generics.type_params, tp,
        tp.type_ident ":" cm.Clone "+ 'static ,"
    ));
    let impl_mutator_generics = {
        let mut impl_mutator_generics = enu.generics.clone();
        for lp in &single_variant_generics.lifetime_params {
            impl_mutator_generics.lifetime_params.push(lp.clone());
        }
        for tp in &single_variant_generics.type_params {
            impl_mutator_generics.type_params.push(tp.clone());
        }
        impl_mutator_generics
    };
    let mut impl_mutator_where_clause = enum_where_clause_plus_cond.clone();
    impl_mutator_where_clause.add_clause_items(join_ts!(&enu.items, item,
        ident!("M" item.ident) ":" item_mutators[&item.ident] ","
    ));

    let item_pattern_match_bindings_to_tuple = |item_ident, append: Option<Ident>, mutable| {
        if item_fields[item_ident].is_empty() {
            if mutable {
                ts!("&mut ()")
            } else {
                ts!("&()")
            }
        } else {
            ts!("("
                join_ts!(item_pattern_match_bindings[item_ident].iter(), binding,
                    if let Some(append) = &append {
                        ident!(binding append)
                    } else {
                        ident!(binding)
                    }
                , separator: ",")
                ")"
            )
        }
    };
    let item_pattern_match_bindings_to_enum_item = |item: &EnumItem| {
        let fields = item.get_struct_data().map(|x| x.1).unwrap_or_default();
        ts!(
            enu.ident "::" item.ident "{"
            if fields.len() == 1 {
                ts!(fields[0].access() ": v")
            } else {
                join_ts!(fields.iter().enumerate(), (i, field),
                    field.access() ": v." i
                , separator: ",")
            }
            "}"
        )
    };

    extend_ts!(tb,
    "#[derive(" cm.Clone ")]
    pub enum " EnumSingleVariant single_variant_generics.removing_eq_type() "{"
    join_ts!(&enu.items, item,
        item.ident "(" ident!("M" item.ident) "),"
    )
    "}
    #[derive(" cm.Default ")]
    pub struct " EnumSingleVariantMutator enum_generics_no_eq enum_where_clause_plus_cond " {
        _phantom:" cm.PhantomData "<(" join_ts!(&enum_generics_no_bounds.type_params, tp, tp, separator: ",") ")>
    }
    #[allow(non_snake_case)]
    impl" enum_generics_no_eq EnumSingleVariantMutator enum_generics_no_bounds enum_where_clause_plus_cond " {"
    join_ts!(&enu.items, item,
        "pub fn" item.ident "<" ident!("M" item.ident) ">(m:" ident!("M" item.ident) ")
            ->" EnumSingleVariant "<"
            join_ts!(&enu.items, item_i,
                if item_i.ident == item.ident {
                    ts!(ident!("M" item.ident))
                } else {
                    ts!(cm.NeverMutator)
                }
            , separator: ",")
            "> where" ident!("M" item.ident) ":" item_mutators[&item.ident]
        "{"
            EnumSingleVariant "::" item.ident "(m)"
        "}"
    )
    "}
    impl " impl_mutator_generics.removing_eq_type() cm.fuzzcheck_mutator_traits_Mutator "<" enu.ident enum_generics_no_bounds "> 
        for " EnumSingleVariant single_variant_generics.removing_bounds_and_eq_type() impl_mutator_where_clause 
    "{
        type Cache = " EnumSingleVariant
            single_variant_generics.mutating_type_params(|tp| {
                tp.type_ident = ts!(tp.type_ident "::Cache")
            }) ";
        type MutationStep = " EnumSingleVariant
            single_variant_generics.mutating_type_params(|tp| {
                tp.type_ident = ts!(tp.type_ident "::MutationStep")
            }) ";
        type ArbitraryStep = " EnumSingleVariant
            single_variant_generics.mutating_type_params(|tp| {
                tp.type_ident = ts!(tp.type_ident "::ArbitraryStep")
            }) ";
        type UnmutateToken = " EnumSingleVariant
            single_variant_generics.mutating_type_params(|tp| {
                tp.type_ident = ts!(tp.type_ident "::UnmutateToken")
            }) ";
    
        fn default_arbitrary_step(&self) -> Self::ArbitraryStep {
            match self {"
                join_ts!(&enu.items, item,
                    EnumSingleVariant "::" item.ident "(m) =>" EnumSingleVariant "::" item.ident "(m.default_arbitrary_step()),"
                )
            "}
        }

        fn cache_from_value(&self, value: &" enu.ident enum_generics_no_bounds ") -> Self::Cache {
            match (self, value) {"
            join_ts!(&enu.items, item,
                "(" EnumSingleVariant "::" item.ident "(m)," item.pattern_match(&enu.ident, None) ") => {
                    "
                    EnumSingleVariant "::" item.ident "(m.cache_from_value(" item_pattern_match_bindings_to_tuple(&item.ident, None, false) "))"
                    "
                }"
            )" _ => unreachable!(),
            }
        }
        fn initial_step_from_value(&self, value: &" enu.ident enum_generics_no_bounds ") -> Self::MutationStep {
            match (self, value) {"
            join_ts!(&enu.items, item,
                "(" EnumSingleVariant "::" item.ident "(m)," item.pattern_match(&enu.ident, None) ") => {
                    "
                    EnumSingleVariant "::" item.ident "(m.initial_step_from_value(" item_pattern_match_bindings_to_tuple(&item.ident, None, false) "))"
                    "
                }"
            )" _ => unreachable!(),
            }
        }
        fn max_complexity(&self) -> f64 {
            match self {"
            join_ts!(&enu.items, item,
                EnumSingleVariant "::" item.ident "(m) => m.max_complexity() ,"
            )"
            }
        }
        fn min_complexity(&self) -> f64 {
            match self {"
            join_ts!(&enu.items, item,
                EnumSingleVariant "::" item.ident "(m) => m.min_complexity() ,"
            )"
            }
        }
        fn complexity(&self, value: &" enu.ident enum_generics_no_bounds ", cache: &Self::Cache) -> f64 {
            match (self, value, cache) {"
            join_ts!(&enu.items, item,
                "(
                    " EnumSingleVariant ":: " item.ident " (m) ,
                    " item.pattern_match(&enu.ident, None) ",
                    " EnumSingleVariant ":: " item.ident " (c) 
                 ) => {
                     m.complexity(" item_pattern_match_bindings_to_tuple(&item.ident, None, false) ", c) 
                 }"
            )   "_ => unreachable!()
            }
        }
        fn ordered_arbitrary(&self, step: &mut Self::ArbitraryStep, max_cplx: f64) -> Option<(" enu.ident enum_generics_no_bounds ", Self::Cache)> {
            match (self, step) {"
            join_ts!(&enu.items, item,
                "(" EnumSingleVariant "::" item.ident "(m)," EnumSingleVariant "::" item.ident "(s)) => {"
                    "if let" cm.Some "((v, c)) = m.ordered_arbitrary(s, max_cplx) {
                        " cm.Some "(("
                            item_pattern_match_bindings_to_enum_item(&item) ","
                            EnumSingleVariant "::" item.ident "(c)
                        ))
                    } else {
                        None
                    }
                }"
            ) "_ => unreachable!()
            }
        }

        fn random_arbitrary(&self, max_cplx: f64) -> (" enu.ident enum_generics_no_bounds ", Self::Cache) {
            match self {"
            join_ts!(&enu.items, item,
                EnumSingleVariant "::" item.ident "(m) => {
                    let (v, c) = m.random_arbitrary(max_cplx);
                    (" 
                        item_pattern_match_bindings_to_enum_item(&item) ",
                        " EnumSingleVariant "::" item.ident "(c)
                    )
                }"
            )"}
        }

        fn ordered_mutate(
            &self,
            value: &mut " enu.ident enum_generics_no_bounds ",
            cache: &mut Self::Cache,
            step: &mut Self::MutationStep,
            max_cplx: f64,
        ) -> Option<Self::UnmutateToken> {
            match (self, value, cache, step) {"
            join_ts!(&enu.items, item,
                "(
                    " EnumSingleVariant "::" item.ident "(m) ,
                    " item.pattern_match(&enu.ident, None) ",
                    " EnumSingleVariant "::" item.ident "(c) ,
                    " EnumSingleVariant "::" item.ident "(s)
                ) => {
                    m.ordered_mutate(" item_pattern_match_bindings_to_tuple(&item.ident, None, true) ", c, s, max_cplx)
                        .map(" EnumSingleVariant "::" item.ident ")
                }"
            )" _ => unreachable!(),
            }
        }
        fn random_mutate(&self, value: &mut " enu.ident enum_generics_no_bounds ", cache: &mut Self::Cache, max_cplx: f64) -> Self::UnmutateToken {
            match (self, value, cache) {"
            join_ts!(&enu.items, item,
                "(
                    " EnumSingleVariant "::" item.ident "(m) ,
                    " item.pattern_match(&enu.ident, None) ",
                    " EnumSingleVariant "::" item.ident "(c)
                ) => {
                    " EnumSingleVariant "::" item.ident "(
                        m.random_mutate(" 
                            item_pattern_match_bindings_to_tuple(&item.ident, None, true) ", c, max_cplx"
                        ")
                    )
                }"
            )   "_ => unreachable!()"
            "}
        }
        fn unmutate(&self, value: &mut " enu.ident enum_generics_no_bounds ", cache: &mut Self::Cache, t: Self::UnmutateToken) {
            match (self, value, cache, t) {"
            join_ts!(&enu.items, item,
                "(
                    " EnumSingleVariant "::" item.ident "(m) ,
                    " item.pattern_match(&enu.ident, None) ",
                    " EnumSingleVariant "::" item.ident "(c) ,
                    " EnumSingleVariant "::" item.ident "(t)
                ) => {"
                    "m.unmutate(" item_pattern_match_bindings_to_tuple(&item.ident, None, true) ", c, t)"
                "}"
            )" _ => unreachable!()
            }
        }
    }
    ");

    let other_single_variant_generics = single_variant_generics_for_prefix(&ident!("N"));

    let impl_or_mutator_generics = {
        let mut impl_or_mutator_generics = impl_mutator_generics.clone();
        for lp in &other_single_variant_generics.lifetime_params {
            impl_or_mutator_generics.lifetime_params.push(lp.clone());
        }
        for tp in &other_single_variant_generics.type_params {
            impl_or_mutator_generics.type_params.push(tp.clone());
        }
        impl_or_mutator_generics
    };
    let mut impl_or_mutator_where_clause = impl_mutator_where_clause.clone();
    impl_or_mutator_where_clause.add_clause_items(join_ts!(&enu.items, item,
        ident!("N" item.ident) ":" item_mutators[&item.ident] ","
    ));
    impl_or_mutator_where_clause.add_clause_items(ts!(
        cm.EqualProof "<"
            EnumSingleVariant single_variant_generics.removing_bounds_and_eq_type() ","
            EnumSingleVariant other_single_variant_generics.removing_bounds_and_eq_type()
        "> :" cm.NotEqual
    ));

    let mut impl_supertype_where_clause = impl_or_mutator_where_clause.clone();

    impl_or_mutator_where_clause.add_clause_items(ts!(join_ts!(&enu.items, item, {
        let fields = &item_fields[&item.ident];
        let field_tys = join_ts!(fields, f, f, separator: ",");
        if fields.is_empty() {
            ts!(
                ident!("M" item.ident) ":"
                    cm.CommonMutatorSuperType "<"
                        "(),"
                        ident!("N" item.ident)
                    "> ,"
            )
        } else {
            ts!(
                ident!("M" item.ident) ":"
                    cm.CommonTupleMutatorSuperType "<
                        ("  field_tys ") ,"
                        Tuplei(fields.len()) "<" field_tys "> ,"
                        ident!("N" item.ident)
                    "> ,"
            )
        }
    })));
    impl_supertype_where_clause.add_clause_items(ts!(join_ts!(&enu.items, item, {
        let fields = &item_fields[&item.ident];
        let field_tys = join_ts!(fields, f, f, separator: ",");
        if fields.is_empty() {
            ts!(
                ident!("M" item.ident) ":"
                    cm.MutatorSuperType "<"
                        "(),"
                        ident!("N" item.ident)
                    "> ,"
            )
        } else {
            ts!(
                ident!("M" item.ident) ":"
                    cm.TupleMutatorSuperType "<
                        ("  field_tys ") ,"
                        Tuplei(fields.len()) "<" field_tys "> ,"
                        ident!("N" item.ident)
                    "> ,"
            )
        }
    })));

    extend_ts!(tb,
    "impl " impl_or_mutator_generics.removing_eq_type()
        cm.CommonMutatorSuperType "<" enu.ident enum_generics_no_bounds "," EnumSingleVariant other_single_variant_generics.removing_bounds_and_eq_type() ">"
        "for " EnumSingleVariant single_variant_generics.removing_bounds_and_eq_type()
        impl_or_mutator_where_clause "
    {
        type Output = " EnumSingleVariant single_variant_generics.mutating_type_params(|tp| {
            tp.type_ident = ts!(tp.type_ident "::Output")
        }) ";
    }
    
    impl " impl_or_mutator_generics.removing_eq_type() 
        cm.MutatorSuperType "<" enu.ident enum_generics_no_bounds "," EnumSingleVariant other_single_variant_generics.removing_bounds_and_eq_type() ">
        for " EnumSingleVariant single_variant_generics.removing_bounds_and_eq_type() "
        " impl_supertype_where_clause "
        {
            fn upcast(m: " EnumSingleVariant other_single_variant_generics ") -> Self {
                match m {"
                join_ts!(&enu.items, item,
                    EnumSingleVariant "::" item.ident "(m) => " EnumSingleVariant "::" item.ident "(" ident!("M" item.ident) "::upcast(m)),"
                )
                "}
            }
        }"
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_make_single_variant_mutator() {
        let code = "
        pub enum AST<T: SomeTrait> where T: Default {
            Text(Vec<char>),
            Child { x: Box<AST>, y: T },
            Leaf1,
            Leaf2 {},
        }
        "
        .parse::<TokenStream>()
        .unwrap();
        let mut parser = TokenParser::new(code);
        let enu = parser.eat_enumeration().unwrap();

        let mut tb = TokenBuilder::new();
        make_single_variant_mutator(&mut tb, &enu);
        let generated = tb.end().to_string();

        let expected = "
        #[derive(:: std :: clone :: Clone)]
        pub enum ASTSingleVariant<MText, MChild, MLeaf1, MLeaf2> {
            Text(MText),
            Child(MChild),
            Leaf1(MLeaf1),
            Leaf2(MLeaf2),
        }
        
        #[derive(::std::default::Default)]
        pub struct ASTSingleVariantMutator<T: SomeTrait> where T: Default, T: ::std::clone::Clone + 'static {
            _phantom: ::std::marker::PhantomData<(T)>
        }

        #[allow(non_snake_case)]
        impl<T: SomeTrait> ASTSingleVariantMutator<T> where T: Default, T: ::std::clone::Clone + 'static {
            pub fn Text<MText>(m: MText) -> ASTSingleVariant<MText, fuzzcheck_mutators::NeverMutator, fuzzcheck_mutators::NeverMutator, fuzzcheck_mutators::NeverMutator>
            where
                MText: fuzzcheck_mutators::TupleMutator<(Vec<char>), fuzzcheck_mutators::Tuple1<Vec<char> > >
            {
                ASTSingleVariant::Text(m)
            }
            pub fn Child<MChild>(m: MChild) -> ASTSingleVariant<fuzzcheck_mutators::NeverMutator, MChild, fuzzcheck_mutators::NeverMutator, fuzzcheck_mutators::NeverMutator>
            where
                MChild: fuzzcheck_mutators::TupleMutator<(Box<AST>, T), fuzzcheck_mutators::Tuple2<Box<AST>, T> >
            {
                ASTSingleVariant::Child(m)
            }
            pub fn Leaf1<MLeaf1>(m: MLeaf1) -> ASTSingleVariant<fuzzcheck_mutators::NeverMutator, fuzzcheck_mutators::NeverMutator, MLeaf1, fuzzcheck_mutators::NeverMutator>
            where
                MLeaf1: fuzzcheck_mutators::fuzzcheck_traits::Mutator<()>
            {
                ASTSingleVariant::Leaf1(m)
            }
            pub fn Leaf2<MLeaf2>(m: MLeaf2) -> ASTSingleVariant<fuzzcheck_mutators::NeverMutator, fuzzcheck_mutators::NeverMutator, fuzzcheck_mutators::NeverMutator, MLeaf2>
            where
                MLeaf2: fuzzcheck_mutators::fuzzcheck_traits::Mutator<()>
            {
                ASTSingleVariant::Leaf2(m)
            }
        }
        impl<T: SomeTrait, MText, MChild, MLeaf1, MLeaf2> fuzzcheck_mutators::fuzzcheck_traits::Mutator<AST<T> > for ASTSingleVariant<MText, MChild, MLeaf1, MLeaf2>
            where 
            T: Default, 
            T: ::std::clone::Clone + 'static ,
            MText: fuzzcheck_mutators::TupleMutator<(Vec<char>), fuzzcheck_mutators::Tuple1<Vec<char> > > ,
            MChild: fuzzcheck_mutators::TupleMutator<(Box<AST>, T), fuzzcheck_mutators::Tuple2<Box<AST>, T> > ,
            MLeaf1: fuzzcheck_mutators::fuzzcheck_traits::Mutator<()> ,
            MLeaf2: fuzzcheck_mutators::fuzzcheck_traits::Mutator<()>
        {
            type Cache = ASTSingleVariant<MText::Cache, MChild::Cache, MLeaf1::Cache, MLeaf2::Cache> ;
            type MutationStep =
                ASTSingleVariant<MText::MutationStep, MChild::MutationStep, MLeaf1::MutationStep, MLeaf2::MutationStep> ;
            type ArbitraryStep = ASTSingleVariant<
                MText::ArbitraryStep,
                MChild::ArbitraryStep,
                MLeaf1::ArbitraryStep,
                MLeaf2::ArbitraryStep
            > ;
            type UnmutateToken = ASTSingleVariant<
                MText::UnmutateToken,
                MChild::UnmutateToken,
                MLeaf1::UnmutateToken,
                MLeaf2::UnmutateToken
            > ;
            fn default_arbitrary_step(&self) -> Self::ArbitraryStep {
                match self {
                    ASTSingleVariant::Text(m) => ASTSingleVariant::Text(m.default_arbitrary_step()) ,
                    ASTSingleVariant::Child(m) => ASTSingleVariant::Child(m.default_arbitrary_step()) ,
                    ASTSingleVariant::Leaf1(m) => ASTSingleVariant::Leaf1(m.default_arbitrary_step()) ,
                    ASTSingleVariant::Leaf2(m) => ASTSingleVariant::Leaf2(m.default_arbitrary_step()) ,
                }
            }

            fn cache_from_value(&self, value: &AST<T>) -> Self::Cache {
                match (self, value) {
                    (ASTSingleVariant::Text(m), AST::Text(_0)) => {
                        ASTSingleVariant::Text(m.cache_from_value((_0)))
                    }
                    (ASTSingleVariant::Child(m), AST::Child { x: x, y: y }) => {
                        ASTSingleVariant::Child(m.cache_from_value((x, y)))
                    }
                    (ASTSingleVariant::Leaf1(m), AST::Leaf1) => {
                        ASTSingleVariant::Leaf1(m.cache_from_value(&()))
                    }
                    (ASTSingleVariant::Leaf2(m), AST::Leaf2 { }) => {
                        ASTSingleVariant::Leaf2(m.cache_from_value(&()))
                    }
                    _ => unreachable!() ,
                }
            }

            fn initial_step_from_value(&self, value: &AST<T>) -> Self::MutationStep {
                match (self, value) {
                    (ASTSingleVariant::Text(m), AST::Text(_0)) => {
                        ASTSingleVariant::Text(m.initial_step_from_value((_0)))
                    }
                    (ASTSingleVariant::Child(m), AST::Child {x: x, y: y}) => {
                        ASTSingleVariant::Child(m.initial_step_from_value((x, y)))
                    }
                    (ASTSingleVariant::Leaf1(m), AST::Leaf1) => {
                        ASTSingleVariant::Leaf1(m.initial_step_from_value(&()))
                    }
                    (ASTSingleVariant::Leaf2(m), AST::Leaf2 { }) => {
                        ASTSingleVariant::Leaf2(m.initial_step_from_value(&()))
                    }
                    _ => unreachable!() ,
                }
            }
            fn max_complexity(&self) -> f64 {
                match self {
                    ASTSingleVariant::Text(m) => m.max_complexity() ,
                    ASTSingleVariant::Child(m) => m.max_complexity() ,
                    ASTSingleVariant::Leaf1(m) => m.max_complexity() ,
                    ASTSingleVariant::Leaf2(m) => m.max_complexity() ,
                }
            }
            fn min_complexity(&self) -> f64 {
                match self {
                    ASTSingleVariant::Text(m) => m.min_complexity() ,
                    ASTSingleVariant::Child(m) => m.min_complexity() ,
                    ASTSingleVariant::Leaf1(m) => m.min_complexity() ,
                    ASTSingleVariant::Leaf2(m) => m.min_complexity() ,
                }
            }
            fn complexity(&self, value: &AST<T> , cache: &Self::Cache) -> f64 {
                match (self, value, cache) {
                    (ASTSingleVariant::Text(m), AST::Text(_0), ASTSingleVariant::Text(c)) => {
                        m.complexity((_0), c)
                    }
                    (ASTSingleVariant::Child(m), AST::Child { x : x , y : y }, ASTSingleVariant::Child(c)) => {
                        m.complexity((x, y), c)
                    }
                    (ASTSingleVariant::Leaf1(m), AST::Leaf1, ASTSingleVariant::Leaf1(c)) => {
                        m.complexity(&(), c)
                    }
                    (ASTSingleVariant::Leaf2(m), AST::Leaf2 { }, ASTSingleVariant::Leaf2(c)) => {
                        m.complexity(&(), c)
                    }
                    _ => unreachable!()
                }
            }

            fn ordered_arbitrary(&self, step: &mut Self::ArbitraryStep, max_cplx: f64) -> Option<(AST<T> , Self::Cache)> {
                match (self, step) {
                    (ASTSingleVariant::Text(m), ASTSingleVariant::Text(s)) => {
                        if let ::std::option::Option::Some((v, c)) = m.ordered_arbitrary(s, max_cplx) {
                            ::std::option::Option::Some((AST::Text { 0: v }, ASTSingleVariant::Text(c)))
                        } else {
                            None
                        }
                    }
                    (ASTSingleVariant::Child(m), ASTSingleVariant::Child(s)) => {
                        if let ::std::option::Option::Some((v, c)) = m.ordered_arbitrary(s, max_cplx) {
                            ::std::option::Option::Some((AST::Child { x: v.0, y: v.1 }, ASTSingleVariant::Child(c)))
                        } else {
                            None
                        }
                    }
                    (ASTSingleVariant::Leaf1(m), ASTSingleVariant::Leaf1(s)) => {
                        if let ::std::option::Option::Some((v, c)) = m.ordered_arbitrary(s, max_cplx) {
                            ::std::option::Option::Some((AST::Leaf1 {}, ASTSingleVariant::Leaf1(c)))
                        } else {
                            None
                        }
                    }
                    (ASTSingleVariant::Leaf2(m), ASTSingleVariant::Leaf2(s)) => {
                        if let ::std::option::Option::Some((v, c)) = m.ordered_arbitrary(s, max_cplx) {
                            ::std::option::Option::Some((AST::Leaf2 {}, ASTSingleVariant::Leaf2(c)))
                        } else {
                            None
                        }
                    }
                    _ => unreachable!()
                }
            }
            fn random_arbitrary(&self, max_cplx: f64) -> (AST<T> , Self::Cache) {
                match self {
                    ASTSingleVariant::Text(m) => {
                        let (v, c) = m.random_arbitrary(max_cplx);
                        (AST::Text { 0: v }, ASTSingleVariant::Text(c))
                    }
                    ASTSingleVariant::Child(m) => {
                        let (v, c) = m.random_arbitrary(max_cplx);
                        (AST::Child { x: v.0, y: v.1 }, ASTSingleVariant::Child(c))
                    }
                    ASTSingleVariant::Leaf1(m) => {
                        let (v, c) = m.random_arbitrary(max_cplx);
                        (AST::Leaf1 { }, ASTSingleVariant::Leaf1(c))
                    }
                    ASTSingleVariant::Leaf2(m) => {
                        let (v, c) = m.random_arbitrary(max_cplx);
                        (AST::Leaf2 { }, ASTSingleVariant::Leaf2(c))
                    }
                }
            }
            fn ordered_mutate(
                &self,
                value: &mut AST<T> ,
                cache: &mut Self::Cache,
                step: &mut Self::MutationStep,
                max_cplx: f64,
            ) -> Option<Self::UnmutateToken> {
                match (self, value, cache, step) {
                    (
                        ASTSingleVariant::Text(m),
                        AST::Text(_0),
                        ASTSingleVariant::Text(c),
                        ASTSingleVariant::Text(s)
                    ) => {
                        m.ordered_mutate((_0), c, s, max_cplx)
                        .map(ASTSingleVariant::Text)
                    }
                    (
                        ASTSingleVariant::Child(m),
                        AST::Child { x: x, y: y },
                        ASTSingleVariant::Child(c),
                        ASTSingleVariant::Child(s)
                    ) => {
                        m.ordered_mutate((x, y), c, s, max_cplx)
                        .map(ASTSingleVariant::Child)
                    }
                    (
                        ASTSingleVariant::Leaf1(m),
                        AST::Leaf1,
                        ASTSingleVariant::Leaf1(c),
                        ASTSingleVariant::Leaf1(s)
                    ) => {
                        m.ordered_mutate(&mut (), c, s, max_cplx)
                        .map(ASTSingleVariant::Leaf1)
                    }
                    (
                        ASTSingleVariant::Leaf2(m),
                        AST::Leaf2 {},
                        ASTSingleVariant::Leaf2(c),
                        ASTSingleVariant::Leaf2(s)
                    ) => { 
                        m.ordered_mutate(&mut (), c, s, max_cplx)
                        .map(ASTSingleVariant::Leaf2) 
                    }
                    _ => unreachable!() ,
                }
            }
            fn random_mutate(&self, value: &mut AST<T> , cache: &mut Self::Cache, max_cplx: f64) -> Self::UnmutateToken {
                match (self, value, cache) {
                    (ASTSingleVariant::Text(m), AST::Text(_0), ASTSingleVariant::Text(c)) => {
                        ASTSingleVariant::Text(m.random_mutate((_0), c, max_cplx))
                    }
                    (ASTSingleVariant::Child(m), AST::Child { x: x, y: y }, ASTSingleVariant::Child(c)) => {
                        ASTSingleVariant::Child(m.random_mutate((x, y), c, max_cplx))
                    }
                    (ASTSingleVariant::Leaf1(m), AST::Leaf1, ASTSingleVariant::Leaf1(c)) => {
                        ASTSingleVariant::Leaf1(m.random_mutate(&mut (), c, max_cplx))
                    }
                    (ASTSingleVariant::Leaf2(m), AST::Leaf2 {}, ASTSingleVariant::Leaf2(c)) => {
                        ASTSingleVariant::Leaf2(m.random_mutate(&mut (), c, max_cplx))
                    }
                    _ => unreachable!()
                }
            }
            fn unmutate(&self, value: &mut AST<T> , cache: &mut Self::Cache, t: Self::UnmutateToken) {
                match (self, value, cache, t) {
                    (
                        ASTSingleVariant::Text(m),
                        AST::Text(_0),
                        ASTSingleVariant::Text(c),
                        ASTSingleVariant::Text(t)
                    ) => {
                        m.unmutate((_0), c, t)
                    }
                    (
                        ASTSingleVariant::Child(m),
                        AST::Child { x: x , y: y },
                        ASTSingleVariant::Child(c),
                        ASTSingleVariant::Child(t)
                    ) => {
                        m.unmutate((x, y), c, t)
                    }
                    (
                        ASTSingleVariant::Leaf1(m),
                        AST::Leaf1,
                        ASTSingleVariant::Leaf1(c),
                        ASTSingleVariant::Leaf1(t)
                    ) => {
                        m.unmutate(&mut (), c, t)
                    }
                    (
                        ASTSingleVariant::Leaf2(m),
                        AST::Leaf2 { },
                        ASTSingleVariant::Leaf2(c),
                        ASTSingleVariant::Leaf2(t)
                    ) => {
                        m.unmutate(&mut (), c, t)
                    }
                    _ => unreachable!()
                }
            }
        }
        "
        .parse::<TokenStream>()
        .unwrap()
        .to_string();
        assert_eq!(generated, expected, "\n\n{}\n\n{}\n\n", generated, expected);
    }
}
