mod parser;
mod token_builder;

//mod struct_derive;

use crate::parser::*;
use crate::token_builder::*;

use proc_macro::{Ident, Literal, Span, TokenStream};

macro_rules! opt_ts {
    ($opt:expr, $map_pat:pat, $($part:expr) *) => {
        {
            if let Some($map_pat) = $opt {
                ts!($($part) *)
            } else {
                ts!()
            }
        }
    };
}

macro_rules! join_ts {
    ($iter:expr) => {
        {
            #[allow(unused_mut)]
            let mut tb = TokenBuilder::new();
            for part in $iter {
                tb.extend(part);
            }
            tb.end()
        }
    };
    ($iter:expr, separator: $sep:expr) => {
        {
            #[allow(unused_mut)]
            let mut tb = TokenBuilder::new();
            let mut add_sep = false;
            for part in $iter {
                if add_sep {
                    $sep.add_to(&mut tb);
                }
                tb.extend(part);
                add_sep = true;
            }
            tb.end()
        }
    };
    ($iter:expr, $part_pat:pat, $($part:expr) *) => {
        {
            #[allow(unused_mut)]
            let mut tb = TokenBuilder::new();
            for $part_pat in $iter {
                extend_ts!(&mut tb,
                    $($part) *
                );
            }
            tb.end()
        }
    };
    ($iter:expr, $part_pat:pat, $($part:expr) *, separator: $sep:expr) => {
        {
            #[allow(unused_mut)]
            let mut tb = TokenBuilder::new();
            let mut add_sep = false;
            for $part_pat in $iter {
                if add_sep {
                    $sep.add_to(&mut tb);
                }
                extend_ts!(&mut tb,
                    $($part) *
                );
                add_sep = true;
            }
            tb.end()
        }
    };
}

macro_rules! extend_ts {
    ($tb:expr, $($part:expr) *) => {
        {
            $(
                $part.add_to($tb);
            )*
        }
    };
}

macro_rules! ts {
    ($($part:expr) *) => {
        {
            #[allow(unused_mut)]
            let mut tb = TokenBuilder::new();
            $(
                $part.add_to(&mut tb);
            )*
            tb.end()
        }
    };
}

macro_rules! ident {
    ($($x:expr) *) => {{
        let mut s = String::new();
        $(
            s.push_str(&$x.to_string());
        )*
        Ident::new(&s, Span::call_site())
    }};
}

#[proc_macro_attribute]
pub fn fuzzcheck_derive_mutator(attr: TokenStream, input: TokenStream) -> TokenStream {
    let mut tb = TokenBuilder::new();
    input.add_to(&mut tb);

    let mut parser = TokenParser::new(input);

    while let Some(_) = parser.eat_outer_attribute() {}

    let mut attr_parser = TokenParser::new(attr);
    let derive_default = attr_parser.eat_ident("DefaultMutator").is_some();

    if let Some(s) = parser.eat_struct() {
        derive_struct_mutator(s, derive_default, &mut tb);
    } else if let Some(e) = parser.eat_enumeration() {
        derive_enum_mutator(e, derive_default, &mut tb)
    } else {
        extend_ts!(&mut tb,
            "compile_error ! ("
            Literal::string("fuzzcheck_mutators_derive could not parse the structure")
            ") ;"
        )
    }
    //tb.eprint();
    tb.end()
}

fn derive_struct_mutator(parsed_struct: Struct, derive_default: bool, tb: &mut TokenBuilder) {
    if !parsed_struct.struct_fields.is_empty() {
        derive_struct_mutator_with_fields(&parsed_struct, derive_default, tb)
    } else {
        derive_unit_mutator(parsed_struct, derive_default, tb);
    }
}

struct DerivedStructFieldIdentifiers {
    orig: TokenStream,
    muta: Ident,
    m_value: Ident,
    m_cache: Ident,
    ty: TokenStream,
    generic_type: TokenStream,
}

fn derive_struct_mutator_with_fields(parsed_struct: &Struct, derive_default: bool, tb: &mut TokenBuilder) {
    let field_idents = parsed_struct
        .struct_fields
        .iter()
        .map(|f| {
            let orig = f.access();
            let muta = f.safe_ident();
            let m_value = ident!(muta "_value");
            let m_cache = ident!(muta "_cache");
            let ty = f.ty.clone();
            let generic_type = ts!(ident!(muta "Type"));
            DerivedStructFieldIdentifiers {
                orig,
                muta,
                m_value,
                m_cache,
                ty,
                generic_type,
            }
        })
        .collect::<Vec<_>>();

    let generics_without_bounds = parsed_struct.generics.removing_bounds_and_eq_type();

    let basic_generics = Generics {
        lifetime_params: Vec::new(),
        type_params: field_idents
            .iter()
            .map(|ids| TypeParam {
                type_ident: ids.generic_type.clone(),
                ..<_>::default()
            })
            .collect(),
    };

    let basic_fields = field_idents
        .iter()
        .map(|ids| StructField {
            attributes: Vec::new(),
            visibility: ts!("pub"),
            identifier: StructFieldIdentifier::Named(ids.muta.clone()),
            ty: ids.generic_type.clone(),
        })
        .collect::<Vec<_>>();

    let value_struct_ident_with_generic_params = ts!(
        parsed_struct.ident generics_without_bounds
    );

    let mutator_struct = {
        let mut generics = parsed_struct.generics.clone();

        let mut where_clause_items = parsed_struct.where_clause.clone().map(|wc| wc.items).unwrap_or(vec![]);

        for ids in field_idents.iter() {
            generics.type_params.push(TypeParam {
                type_ident: ids.generic_type.clone(),
                ..<_>::default()
            });
            where_clause_items.push(WhereClauseItem {
                for_lifetimes: None,
                lhs: ids.ty.clone(),
                rhs: ts!(":: core :: clone :: Clone"),
            });
            where_clause_items.push(WhereClauseItem {
                for_lifetimes: None,
                lhs: ids.generic_type.clone(),
                rhs: ts!("fuzzcheck_mutators :: fuzzcheck_traits :: Mutator < Value = " ids.ty ">"),
            });
        }

        where_clause_items.push(WhereClauseItem {
            for_lifetimes: None,
            lhs: value_struct_ident_with_generic_params.clone(),
            rhs: ts!(":: core :: clone :: Clone"),
        });

        let mut mutator_struct_fields = basic_fields.clone();
        mutator_struct_fields.push(StructField {
            attributes: Vec::new(),
            visibility: ts!("pub"),
            identifier: StructFieldIdentifier::Named(ident!("rng")),
            ty: ts!("fuzzcheck_mutators :: fastrand :: Rng"),
        });

        Struct {
            visibility: parsed_struct.visibility.clone(),
            ident: ident!(parsed_struct.ident "Mutator"),
            generics,
            kind: Some(StructKind::Struct),
            where_clause: Some(WhereClause {
                items: where_clause_items,
            }),
            struct_fields: mutator_struct_fields,
        }
    };

    tb.extend(&mutator_struct);

    let mutator_cache_struct = {
        let mut cache_fields = basic_fields.clone();
        cache_fields.push(StructField {
            attributes: Vec::new(),
            visibility: ts!("pub"),
            identifier: StructFieldIdentifier::Named(ident!("cplx")),
            ty: ts!("f64"),
        });
        Struct {
            visibility: parsed_struct.visibility.clone(),
            ident: ident!(parsed_struct.ident "MutatorCache"),
            generics: basic_generics.clone(),
            kind: Some(StructKind::Struct),
            where_clause: None,
            struct_fields: cache_fields.clone(),
        }
    };

    extend_ts!(tb,
        "# [ derive ( :: core :: clone :: Clone ) ]
        # [ allow ( non_camel_case_types ) ]"
        mutator_cache_struct
    );

    let (mutator_inner_step_enum, mutator_step_struct) = {
        let inner = Enum {
            visibility: parsed_struct.visibility.clone(),
            ident: ident!(parsed_struct.ident "InnerMutationStep"),
            generics: Generics::default(),
            where_clause: None,
            items: field_idents
                .iter()
                .map(|f| EnumItem {
                    attributes: Vec::new(),
                    ident: f.muta.clone(),
                    data: None,
                })
                .collect(),
        };

        let mut step_fields = basic_fields.clone();
        step_fields.push(StructField {
            attributes: Vec::new(),
            visibility: ts!("pub"),
            identifier: StructFieldIdentifier::Named(ident!("step")),
            ty: ts!("usize"),
        });
        step_fields.push(StructField {
            visibility: ts!("pub"),
            identifier: StructFieldIdentifier::Named(ident!("inner")),
            ty: ts!("Vec <" inner.ident ">"),
            ..StructField::default()
        });

        (
            inner,
            Struct {
                visibility: parsed_struct.visibility.clone(),
                ident: ident!(parsed_struct.ident "MutationStep"),
                generics: basic_generics.clone(),
                kind: Some(StructKind::Struct),
                where_clause: None,
                struct_fields: step_fields,
            },
        )
    };

    extend_ts!(tb,
        "# [ derive ( :: core :: clone :: Clone ) ]
        # [ allow ( non_camel_case_types ) ]"
        mutator_inner_step_enum
        "# [ derive ( :: core :: clone :: Clone ) ]
        # [ allow ( non_camel_case_types ) ]"
        mutator_step_struct
    );

    let /*(arbitrary_inner_step_enum,*/ arbitrary_step_struct /*)*/ = {
        // let inner_enum = Enum {
        //     visibility: ts!("pub"),
        //     ident: ident!(parsed_struct.ident "InnerArbitraryStep"),
        //     generics: <_>::default(),
        //     where_clause: None,
        //     items: field_idents.iter().map(|ids| {
        //         EnumItem {
        //             attributes: Vec::new(),
        //             ident: ids.muta.clone(),
        //             data: None,
        //         }
        //     }).collect()
        // };
        let /*mut*/ step_fields = field_idents.iter().map(|ids|
            StructField {
                identifier: StructFieldIdentifier::Named(ids.muta.clone()), 
                ty: ts!(ids.generic_type),
                ..<_>::default()
            }
        ).collect::<Vec<_>>();
    
        // step_fields.push(StructField {
        //     identifier: StructFieldIdentifier::Named(ident!("fields")), 
        //     ty: ts!(":: std :: vec :: Vec <" inner_enum.ident ">"),
        //     ..<_>::default()
        // });

        let step_struct = Struct {
            visibility: ts!("pub"),
            ident: ident!(parsed_struct.ident "ArbitraryStep"),
            generics: basic_generics.clone(),
            kind: Some(StructKind::Struct),
            where_clause: None,
            struct_fields: step_fields,
        };
        /*(inner_enum, */ step_struct /*)*/
    };
    // let ar_step_default_where_clause = WhereClause {
    //     items: arbitrary_step_struct.generics.type_params.iter().map(|tp|
    //         WhereClauseItem {
    //             for_lifetimes: None,
    //             lhs: tp.type_ident.clone(),
    //             rhs: ts!(":: core :: default :: Default"),
    //         }
    //     ).collect(),
    // };

    extend_ts!(tb,
        //"# [ derive ( :: core :: clone :: Clone ) ]
        //# [ allow ( non_camel_case_types ) ]"
        //arbitrary_inner_step_enum
        "# [ derive ( :: core :: clone :: Clone , :: core :: default :: Default ) ]
        # [ allow ( non_camel_case_types ) ]"
        arbitrary_step_struct

    //     "impl" arbitrary_step_struct.generics ":: core :: default :: Default for" arbitrary_step_struct.ident
    //         arbitrary_step_struct.generics ar_step_default_where_clause
    //     "{
    //         fn default ( ) -> Self {
    //             Self {"
    //                 join_ts!(&field_idents, ids,
    //                     ids.muta ": < _ > :: default ( ) ,"
    //                 )
    //                 "fields : < _ > :: default ( )
    //             }
    //         }
    //     }"
    );

    let unmutate_token_struct = {
        let mut step_fields = basic_fields
            .iter()
            .map(|field| StructField {
                ty: ts!(":: std :: option :: Option <" field.ty ">"),
                ..field.clone()
            })
            .collect::<Vec<StructField>>();

        step_fields.push(StructField {
            attributes: Vec::new(),
            visibility: ts!("pub"),
            identifier: StructFieldIdentifier::Named(ident!("cplx")),
            ty: ts!("f64"),
        });
        Struct {
            visibility: parsed_struct.visibility.clone(),
            ident: ident!(parsed_struct.ident "UnmutateToken"),
            generics: basic_generics,
            kind: Some(StructKind::Struct),
            where_clause: None,
            struct_fields: step_fields,
        }
    };

    extend_ts!(tb,
        "# [ derive ( :: core :: clone :: Clone ) ]
        # [ allow ( non_camel_case_types ) ]"
        unmutate_token_struct
    );

    // default impl for unmutate token
    extend_ts!(tb,
        "impl" unmutate_token_struct.generics ":: core :: default :: Default for" unmutate_token_struct.ident unmutate_token_struct.generics "{
            fn default ( ) -> Self {
                Self {"
                    join_ts!(&field_idents, ids,
                        ids.muta ": None ,"
                    )
                    "cplx : f64 :: default ( )
                }
            }
        }"
    );

    // implementation of Mutator trait
    extend_ts!(tb,
        // TODO: I think there is a bug here if the generics have an equal clause in the struct
        "impl" mutator_struct.generics "fuzzcheck_mutators :: fuzzcheck_traits :: Mutator for"
            mutator_struct.ident mutator_struct.generics.removing_bounds_and_eq_type() mutator_struct.where_clause
        "{
            type Value = " value_struct_ident_with_generic_params ";
            type Cache = " mutator_cache_struct.ident
                mutator_cache_struct.generics.mutating_type_params(|tp|
                    tp.type_ident = ts!("<" tp.type_ident "as fuzzcheck_mutators :: fuzzcheck_traits :: Mutator > :: Cache")
                )
                ";
            
            type MutationStep = "
                mutator_step_struct.ident
                mutator_step_struct.generics.mutating_type_params(|tp|
                    tp.type_ident = ts!("<" tp.type_ident "as fuzzcheck_mutators :: fuzzcheck_traits :: Mutator > :: MutationStep")
                )
                ";

            type ArbitraryStep = "
                arbitrary_step_struct.ident
                arbitrary_step_struct.generics.mutating_type_params(|tp|
                    tp.type_ident = ts!("<" tp.type_ident "as fuzzcheck_mutators :: fuzzcheck_traits :: Mutator > :: ArbitraryStep")
                )
                ";
            type UnmutateToken = "
                unmutate_token_struct.ident
                unmutate_token_struct.generics.mutating_type_params(|tp|
                    tp.type_ident = ts!("<" tp.type_ident "as fuzzcheck_mutators :: fuzzcheck_traits :: Mutator > :: UnmutateToken")
                )
                ";

            fn max_complexity ( & self ) -> f64 {"
                join_ts!(&field_idents, ids,
                    "self . " ids.muta ". max_complexity ( )"
                , separator: "+")
            "}

            fn min_complexity ( & self ) -> f64 {"
                join_ts!(&field_idents, ids,
                    "self . " ids.muta ". min_complexity ( )"
                , separator: "+")
            "}
            
            fn complexity ( & self , value : & Self :: Value , cache : & Self :: Cache ) -> f64 { cache . cplx }

            fn cache_from_value ( & self , value : & Self :: Value ) -> Self :: Cache {"
                // declare all subcaches
                join_ts!(&field_idents, ids,
                    "let" ids.muta "= self ." ids.muta ". cache_from_value ( & value ." ids.orig ") ;"
                )
                // compute cplx
                "let cplx =" join_ts!(&field_idents, ids,
                    "self . " ids.muta ". complexity ( & value ." ids.orig ", &" ids.muta ")"
                , separator: "+") ";"

                "Self :: Cache {"
                    join_ts!(&field_idents, ids , ids.muta , separator: ",")
                    ", cplx
                }
            }

            fn initial_step_from_value ( & self , value : & Self :: Value ) -> Self :: MutationStep {"
                // init all substeps
                join_ts!(&field_idents, ids,
                    "let" ids.muta "= self ." ids.muta ". initial_step_from_value ( & value ." ids.orig ") ;"
                )

                "let step = 0 ;

                Self :: MutationStep {"
                    join_ts!(&field_idents, ids , ids.muta , separator: ",")
                    ", inner : vec ! [" join_ts!(&mutator_inner_step_enum.items, item,
                        mutator_inner_step_enum.ident "::" item.ident
                    , separator: ",") "]
                    , step
                }
            }

            fn ordered_arbitrary ( & mut self , step : & mut Self :: ArbitraryStep , max_cplx : f64 ) -> Option < ( Self :: Value , Self :: Cache ) > {
                Some ( self . random_arbitrary ( max_cplx ) )
            }

            fn random_arbitrary ( & mut self , max_cplx : f64 ) -> ( Self :: Value , Self :: Cache ) {"
                join_ts!(&field_idents, ids,
                    "let mut" ids.m_value ": Option < _ > = None ;
                     let mut" ids.m_cache ": Option < _ > = None ;"
                )
                "let mut indices = ( 0 .. " field_idents.len() ") . collect :: < Vec < _ > > ( ) ;
                fuzzcheck_mutators :: fastrand :: shuffle ( & mut indices ) ;
                let seed = fuzzcheck_mutators :: fastrand :: usize ( .. ) ;
                let mut cplx = f64 :: default ( ) ;

                for idx in indices . iter ( ) {
                    match idx {"
                    join_ts!(field_idents.iter().enumerate(), (i, ids),
                        i "=> {
                            let ( value , cache ) = self . " ids.muta " . random_arbitrary ( max_cplx - cplx ) ;
                            cplx += self . " ids.muta ". complexity ( & value , & cache ) ; " 
                            ids.m_value "= Some ( value ) ;"
                            ids.m_cache "= Some ( cache ) ;
                        }"
                    )
                            "_ => unreachable ! ( )
                    }
                }
                (
                    Self :: Value {"
                        join_ts!(&field_idents, ids,
                            ids.orig ":" ids.m_value ". unwrap ( ) ,"
                        )
                    "} ,
                    Self :: Cache {"
                        join_ts!(&field_idents, ids,
                            ids.muta ":" ids.m_cache " . unwrap ( ) ,"
                        )
                        "cplx
                    }
                )
            }

            fn ordered_mutate ( & mut self , value : & mut Self :: Value , cache : & mut Self :: Cache , step : & mut Self :: MutationStep , max_cplx : f64 ) -> Option < Self :: UnmutateToken >
            {
                if step . inner . is_empty ( ) {
                    return None
                }
                let orig_step = step . step ;
                step . step += 1 ;
                let current_cplx = self . complexity ( value , cache ) ;
                let mut inner_step_to_remove : Option < usize > = None ;
                let mut recurse = false ;
                match step . inner [ orig_step % step . inner . len ( ) ] {"
                join_ts!(field_idents.iter(), ids,
                    mutator_inner_step_enum.ident "::" ids.muta " => {
                        let current_field_cplx = self ." ids.muta ". complexity ( & value ." ids.orig ", & cache ." ids.muta ") ;
                        let max_field_cplx = max_cplx - current_cplx + current_field_cplx ;
                        if let Some ( token ) = self ." ids.muta ". ordered_mutate ( & mut  value ." ids.orig ", & mut cache ." ids.muta ", & mut step ." ids.muta ", max_field_cplx ) {
                            let new_field_complexity = self ." ids.muta ". complexity ( & value ." ids.orig ", & cache ." ids.muta ") ;
                            cache . cplx = cache . cplx - current_field_cplx + new_field_complexity ;
                            return Some ( Self :: UnmutateToken {"
                                ids.muta ": Some ( token ) ,
                                cplx : current_cplx ,
                                .. Self :: UnmutateToken :: default ( )
                            } )
                        } else {
                            inner_step_to_remove = Some ( orig_step % step . inner . len ( ) ) ;
                            recurse = true ;
                        }
                    }"
                )
                "}
                if let Some ( idx ) = inner_step_to_remove {
                    step . inner . remove ( idx ) ;
                }
                if recurse {
                    self . ordered_mutate ( value , cache , step , max_cplx )
                } else {
                    unreachable ! ( )
                }
            }

            fn random_mutate ( & mut self , value : & mut Self :: Value , cache : & mut Self :: Cache , max_cplx : f64 ) -> Self :: UnmutateToken {
                let current_cplx = self . complexity ( value , cache ) ;
                match self . rng . usize ( .. ) % " field_idents.len() " {"
                    join_ts!(field_idents.iter().enumerate(), (i, ids),
                        i "=> {
                            let current_field_cplx = self ." ids.muta ". complexity ( & value ." ids.orig ", & cache ." ids.muta ") ;
                            let max_field_cplx = max_cplx - current_cplx + current_field_cplx ;
                            let token = self ." ids.muta ". random_mutate ( & mut  value ." ids.orig ", & mut cache ." ids.muta ", max_field_cplx ) ;
                            let new_field_complexity = self ." ids.muta ". complexity ( & value ." ids.orig ", & cache ." ids.muta ") ;
                            cache . cplx = cache . cplx - current_field_cplx + new_field_complexity ;
                            return Self :: UnmutateToken {"
                                ids.muta ": Some ( token ) ,
                                cplx : current_cplx ,
                                .. Self :: UnmutateToken :: default ( )
                            }
                        }"
                    )
                    "_ => unreachable ! ( )"
                "}
            }

            fn unmutate ( & self , value : & mut Self :: Value , cache : & mut Self :: Cache , t : Self :: UnmutateToken )
            {
                cache . cplx = t . cplx ;"
                join_ts!(&field_idents, ids,
                    "if let Some ( subtoken ) = t ." ids.muta "{"
                        "self ." ids.muta ". unmutate ( & mut value ." ids.orig ", & mut cache ." ids.muta ", subtoken ) ;"
                    "}"
                )
            "}
        }"
    );
    if derive_default {
        // default impl
        let where_clause = {
            let mut where_clause = mutator_struct.where_clause.clone().unwrap_or_default();

            for field in &basic_fields {
                where_clause.items.push(WhereClauseItem {
                    for_lifetimes: None,
                    lhs: ts!(field.ty),
                    rhs: ts!(":: core :: default :: Default"),
                });
            }
            where_clause
        };

        extend_ts!(tb,
        "impl" mutator_struct.generics ":: core :: default :: Default for" mutator_struct.ident
            mutator_struct.generics.removing_bounds_and_eq_type() where_clause
        "{
            fn default ( ) -> Self {
                Self {"
                    join_ts!(&mutator_struct.struct_fields, field,
                        field.safe_ident() ":" "<" field.ty "as :: core :: default :: Default > :: default ( )"
                    , separator: ",")
                "}
            }
        }"
        );

        let where_clause = {
            let mut where_clause = parsed_struct.where_clause.clone().unwrap_or_default();
            for field in parsed_struct.struct_fields.iter() {
                where_clause.items.extend(vec![
                    WhereClauseItem {
                        for_lifetimes: None,
                        lhs: field.ty.clone(),
                        rhs: ts!(":: core :: clone :: Clone + fuzzcheck_mutators :: DefaultMutator"),
                    },
                    WhereClauseItem {
                        for_lifetimes: None,
                        lhs: ts!("<" field.ty " as fuzzcheck_mutators :: DefaultMutator > :: Mutator"),
                        rhs: ts!(":: core :: default :: Default"),
                    },
                    WhereClauseItem {
                        for_lifetimes: None,
                        lhs: value_struct_ident_with_generic_params.clone(),
                        rhs: ts!(":: core :: clone :: Clone"),
                    },
                ]);
            }
            where_clause
        };

        let generics_mutator = {
            let mut type_params = generics_without_bounds.type_params.clone();
            for field in parsed_struct.struct_fields.iter() {
                type_params.push(TypeParam {
                    type_ident: ts!("<" field.ty "as fuzzcheck_mutators :: DefaultMutator > :: Mutator"),
                    ..<_>::default()
                });
            }
            Generics {
                lifetime_params: generics_without_bounds.lifetime_params.clone(),
                type_params,
            }
        };

        extend_ts!(tb,
        "impl" parsed_struct.generics "fuzzcheck_mutators :: DefaultMutator for" parsed_struct.ident
            generics_without_bounds where_clause
        "{
            type Mutator = " mutator_struct.ident generics_mutator ";

            fn default_mutator ( ) -> Self :: Mutator {
                Self :: Mutator :: default ( )
            }
        }"
        )
    }
}

fn derive_enum_mutator(parsed_enum: Enum, derive_default: bool, tb: &mut TokenBuilder) {
    if !parsed_enum.items.is_empty() {
        derive_enum_mutator_with_items(&parsed_enum, derive_default, tb)
    } else {
        todo!("Build mutator for empty enum");
    }
}

fn derive_enum_mutator_with_items(parsed_enum: &Enum, derive_default: bool, tb: &mut TokenBuilder) {
    let (basic_generics, generic_items, flattened_fields, submutator_fields, mutator_struct) = {
        // mutator struct
        /*
        generics: existing generics + generic mutator type params
        where_clause_items: existing where_clause + “Mutator” conditions
        mutator_field_types: the generic types for the sub-mutators, one for each field
        */
        let mut generics = parsed_enum.generics.clone();
        let mut where_clause = parsed_enum.where_clause.clone().unwrap_or_default();

        let generic_items = parsed_enum
            .items
            .iter()
            .map(|item| match &item.data {
                Some(EnumItemData::Struct(_, fields)) if !fields.is_empty() => Some(EnumItem {
                    attributes: Vec::new(),
                    ident: item.ident.clone(),
                    data: Some(EnumItemData::Struct(
                        StructKind::Struct,
                        fields
                            .iter()
                            .map(|f| StructField {
                                identifier: StructFieldIdentifier::Named(ident!(item.ident f.safe_ident())),
                                ty: ts!(ident!(item.ident f.safe_ident() "Type")),
                                ..StructField::default()
                            })
                            .collect(),
                    )),
                }),
                _ => None,
            })
            .collect::<Vec<_>>();

        let submutator_fields = generic_items
            .iter()
            .filter_map(|x| x.as_ref())
            .flat_map(|x| x.get_fields_unchecked())
            .cloned()
            .collect::<Vec<_>>();

        // the generic types corresponding to each field in each item
        let basic_generics = Generics {
            lifetime_params: Vec::new(),
            type_params: submutator_fields
                .iter()
                .map(|x| TypeParam {
                    type_ident: x.ty.clone(),
                    ..<_>::default()
                })
                .collect(),
        };

        let flattened_fields = parsed_enum
            .items
            .iter()
            .flat_map(|item| match item.get_struct_data() {
                Some((_, fields)) if !fields.is_empty() => fields.to_vec(),
                _ => vec![],
            })
            .collect::<Vec<_>>();

        /*
           for each field, add a generic parameter for its mutator as well as a where_clause_item
           ensuring it impls the Mutator trait and that it impls the Clone trait
        */
        for (field, mutator_field) in flattened_fields.iter().zip(submutator_fields.clone()) {
            let ty_param = TypeParam {
                type_ident: mutator_field.ty.clone(),
                ..<_>::default()
            };
            generics.type_params.push(ty_param);
            where_clause.items.push(WhereClauseItem {
                for_lifetimes: None,
                lhs: field.ty.clone(),
                rhs: ts!(":: core :: clone :: Clone"),
            });
            where_clause.items.push(WhereClauseItem {
                for_lifetimes: None,
                lhs: mutator_field.ty.clone(),
                rhs: ts!("fuzzcheck_mutators :: fuzzcheck_traits :: Mutator < Value = " field.ty ">"),
            });
        }
        /* also add requirement that the whole value is Clone */
        where_clause.items.push(WhereClauseItem {
            for_lifetimes: None,
            lhs: ts!(parsed_enum.ident parsed_enum.generics.removing_bounds_and_eq_type()),
            rhs: ts!(":: core :: clone :: Clone"),
        });

        let mut mutator_struct_fields = submutator_fields.clone();
        mutator_struct_fields.push(StructField {
            attributes: Vec::new(),
            visibility: ts!("pub"),
            identifier: StructFieldIdentifier::Named(ident!("rng")),
            ty: ts!("fuzzcheck_mutators :: fastrand :: Rng"),
        });

        (
            basic_generics,
            generic_items,
            flattened_fields,
            submutator_fields,
            Struct {
                visibility: parsed_enum.visibility.clone(),
                ident: ident!(parsed_enum.ident "Mutator"),
                generics,
                kind: Some(StructKind::Struct),
                where_clause: Some(where_clause),
                struct_fields: mutator_struct_fields,
            },
        )
    };
    extend_ts!(tb, mutator_struct);

    let filtered_enum_items = parsed_enum
        .items
        .clone()
        .into_iter()
        .filter(|x| {
            if let Some(data) = x.get_struct_data() {
                !data.1.is_empty()
            } else {
                false
            }
        })
        .collect::<Vec<_>>();
    let filtered_generic_items = generic_items.clone().into_iter().filter_map(|x| x).collect::<Vec<_>>();

    let empty_enum_items = parsed_enum
        .items
        .clone()
        .into_iter()
        .filter(|x| {
            if let Some(data) = x.get_struct_data() {
                data.1.is_empty()
            } else {
                true
            }
        })
        .collect::<Vec<_>>();

    let (cache_enum, cache_struct) = {
        // mutator cache
        let cache_enum = Enum {
            visibility: parsed_enum.visibility.clone(),
            ident: ident!(parsed_enum.ident "InnerMutatorCache"),
            generics: basic_generics.clone(),
            where_clause: None,
            items: filtered_generic_items.clone(),
        };

        let cache_struct = Struct {
            visibility: parsed_enum.visibility.clone(),
            ident: ident!(parsed_enum.ident "MutatorCache"),
            generics: basic_generics.clone(),
            kind: Some(StructKind::Struct),
            where_clause: None,
            struct_fields: vec![
                StructField {
                    visibility: ts!("pub"),
                    identifier: StructFieldIdentifier::Named(ident!("inner")),
                    ty: ts!("Option <" cache_enum.ident cache_enum.generics ">"),
                    ..<_>::default()
                },
                StructField {
                    visibility: ts!("pub"),
                    identifier: StructFieldIdentifier::Named(ident!("cplx")),
                    ty: ts!("f64"),
                    ..<_>::default()
                },
            ],
        };

        (cache_enum, cache_struct)
    };

    extend_ts!(tb,
        "# [ derive ( core :: clone :: Clone ) ] "
        cache_enum
        "# [ derive ( core :: clone :: Clone ) ] "
        cache_struct
    );

    let (ar_step_enum, ar_step_struct) = {
        // mutation step
        let step_enum = Enum {
            visibility: parsed_enum.visibility.clone(),
            ident: ident!(parsed_enum.ident "InnerArbitraryStep"),
            generics: basic_generics.clone(),
            where_clause: None,
            items: filtered_generic_items
                .iter()
                .map(|item| {
                    if let Some((_, fields @ [_, _, ..])) = item.get_struct_data() {
                        let mut fields = fields.to_vec();
                        fields.push(StructField {
                            identifier: StructFieldIdentifier::Named(ident!("step")),
                            ty: ts!("usize"),
                            ..StructField::default()
                        });
                        EnumItem {
                            data: Some(EnumItemData::Struct(StructKind::Struct, fields)),
                            ..item.clone()
                        }
                    } else {
                        item.clone()
                    }
                })
                .collect(),
        };

        let field_inner = StructField {
            visibility: ts!("pub"),
            identifier: StructFieldIdentifier::Named(ident!("inner")),
            ty: ts!("Vec <" step_enum.ident step_enum.generics ">"),
            ..<_>::default()
        };
        let field_step = StructField {
            visibility: ts!("pub"),
            identifier: StructFieldIdentifier::Named(ident!("step")),
            ty: ts!("usize"),
            ..<_>::default()
        };

        let step_struct = Struct {
            visibility: parsed_enum.visibility.clone(),
            ident: ident!(parsed_enum.ident "ArbitraryStep"),
            generics: basic_generics.clone(),
            kind: Some(StructKind::Struct),
            where_clause: None,
            struct_fields: vec![field_inner, field_step],
        };

        (step_enum, step_struct)
    };
    extend_ts!(tb,
        "# [ derive ( core :: clone :: Clone ) ] "
        ar_step_enum
        "# [ derive ( core :: clone :: Clone ) ] "
        ar_step_struct
    );

    let mut sorted_ar_step_enum_items = ar_step_enum.items.clone();
    sorted_ar_step_enum_items.sort_by(|x, y| {
        let x = x.get_struct_data().map(|fs| fs.1.len()).unwrap_or(0);
        let y = y.get_struct_data().map(|fs| fs.1.len()).unwrap_or(0);
        x.cmp(&y)
    });

    // Default impl for ar_step_struct
    {
        let ar_step_default_where_clause = WhereClause {
            items: ar_step_struct
                .generics
                .type_params
                .iter()
                .map(|tp| WhereClauseItem {
                    for_lifetimes: None,
                    lhs: tp.type_ident.clone(),
                    rhs: ts!(":: core :: default :: Default"),
                })
                .collect(),
        };

        extend_ts!(tb,
            "impl" ar_step_struct.generics ":: core :: default :: Default for" ar_step_struct.ident
                ar_step_struct.generics ar_step_default_where_clause
            "{
                fn default ( ) -> Self {
                    Self {
                        inner : vec ! ["
                            join_ts!(&sorted_ar_step_enum_items, item,
                                ar_step_enum.ident "::" item.ident "{"
                                    join_ts!(item.get_fields_unchecked(), field,
                                        field.safe_ident() ": < _ > :: default ( )"
                                    , separator: ",")
                                "}"
                            , separator: ",")
                        "] ,
                        step : < _ > :: default ( )
                    }"
                "}
            }"
        );
    }

    let (step_enum_inners, step_enum, step_struct) = {
        let step_enum_inners = filtered_generic_items
            .iter()
            .map(|item| {
                let fields = item.get_fields_unchecked();

                let generics = Generics {
                    lifetime_params: vec![],
                    type_params: fields
                        .iter()
                        .map(|f| TypeParam {
                            type_ident: f.ty.clone(),
                            ..TypeParam::default()
                        })
                        .collect(),
                };
                Enum {
                    visibility: parsed_enum.visibility.clone(),
                    ident: ident!(parsed_enum.ident item.ident "InnerMutationStep"),
                    generics,
                    where_clause: None,
                    items: fields
                        .iter()
                        .map(|f| EnumItem {
                            attributes: vec![],
                            ident: f.safe_ident(),
                            data: Some(EnumItemData::Struct(
                                StructKind::Tuple,
                                vec![StructField {
                                    ty: f.ty.clone(),
                                    ..StructField::default()
                                }],
                            )),
                        })
                        .collect(),
                }
            })
            .collect::<Vec<_>>();

        let step_enum_items = filtered_generic_items
            .iter()
            .zip(step_enum_inners.iter())
            .map(|(inner_item, step)| EnumItem {
                attributes: vec![],
                ident: ident!(inner_item.ident),
                data: Some(EnumItemData::Struct(
                    StructKind::Tuple,
                    vec![StructField {
                        ty: ts!("Vec <" step.ident step.generics ">"),
                        ..StructField::default()
                    }],
                )),
            })
            .collect::<Vec<_>>();

        let step_enum = Enum {
            visibility: parsed_enum.visibility.clone(),
            ident: ident!(parsed_enum.ident "InnerMutationStep"),
            generics: basic_generics.clone(),
            where_clause: None,
            items: step_enum_items,
        };

        let field_inner = StructField {
            visibility: ts!("pub"),
            identifier: StructFieldIdentifier::Named(ident!("inner")),
            ty: ts!("Option <" step_enum.ident step_enum.generics ">"),
            ..<_>::default()
        };
        let field_step = StructField {
            visibility: ts!("pub"),
            identifier: StructFieldIdentifier::Named(ident!("step")),
            ty: ts!("usize"),
            ..<_>::default()
        };

        let mut generics = basic_generics.clone();
        generics.type_params.push(TypeParam {
            type_ident: ts!("ArbitraryStep"),
            ..TypeParam::default()
        });

        let field_ar_step = StructField {
            identifier: StructFieldIdentifier::Named(ident!("arbitrary_step")),
            ty: ts!("Option < ArbitraryStep >"),
            ..<_>::default()
        };

        let step_struct = Struct {
            visibility: parsed_enum.visibility.clone(),
            ident: ident!(parsed_enum.ident "MutationStep"),
            generics,
            kind: Some(StructKind::Struct),
            where_clause: None,
            struct_fields: vec![field_inner, field_step, field_ar_step],
        };

        (step_enum_inners, step_enum, step_struct)
    };

    extend_ts!(tb,
        join_ts!(&step_enum_inners, e,
            "# [ derive ( core :: clone :: Clone ) ] "
            e
        )
        "# [ derive ( core :: clone :: Clone ) ] "
        step_enum
        "# [ derive ( core :: clone :: Clone ) ] "
        step_struct
    );

    let step_struct_generics_for_assoc_type = {
        let mut generics = step_struct.generics.clone();
        let _ = generics.type_params.pop().unwrap();
        generics = generics.mutating_type_params(|tp| {
            tp.type_ident =
                ts!("<" tp.type_ident "as fuzzcheck_mutators :: fuzzcheck_traits :: Mutator > :: MutationStep")
        });
        generics.type_params.push(TypeParam {
            type_ident: ts!("Self :: ArbitraryStep"),
            ..TypeParam::default()
        });
        generics
    };

    let (unmutate_enum, unmutate_struct) = {
        let mut items = filtered_generic_items
            .iter()
            .map(|inner_item| EnumItem {
                data: match &inner_item.data {
                    Some(EnumItemData::Struct(kind, fields)) => Some(EnumItemData::Struct(
                        *kind,
                        fields
                            .into_iter()
                            .map(|field| StructField {
                                ty: ts!(":: std :: option :: Option :: <" field.ty ">"),
                                ..field.clone()
                            })
                            .collect(),
                    )),
                    data @ Some(EnumItemData::Discriminant(_)) | data @ None => data.clone(),
                },
                ..inner_item.clone()
            })
            .collect::<Vec<_>>();

        items.push(EnumItem {
            attributes: Vec::new(),
            ident: ident!("___Replace"),
            data: Some(EnumItemData::Struct(
                StructKind::Tuple,
                vec![
                    StructField {
                        identifier: StructFieldIdentifier::Position(0),
                        ty: ts!(ident!("___Value")),
                        ..StructField::default()
                    },
                    StructField {
                        identifier: StructFieldIdentifier::Position(1),
                        ty: ts!(ident!("___Cache")),
                        ..StructField::default()
                    },
                ],
            )),
        });

        let mut generics = basic_generics.clone();
        generics.type_params.push(TypeParam {
            type_ident: ts!(ident!("___Value")),
            ..TypeParam::default()
        });
        generics.type_params.push(TypeParam {
            type_ident: ts!(ident!("___Cache")),
            ..TypeParam::default()
        });

        let unmutate_enum = Enum {
            visibility: parsed_enum.visibility.clone(),
            ident: ident!(parsed_enum.ident "InnerUnmutateToken"),
            generics: generics.clone(),
            where_clause: None,
            items,
        };

        let unmutate_struct = Struct {
            visibility: parsed_enum.visibility.clone(),
            ident: ident!(parsed_enum.ident "UnmutateToken"),
            generics,
            kind: Some(StructKind::Struct),
            where_clause: None,
            struct_fields: vec![
                StructField {
                    visibility: ts!("pub"),
                    identifier: StructFieldIdentifier::Named(ident!("inner")),
                    ty: ts!(unmutate_enum.ident unmutate_enum.generics),
                    ..StructField::default()
                },
                StructField {
                    visibility: ts!("pub"),
                    identifier: StructFieldIdentifier::Named(ident!("cplx")),
                    ty: ts!("f64"),
                    ..StructField::default()
                },
            ],
        };

        (unmutate_enum, unmutate_struct)
    };

    let unmutate_struct_generics_for_assoc_type = {
        let mut generics = unmutate_struct.generics.clone();
        let _ = generics.type_params.pop().unwrap();
        let _ = generics.type_params.pop().unwrap();
        generics = generics.mutating_type_params(|tp| {
            tp.type_ident =
                ts!("<" tp.type_ident "as fuzzcheck_mutators :: fuzzcheck_traits :: Mutator > :: UnmutateToken")
        });
        generics.type_params.push(TypeParam {
            type_ident: ts!("Self :: Value"),
            ..TypeParam::default()
        });
        generics.type_params.push(TypeParam {
            type_ident: ts!("Self :: Cache"),
            ..TypeParam::default()
        });
        generics
    };

    tb.extend(&unmutate_enum);
    tb.extend(&unmutate_struct);

    {
        // impl Mutator
        let parsed_enum_generics_without_bounds = parsed_enum.generics.removing_bounds_and_eq_type();
        let mutator_struct_generics_without_bounds = mutator_struct.generics.removing_bounds_and_eq_type();

        let cplx_choose_item = ((parsed_enum.items.len() as f64).log2() * 100.0).round() / 100.0;

        extend_ts!(tb,
            "# [ allow ( non_shorthand_field_patterns ) ]
            impl" mutator_struct.generics "fuzzcheck_mutators :: fuzzcheck_traits :: Mutator for"
                mutator_struct.ident mutator_struct_generics_without_bounds mutator_struct.where_clause
            "{
                type Value = " parsed_enum.ident parsed_enum_generics_without_bounds ";
                type Cache = " 
                    cache_struct.ident cache_struct.generics.mutating_type_params(|tp|
                        tp.type_ident = ts!("<" tp.type_ident "as fuzzcheck_mutators :: fuzzcheck_traits :: Mutator > :: Cache")
                    )
                    ";
                type ArbitraryStep = "
                ar_step_struct.ident ar_step_struct.generics.mutating_type_params(|tp|
                    tp.type_ident = ts!("<" tp.type_ident "as fuzzcheck_mutators :: fuzzcheck_traits :: Mutator > :: ArbitraryStep")
                )
                ";
                type MutationStep = " step_struct.ident step_struct_generics_for_assoc_type ";
                type UnmutateToken = " unmutate_struct.ident unmutate_struct_generics_for_assoc_type ";

                fn max_complexity ( & self ) -> f64 {"
                    cplx_choose_item
                    if !filtered_generic_items.is_empty() {
                        ts!(
                            "+ [ "
                                join_ts!(&filtered_generic_items, item,
                                    join_ts!(item.get_fields_unchecked(), field,
                                        "self ." field.safe_ident() ". max_complexity ( ) "
                                    , separator: "+")
                                , separator: ",")
                            "] . iter ( ) . max_by ( | x , y | x . partial_cmp ( y ) . unwrap_or ( core :: cmp :: Ordering :: Equal ) ) . unwrap ( )"
                        )
                    } else {
                        ts!()
                    }
                "}
                fn min_complexity ( & self ) -> f64 {"
                    cplx_choose_item
                    if !filtered_generic_items.is_empty() {
                        ts!(
                            "+ ["
                                join_ts!(&filtered_generic_items, item,
                                    join_ts!(item.get_fields_unchecked(), field,
                                        "self ." field.safe_ident() ". min_complexity ( ) "
                                    , separator: "+")
                                , separator: ",")
                            "] . iter ( ) . min_by ( | x , y | x . partial_cmp ( y ) . unwrap_or ( core :: cmp :: Ordering :: Equal ) ) . unwrap ( )"
                        )
                    } else {
                        ts!()
                    }
                "}
                fn complexity ( & self , value : & Self :: Value , cache : & Self :: Cache ) -> f64 { 
                    cache . cplx 
                }

                fn cache_from_value ( & self , value : & Self :: Value ) -> Self :: Cache {
                    match value {"
                        join_ts!(filtered_enum_items.iter().zip(filtered_generic_items.iter()), (item, generic_item),
                            item.pattern_match(&parsed_enum.ident, None) "=> {"
                                {
                                    let generic_fields = generic_item.get_fields_unchecked();
                                    let item_fields = item.get_fields_unchecked();
                                    ts!(
                                        "let mut cplx = " cplx_choose_item ";"
                                        join_ts!(item_fields.iter().zip(generic_fields.iter()), (f, generic_f),
                                            "let" ident!("inner_" f.safe_ident()) "= self ." generic_f.safe_ident() ". cache_from_value ( &" f.safe_ident() ") ;"
                                            "cplx += self ." generic_f.safe_ident() ". complexity ( &" f.safe_ident() ", &" ident!("inner_" f.safe_ident()) " ) ;"
                                        )
                                        "let inner = Some (" cache_enum.ident " :: " item.ident
                                        "{"
                                            join_ts!(item_fields.iter().zip(generic_fields.iter()), (item_f, generic_f),
                                                generic_f.safe_ident() ":" ident!("inner_" item_f.safe_ident())
                                            , separator: ",")
                                        "} ) ;
                                        " cache_struct.ident " {
                                            inner ,
                                            cplx ,
                                        }"
                                    )
                                }
                            "}"
                        )
                        "_ => {"
                            cache_struct.ident " {
                                inner : None ,
                                cplx : " cplx_choose_item "
                            }
                        }"
                    "}
                }

                fn initial_step_from_value ( & self , value : & Self :: Value ) -> Self :: MutationStep {
                    match value {"
                    join_ts!(filtered_enum_items.iter().zip(filtered_generic_items.iter()).zip(step_enum_inners.iter()), ((item, generic_item), step_enum_inner),
                        item.pattern_match(&parsed_enum.ident, None) "=> {"
                            {
                                let generic_fields = generic_item.get_fields_unchecked();
                                let value_fields = item.get_fields_unchecked();
                                ts!(
                                    "let inner = Some (" step_enum.ident " :: " item.ident
                                    "( vec ! [ "
                                        join_ts!(value_fields.iter().zip(generic_fields.iter()), (value_f, generic_f),
                                                step_enum_inner.ident "::" generic_f.safe_ident() "( self ." generic_f.safe_ident() ". initial_step_from_value ( &" value_f.safe_ident() ") )"
                                        , separator: ",")
                                    "] ) ) ;
                                    let step = 0 ;
                                    " step_struct.ident " {
                                        inner ,
                                        step ,
                                        arbitrary_step : None
                                    }"
                                )
                            }
                        "}"
                    )
                        "_ => {"
                            step_struct.ident " {
                                inner : None ,
                                step : 0 ,
                                arbitrary_step : Some ( < _ > :: default ( ) )
                            }
                        }"
                    "}
                }

                fn ordered_arbitrary ( & mut self , step : & mut Self :: ArbitraryStep , max_cplx : f64 ) -> Option < ( Self :: Value , Self :: Cache ) > {
                    let orig_step = step . step ;
                    step . step += 1 ;
                    if orig_step < " parsed_enum.items.len() - filtered_enum_items.len() "{"
                        "match orig_step {"
                            join_ts!(empty_enum_items.iter().enumerate(), (i, item),
                                i "=> {
                                        let value =" parsed_enum.ident "::" item.ident opt_ts!(item.get_struct_data().map(|d| d.0), kind, kind.open() kind.close()) ";
                                    let cache =" cache_struct.ident " {
                                        inner : None ,
                                        cplx : " cplx_choose_item "
                                    } ;
                                    return Some ( ( value , cache ) )
                                }"
                            )
                            "_ => unreachable ! ( )"
                        "}
                    } else {"
                        if filtered_enum_items.is_empty() {
                        ts!("return None")
                        } else { ts!(
                        "
                        if step . inner . is_empty ( ) { return None }
                        let mut inner_step_to_remove : Option < usize > = None ;
                        let mut recurse = false ;
                        let inner_len = step . inner . len ( ) ;
                        let orig_step = orig_step % inner_len ;
                        match & mut step . inner [ orig_step ] {"
                        join_ts!(filtered_enum_items.iter().zip(ar_step_enum.items), (item, ar_step_item),
                            ar_step_item.pattern_match(&ar_step_enum.ident, None)
                            "=> {"{
                                let fields = item.get_fields_unchecked();
                                let kind = item.get_struct_data().unwrap().0;
                                if fields.len() == 1 {
                                    let inner_field_ident = ar_step_item.get_fields_unchecked()[0].safe_ident();
                                    let field = &fields[0];
                                    ts!(
                                        "if let Some ( ( inner_value , inner_cache ) ) = self ." inner_field_ident " . ordered_arbitrary (" inner_field_ident ", max_cplx ) {"
                                            "let cplx = " cplx_choose_item " + self ." inner_field_ident ". complexity ( & inner_value , & inner_cache ) ;
                                            let value = " parsed_enum.ident "::" item.ident
                                                kind.open()
                                                    field.expr_field(ts!("inner_value"))
                                                kind.close()
                                                ";
                                            let cache = " cache_struct.ident " {
                                                inner : Some (" cache_enum.ident " :: " ar_step_item.ident "{"
                                                    inner_field_ident ": inner_cache"
                                                "} ) ,
                                                cplx
                                            } ;
                                            return Some ( ( value , cache ) )"
                                        "} else {
                                            step . step -= 1 ;
                                            inner_step_to_remove = Some ( orig_step ) ;
                                            recurse = true ;
                                        }"
                                    )
                                } else {
                                    let inner_fields_identifiers = ar_step_item.get_fields_unchecked().iter().map(|f| f.safe_ident()).take(fields.len()).collect::<Vec<_>>();
                                    ts!(
                                        "let orig_step = * step ;
                                        * step += 1 ;
                                        match orig_step %" fields.len() "{"
                                            join_ts!(0 .. fields.len(), i,
                                                i "=> {"
                                                    join_ts!(inner_fields_identifiers.iter().enumerate(), (j, ident),
                                                        "let (" ident!(ident "_value") ", " ident!(ident "_cache")  ") ="
                                                        if j == i {
                                                            ts!("self ." ident ". ordered_arbitrary ( " ident ", max_cplx ) . unwrap_or_else ( | | {
                                                                self . " ident ". random_arbitrary ( max_cplx )
                                                            } )")
                                                        } else {
                                                            ts!("self . " ident ". random_arbitrary ( max_cplx )")
                                                        }
                                                        ";"
                                                    )
                                                    "let cplx =" cplx_choose_item join_ts!(&inner_fields_identifiers, ident,
                                                        "+ self ." ident ". complexity ( &" ident!(ident "_value") ", & " ident!(ident "_cache") ")"
                                                    ) ";
                                                    let value = " parsed_enum.ident "::" item.ident
                                                    kind.open()
                                                        join_ts!(fields.iter().zip(inner_fields_identifiers.iter()), (field, inner_ident),
                                                            field.expr_field(ts!(ident!(inner_ident "_value")))
                                                        , separator: ",")
                                                    kind.close()
                                                    ";
                                                    let cache = " cache_struct.ident " {
                                                        inner : Some (" cache_enum.ident " :: " ar_step_item.ident "{"
                                                            join_ts!(&inner_fields_identifiers, ident,
                                                                ident ":" ident!(ident "_cache")
                                                            , separator: ",")
                                                        "} )
                                                        , cplx
                                                    } ;
                                                    return Some ( ( value , cache ) )
                                                    "
                                                "}"
                                            )
                                            "_ => unreachable ! ( )"
                                        "}"
                                    )
                                }
                            }
                            "}"

                        )
                        "}
                        # [ allow ( unreachable_code ) ]
                        {
                            if let Some ( idx ) = inner_step_to_remove {
                                step . inner . remove ( idx ) ;
                            }
                            if recurse {
                                self . ordered_arbitrary ( step , max_cplx )
                            } else {
                                None
                            }
                        }"
                        )}
                    "}
                }

                fn random_arbitrary ( & mut self , max_cplx : f64 ) -> ( Self :: Value , Self :: Cache ) {
                    let step = self . rng . usize ( .. ) ;
                    let max_cplx = max_cplx - " cplx_choose_item ";
                    match step % " parsed_enum.items.len() " {"
                    join_ts!(parsed_enum.items.iter().zip(generic_items.iter()).enumerate(), (i, (item, generic_item)),
                        i "=> {"
                            match &item.data {
                                Some(EnumItemData::Struct(kind, fields)) if !fields.is_empty() => {
                                    let generic_item = generic_item.as_ref().unwrap();
                                    let inner_fields_identifiers = generic_item.get_fields_unchecked().iter().map(|f| f.safe_ident()).take(fields.len()).collect::<Vec<_>>();
                                    ts!(
                                        join_ts!(&inner_fields_identifiers, ident,
                                            "let (" ident!(ident "_value") "," ident!(ident "_cache")" ) = self ." ident ". random_arbitrary ( max_cplx ) ;"
                                        )
                                        "let cplx = "
                                        cplx_choose_item join_ts!(&inner_fields_identifiers, ident,
                                            "+ self ." ident ". complexity ( &" ident!(ident "_value") ", &" ident!(ident "_cache") ")"
                                        ) ";
                                        let value = " parsed_enum.ident "::" item.ident
                                            kind.open()
                                            join_ts!(fields.iter().zip(inner_fields_identifiers.iter()), (field, inner_ident),
                                                field.expr_field(ts!(ident!(inner_ident "_value")))
                                            , separator: ",")
                                            kind.close()
                                            ";
                                        let cache = " cache_struct.ident "{
                                            inner : Some (" cache_enum.ident "::" generic_item.ident "{"
                                                join_ts!(&inner_fields_identifiers, ident,
                                                    ident ":" ident!(ident "_cache")
                                                , separator: ",")
                                            "} ) ,
                                            cplx"
                                        "} ;
                                        ( value , cache )
                                        "
                                    )
                                }
                                _ => {
                                    let kind = item.get_struct_data().map(|data| data.0);
                                    ts!(
                                        "let value = " parsed_enum.ident "::" item.ident opt_ts!(kind, k, k.open() k.close()) ";
                                        let cache = " cache_struct.ident "{
                                            inner : None ,
                                            cplx : " cplx_choose_item
                                        "} ;
                                        ( value , cache )"
                                    )
                                }
                            }
                        "}"
                    )
                        "_ => unreachable ! ( )"
                    "}
                }

                fn ordered_mutate ( & mut self , mut value : & mut Self :: Value , mut cache : & mut Self :: Cache , step : & mut Self :: MutationStep , max_cplx : f64 ) -> Option < Self :: UnmutateToken > {
                    if let Some ( ar_step ) = & mut step . arbitrary_step {
                        if let Some ( ( v , c ) ) = self . ordered_arbitrary ( ar_step , max_cplx ) {
                            let old_value = std :: mem :: replace ( value , v ) ;
                            let old_cache = std :: mem :: replace ( cache , c ) ;
                            return Some (" unmutate_struct.ident " { inner : " unmutate_enum.ident " :: ___Replace ( old_value , old_cache ) , cplx : f64 :: default ( ) } )
                        } else {
                            return None
                        }
                    }
                    let mut recurse = false ;
                    match ( & mut value , & mut cache , & mut step . inner ) {"
                    join_ts!(filtered_enum_items.iter().zip(filtered_generic_items.iter()).zip(step_enum_inners.iter()), ((item, generic_item), inner_step_enum),
                        {
                            let fields = item.get_fields_unchecked();
                            let generic_fields = generic_item.get_fields_unchecked();
                            ts!(
                                "("
                                    item.pattern_match(&parsed_enum.ident, Some(ident!("_value")))
                                ","
                                    cache_struct.ident "{ inner : Some (" generic_item.pattern_match(&cache_enum.ident, Some(ident!("_cache"))) ") , cplx }"
                                ","
                                    "Some (" step_enum.ident "::" generic_item.ident "( steps ) )"
                                ") => {"
                                ts!(
                                    "
                                    if steps . is_empty ( ) {
                                        step . arbitrary_step = Some ("
                                            ar_step_struct.ident "{
                                                inner : vec ! ["
                                                    join_ts!(&sorted_ar_step_enum_items, ar_item,
                                                        if ar_item.ident.to_string() == generic_item.ident.to_string() {
                                                            ts!()
                                                        } else {
                                                            ts!(
                                                                ar_step_enum.ident "::" ar_item.ident "{"
                                                                join_ts!(ar_item.get_fields_unchecked(), field,
                                                                    field.safe_ident() ": < _ > :: default ( )"
                                                                , separator: ",")
                                                            "} ,"
                                                            )
                                                        }
                                                    )
                                                "] ,
                                                step : < _ > :: default ( )
                                            }"
                                        ") ;
                                        recurse = true ;
                                    } else {
                                        let orig_step = step . step % steps . len ( ) ;
                                        let mut step_to_remove : Option < usize > = None ;
                                        step . step += 1 ;
                                        match & mut steps [ orig_step ] {"
                                        join_ts!(inner_step_enum.items.iter().enumerate().zip(fields.iter()).zip(generic_fields.iter()), (((i, inner_step_item), field), generic_field),
                                            inner_step_enum.ident "::" inner_step_item.ident "( inner_step ) => {"{
                                                let generic_ident = generic_field.safe_ident();
                                                let value_ident = ident!(field.safe_ident() "_value");
                                                let cache_ident = ident!(generic_field.safe_ident() "_cache");
                                                ts!("
                                                    let old_field_cplx = self ." generic_ident ". complexity ( & " value_ident ", & " cache_ident ") ;
                                                    let max_cplx = max_cplx - " cplx_choose_item "- old_field_cplx ;
                                                    if let Some ( field_token ) = self ." generic_ident ". ordered_mutate ( " value_ident "," cache_ident ", inner_step , max_cplx ) {
                                                        let new_field_cplx = self ." generic_ident ". complexity ( & " value_ident ", & " cache_ident ") ;
                                                        let old_cplx = * cplx ;
                                                        * cplx += new_field_cplx - old_field_cplx ;
                                                        return Some (" unmutate_struct.ident " { inner : " unmutate_enum.ident "::" generic_item.ident "{"
                                                            join_ts!(0 .. fields.len(), j,
                                                                if i == j {
                                                                    ts!(generic_ident ": Some ( field_token )")
                                                                } else {
                                                                    let generic_field = &generic_fields[j];
                                                                    let generic_field_ident = generic_field.safe_ident();
                                                                    ts!(generic_field_ident ": None")
                                                                }
                                                            , separator: ",")
                                                        "} , cplx : old_cplx } )"
                                                    "} else {
                                                        step_to_remove = Some ( orig_step ) ;
                                                    }"
                                                )
                                            }"}"
                                        )"}
                                        if let Some ( idx ) = step_to_remove {
                                            steps . remove ( idx ) ;
                                            recurse = true ;
                                        }
                                    }"
                                )
                                "}"
                            )
                        })
                        "( value , cache , _ ) => unreachable ! ( )
                    }
                    # [ allow ( unreachable_code ) ] 
                    {
                        if recurse {
                            self . ordered_mutate ( value , cache , step , max_cplx )
                        } else {
                            None
                        }
                    }
                }

                fn random_mutate ( & mut self , mut value : & mut Self :: Value , mut cache : & mut Self :: Cache , max_cplx : f64 ) -> Self :: UnmutateToken {
                    let use_arbitrary = self . rng . f64 ( ) <" cplx_choose_item " / self . complexity ( & value , & cache ) ;
                    if use_arbitrary {
                        let ( v , c ) = self . random_arbitrary ( max_cplx ) ;
                        let old_value = std :: mem :: replace ( value , v ) ;
                        let old_cache = std :: mem :: replace ( cache , c ) ;
                        return" unmutate_struct.ident " { inner : " unmutate_enum.ident " :: ___Replace ( old_value , old_cache ) , cplx : f64 :: default ( ) }
                    } else {
                        match ( & mut value , & mut cache ) {"
                        join_ts!(filtered_enum_items.iter().zip(filtered_generic_items.iter()), (item, generic_item), {
                            let generic_item = generic_item;
                            let generic_fields = generic_item.get_fields_unchecked();
                            let fields = item.get_fields_unchecked();
                            ts!(
                                "(" item.pattern_match(&parsed_enum.ident, Some(ident!("_value")))
                                "," cache_struct.ident "{ inner : Some ("
                                    generic_item.pattern_match(&cache_enum.ident, Some(ident!("_cache")))
                                    ") , cplx }"
                                ") => {
                                    match self . rng . usize ( .. ) % " fields.len() "{"
                                    join_ts!(fields.iter().zip(generic_fields.iter()).enumerate(), (i, (field, generic_field)),
                                        i "=> {"{
                                        let generic_ident = generic_field.safe_ident();
                                        let value_ident = ident!(field.safe_ident() "_value");
                                        let cache_ident = ident!(generic_field.safe_ident() "_cache");
                                        ts!("
                                            let old_field_cplx = self ." generic_ident ". complexity ( & " value_ident ", & " cache_ident ") ;
                                            let max_cplx = max_cplx - " cplx_choose_item "- old_field_cplx ;
                                            let field_token = self ." generic_ident ". random_mutate ( " value_ident "," cache_ident ", max_cplx ) ;
                                            let new_field_cplx = self ." generic_ident ". complexity ( & " value_ident ", & " cache_ident ") ;
                                            let old_cplx = * cplx ;
                                            * cplx += new_field_cplx - old_field_cplx ;
                                            return" unmutate_struct.ident " { inner : " unmutate_enum.ident "::" generic_item.ident "{"
                                                join_ts!(0 .. fields.len(), j,
                                                    if i == j {
                                                        ts!(generic_ident ": Some ( field_token )")
                                                    } else {
                                                        let generic_field = &generic_fields[j];
                                                        let generic_field_ident = generic_field.safe_ident();
                                                        ts!(generic_field_ident ": None")
                                                    }
                                                , separator: ",")
                                            "} , cplx : old_cplx } "
                                        )
                                        }"}"
                                    )
                                        "_ => unreachable ! ( )"
                                    "}
                                }"
                            )
                        })
                            "( value , cache ) => {
                                let ( v , c ) = self . random_arbitrary ( max_cplx ) ;
                                let old_value = std :: mem :: replace ( * value , v ) ;
                                let old_cache = std :: mem :: replace ( * cache , c ) ;
                                return" unmutate_struct.ident " { inner : " unmutate_enum.ident " :: ___Replace ( old_value , old_cache ) , cplx : f64 :: default ( ) }
                            }"
                        "}
                    }
                }

                fn unmutate ( & self , value : & mut Self :: Value , cache : & mut Self :: Cache , t : Self :: UnmutateToken ) {
                    match ( t , value , cache ) {"
                        join_ts!(filtered_enum_items.iter().zip(filtered_generic_items.iter()), (item, generic_item),
                            if let Some((_, fields)) = item.get_struct_data(){
                                let generic_fields = generic_item.get_fields_unchecked();
                                ts!("("
                                    unmutate_struct.ident "{
                                        inner :" generic_item.pattern_match(&unmutate_enum.ident, Some(ident!("_token")))
                                        ", cplx : cplx_token
                                    }
                                    ,"
                                    item.pattern_match(&parsed_enum.ident, Some(ident!("_value")))
                                    ","
                                    cache_struct.ident "{
                                        inner : Some ("
                                            generic_item.pattern_match(&cache_enum.ident, Some(ident!("_cache")))
                                        ") , cplx"
                                    "}
                                ) => {"
                                    join_ts!(fields.iter().zip(generic_fields.iter()), (f, generic_f),
                                        "if let Some ( t ) =" ident!(generic_f.safe_ident() "_token") "{
                                            self . " generic_f.safe_ident() " . unmutate ( 
                                                " ident!(f.safe_ident() "_value") ",
                                                " ident!(generic_f.safe_ident() "_cache") ",
                                                t
                                            )
                                        }"
                                    )
                                    "* cplx = cplx_token"
                                    "}"
                                )
                            } else {
                                ts!()
                            }
                        )
                        "(" unmutate_struct.ident "{ inner :" unmutate_enum.ident ":: ___Replace ( v , c ) , cplx : _ } , value , cache ) => {
                            let _ = std :: mem :: replace ( value , v ) ;
                            let _ = std :: mem :: replace ( cache , c ) ;
                        }
                        _ => unreachable ! ( )"
                    "}
                }
            }"
        );
        if derive_default {
            // default impl
            let where_clause = {
                let mut where_clause = mutator_struct.where_clause.clone().unwrap_or_default();

                for field in &submutator_fields {
                    where_clause.items.push(WhereClauseItem {
                        for_lifetimes: None,
                        lhs: ts!(field.ty),
                        rhs: ts!(":: core :: default :: Default"),
                    });
                }
                where_clause
            };

            extend_ts!(tb,
            "impl" mutator_struct.generics ":: core :: default :: Default for" mutator_struct.ident
                mutator_struct.generics.removing_bounds_and_eq_type() where_clause
            "{
                fn default ( ) -> Self {
                    Self {"
                        join_ts!(&mutator_struct.struct_fields, field,
                            field.safe_ident() ":" "<" field.ty "as :: core :: default :: Default > :: default ( )"
                        , separator: ",")
                    "}
                }
            }"
            );

            let generics_without_bounds = parsed_enum.generics.removing_bounds_and_eq_type();

            let where_clause = {
                let mut where_clause = parsed_enum.where_clause.clone().unwrap_or_default();
                for field in flattened_fields.iter() {
                    where_clause.items.extend(vec![
                        WhereClauseItem {
                            for_lifetimes: None,
                            lhs: field.ty.clone(),
                            rhs: ts!(":: core :: clone :: Clone + fuzzcheck_mutators :: DefaultMutator"),
                        },
                        WhereClauseItem {
                            for_lifetimes: None,
                            lhs: ts!("<" field.ty " as fuzzcheck_mutators :: DefaultMutator > :: Mutator"),
                            rhs: ts!(":: core :: default :: Default"),
                        },
                        WhereClauseItem {
                            for_lifetimes: None,
                            lhs: ts!(parsed_enum.ident generics_without_bounds),
                            rhs: ts!(":: core :: clone :: Clone"),
                        },
                    ]);
                }
                where_clause
            };

            let generics_mutator = {
                let mut type_params = generics_without_bounds.type_params.clone();
                for field in &flattened_fields {
                    type_params.push(TypeParam {
                        type_ident: ts!("<" field.ty "as fuzzcheck_mutators :: DefaultMutator > :: Mutator"),
                        ..<_>::default()
                    });
                }
                Generics {
                    lifetime_params: generics_without_bounds.lifetime_params.clone(),
                    type_params,
                }
            };

            extend_ts!(tb,
            "impl" parsed_enum.generics "fuzzcheck_mutators :: DefaultMutator for" parsed_enum.ident
                generics_without_bounds where_clause
            "{
                type Mutator = " mutator_struct.ident generics_mutator ";
    
                fn default_mutator ( ) -> Self :: Mutator {
                    Self :: Mutator :: default ( )
                }
            }"
            )
        }
    }
}

fn derive_unit_mutator(parsed_struct: Struct, derive_default: bool, tb: &mut TokenBuilder) {
    if derive_default {
        let generics_without_bounds = parsed_struct.generics.clone().removing_bounds_and_eq_type();
        let mutator_ident = ident!(parsed_struct.ident "Mutator");

        extend_ts!(tb,
            "type" mutator_ident generics_without_bounds
                "= fuzzcheck_mutators :: unit :: UnitMutator < " parsed_struct.ident generics_without_bounds "> ;"

            "impl" parsed_struct.generics "fuzzcheck_mutators :: DefaultMutator for"
                parsed_struct.ident generics_without_bounds parsed_struct.where_clause
            "{
                type Mutator = " mutator_ident generics_without_bounds ";
                fn default_mutator ( ) -> Self :: Mutator {
                    Self :: Mutator :: default ( )
                }
            }"
        );
    }
}
