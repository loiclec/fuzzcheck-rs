#![allow(dead_code)]
#![feature(proc_macro_quote)]

mod token_builder;
mod parser;

//mod struct_derive;

use crate::token_builder::*;
use crate::parser::*;

use proc_macro::quote;

use proc_macro::{Delimiter, Ident, Literal, Span, TokenStream, TokenTree};

#[proc_macro_derive(HasDefaultMutator)]
pub fn derive_mutator(input: TokenStream) -> TokenStream {
    let mut parser = TokenParser::new(input);
    let mut tb = TokenBuilder::new();

    if let Some(s) = parser.eat_struct() {
        derive_struct_mutator(s, &mut tb);
    } else if let Some(e) = parser.eat_enumeration() {
        // derive_enum_mutator(e, &mut tb)
    } else {
        tb.add("compile_error ! (");
        tb.extend(TokenTree::from(Literal::string("fuzzcheck_mutators_derive could not parse the structure")));
        tb.add(") ;");
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
    let fields = &parsed_struct.struct_fields;

    let mutator_field_names = fields
        .iter()
        .enumerate()
        .map(|(i, f)| {
            if let Some(ident) = &f.identifier {
                format!("_{}", ident)
            } else {
                format!("_{}", i)
            }
        })
        .collect::<Vec<_>>();

    let mutator_field_idents = mutator_field_names
        .iter()
        .map(|name| Ident::new(name, Span::call_site()))
        .collect::<Vec<_>>();

    let field_names = parsed_struct
        .struct_fields
        .iter()
        .enumerate()
        .map(|(i, f)| {
            if let Some(ident) = &f.identifier {
                ident.to_string()
            } else {
                format!("{}", i)
            }
        })
        .collect::<Vec<_>>();

    let generic_types_for_field = mutator_field_names
        .iter()
        .map(|name| Ident::new(&format!("{}Type", name), Span::call_site()));

    let basic_generics = Generics {
        lifetime_params: Vec::new(),
        type_params: generic_types_for_field
            .clone()
            .map(|ident| TypeParam {
                attributes: Vec::new(),
                type_ident: TokenTree::Ident(ident).into(),
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
            visibility: {
                let mut tb = TokenBuilder::new();
                tb.add("pub");
                Some(tb.end())
            },
            identifier: Some(identifier.clone()),
            ty: TokenTree::Ident(ty.clone()).into(),
        })
        .collect::<Vec<_>>();

    let value_struct_ident_with_generic_params = token_stream(&[
        &parsed_struct.ident, &parsed_struct.generics.clone().removing_bounds_and_eq_type().0
    ]);

    let (
        mutator_struct,
        value_generics_with_bounds_moved_to_where_clause,
        value_where_clause_with_added_items_from_generics,
    ) = {
        /*
        generics: existing generics without the bounds + generic mutator type params
        where_clause_items: existing where_clause + existing generic bounds + “Mutator” conditions
        mutator_field_types: the generic types for the sub-mutators, one for each field
        */
        /*
        1. remove the bounds from the existing generics
        2. put those bounds on new where_clause_items
        */
        let (mut generics, mut where_clause_items): (Generics, Vec<WhereClauseItem>) = {
            let generics = &parsed_struct.generics;
            let (generics, bounds) = generics.removing_bounds_and_eq_type();
            let where_clause_items = generics
                .lifetime_params
                .iter()
                .map(|lp| lp.ident.clone())
                .chain(generics.type_params.iter().map(|tp| tp.type_ident.clone()))
                .zip(bounds.iter())
                .filter_map(|(lhs, rhs)| if let Some(rhs) = rhs { Some((lhs, rhs)) } else { None })
                .map(|(lhs, rhs)| WhereClauseItem {
                    for_lifetimes: None,
                    lhs,
                    rhs: rhs.clone(),
                })
                .collect();

            (generics, where_clause_items)
        };

        let value_generics_with_bounds_moved_to_where_clause = generics.clone();

        /*
        3. extend the existing where_clause_items with those found in 2.
        */
        if let Some(where_clause) = &parsed_struct.where_clause {
            where_clause_items.extend(where_clause.items.iter().cloned());
        }
        let value_where_clause_with_added_items_from_generics = if where_clause_items.is_empty() {
            None
        } else {
            Some(WhereClause {
                items: where_clause_items.clone(),
            })
        };

        /*
        4. for each field, add a generic parameter for its mutator as well as a where_clause_item
           ensuring it impls the Mutator trait and that it impls the Clone trait
        */
        for (field, generic_ty_for_field) in parsed_struct.struct_fields.iter().zip(generic_types_for_field.clone()) {
            let ty_param = TypeParam {
                attributes: Vec::new(),
                type_ident: TokenTree::Ident(generic_ty_for_field.clone()).into(),
                bounds: None,
                equal_ty: None,
            };
            generics.type_params.push(ty_param);
            where_clause_items.push(WhereClauseItem {
                for_lifetimes: None,
                lhs: field.ty.clone(),
                rhs: {
                    let mut tb = TokenBuilder::new();
                    tb.add(":: core :: clone :: Clone");
                    tb.end()
                },
            });
            where_clause_items.push(WhereClauseItem {
                for_lifetimes: None,
                lhs: TokenTree::Ident(generic_ty_for_field.clone()).into(),
                rhs: {
                    let mut tb = TokenBuilder::new();
                    tb.add("fuzzcheck_mutators :: fuzzcheck_traits :: Mutator < Value = ");
                    tb.stream(field.ty.clone());
                    tb.add(">");
                    tb.end()
                },
            });
        }
        /* 5. also add requirement that the whole value is Clone */
        where_clause_items.push(WhereClauseItem {
            for_lifetimes: None,
            lhs: value_struct_ident_with_generic_params.clone(),
            rhs: {
                let mut tb = TokenBuilder::new();
                tb.add(":: core :: clone :: Clone");
                tb.end()
            },
        });

        let main_mutator_where_clause = {
            WhereClause {
                items: where_clause_items,
            }
        };

        let mut mutator_struct_fields = basic_fields.clone();
        mutator_struct_fields.push(StructField {
            attributes: Vec::new(),
            visibility: {
                let mut tb = TokenBuilder::new();
                tb.add("pub");
                Some(tb.end())
            },
            identifier: Some(Ident::new("rng", Span::call_site())),
            ty: {
                let mut tb = TokenBuilder::new();
                tb.add("fuzzcheck_mutators :: fastrand :: Rng");
                tb.end()
            },
        });

        (
            Struct {
                visibility: parsed_struct.visibility.clone(),
                ident: Ident::new(&format!("{}Mutator", parsed_struct.ident), Span::call_site()),
                generics,
                kind: StructKind::Struct,
                where_clause: Some(main_mutator_where_clause),
                struct_fields: mutator_struct_fields,
            },
            value_generics_with_bounds_moved_to_where_clause,
            value_where_clause_with_added_items_from_generics,
        )
    };

    tb.stream(token_stream(&[&mutator_struct])); // TODO: change with direct method on tb

    let mutator_cache_struct = {
        let mut cache_fields = basic_fields.clone();
        cache_fields.push(StructField {
            attributes: Vec::new(),
            visibility: None,
            identifier: Some(Ident::new("cplx", Span::call_site())),
            ty: TokenTree::Ident(Ident::new("f64", Span::call_site())).into(),
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

    tb.add("# [ derive ( :: core :: clone :: Clone ) ]");
    tb.add_part(&mutator_cache_struct);

    let mutator_step_struct = {
        let mut step_fields = basic_fields.clone();
        step_fields.push(StructField {
            attributes: Vec::new(),
            visibility: None,
            identifier: Some(Ident::new("step", Span::call_site())),
            ty: TokenTree::Ident(Ident::new("u64", Span::call_site())).into(),
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

    tb.add("# [ derive ( :: core :: clone :: Clone ) ]");
    tb.add_part(&mutator_step_struct);

    let unmutate_token_struct = {
        let mut step_fields = basic_fields
            .iter()
            .map(|field| {
                let mut tb = TokenBuilder::new();
                tb.add(":: std :: option :: Option <");
                tb.stream(field.ty.clone());
                tb.add(">");
                StructField {
                    ty: tb.end(),
                    ..field.clone()
                }
            })
            .collect::<Vec<StructField>>();

        step_fields.push(StructField {
            attributes: Vec::new(),
            visibility: None,
            identifier: Some(Ident::new("cplx", Span::call_site())),
            ty: TokenTree::Ident(Ident::new("f64", Span::call_site())).into(),
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

    tb.add("# [ derive ( :: core :: clone :: Clone ) ]");
    tb.add_part(&unmutate_token_struct);

    // default impl for unmutate token
    tb.stream(token_stream(&[
        &"impl", &unmutate_token_struct.generics, &":: core :: default :: Default for", &unmutate_token_struct.ident, &unmutate_token_struct.generics, &"{",
            &"fn default ( ) -> Self {",
                &mutator_field_names.iter().map(|m|
                    token_stream(&[
                        &m.as_str(), &": None ," // TODO: field names as ident
                    ])
                ).collect::<Vec<_>>(),
            &"}",
        &"}"
    ]));


    let fields_iter = field_names.iter().zip(mutator_field_names.iter());

    // implementation of Mutator trait
    tb.stream(token_stream(&[
        &"impl", &mutator_struct.generics, &"fuzzcheck_mutators :: fuzzcheck_traits :: Mutator for", 
            &mutator_struct.ident, &mutator_struct.generics, &mutator_struct.where_clause, 
        &"{",
            &"type Value = ", &value_struct_ident_with_generic_params, &";",
            &"type Cache = ", &mutator_cache_struct.ident,
                &mutator_cache_struct.generics.mutating_type_params(|tp|
                    tp.type_ident = token_stream(&[&"<", &tp.type_ident, &"as fuzzcheck_mutators :: fuzzcheck_traits :: Mutator > :: Cache"])
                ),
                &";",
            
            &"type MutationStep = ",
                &mutator_step_struct.ident,
                &mutator_step_struct.generics.mutating_type_params(|tp|
                    tp.type_ident = token_stream(&[&"<", &tp.type_ident, &"as fuzzcheck_mutators :: fuzzcheck_traits :: Mutator > :: Cache"])
                ),
                &";",

            &"type UnmutateToken = ", 
                &unmutate_token_struct.ident,
                &unmutate_token_struct.generics.mutating_type_params(|tp|
                    tp.type_ident = token_stream(&[&"<", &tp.type_ident, &"as fuzzcheck_mutators :: fuzzcheck_traits :: Mutator > :: UnmutateToken"])
                ),
                &";",

            &"fn max_complexity ( & self ) -> f64 {",
                &joined_token_streams(mutator_field_names.iter().map(|field_name| { // TODO: make field name an ident
                    token_stream(&[&"self . ", &field_name.as_str(), &". max_complexity ( )"])
                }), "+"),
            &"}",

            &"fn min_complexity ( & self ) -> f64 {",
                &joined_token_streams(mutator_field_names.iter().map(|field_name| { // TODO: make field name an ident
                    token_stream(&[&"self . ", &field_name.as_str(), &". min_complexity ( )"])
                }), "+"),
            &"}",
            
            &"fn complexity ( & self , value : & Self :: Value , cache : & Self :: Cache ) -> f64 { cache . cplx }",

            &"fn cache_from_value ( & self , value : & Self :: Value ) -> Self :: Cache {",
                // declare all subcaches
                &fields_iter.clone().map(|(field, mutator_field)| { // TODO: make field and mutator_field idents or streams
                    let (m, f) = (mutator_field.as_str(), field.as_str());
                    token_stream(&[&"let", &m, &"= self .", &m, &". cache_from_value ( & value .", &f, &") ;"])
                }).collect::<Vec<_>>(),
                // compute cplx
                &joined_token_streams(fields_iter.clone().map(|(field, mutator_field)| {
                    let (m, f) = (mutator_field.as_str(), field.as_str());
                    token_stream(&[&"self . ", &m, &". complexity ( & value .", &f , &", &" , &m, &")"])
                }), "+"),
                
                &"Self :: Cache {",
                    &joined_token_streams(mutator_field_names.iter().map(|x| x.as_str()), ","), // mutator field names not string
                    &"cplx",
                &"}",
            &"}",

            &"fn initial_step_from_value ( & self , value : & Self :: Value ) -> Self :: MutationStep {",
                // init all substeps
                &fields_iter.clone().map(|(field, mutator_field)| {
                    let (m, f) = (mutator_field.as_str(), field.as_str());
                    token_stream(&[&"self . ", &m, &". initial_step_from_value ( & value .", &f , &") ;"])
                }).collect::<Vec<_>>(),

                &"let step = 0",

                &"Self :: MutationStep {",
                    &joined_token_streams(mutator_field_names.iter().map(|x| x.as_str()), ","), // mutator field names not string
                    &"step",
                &"}",
            &"}",

            &r#"fn ordered_arbitrary ( & mut self , _seed : usize , max_cplx : f64 ) -> Option < ( Self :: Value , Self :: Cache ) > {
                Some ( self . random_arbitrary ( max_cplx ) )
            }"#,
            
            // some parts were easier to write as a format string
            &format!(r##"
            "fn random_arbitrary ( & mut self , max_cplx : f64 ) -> ( Self :: Value , Self :: Cache ) {{
                {subvals_decl}

                let mut indices = ( 0 .. {nbr_fields} ) . collect :: < Vec < _ > > ( ) ;
                fuzzcheck_mutators :: fastrand :: shuffle ( & mut indices ) ;
                let seed = fuzzcheck_mutators :: fastrand :: usize ( .. ) ;
                let mut cplx = f64 :: default ( ) ;

                for idx in indices . iter ( ) {{
                    match idx {{
                        {body_match}
                        _ => unreachable ! ( )
                    }}
                }}
                ((
                    Self :: Value {{
                        {values}
                    }} ,
                    Self :: Cache {{
                        {caches} ,
                        cplx ,
                    }} ,
                ))
            }}
            "##, 
            subvals_decl = mutator_field_names.iter().map(|mutator_field| {
                format!(r#"
                    let mut {m}_value : Option < _ > = None ;
                    let mut {m}_cache : Option < _ > = None ;
                    "#, m = mutator_field)
                }).collect::<Vec<_>>().join(""),

            nbr_fields = mutator_field_names.len(),

            body_match = mutator_field_names.iter().enumerate().map(|(idx, mutator_field)| {
                format!(
                    r#"
                    {i} => {{ 
                        let ( value , cache ) = self . {m} . random_arbitrary ( max_cplx - cplx ) ) ;
                        cplx += self . {m} . complexity ( & value , & cache ) ; 

                        {m}_value = Some ( value ) ;
                        {m}_cache = Some ( cache ) ;
                    }}
                    "#,
                        i = idx,
                        m = mutator_field
                    )
                }).collect::<Vec<_>>().join(""),

                values = fields_iter.clone().map(|(field, mutator_field)| {
                    format!("{f} : {m}_value . unwrap ( )", f = field, m = mutator_field)
                }).collect::<Vec<_>>().join(",") ,

                caches = mutator_field_names.iter().map(|mutator_field| {
                    format!("{m} : {m}_cache . unwrap ( )", m = mutator_field)
                }).collect::<Vec<_>>().join(",") ,
            ).as_str(),

            &r#"fn mutate ( & mut self , value : & mut Self :: Value , cache : & mut Self :: Cache , step : & mut Self :: MutationStep , max_cplx : f64 ) -> Self :: UnmutateToken
            {
                let orig_step = step . step ;
                step . step += 1 ;
                let current_cplx = self . complexity ( value , cache ) ;"#,
                &format!("match orig_step % {}", mutator_field_names.len()).as_str(),
                &"{",
                    &mutator_field_names.iter().zip(field_names.iter()).enumerate().map(|(i, (mutator_field, field))| {
                        format!(
                            r#"
                            {i} => {{
                                let current_field_cplx = self . {m} . complexity ( & value . {f} , & cache . {m} ) ;
                                let max_field_cplx = max_cplx - current_cplx - current_field_cplx ;
                                let token = self
                                    . {m}
                                    . mutate ( & mut  value . {f} , & mut cache . {m} , & mut step . {m} , max_field_cplx ) ;
                                let new_field_complexity = self . {m} . complexity ( & value . {f} , & cache . {m} ) ;
                                cache . cplx = cache . cplx - current_field_cplx + new_field_complexity ;
                                Self :: UnmutateToken {{
                                    {m} : Some ( token ) ,
                                    cplx : current_cplx ,
                                    .. Self :: UnmutateToken :: default ( )
                                }}
                            }}
                        "#,
                            i = i,
                            m = mutator_field,
                            f = field
                        )
                    }).collect::<Vec<_>>(),
                    &"_ => unreachable ! ( ) ",
                &"}",
            &"}",

            &r#"fn unmutate ( & self , value : & mut Self :: Value , cache : & mut Self :: Cache , t : Self :: UnmutateToken )
            {
                cache . cplx = t . cplx ;"#,
                &mutator_field_names.iter().zip(field_names.iter()).map(|(mutator_field, field)| {
                    format!(r#"
                        if let Some ( subtoken ) = t . {m} {{
                            self . {m} . unmutate ( & mut value . {f} , & mut cache . {m} , subtoken ) ;
                        }}"#,
                        f = field,
                        m = mutator_field
                    )
                }).collect::<Vec<_>>(),
            &"}",
        &"}"
    ]));
    /*

    {
        // implementation of Default trait when generic mutator params are Default
        let mut where_items = Vec::<WhereClauseItem>::new();
        for ty in generic_types_for_field.clone() {
            let where_item = WhereClauseItem {
                for_lifetimes: None,
                lhs: TokenTree::Ident(ty.clone()).into(),
                rhs: {
                    let mut tb = TokenBuilder::new();
                    tb.add(":: core :: default :: Default");
                    tb.end()
                },
            };
            where_items.push(where_item);
        }
        let where_clause = if let Some(mut where_clause) = mutator_struct.where_clause.clone() {
            where_clause.items.extend(where_items);
            where_clause
        } else {
            WhereClause { items: where_items }
        };

        let generics_stream = mutator_struct.generics.clone().to_token_stream();

        tb.add("impl");
        tb.stream(generics_stream.clone());
        tb.add(":: core :: default :: Default for");
        tb.extend(mutator_struct.ident.clone());
        tb.stream(generics_stream.clone());
        tb.stream(where_clause.to_token_stream());
        tb.push_group(Delimiter::Brace);

        tb.add("fn default ( ) -> Self");
        tb.push_group(Delimiter::Brace);
        tb.add("Self ");

        tb.push_group(Delimiter::Brace);
        for field in mutator_struct.struct_fields.iter() {
            tb.extend(field.identifier.clone().unwrap());
            tb.add(":");
            tb.add("<");
            tb.stream(field.ty.clone());
            tb.add(" as :: core :: default :: Default > :: default ( ) ,");
        }
        tb.pop_group(Delimiter::Brace);

        tb.pop_group(Delimiter::Brace);

        tb.pop_group(Delimiter::Brace);
    }

    {
        // implementation of HasDefaultMutator trait when generic mutator params are HasDefaultMutator
        let generics = value_generics_with_bounds_moved_to_where_clause; //parsed_struct.generics.clone().map(|g| g.removing_bounds_and_eq_type().0.to_token_stream());

        let mut where_items = Vec::<WhereClauseItem>::new();
        for field in parsed_struct.struct_fields.iter() {
            let where_item = WhereClauseItem {
                for_lifetimes: None,
                lhs: field.ty.clone(),
                rhs: {
                    let mut tb = TokenBuilder::new();
                    tb.add("fuzzcheck_mutators :: HasDefaultMutator");
                    tb.end()
                },
            };
            where_items.push(where_item);
            let where_item = WhereClauseItem {
                for_lifetimes: None,
                lhs: {
                    let mut tb = TokenBuilder::new();
                    tb.add("<");
                    tb.stream(field.ty.clone());
                    tb.add(" as fuzzcheck_mutators :: HasDefaultMutator > :: Mutator");
                    tb.end()
                },
                rhs: {
                    let mut tb = TokenBuilder::new();
                    tb.add(":: core :: default :: Default");
                    tb.end()
                },
            };
            where_items.push(where_item);
            let where_item = WhereClauseItem {
                for_lifetimes: None,
                lhs: field.ty.clone(),
                rhs: {
                    let mut tb = TokenBuilder::new();
                    tb.add(":: core :: clone :: Clone");
                    tb.end()
                },
            };
            where_items.push(where_item);
        }
        let where_item = WhereClauseItem {
            for_lifetimes: None,
            lhs: value_struct_ident_with_generic_params.clone(),
            rhs: {
                let mut tb = TokenBuilder::new();
                tb.add(":: core :: clone :: Clone");
                tb.end()
            },
        };
        where_items.push(where_item);

        let where_clause = if let Some(mut where_clause) = value_where_clause_with_added_items_from_generics.clone() {
            where_clause.items.extend(where_items);
            where_clause
        } else {
            WhereClause { items: where_items }
        };

        let generics_stream = generics.to_token_stream();

        tb.add("impl");
        tb.stream(generics_stream.clone());
        tb.add("fuzzcheck_mutators :: HasDefaultMutator for");
        tb.extend(parsed_struct.ident.clone());
        tb.stream(generics_stream.clone());
        tb.stream(where_clause.to_token_stream());
        tb.push_group(Delimiter::Brace);

        // associated type
        let generics_mutator = {
            let generics = parsed_struct.generics.removing_bounds_and_eq_type().0;
            let mut type_params = generics.type_params.clone();
            for field in parsed_struct.struct_fields.iter() {
                let mut ty_ident = TokenBuilder::new();
                ty_ident.add("<");
                ty_ident.stream(field.ty.clone());
                ty_ident.add(" as fuzzcheck_mutators :: HasDefaultMutator > :: Mutator");

                type_params.push(TypeParam {
                    attributes: Vec::new(),
                    type_ident: ty_ident.end(),
                    bounds: None,
                    equal_ty: None,
                });
            }
            Generics {
                lifetime_params: generics.lifetime_params.clone(),
                type_params,
            }
        };
        tb.add(&format!("type Mutator = {}", mutator_struct.ident));
        tb.stream(generics_mutator.to_token_stream());
        tb.add(";");

        tb.add(
            r#"fn default_mutator ( ) -> Self :: Mutator {
            Self :: Mutator :: default ( )
        }"#,
        );

        tb.pop_group(Delimiter::Brace);
        
    }*/
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

//                     tb.add(r#"XMutatorCache {
//                         inner ,
//                         cplx ,
//                     }"#);

//                     tb.pop_group(Delimiter::Brace);
//                 } else {
//                     tb.add("=>");
//                     tb.push_group(Delimiter::Brace);
//                     tb.add(&format!(r#"XMutatorCache {{
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

//                     tb.add(r#"XMutationStep {
//                         inner ,
//                         step ,
//                     }"#);

//                     tb.pop_group(Delimiter::Brace);
//                 } else {
//                     tb.add("=>");
//                     tb.push_group(Delimiter::Brace);
//                     tb.add(&format!(r#"XMutationStep {{
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

//                     tb.add(r#"XMutationStep {
//                         inner ,
//                         step ,
//                     }"#);

//                     tb.pop_group(Delimiter::Brace);
//                 } else {
//                     tb.add("=>");
//                     tb.push_group(Delimiter::Brace);
//                     tb.add(&format!(r#"XMutationStep {{
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
