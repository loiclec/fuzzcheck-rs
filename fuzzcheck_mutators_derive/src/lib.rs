#![allow(dead_code)]
mod macro_lib;
mod macro_lib_test;

//mod struct_derive;

use crate::macro_lib::*;
use proc_macro::{Delimiter, Ident, Literal, Span, TokenStream, TokenTree};

#[proc_macro_derive(HasDefaultMutator)]
pub fn derive_mutator(input: TokenStream) -> TokenStream {
    let mut parser = TokenParser::new(input);
    let mut tb = TokenBuilder::new();
    //tb.add("mod X { ");
    if let Some(s) = parser.eat_struct() {
        derive_struct_mutator(s, &mut tb);
    } else if let Some(_) = parser.eat_enumeration() {
        //tb.stream(e.whole);
    }
    // tb.add("}");
    tb.eprint();

    tb.end()
}

fn derive_struct_mutator(parsed_struct: Struct, tb: &mut TokenBuilder) {
    if !parsed_struct.struct_fields.is_empty() {
        derive_struct_mutator_with_fields(&parsed_struct, tb)
    } else {
        todo!("Build mutator for empty struct");
    }
}

fn derive_struct_mutator_with_fields(parsed_struct: &Struct, tb: &mut TokenBuilder) {
    let fields = &parsed_struct.struct_fields;
    // let field_types = fields.iter().map(|f| f.ty.clone()).collect::<Vec<_>>();

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

    let field_names = parsed_struct.struct_fields.iter().enumerate().map(|(i, f)| {
            if let Some(ident) = &f.identifier {
                ident.to_string()
            } else {
                format!("{}", i)
            }
        }).collect::<Vec<_>>();

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
            visibility: None,
            identifier: Some(identifier.clone()),
            ty: TokenTree::Ident(ty.clone()).into(),
        })
        .collect::<Vec<_>>();

    let mutator_struct = {
        /*
        generics: existing generics without the bounds + generic mutator type params
        where_clause_items: existing where_clause + existing generic bounds + “Mutator” conditions
        mutator_field_types: the generic types for the sub-mutators, one for each field
        */
        /*
        1. remove the bounds from the existing generics
        2. put those bounds on new where_clause_items
        */
        let (mut generics, mut where_clause_items) = if let Some(generics) = &parsed_struct.generics {
            let (generics, bounds) = generics.removing_bounds_and_eq_type();
            let where_clause_items = generics
                .lifetime_params
                .iter()
                .map(|lp| lp.ident.clone())
                .chain(
                    generics
                        .type_params
                        .iter()
                        .map(|tp| tp.type_ident.clone()),
                )
                .zip(bounds.iter())
                .filter_map(|(lhs, rhs)| if let Some(rhs) = rhs { Some((lhs, rhs)) } else { None })
                .map(|(lhs, rhs)| WhereClauseItem {
                    for_lifetimes: None,
                    lhs,
                    rhs: rhs.clone(),
                })
                .collect();

            (generics, where_clause_items)
        } else {
            (
                Generics {
                    lifetime_params: Vec::new(),
                    type_params: Vec::new(),
                },
                Vec::new(),
            )
        };
        /*
        3. extend the existing where_clause_items with those found in 2.
        */
        if let Some(where_clause) = &parsed_struct.where_clause {
            where_clause_items.extend(where_clause.items.iter().cloned());
        }
        /*
        4. for each field, add a generic parameter for its mutator as well as a where_clause_item
           ensuring it impls the Mutator trait
        */
        for (field, generic_ty_for_field) in parsed_struct.struct_fields.iter().zip(generic_types_for_field) {
            let ty_param = TypeParam {
                attributes: Vec::new(),
                type_ident: TokenTree::Ident(generic_ty_for_field.clone()).into(),
                bounds: None,
                equal_ty: None,
            };
            generics.type_params.push(ty_param);
            where_clause_items.push(WhereClauseItem {
                for_lifetimes: None,
                lhs: TokenTree::Ident(generic_ty_for_field.clone()).into(),
                rhs: {
                    let mut tb = TokenBuilder::new();
                    tb.add("fuzzcheck_mutators :: fuzzcheck_traits :: Mutator < Value = ")
                        .stream(field.ty.clone())
                        .punct(">");
                    tb.end()
                },
            });
        }

        let main_mutator_where_clause = {
            WhereClause {
                items: where_clause_items,
            }
        };

        let mut mutator_struct_fields = basic_fields.clone();
        mutator_struct_fields.push(StructField {
            attributes: Vec::new(),
            visibility: None,
            identifier: Some(Ident::new("rng", Span::call_site())),
            ty: {
                let mut tb = TokenBuilder::new();
                tb.add("fuzzcheck_mutators :: fastrand :: Rng");
                tb.end()
            },
        });

        Struct {
            visibility: parsed_struct.visibility.clone(),
            ident: Ident::new(&format!("{}Mutator", parsed_struct.ident), Span::call_site()),
            generics: Some(generics),
            kind: StructKind::Struct,
            where_clause: Some(main_mutator_where_clause),
            struct_fields: mutator_struct_fields,
        }
    };

    tb.stream(mutator_struct.clone().to_token_stream());

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
            generics: Some(basic_generics.clone()),
            kind: StructKind::Struct,
            where_clause: None,
            struct_fields: cache_fields.clone(),
        }
    };

    tb.stream(mutator_cache_struct.clone().to_token_stream());

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
            generics: Some(basic_generics.clone()),
            kind: StructKind::Struct,
            where_clause: None,
            struct_fields: step_fields,
        }
    };

    tb.stream(mutator_step_struct.clone().to_token_stream());

    let unmutate_token_struct = {
        let step_fields = basic_fields
            .iter()
            .map(|field| {
                let mut tb = TokenBuilder::new();
                tb.add(":: std :: option :: Option <").stream(field.ty.clone()).add(">");
                StructField {
                    ty: tb.end(),
                    ..field.clone()
                }
            })
            .collect();
        Struct {
            visibility: parsed_struct.visibility.clone(),
            ident: Ident::new(&format!("{}UnmutateToken", parsed_struct.ident), Span::call_site()),
            generics: Some(basic_generics),
            kind: StructKind::Struct,
            where_clause: None,
            struct_fields: step_fields,
        }
    };

    tb.stream(unmutate_token_struct.clone().to_token_stream());

    {
        // implementation of Mutator trait

        let generics = mutator_struct.generics.clone().map(|g| g.to_token_stream());
        tb.add("impl")
            .stream_opt(generics.clone())
            .add("fuzzcheck_mutators :: fuzzcheck_traits :: Mutator for")
            .extend_ident(mutator_struct.ident.clone())
            .stream_opt(generics)
            .stream_opt(mutator_struct.where_clause.clone().map(|wc| wc.to_token_stream()))
            .push_group(Delimiter::Brace);

            { // associated types
                tb.add("type Value = ");
                let value_struct_ident_with_generic_params = {
                    let mut tb = TokenBuilder::new();
                    tb.extend_ident(parsed_struct.ident.clone());
                    let generic_args = parsed_struct.generics.clone().map(|g| {
                        let (g, _) = g.removing_bounds_and_eq_type();
                        g.to_token_stream()
                    });
                    tb.stream_opt(generic_args);
                    tb.end()
                };
                tb.stream(value_struct_ident_with_generic_params);
                tb.add(";");

                tb.add("type Cache = ");
                let cache_struct_ident_with_generic_params = {
                    let mut tb = TokenBuilder::new();
                    tb.extend_ident(mutator_cache_struct.ident.clone());
                    let generic_args = mutator_cache_struct.generics.map(|g| {
                        let mut g = g.clone();
                        for tp in g.type_params.iter_mut() {
                            let mut tb = TokenBuilder::new();
                            tb.add("<");
                            tb.stream(tp.type_ident.clone());
                            tb.add(" as fuzzcheck_mutators :: fuzzcheck_traits :: Mutator > :: Cache");
                            tp.type_ident = tb.end();
                        }
                        g.to_token_stream()
                    });
                    tb.stream_opt(generic_args);
                    tb.end()
                };
                tb.stream(cache_struct_ident_with_generic_params);
                tb.add(";");

                tb.add("type MutationStep = ");
                let mutator_step_struct_ident_with_generic_params = {
                    let mut tb = TokenBuilder::new();
                    tb.extend_ident(mutator_step_struct.ident.clone());
                    let generic_args = mutator_step_struct.generics.map(|g| {
                        let mut g = g.clone();
                        for tp in g.type_params.iter_mut() {
                            let mut tb = TokenBuilder::new();
                            tb.add("<");
                            tb.stream(tp.type_ident.clone());
                            tb.add(" as fuzzcheck_mutators :: fuzzcheck_traits :: Mutator > :: MutationStep");
                            tp.type_ident = tb.end();
                        }
                        g.to_token_stream()
                    });
                    tb.stream_opt(generic_args);
                    tb.end()
                };
                tb.stream(mutator_step_struct_ident_with_generic_params);
                tb.add(";");

                tb.add("type UnmutateToken = ");
                let unmutate_token_struct_ident_with_generic_params = {
                    let mut tb = TokenBuilder::new();
                    tb.extend_ident(unmutate_token_struct.ident.clone());
                    let generic_args = unmutate_token_struct.generics.map(|g| {
                        let mut g = g.clone();
                        for tp in g.type_params.iter_mut() {
                            let mut tb = TokenBuilder::new();
                            tb.add("<");
                            tb.stream(tp.type_ident.clone());
                            tb.add(" as fuzzcheck_mutators :: fuzzcheck_traits :: Mutator > :: UnmutateToken");
                            tp.type_ident = tb.end();
                        }
                        g.to_token_stream()
                    });
                    tb.stream_opt(generic_args);
                    tb.end()
                };
                tb.stream(unmutate_token_struct_ident_with_generic_params);
                tb.add(";");
            }

            { // max_complexity
                tb.add("fn max_complexity ( & self ) -> f64 { ");
                let mut mutator_field_names_iter = mutator_field_names.iter();
                if let Some(fst_field_name) = mutator_field_names_iter.next() {
                    tb.add(&format!("self . {} . max_complexity ( ) ", fst_field_name));
                }
                for field_name in mutator_field_names_iter {
                    tb.add(&format!("+ self . {} . max_complexity ( ) ", field_name));
                }

                tb.add("}");
            }
            { // min_complexity
                tb.add("fn min_complexity ( & self ) -> f64 { ");
                let mut mutator_field_names_iter = mutator_field_names.iter();
                if let Some(fst_field_name) = mutator_field_names_iter.next() {
                    tb.add(&format!("self . {} . min_complexity ( ) ", fst_field_name));
                }
                for field_name in mutator_field_names_iter {
                    tb.add(&format!("+ self . {} . min_complexity ( ) ", field_name));
                }

                tb.add("}");
            }
            // { // complexity
            //     tb.add("fn complexity ( & self , value : & Self :: Value , cache : & Self :: Cache ) -> f64 { ");
            //     let mutator_field_names_iter = safe_field_names.iter();
            //     let struct_field_names_iter = parsed_struct.struct_fields.iter().enumerate().map(|(i, f)| {
            //         if let Some(ident) = &f.identifier {
            //             ident.to_string()
            //         } else {
            //             format!("{}", i)
            //         }
            //     });
            //     let mut field_names_iter = mutator_field_names_iter.zip(struct_field_names_iter);
            //     if let Some((fst_mutator_field_name, fst_struct_field_name)) = field_names_iter.next() {
            //         // TODO: know real struct field names
            //         tb.add(&format!("self . {m} . complexity ( & value . {s} , & cache . {m} ) ", m=fst_mutator_field_name, s=fst_struct_field_name));
            //     }
            //     for (fst_mutator_field_name, fst_struct_field_name) in field_names_iter {
            //         tb.add(&format!("+ self . {m} . complexity ( & value . {s} , & cache . {m} ) ", m=fst_mutator_field_name, s=fst_struct_field_name));
            //     }

            //     tb.add("}");
            // }
            { // complexity, cached
                tb.add("fn complexity ( & self , value : & Self :: Value , cache : & Self :: Cache ) -> f64 { cache . cplx }");
            }

            { // cache_from_value
                tb.add("fn cache_from_value ( & self , value : & Self :: Value ) -> Self :: Cache {");
                let fields_iter = field_names.iter().zip(mutator_field_names.iter());
                for (field, mutator_field) in fields_iter.clone() {
                    tb.add(&format!("let {m} = self . {m} . cache_from_value ( & value . {f} ) ; ", f=field, m=mutator_field));
                }
                let mut fields_iter = fields_iter;
                tb.add("let cplx = ");
                if let Some((field, mutator_field)) = fields_iter.next() {
                    tb.add(&format!("self . {m} . complexity ( & value . {f} , & {m} ) ", f=field, m=mutator_field));
                }
                for (field, mutator_field) in fields_iter {
                    tb.add(&format!("+ self . {m} . complexity ( & value . {f} , & {m} ) ", f=field, m=mutator_field));
                }
                tb.add(";");
            
                tb.add("Self :: Cache {");
                for mutator_field in mutator_field_names.iter() {
                    tb.add(mutator_field);
                    tb.add(",");
                }
                tb.add("cplx , } }");
            }

            { // mutation_step_from_value
                tb.add("fn mutation_step_from_value ( & self , value : & Self :: Value  ) -> Self :: MutationStep { ");

                let fields_iter = field_names.iter().zip(mutator_field_names.iter());
                for (field, mutator_field) in fields_iter.clone() {
                    tb.add(&format!("let {m} = self . {m} . mutation_step_from_value ( & value . {f} ) ; ", f=field, m=mutator_field));
                }
                tb.add(";");
            
                tb.add("let step = 0 ;");

                tb.add("Self :: MutationStep {");
                for mutator_field in mutator_field_names.iter() {
                    tb.add(mutator_field);
                    tb.add(",");
                }
                tb.add("step , }");

                tb.add("}");
            }

            { // arbitrary
                tb.add("fn arbitrary ( & mut self , seed : usize , max_cplx : f64 ) -> ( Self :: Value , Self :: Cache )");
                tb.push_group(Delimiter::Brace);

                // create option value for all fields
                for mutator_field in mutator_field_names.iter() {
                    tb.add(&format!("let mut {m}_value : Option < _ > = None ;", m=mutator_field));
                    tb.add(&format!("let mut {m}_cache : Option < _ > = None ;", m=mutator_field));
                }
                // create array of numbers, then shuffle it
                tb.add(&format!("let mut indices = ( 0 .. {} ) . iter ( ) . collect :: < Vec < _ > > ( ) ;", mutator_field_names.len()));
                tb.add(" fuzzcheck_mutators :: fastrand :: shuffle ( & mut indices ) ;");
                tb.add("let seed = fuzzcheck_mutators :: fastrand :: usize ( .. ) ;");

                tb.add("let mut cplx = ");
                tb.extend(TokenTree::Literal(Literal::f64_suffixed(0.0)));
                tb.add(";");

                tb.add("for idx in indices . iter ( )");
                tb.push_group(Delimiter::Brace);

                tb.add("match idx");
                tb.push_group(Delimiter::Brace);
                for (idx, mutator_field) in mutator_field_names.iter().enumerate() {
                    tb.add(&format!(r#"
                    {i} => {{ 
                        let ( value , cache ) = self . {m} . arbitrary ( seed , max_cplx - cplx ) ; 
                        
                        cplx += self . {m} . complexity ( & value , & cache ) ; 

                        {m}_value = Some ( value ) ;
                        {m}_cache = Some ( cache ) ;
                    }} ,
                    "#, i=idx, m=mutator_field));
                }
                tb.pop_group(Delimiter::Brace);
                tb.pop_group(Delimiter::Brace);

                tb.push_group(Delimiter::Parenthesis);

                tb.add("Self :: Value ");
                tb.push_group(Delimiter::Brace);
                for (field, mutator_field) in field_names.iter().zip(mutator_field_names.iter()) {
                    tb.add(&format!("{f} : {m}_value . unwrap ( ) ,", f=field, m=mutator_field));
                }
                tb.pop_group(Delimiter::Brace);
                tb.add(",");
                tb.add("Self :: Cache ");
                tb.push_group(Delimiter::Brace);
                for mutator_field in mutator_field_names.iter() {
                    tb.add(&format!("{m} : {m}_cache . unwrap ( ) ,", m=mutator_field));
                }
                tb.add("cplx ,");
                tb.pop_group(Delimiter::Brace);
                tb.pop_group(Delimiter::Parenthesis);

                tb.pop_group(Delimiter::Brace);
            }

            tb.pop_group(Delimiter::Brace);
    }
}
