#![allow(dead_code)]
#![feature(proc_macro_quote)]

mod token_builder;
mod parser;

//mod struct_derive;

use crate::token_builder::*;
use crate::parser::*;

use proc_macro::{Ident, Literal, Span, TokenStream};

macro_rules! joined_token_streams {
    ($iter:expr, $sep:expr) => {
        {
            let mut iter = $iter.into_iter();
            #[allow(unused_mut)]
            let mut tb = TokenBuilder::new();
            let mut add_sep = false;
            while let Some(x) = iter.next() {
                if add_sep {
                    $sep.add_to(&mut tb);
                }
                x.add_to(&mut tb);
                add_sep = true;
            }
            tb.end()
        }
    };
}

macro_rules! extend_token_builder {
    ($tb:expr, $($part:expr) *) => { 
        {
            $(
                $part.add_to($tb);
            )*
        }
    };
}

macro_rules! token_stream {
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

#[proc_macro_derive(HasDefaultMutator)]
pub fn derive_mutator(input: TokenStream) -> TokenStream {
    let mut parser = TokenParser::new(input);
    let mut tb = TokenBuilder::new();

    if let Some(s) = parser.eat_struct() {
        derive_struct_mutator(s, &mut tb);
    } else if let Some(_) = parser.eat_enumeration() {
        // derive_enum_mutator(e, &mut tb)
    } else {
        extend_token_builder!(&mut tb,
            "compile_error ! ("
            Literal::string("fuzzcheck_mutators_derive could not parse the structure")
            ") ;"
        )
    }

    // tb.eprint();
    tb.eprint();
    tb.end()
}

fn derive_struct_mutator(parsed_struct: Struct, tb: &mut TokenBuilder) {
    if !parsed_struct.struct_fields.is_empty() {
        derive_struct_mutator_with_fields(&parsed_struct, tb)
    } else {
        //derive_unit_mutator(parsed_struct, tb);
    }
}

fn derive_struct_mutator_with_fields(parsed_struct: &Struct, tb: &mut TokenBuilder) {

    let field_idents = parsed_struct
        .struct_fields
        .iter()
        .enumerate()
        .map(|(i, f)| {
            let x = if let Some(ident) = &f.identifier {
                ident.to_string()
            } else {
                format!("{}", i)
            };
            Ident::new(&x, Span::call_site())
        })
        .collect::<Vec<_>>();

    let mutator_field_idents = field_idents
        .iter()
        .map(|x| {
            let x = format!("_{}", x);
            Ident::new(&x, Span::call_site())
        })
        .collect::<Vec<_>>();

    let generic_types_for_field = mutator_field_idents
        .iter()
        .map(|name| Ident::new(&format!("{}Type", name), Span::call_site()));

    let generics_without_bounds = parsed_struct.generics.removing_bounds_and_eq_type();

    let basic_generics = Generics {
        lifetime_params: Vec::new(),
        type_params: generic_types_for_field
            .clone()
            .map(|ident| TypeParam {
                attributes: Vec::new(),
                type_ident: token_stream!(ident),
                bounds: None,
                equal_ty: None,
            })
            .collect(),
    };

    let basic_fields = mutator_field_idents
        .iter()
        .zip(generic_types_for_field.clone())
        .map(|(identifier, ty)| StructField {
            attributes: Vec::new(),
            visibility: Some(token_stream!("pub")),
            identifier: Some(identifier.clone()),
            ty: token_stream!(ty),
        })
        .collect::<Vec<_>>();

    let value_struct_ident_with_generic_params = token_stream!(
        parsed_struct.ident generics_without_bounds
    );

    let mutator_struct = {
        let mut generics = parsed_struct.generics.clone();

        let mut where_clause_items = parsed_struct.where_clause.clone().map(|wc| wc.items).unwrap_or(vec![]);

        for (field, generic_ty_for_field) in parsed_struct.struct_fields.iter().zip(generic_types_for_field.clone()) {
            generics.type_params.push(TypeParam {
                type_ident: token_stream!(generic_ty_for_field),
                ..TypeParam::default()
            });
            where_clause_items.push(WhereClauseItem {
                for_lifetimes: None,
                lhs: field.ty.clone(),
                rhs: token_stream!(":: core :: clone :: Clone"),
            });
            where_clause_items.push(WhereClauseItem {
                for_lifetimes: None,
                lhs: token_stream!(generic_ty_for_field),
                rhs: token_stream!("fuzzcheck_mutators :: fuzzcheck_traits :: Mutator < Value = " field.ty ">"),
            });
        }

        where_clause_items.push(WhereClauseItem {
            for_lifetimes: None,
            lhs: value_struct_ident_with_generic_params.clone(),
            rhs: token_stream!(":: core :: clone :: Clone"),
        });

        let mut mutator_struct_fields = basic_fields.clone();
        mutator_struct_fields.push(StructField {
            attributes: Vec::new(),
            visibility: Some(token_stream!("pub")),
            identifier: Some(Ident::new("rng", Span::call_site())),
            ty: token_stream!("fuzzcheck_mutators :: fastrand :: Rng"),
        });

        Struct {
            visibility: parsed_struct.visibility.clone(),
            ident: Ident::new(&format!("{}Mutator", parsed_struct.ident), Span::call_site()),
            generics,
            kind: StructKind::Struct,
            where_clause: Some(WhereClause { items: where_clause_items }),
            struct_fields: mutator_struct_fields,
        }
    };

    tb.extend(&mutator_struct);

    let mutator_cache_struct = {
        let mut cache_fields = basic_fields.clone();
        cache_fields.push(StructField {
            attributes: Vec::new(),
            visibility: None,
            identifier: Some(Ident::new("cplx", Span::call_site())),
            ty: token_stream!("f64"),
        });
        Struct {
            visibility: parsed_struct.visibility.clone(),
            ident: Ident::new(&format!("{}MutatorCache", parsed_struct.ident), Span::call_site()),
            generics: basic_generics.clone(),
            kind: StructKind::Struct,
            where_clause: None,
            struct_fields: cache_fields.clone(),
        }
    };

    extend_token_builder!(tb, 
        "# [ derive ( :: core :: clone :: Clone ) ]" 
        mutator_cache_struct
    );

    let mutator_step_struct = {
        let mut step_fields = basic_fields.clone();
        step_fields.push(StructField {
            attributes: Vec::new(),
            visibility: None,
            identifier: Some(Ident::new("step", Span::call_site())),
            ty: token_stream!("u64"),
        });
        Struct {
            visibility: parsed_struct.visibility.clone(),
            ident: Ident::new(&format!("{}MutationStep", parsed_struct.ident), Span::call_site()),
            generics: basic_generics.clone(),
            kind: StructKind::Struct,
            where_clause: None,
            struct_fields: step_fields,
        }
    };

    extend_token_builder!(tb, 
        "# [ derive ( :: core :: clone :: Clone ) ]" 
        mutator_step_struct
    );

    let unmutate_token_struct = {
        let mut step_fields = basic_fields
            .iter()
            .map(|field| {
                StructField {
                    ty: token_stream!(":: std :: option :: Option <" field.ty ">"),
                    ..field.clone()
                }
            })
            .collect::<Vec<StructField>>();

        step_fields.push(StructField {
            attributes: Vec::new(),
            visibility: None,
            identifier: Some(Ident::new("cplx", Span::call_site())),
            ty: token_stream!("f64"),
        });
        Struct {
            visibility: parsed_struct.visibility.clone(),
            ident: Ident::new(&format!("{}UnmutateToken", parsed_struct.ident), Span::call_site()),
            generics: basic_generics,
            kind: StructKind::Struct,
            where_clause: None,
            struct_fields: step_fields,
        }
    };

    extend_token_builder!(tb, 
        "# [ derive ( :: core :: clone :: Clone ) ]" 
        unmutate_token_struct
    );

    // default impl for unmutate token
    extend_token_builder!(tb,
        "impl" unmutate_token_struct.generics ":: core :: default :: Default for" unmutate_token_struct.ident unmutate_token_struct.generics "{
            fn default ( ) -> Self {
                Self {"
                mutator_field_idents.iter().map(|m|
                    token_stream!(m ": None ,")
                ).collect::<Vec<_>>()
                    "cplx : f64 :: default ( )
                }
            }
        }"
    );

    let fields_iter = field_idents.iter().zip(mutator_field_idents.iter());

    // TODO: arbitrary/random/ordered

    // implementation of Mutator trait
    extend_token_builder!(tb,
        "impl" mutator_struct.generics "fuzzcheck_mutators :: fuzzcheck_traits :: Mutator for"
            mutator_struct.ident mutator_struct.generics.removing_bounds_and_eq_type() mutator_struct.where_clause
        "{
            type Value = " value_struct_ident_with_generic_params ";
            type Cache = " mutator_cache_struct.ident
                mutator_cache_struct.generics.mutating_type_params(|tp|
                    tp.type_ident = token_stream!("<" tp.type_ident "as fuzzcheck_mutators :: fuzzcheck_traits :: Mutator > :: Cache")
                )
                ";
            
            type MutationStep = "
                mutator_step_struct.ident
                mutator_step_struct.generics.mutating_type_params(|tp|
                    tp.type_ident = token_stream!("<" tp.type_ident "as fuzzcheck_mutators :: fuzzcheck_traits :: Mutator > :: MutationStep")
                )
                ";

            type UnmutateToken = "
                unmutate_token_struct.ident
                unmutate_token_struct.generics.mutating_type_params(|tp|
                    tp.type_ident = token_stream!("<" tp.type_ident "as fuzzcheck_mutators :: fuzzcheck_traits :: Mutator > :: UnmutateToken")
                )
                ";

            fn max_complexity ( & self ) -> f64 {"
                joined_token_streams!(mutator_field_idents.iter().map(|field_name| {
                    token_stream!("self . " field_name ". max_complexity ( )")
                }), "+")
            "}

            fn min_complexity ( & self ) -> f64 {"
                joined_token_streams!(mutator_field_idents.iter().map(|field_name| {
                    token_stream!("self . " field_name ". min_complexity ( )")
                }), "+")
            "}
            
            fn complexity ( & self , value : & Self :: Value , cache : & Self :: Cache ) -> f64 { cache . cplx }

            fn cache_from_value ( & self , value : & Self :: Value ) -> Self :: Cache {"
                // declare all subcaches
                fields_iter.clone().map(|(f, m)| {
                    token_stream!("let" m "= self ." m ". cache_from_value ( & value ." f ") ;")
                }).collect::<Vec<_>>()
                // compute cplx
                "let cplx =" joined_token_streams!(fields_iter.clone().map(|(f, m)| {
                    token_stream!("self . " m ". complexity ( & value ." f ", &" m ")")
                }), "+") ";"
                
                "Self :: Cache {"
                    joined_token_streams!(mutator_field_idents.iter(), ",")
                    ", cplx
                }
            }

            fn initial_step_from_value ( & self , value : & Self :: Value ) -> Self :: MutationStep {"
                // init all substeps
                fields_iter.clone().map(|(f, m)| {
                    token_stream!("let " m " = self . " m ". initial_step_from_value ( & value ." f ") ;")
                }).collect::<Vec<_>>()

                "let step = 0 ;

                Self :: MutationStep {"
                    joined_token_streams!(mutator_field_idents.iter(), ",")
                    ", step
                }
            }
            
            fn random_step_from_value ( & self , value : & Self :: Value ) -> Self :: MutationStep {"
                // init all substeps
                fields_iter.clone().map(|(f, m)| {
                    token_stream!("let " m " = self . " m ". random_step_from_value ( & value ." f ") ;")
                }).collect::<Vec<_>>()

                "let step = self . rng . u64 ( .. ) ;

                Self :: MutationStep {"
                    joined_token_streams!(mutator_field_idents.iter(), ",")
                    ", step
                }
            }

            fn ordered_arbitrary ( & mut self , _seed : usize , max_cplx : f64 ) -> Option < ( Self :: Value , Self :: Cache ) > {
                Some ( self . random_arbitrary ( max_cplx ) )
            }

            fn random_arbitrary ( & mut self , max_cplx : f64 ) -> ( Self :: Value , Self :: Cache ) {"
                mutator_field_idents.iter().map(|mutator_field| {
                    format!("
                        let mut {m}_value : Option < _ > = None ;
                        let mut {m}_cache : Option < _ > = None ;
                        ", m = mutator_field)
                    }
                ).collect::<Vec<_>>()
                "let mut indices = ( 0 .. " mutator_field_idents.len() ") . collect :: < Vec < _ > > ( ) ;
                fuzzcheck_mutators :: fastrand :: shuffle ( & mut indices ) ;
                let seed = fuzzcheck_mutators :: fastrand :: usize ( .. ) ;
                let mut cplx = f64 :: default ( ) ;

                for idx in indices . iter ( ) {
                    match idx {"
                    mutator_field_idents.iter().enumerate().map(|(idx, mutator_field)| {
                        format!(
                            "
                            {i} => {{ 
                                let ( value , cache ) = self . {m} . random_arbitrary ( max_cplx - cplx ) ;
                                cplx += self . {m} . complexity ( & value , & cache ) ; 
        
                                {m}_value = Some ( value ) ;
                                {m}_cache = Some ( cache ) ;
                            }}
                            ",
                                i = idx,
                                m = mutator_field
                            )
                        }
                    ).collect::<Vec<_>>()
                            "_ => unreachable ! ( )
                    }
                }
                (
                    Self :: Value {"
                        fields_iter.clone().map(|(f, m)| {
                            format!("{f} : {m}_value . unwrap ( ) ,", f = f, m = m)
                        }).collect::<Vec<_>>()
                    "} ,
                    Self :: Cache {"
                        mutator_field_idents.iter().map(|m| {
                            format!("{m} : {m}_cache . unwrap ( ) ,", m = m)
                        }).collect::<Vec<_>>()
                        "cplx 
                    }
                )
            }

            fn mutate ( & mut self , value : & mut Self :: Value , cache : & mut Self :: Cache , step : & mut Self :: MutationStep , max_cplx : f64 ) -> Option < Self :: UnmutateToken >
            {
                let orig_step = step . step ;
                step . step += 1 ;
                let current_cplx = self . complexity ( value , cache ) ;
                match orig_step % " mutator_field_idents.len() "{"
                fields_iter.clone().enumerate().map(|(i, (f, m))| {
                    token_stream!(
                        i " => {
                            let current_field_cplx = self ." m ". complexity ( & value ." f ", & cache ." m ") ;
                            let max_field_cplx = max_cplx - current_cplx - current_field_cplx ;
                            let token = self ." m ". mutate ( & mut  value ." f ", & mut cache ." m ", & mut step ." m ", max_field_cplx ) ;
                            let new_field_complexity = self ." m ". complexity ( & value ." f ", & cache ." m ") ;
                            cache . cplx = cache . cplx - current_field_cplx + new_field_complexity ;
                            Some ( Self :: UnmutateToken {"
                                m ": token ,
                                cplx : current_cplx ,
                                .. Self :: UnmutateToken :: default ( )
                            } )
                        }"
                    )
                }).collect::<Vec<_>>()
                        "_ => unreachable ! ( )
                }
            }

            fn unmutate ( & self , value : & mut Self :: Value , cache : & mut Self :: Cache , t : Self :: UnmutateToken )
            {
                cache . cplx = t . cplx ;"
                fields_iter.map(|(f, m)| {
                    token_stream!(
                        "if let Some ( subtoken ) = t ." m "{"
                            "self ." m ". unmutate ( & mut value ." f ", & mut cache ." m ", subtoken ) ;"
                        "}"
                    )
                }).collect::<Vec<_>>()
            "}
        }"
    );

    { // default impl
        let mut additional_where_items = Vec::<WhereClauseItem>::new();
        for ty in generic_types_for_field.clone() {
            let where_item = WhereClauseItem {
                for_lifetimes: None,
                lhs: token_stream!(ty),
                rhs: token_stream!(":: core :: default :: Default")
            };
            additional_where_items.push(where_item);
        }
        let where_clause = if let Some(mut where_clause) = mutator_struct.where_clause.clone() {
            where_clause.items.extend(additional_where_items);
            where_clause
        } else {
            WhereClause { items: additional_where_items }
        };

        extend_token_builder!(tb,
        "impl" mutator_struct.generics ":: core :: default :: Default for" mutator_struct.ident
            mutator_struct.generics.removing_bounds_and_eq_type() where_clause 
        "{
            fn default ( ) -> Self {
                Self {"
                    mutator_struct.struct_fields.iter().map(|field| {
                        token_stream!(field.identifier ":" "<" field.ty "as :: core :: default :: Default > :: default ( ) ,")
                    }).collect::<Vec<_>>()
                "}
            }
        }"
        )
    }
    {
        // implementation of HasDefaultMutator trait when generic mutator params are HasDefaultMutator
        let mut where_items = Vec::<WhereClauseItem>::new();
        for field in parsed_struct.struct_fields.iter() {
            let where_item = WhereClauseItem {
                for_lifetimes: None,
                lhs: field.ty.clone(),
                rhs: token_stream!("fuzzcheck_mutators :: HasDefaultMutator"),
            };
            where_items.push(where_item);
            let where_item = WhereClauseItem {
                for_lifetimes: None,
                lhs: token_stream!("<" field.ty " as fuzzcheck_mutators :: HasDefaultMutator > :: Mutator"),
                rhs: token_stream!(":: core :: default :: Default"),
            };
            where_items.push(where_item);
            let where_item = WhereClauseItem {
                for_lifetimes: None,
                lhs: field.ty.clone(),
                rhs: token_stream!(":: core :: clone :: Clone"),
            };
            where_items.push(where_item);
        }
        let where_item = WhereClauseItem {
            for_lifetimes: None,
            lhs: value_struct_ident_with_generic_params.clone(),
            rhs: token_stream!(":: core :: clone :: Clone"),
        };
        where_items.push(where_item);

        let where_clause = if let Some(mut where_clause) = parsed_struct.where_clause.clone() {
            where_clause.items.extend(where_items);
            where_clause
        } else {
            WhereClause { items: where_items }
        };

        let generics_mutator = {
            let mut type_params = generics_without_bounds.type_params.clone();
            for field in parsed_struct.struct_fields.iter() {
                type_params.push(TypeParam {
                    type_ident: token_stream!("<" field.ty "as fuzzcheck_mutators :: HasDefaultMutator > :: Mutator"),
                    ..TypeParam::default()
                });
            }
            Generics {
                lifetime_params: generics_without_bounds.lifetime_params.clone(),
                type_params,
            }
        };

        extend_token_builder!(tb,
        "impl" parsed_struct.generics "fuzzcheck_mutators :: HasDefaultMutator for" parsed_struct.ident 
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

// fn derive_enum_mutator(parsed_enum: Enum, tb: &mut TokenBuilder) {
//     if !parsed_enum.items.is_empty() {
//         derive_enum_mutator_with_items(&parsed_enum, tb)
//     } else {
//         todo!("Build mutator for empty enum");
//     }
// }

// struct EnumItemDataForMutatorDerive {
//     item: EnumItem, // Aa
//     fields: Vec<EnumItemDataFieldForMutatorDerive>, // (u8, _Aa_0, _Aa_0_Type) or (pub x: u16, _Aa_x, _Aa_x_Type),  }
// }
// struct EnumItemDataFieldForMutatorDerive {
//     field: StructField,
//     name: Ident,
//     mutator_ty: Ident,
// }

// fn derive_enum_mutator_with_items(parsed_enum: &Enum, tb: &mut TokenBuilder) {
//     // let item_idents = parsed_enum
//     //     .items
//     //     .iter()
//     //     .map(|f| f.ident.clone())
//     //     .collect::<Vec<_>>();

//     // let item_names = item_idents
//     //     .iter()
//     //     .map(|name| name.to_string())
//     //     .collect::<Vec<_>>();

//     // let fields_of_items = parsed_enum.items.iter().map(|item| {
//     //     match &item.data {
//     //         Some(EnumItemData::Discriminant(_)) | None => {
//     //             todo!("generate mutator for enum item with no fields")
//     //         }
//     //         Some(EnumItemData::Struct(struct_kind, fields)) => {

//     //         }
//     //     }
//     // });


//     let (basic_generics, items_for_derive, mutator_struct, value_struct_ident_with_generic_params) = { // mutator struct
//         /*
//         generics: existing generics without the bounds + generic mutator type params
//         where_clause_items: existing where_clause + existing generic bounds + “Mutator” conditions
//         mutator_field_types: the generic types for the sub-mutators, one for each field
//         */
//         /*
//         1. remove the bounds from the existing generics
//         2. put those bounds on new where_clause_items
//         */
//         let (mut generics, mut where_clause_items): (Generics, Vec<WhereClauseItem>) = {
//             let generics = &parsed_enum.generics;
//             let (generics, bounds) = generics.removing_bounds_and_eq_type();
//             let where_clause_items = generics
//                 .lifetime_params
//                 .iter()
//                 .map(|lp| lp.ident.clone())
//                 .chain(generics.type_params.iter().map(|tp| tp.type_ident.clone()))
//                 .zip(bounds.iter())
//                 .filter_map(|(lhs, rhs)| if let Some(rhs) = rhs { Some((lhs, rhs)) } else { None })
//                 .map(|(lhs, rhs)| WhereClauseItem {
//                     for_lifetimes: None,
//                     lhs,
//                     rhs: rhs.clone(),
//                 })
//                 .collect();

//             (generics, where_clause_items)
//         };


//         /*
//         3. extend the existing where_clause_items with those found in 2.
//         */
//         if let Some(where_clause) = &parsed_enum.where_clause {
//             where_clause_items.extend(where_clause.items.iter().cloned());
//         }
//         // let value_where_clause_with_added_items_from_generics = if where_clause_items.is_empty() {
//         //     None
//         // } else {
//         //     Some(WhereClause {
//         //         items: where_clause_items.clone(),
//         //     })
//         // };

//         let items_for_derive = parsed_enum.items.iter().map(|item| {
//             EnumItemDataForMutatorDerive {
//                 item: item.clone(),
//                 fields: match &item.data {
//                     Some(EnumItemData::Struct(_, fields)) => {
//                         fields.iter().enumerate().map(|(i, field)| {
//                             let submutator_name = {
//                                 Ident::new(
//                                     &format!(
//                                         "_{}_{}", 
//                                         item.ident, field.clone().identifier.map(|ident| ident.to_string()).unwrap_or(
//                                             format!("{}", i)
//                                         )
//                                     ), 
//                                     Span::call_site()
//                                 )
//                             };
//                             let submutator_type_ident = {
//                                 Ident::new(
//                                     &format!("{}_Type", submutator_name), 
//                                     Span::call_site()
//                                 )
//                             };
//                             EnumItemDataFieldForMutatorDerive {
//                                 field: field.clone(), 
//                                 name: submutator_name, 
//                                 mutator_ty: submutator_type_ident
//                             }
//                         }).collect::<Vec<_>>()
//                     }
//                     Some(EnumItemData::Discriminant(_)) => vec![],
//                     None => vec![]
//                 }
//             }
//         }).collect::<Vec<_>>();

//         let basic_generics = Generics {
//             lifetime_params: Vec::new(),
//             type_params: items_for_derive.iter().flat_map(|item| item.fields.iter()).map(|x| x.mutator_ty.clone())
//                 .map(|ident| TypeParam {
//                     attributes: Vec::new(),
//                     type_ident: TokenTree::Ident(ident).into(),
//                     bounds: None,
//                     equal_ty: None,
//                 })
//                 .collect(),
//         };

//         let basic_fields = items_for_derive.iter().flat_map(|item| item.fields.iter()).map(|x| (x.name.clone(), x.mutator_ty.clone()))
//             .map(|(identifier, ty)| StructField {
//                 attributes: Vec::new(),
//                 visibility: {
//                     let mut tb = TokenBuilder::new();
//                     tb.add("pub");
//                     Some(tb.end())
//                 },
//                 identifier: Some(identifier.clone()),
//                 ty: TokenTree::Ident(ty.clone()).into(),
//             })
//             .collect::<Vec<_>>();

//         let value_struct_ident_with_generic_params = {
//             let mut tb = TokenBuilder::new();
//             tb.extend(parsed_enum.ident.clone());
//             let generic_args = parsed_enum
//                 .generics
//                 .clone()
//                 .removing_bounds_and_eq_type()
//                 .0
//                 .to_token_stream();
//             tb.stream(generic_args);
//             tb.end()
//         };

//         /*
//         4. for each field, add a generic parameter for its mutator as well as a where_clause_item
//            ensuring it impls the Mutator trait and that it impls the Clone trait
//         */
//         for EnumItemDataFieldForMutatorDerive { field, name: _, mutator_ty: generic_ty_for_field } in items_for_derive.iter().flat_map(|item| item.fields.iter()) {
//             let ty_param = TypeParam {
//                 attributes: Vec::new(),
//                 type_ident: TokenTree::Ident(generic_ty_for_field.clone()).into(),
//                 bounds: None,
//                 equal_ty: None,
//             };
//             generics.type_params.push(ty_param);
//             where_clause_items.push(WhereClauseItem {
//                 for_lifetimes: None,
//                 lhs: field.ty.clone(),
//                 rhs: {
//                     let mut tb = TokenBuilder::new();
//                     tb.add(":: core :: clone :: Clone");
//                     tb.end()
//                 },
//             });
//             where_clause_items.push(WhereClauseItem {
//                 for_lifetimes: None,
//                 lhs: TokenTree::Ident(generic_ty_for_field.clone()).into(),
//                 rhs: {
//                     let mut tb = TokenBuilder::new();
//                     tb.add("fuzzcheck_mutators :: fuzzcheck_traits :: Mutator < Value = ");
//                     tb.stream(field.ty.clone());
//                     tb.add(">");
//                     tb.end()
//                 },
//             });
//         }
//         /* 5. also add requirement that the whole value is Clone */
//         where_clause_items.push(WhereClauseItem {
//             for_lifetimes: None,
//             lhs: value_struct_ident_with_generic_params.clone(),
//             rhs: {
//                 let mut tb = TokenBuilder::new();
//                 tb.add(":: core :: clone :: Clone");
//                 tb.end()
//             },
//         });

//         let main_mutator_where_clause = {
//             WhereClause {
//                 items: where_clause_items,
//             }
//         };

//         let mut mutator_struct_fields = basic_fields.clone();
//         mutator_struct_fields.push(StructField {
//             attributes: Vec::new(),
//             visibility: {
//                 let mut tb = TokenBuilder::new();
//                 tb.add("pub");
//                 Some(tb.end())
//             },
//             identifier: Some(Ident::new("rng", Span::call_site())),
//             ty: {
//                 let mut tb = TokenBuilder::new();
//                 tb.add("fuzzcheck_mutators :: fastrand :: Rng");
//                 tb.end()
//             },
//         });

//         (basic_generics, items_for_derive, Struct {
//             visibility: parsed_enum.visibility.clone(),
//             ident: Ident::new(&format!("{}Mutator", parsed_enum.ident), Span::call_site()),
//             generics,
//             kind: StructKind::Struct,
//             where_clause: Some(main_mutator_where_clause),
//             struct_fields: mutator_struct_fields,
//         }, value_struct_ident_with_generic_params)
//     };

//     tb.stream(mutator_struct.clone().to_token_stream());

//     let inner_items = {
//         let mut items = Vec::<EnumItem>::new();

//         for item in &items_for_derive {
//             items.push(EnumItem {
//                 attributes: Vec::new(),
//                 ident: item.item.ident.clone(),
//                 data: if item.fields.is_empty() { None } else { 
//                     Some(EnumItemData::Struct(
//                         StructKind::Struct, 
//                         item.fields.iter().map(|field| {
//                             StructField {
//                                 attributes: Vec::new(),
//                                 visibility: None,
//                                 identifier: Some(field.name.clone()),
//                                 ty: TokenTree::Ident(field.mutator_ty.clone()).into(),
//                             }
//                         }).collect()
//                     ))
//                 },
//             });
//         }
//         items
//     };

//     let (cache_enum, cache_struct) = { // mutator cache
//         let cache_enum = Enum {
//             visibility: parsed_enum.visibility.clone(),
//             ident: Ident::new(&format!("{}InnerMutatorCache", parsed_enum.ident.clone()), Span::call_site()),
//             generics: basic_generics.clone(),
//             where_clause: None,
//             items: inner_items.clone(),
//          };

//          let cache_struct = Struct {
//              visibility: parsed_enum.visibility.clone(),
//              ident: Ident::new(&format!("{}MutatorCache", parsed_enum.ident.clone()), Span::call_site()),
//              generics: basic_generics.clone(),
//              kind: StructKind::Struct,
//              where_clause: None,
//              struct_fields: vec![
//                  StructField { 
//                      attributes: Vec::new(), 
//                      visibility: None, 
//                      identifier: Some(Ident::new("inner", Span::call_site())),
//                      ty: {
//                         let mut tb = TokenBuilder::new();
//                         tb.extend(cache_enum.ident.clone());
//                         tb.stream(cache_enum.generics.clone().to_token_stream());
//                         tb.end()
//                      }
//                 },
//                 StructField { 
//                     attributes: Vec::new(), 
//                     visibility: None, 
//                     identifier: Some(Ident::new("cplx", Span::call_site())),
//                     ty: TokenTree::Ident(Ident::new("f64", Span::call_site())).into()
//                }
//              ],
            
//          };

//          (cache_enum, cache_struct)
//     };

//     tb.stream(cache_enum.clone().to_token_stream());
//     tb.stream(cache_struct.clone().to_token_stream());

//     let (step_enum, step_struct) = { // mutation step
//         let step_enum = Enum {
//             visibility: parsed_enum.visibility.clone(),
//             ident: Ident::new(&format!("{}InnerMutationStep", parsed_enum.ident.clone()), Span::call_site()),
//             generics: basic_generics.clone(),
//             where_clause: None,
//             items: inner_items.clone(),
//          };

//          let field_inner = StructField {
//              attributes: Vec::new(),
//              visibility: None,
//              identifier: Some(Ident::new("inner", Span::call_site())),
//              ty: {
//                  let mut tb = TokenBuilder::new();
//                  tb.extend(step_enum.ident.clone());
//                  tb.stream(step_enum.generics.clone().to_token_stream());
//                  tb.end()
//              },
//          };
//          let field_step = StructField {
//             attributes: Vec::new(),
//             visibility: None,
//             identifier: Some(Ident::new("step", Span::call_site())),
//             ty: {
//                 let mut tb = TokenBuilder::new();
//                 tb.add("u64");
//                 tb.end()
//             },
//         };

//          let step_struct = Struct {
//              visibility: parsed_enum.visibility.clone(),
//              ident: Ident::new(&format!("{}MutationStep", parsed_enum.ident.clone()), Span::call_site()),
//              generics: basic_generics.clone(),
//              kind: StructKind::Struct,
//              where_clause: None,
//              struct_fields: vec![field_inner, field_step],
//          };

//          (step_enum, step_struct)
//     };

//     tb.stream(step_enum.clone().to_token_stream());
//     tb.stream(step_struct.clone().to_token_stream());

//     let unmutate_enum = {
//         let unmutate_enum = Enum {
//             visibility: parsed_enum.visibility.clone(),
//             ident: Ident::new(&format!("{}UnmutateToken", parsed_enum.ident.clone()), Span::call_site()),
//             generics: basic_generics.clone(),
//             where_clause: None,
//             items: inner_items.clone().into_iter().map(|inner_item| {
//                 EnumItem {
//                     attributes: inner_item.attributes,
//                     ident: inner_item.ident,
//                     data: match inner_item.data {
//                         Some(EnumItemData::Struct(kind, fields)) => {
//                             Some(EnumItemData::Struct(kind, fields.into_iter().map(|field| {
//                                 StructField {
//                                     attributes: field.attributes,
//                                     visibility: field.visibility,
//                                     identifier: field.identifier,
//                                     ty: {
//                                         let mut tb = TokenBuilder::new();
//                                         tb.add(":: std :: option :: Option :: <");
//                                         tb.stream(field.ty);
//                                         tb.add(">");
//                                         tb.end()
//                                     },
//                                 }
//                             }).collect()))
//                         }
//                         data @ Some(EnumItemData::Discriminant(_)) | data @ None => { 
//                             data
//                         }
//                     },
//                 }
//             }).collect(),
//          };
//          unmutate_enum
//     };

//     tb.stream(unmutate_enum.clone().to_token_stream());

//     { // impl Mutator
//         let generics = mutator_struct.generics.clone().to_token_stream();
//         tb.add("impl");
//         tb.stream(generics.clone());
//         tb.add("fuzzcheck_mutators :: fuzzcheck_traits :: Mutator for");
//         tb.extend(mutator_struct.ident.clone());
//         tb.stream(generics);
//         tb.stream_opt(mutator_struct.where_clause.clone().map(|wc| wc.to_token_stream()));
//         tb.push_group(Delimiter::Brace);

//         {
//             // associated types
//             tb.add("type Value = ");
//             tb.stream(value_struct_ident_with_generic_params.clone());
//             tb.add(";");

//             tb.add("type Cache = ");
//             let cache_struct_ident_with_generic_params = {
//                 let mut tb = TokenBuilder::new();
//                 tb.extend(cache_struct.ident.clone());
//                 let generic_args = {
//                     let mut g = cache_struct.generics.clone();
//                     for tp in g.type_params.iter_mut() {
//                         let mut tb = TokenBuilder::new();
//                         tb.add("<");
//                         tb.stream(tp.type_ident.clone());
//                         tb.add(" as fuzzcheck_mutators :: fuzzcheck_traits :: Mutator > :: Cache");
//                         tp.type_ident = tb.end();
//                     }
//                     g.to_token_stream()
//                 };
//                 tb.stream(generic_args);
//                 tb.end()
//             };
//             tb.stream(cache_struct_ident_with_generic_params);
//             tb.add(";");

//             tb.add("type MutationStep = ");
//             let mutator_step_struct_ident_with_generic_params = {
//                 let mut tb = TokenBuilder::new();
//                 tb.extend(step_struct.ident.clone());
//                 let generic_args = {
//                     let mut g = step_struct.generics.clone();
//                     for tp in g.type_params.iter_mut() {
//                         let mut tb = TokenBuilder::new();
//                         tb.add("<");
//                         tb.stream(tp.type_ident.clone());
//                         tb.add(" as fuzzcheck_mutators :: fuzzcheck_traits :: Mutator > :: MutationStep");
//                         tp.type_ident = tb.end();
//                     }
//                     g.to_token_stream()
//                 };
//                 tb.stream(generic_args);
//                 tb.end()
//             };
//             tb.stream(mutator_step_struct_ident_with_generic_params);
//             tb.add(";");

//             tb.add("type UnmutateToken = ");
//             let unmutate_token_struct_ident_with_generic_params = {
//                 let mut tb = TokenBuilder::new();
//                 tb.extend(unmutate_enum.ident.clone());
//                 let generic_args = {
//                     let mut g = unmutate_enum.generics.clone();
//                     for tp in g.type_params.iter_mut() {
//                         let mut tb = TokenBuilder::new();
//                         tb.add("<");
//                         tb.stream(tp.type_ident.clone());
//                         tb.add(" as fuzzcheck_mutators :: fuzzcheck_traits :: Mutator > :: UnmutateToken");
//                         tp.type_ident = tb.end();
//                     }
//                     g.to_token_stream()
//                 };
//                 tb.stream(generic_args);
//                 tb.end()
//             };
//             tb.stream(unmutate_token_struct_ident_with_generic_params);
//             tb.add(";");
//         }

//         let cplx_choose_item = ((parsed_enum.items.len() as f64).log2() * 100.0).round() / 100.0;
//         let cplx_choose_item_literal = TokenTree::Literal(Literal::f64_suffixed(cplx_choose_item));

//         {
//             // max_complexity
//             tb.add("fn max_complexity ( & self ) -> f64 { ");
            
//             tb.extend(cplx_choose_item_literal.clone());
//             if items_for_derive.iter().find(|item| !item.fields.is_empty()).is_some() {
//                 tb.add("+ core :: cmp :: max ( ");
//                 for item_data in items_for_derive.iter().filter(|item| !item.fields.is_empty()) {
//                     let mut mutator_field_names_iter = item_data.fields.iter().map(|field| field.name.clone());
//                     if let Some(fst_field_name) = mutator_field_names_iter.next() {
//                         tb.add(&format!("self . {} . max_complexity ( ) ", fst_field_name));
//                     }
//                     for field_name in mutator_field_names_iter {
//                         tb.add(&format!("+ self . {} . max_complexity ( ) ", field_name));
//                     }
//                     tb.add(",");
//                 }
//                 tb.add(")");
//             }

//             tb.add("}");
//         }

//         {
//             // min_complexity
//             tb.add("fn min_complexity ( & self ) -> f64 { ");
            
//             tb.extend(cplx_choose_item_literal.clone());
//             if items_for_derive.iter().find(|item| !item.fields.is_empty()).is_some() {
//                 tb.add("+ core :: cmp :: min ( ");
//                 for item_data in items_for_derive.iter().filter(|item| !item.fields.is_empty()) {
//                     let mut mutator_field_names_iter = item_data.fields.iter().map(|field| field.name.clone());
//                     if let Some(fst_field_name) = mutator_field_names_iter.next() {
//                         tb.add(&format!("self . {} . min_complexity ( ) ", fst_field_name));
//                     }
//                     for field_name in mutator_field_names_iter {
//                         tb.add(&format!("+ self . {} . min_complexity ( ) ", field_name));
//                     }
//                     tb.add(",");
//                 }
//                 tb.add(")");
//             }

//             tb.add("}");
//         }

//         {
//             // complexity
//             tb.add(
//                 "fn complexity ( & self , value : & Self :: Value , cache : & Self :: Cache ) -> f64 { cache . cplx }",
//             );
//         }
        
//         { // cache from value
//             tb.add("fn cache_from_value ( & self , value : & Self :: Value ) -> Self :: Cache");
//             tb.push_group(Delimiter::Brace);

//             tb.add("match value");
//             tb.push_group(Delimiter::Brace);
//             for (item_for_derive, item) in items_for_derive.iter().zip(parsed_enum.items.iter()) {
//                 tb.add(&format!("{} :: {}", parsed_enum.ident, item.ident));
//                 if let Some(EnumItemData::Struct(kind, _)) = &item.data {
//                     let delimiter = match kind {
//                         StructKind::Struct => Delimiter::Brace,
//                         StructKind::Tuple => Delimiter::Parenthesis
//                     };
//                     tb.push_group(delimiter);
//                     for field in item_for_derive.fields.iter() {
//                         tb.extend(field.name.clone());
//                         tb.add(",");
//                     }
//                     tb.pop_group(delimiter);
//                     tb.add("=>");
//                     tb.push_group(Delimiter::Brace);

//                     tb.add("let inner = ");
//                     tb.add(&format!("XInnerMutatorCache :: {}", item.ident));
//                     tb.push_group(Delimiter::Brace);
//                     for field in item_for_derive.fields.iter() {
//                         tb.extend(field.name.clone());
//                         tb.add(&format!(": self . {name} . cache_from_value ( & {name} ) ,", name=field.name));
//                     }
//                     tb.pop_group(Delimiter::Brace);
//                     tb.add(";");

//                     tb.add("let cplx = ");
//                     tb.extend(cplx_choose_item_literal.clone());
//                     for field in item_for_derive.fields.iter() {
//                         tb.add(&format!("+ self . {name} . complexity ( & {name} ) ", name=field.name));
//                     }
//                     tb.add(";");

//                     tb.add("XMutatorCache {
//                         inner ,
//                         cplx ,
//                     }"#);

//                     tb.pop_group(Delimiter::Brace);
//                 } else {
//                     tb.add("=>");
//                     tb.push_group(Delimiter::Brace);
//                     tb.add(&format!("XMutatorCache {{
//                         inner : XInnerMutatorCache :: {} ,"#
//                         , item.ident
//                     ));
//                     tb.add("cplx :");
//                     tb.extend(cplx_choose_item_literal.clone());
//                     tb.add("}");

//                     tb.pop_group(Delimiter::Brace);
//                 }

//             }

//             { // initial step from value
                
//             }

//             tb.pop_group(Delimiter::Brace);

//             tb.pop_group(Delimiter::Brace);
//         }

//         { // initial step from value
//             tb.add("fn initial_step_from_value ( & self , value : & Self :: Value ) -> Self :: MutationStep");
//             tb.push_group(Delimiter::Brace);

//             tb.add("match value");
//             tb.push_group(Delimiter::Brace);
//             for (item_for_derive, item) in items_for_derive.iter().zip(parsed_enum.items.iter()) {
//                 tb.add(&format!("{} :: {}", parsed_enum.ident, item.ident));
//                 if let Some(EnumItemData::Struct(kind, _)) = &item.data {
//                     let delimiter = match kind {
//                         StructKind::Struct => Delimiter::Brace,
//                         StructKind::Tuple => Delimiter::Parenthesis
//                     };
//                     tb.push_group(delimiter);
//                     for field in item_for_derive.fields.iter() {
//                         tb.extend(field.name.clone());
//                         tb.add(",");
//                     }
//                     tb.pop_group(delimiter);
//                     tb.add("=>");
//                     tb.push_group(Delimiter::Brace);

//                     tb.add("let inner = ");
//                     tb.add(&format!("XInnerMutationStep :: {}", item.ident));
//                     tb.push_group(Delimiter::Brace);
//                     for field in item_for_derive.fields.iter() {
//                         tb.extend(field.name.clone());
//                         tb.add(&format!(": self . {name} . initial_step_from_value ( & {name} ) ,", name=field.name));
//                     }
//                     tb.pop_group(Delimiter::Brace);
//                     tb.add(";");

//                     tb.add("let step = ");
//                     tb.extend(Literal::u64_suffixed(0));
//                     tb.add(";");

//                     tb.add("XMutationStep {
//                         inner ,
//                         step ,
//                     }"#);

//                     tb.pop_group(Delimiter::Brace);
//                 } else {
//                     tb.add("=>");
//                     tb.push_group(Delimiter::Brace);
//                     tb.add(&format!("XMutationStep {{
//                         inner : XInnerMutationStep :: {} ,"#
//                         , item.ident
//                     ));
//                     tb.add("step :");
//                     tb.extend(Literal::u64_suffixed(0));
//                     tb.add("}");

//                     tb.pop_group(Delimiter::Brace);
//                 }

//             }

//             { // initial step from value
                
//             }

//             tb.pop_group(Delimiter::Brace);

//             tb.pop_group(Delimiter::Brace);
//         }

//         { // random step from value
//             tb.add("fn random_step_from_value ( & self , value : & Self :: Value ) -> Self :: MutationStep");
//             tb.push_group(Delimiter::Brace);

//             tb.add("match value");
//             tb.push_group(Delimiter::Brace);
//             for (item_for_derive, item) in items_for_derive.iter().zip(parsed_enum.items.iter()) {
//                 tb.add(&format!("{} :: {}", parsed_enum.ident, item.ident));
//                 if let Some(EnumItemData::Struct(kind, _)) = &item.data {
//                     let delimiter = match kind {
//                         StructKind::Struct => Delimiter::Brace,
//                         StructKind::Tuple => Delimiter::Parenthesis
//                     };
//                     tb.push_group(delimiter);
//                     for field in item_for_derive.fields.iter() {
//                         tb.extend(field.name.clone());
//                         tb.add(",");
//                     }
//                     tb.pop_group(delimiter);
//                     tb.add("=>");
//                     tb.push_group(Delimiter::Brace);

//                     tb.add("let inner = ");
//                     tb.add(&format!("XInnerMutationStep :: {}", item.ident));
//                     tb.push_group(Delimiter::Brace);
//                     for field in item_for_derive.fields.iter() {
//                         tb.extend(field.name.clone());
//                         tb.add(&format!(": self . {name} . random_step_from_value ( & {name} ) ,", name=field.name));
//                     }
//                     tb.pop_group(Delimiter::Brace);
//                     tb.add(";");

//                     tb.add("let step = self . rng . u64 ( .. ) ;");
//                     tb.add(";");

//                     tb.add("XMutationStep {
//                         inner ,
//                         step ,
//                     }"#);

//                     tb.pop_group(Delimiter::Brace);
//                 } else {
//                     tb.add("=>");
//                     tb.push_group(Delimiter::Brace);
//                     tb.add(&format!("XMutationStep {{
//                         inner : XInnerMutationStep :: {} ,"#
//                         , item.ident
//                     ));
//                     tb.add("step : self . rng . u64 ( .. )");
//                     tb.add("}");

//                     tb.pop_group(Delimiter::Brace);
//                 }

//             }

//             { // initial step from value
                
//             }

//             tb.pop_group(Delimiter::Brace);

//             tb.pop_group(Delimiter::Brace);
//         }

//         { // arbitrary
            
//         }

//         tb.pop_group(Delimiter::Brace);
//     }
// }

// fn derive_unit_mutator(parsed_struct: Struct, tb: &mut TokenBuilder) {
    
//     let generics_without_bounds = parsed_struct.generics.clone().removing_bounds_and_eq_type().0;
    
//     let parsed_struct_ident: TokenStream = {
//         let mut tb = TokenBuilder::new();
//         tb.extend(parsed_struct.ident.clone());
//         tb.end()
//     };

//     let generics = parsed_struct.generics.clone().to_token_stream();
//     let wc = parsed_struct.where_clause.clone().map(|wc| wc.to_token_stream()).unwrap_or(quote!{});
//     tb.stream(quote! { type $parsed_struct_ident Mutator $generics $wc });

//     tb.add(&format!("type {name}Mutator",name=parsed_struct.ident));
//     tb.stream(parsed_struct.generics.clone().to_token_stream());
//     tb.stream_opt(parsed_struct.where_clause.clone().map(|wc| wc.to_token_stream()));
//     tb.add(&format!("= fuzzcheck_mutators :: unit :: UnitMutator < {name}", name=parsed_struct.ident));
//     tb.stream(generics_without_bounds.clone().to_token_stream());
//     tb.add("> ;");

//     tb.add("impl");
//     tb.stream(parsed_struct.generics.clone().to_token_stream());
//     tb.add(&format!("HasDefaultMutator for {name}", name=parsed_struct.ident));
//     tb.stream(generics_without_bounds.clone().to_token_stream());
//     tb.stream_opt(parsed_struct.where_clause.clone().map(|wc| wc.to_token_stream()));
//     tb.push_group(Delimiter::Brace);
    
//     tb.add(&format!("type Mutator = {name}Mutator", name=parsed_struct.ident));
//     tb.stream(generics_without_bounds.clone().to_token_stream());
//     tb.add(";");
//     tb.add("fn default_mutator ( ) -> Self :: Mutator");
    
//     tb.push_group(Delimiter::Brace);
//     tb.add(&format!("Self :: Mutator :: new ( {name} {{ }} ) ", name=parsed_struct.ident));
//     tb.pop_group(Delimiter::Brace);
    
//     tb.pop_group(Delimiter::Brace);
// }
