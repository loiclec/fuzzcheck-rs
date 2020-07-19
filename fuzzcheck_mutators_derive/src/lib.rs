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

    if let Some(s) = parser.eat_struct() {
        derive_struct_mutator(s, &mut tb);
    } else if let Some(_) = parser.eat_enumeration() {
        tb.add("compile_error ! (")
            .string("fuzzcheck_mutators_derive cannot derive mutators for enumerations")
            .add(") ;");
    //tb.stream(e.whole);
    } else {
        tb.add("compile_error ! (")
            .string("fuzzcheck_mutators_derive could not parse the structure")
            .add(") ;");
    }

    // tb.eprint();

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

    let value_struct_ident_with_generic_params = {
        let mut tb = TokenBuilder::new();
        tb.extend_ident(parsed_struct.ident.clone());
        let generic_args = parsed_struct
            .generics
            .clone()
            .removing_bounds_and_eq_type()
            .0
            .to_token_stream();
        tb.stream(generic_args);
        tb.end()
    };

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
                    tb.add("fuzzcheck_mutators :: fuzzcheck_traits :: Mutator < Value = ")
                        .stream(field.ty.clone())
                        .punct(">");
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
            visibility: None,
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
            generics: basic_generics.clone(),
            kind: StructKind::Struct,
            where_clause: None,
            struct_fields: cache_fields.clone(),
        }
    };

    tb.add("# [ derive ( :: core :: clone :: Clone ) ]");
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
            generics: basic_generics.clone(),
            kind: StructKind::Struct,
            where_clause: None,
            struct_fields: step_fields,
        }
    };

    tb.add("# [ derive ( :: core :: clone :: Clone ) ]");
    tb.stream(mutator_step_struct.clone().to_token_stream());

    let unmutate_token_struct = {
        let mut step_fields = basic_fields
            .iter()
            .map(|field| {
                let mut tb = TokenBuilder::new();
                tb.add(":: std :: option :: Option <").stream(field.ty.clone()).add(">");
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
    tb.stream(unmutate_token_struct.clone().to_token_stream());

    {
        // default impl for unmutate token
        let unmutate_fields_default = {
            let mut s = String::new();
            for mutator_field in mutator_field_names.iter() {
                s.push_str(&format!(" {m} : None , ", m = mutator_field));
            }
            s.push_str("cplx : f64 :: default ( ) ,");
            s
        };

        let generics = unmutate_token_struct.generics.clone().to_token_stream();

        tb.add("impl");
        tb.stream(generics.clone());
        tb.add(" :: core :: default :: Default for ");
        tb.extend_ident(unmutate_token_struct.ident.clone());
        tb.stream(generics);
        tb.push_group(Delimiter::Brace);
        tb.add(&format!(
            r#"
            fn default ( ) -> Self {{
                Self {{
                    {}
                }}
            }}
        "#,
            unmutate_fields_default
        ));
        tb.pop_group(Delimiter::Brace);
    }

    {
        // implementation of Mutator trait
        let generics = mutator_struct.generics.clone().to_token_stream();
        tb.add("impl")
            .stream(generics.clone())
            .add("fuzzcheck_mutators :: fuzzcheck_traits :: Mutator for")
            .extend_ident(mutator_struct.ident.clone())
            .stream(generics)
            .stream_opt(mutator_struct.where_clause.clone().map(|wc| wc.to_token_stream()))
            .push_group(Delimiter::Brace);

        {
            // associated types
            tb.add("type Value = ");
            tb.stream(value_struct_ident_with_generic_params.clone());
            tb.add(";");

            tb.add("type Cache = ");
            let cache_struct_ident_with_generic_params = {
                let mut tb = TokenBuilder::new();
                tb.extend_ident(mutator_cache_struct.ident.clone());
                let generic_args = {
                    let mut g = mutator_cache_struct.generics.clone();
                    for tp in g.type_params.iter_mut() {
                        let mut tb = TokenBuilder::new();
                        tb.add("<");
                        tb.stream(tp.type_ident.clone());
                        tb.add(" as fuzzcheck_mutators :: fuzzcheck_traits :: Mutator > :: Cache");
                        tp.type_ident = tb.end();
                    }
                    g.to_token_stream()
                };
                tb.stream(generic_args);
                tb.end()
            };
            tb.stream(cache_struct_ident_with_generic_params);
            tb.add(";");

            tb.add("type MutationStep = ");
            let mutator_step_struct_ident_with_generic_params = {
                let mut tb = TokenBuilder::new();
                tb.extend_ident(mutator_step_struct.ident.clone());
                let generic_args = {
                    let mut g = mutator_step_struct.generics.clone();
                    for tp in g.type_params.iter_mut() {
                        let mut tb = TokenBuilder::new();
                        tb.add("<");
                        tb.stream(tp.type_ident.clone());
                        tb.add(" as fuzzcheck_mutators :: fuzzcheck_traits :: Mutator > :: MutationStep");
                        tp.type_ident = tb.end();
                    }
                    g.to_token_stream()
                };
                tb.stream(generic_args);
                tb.end()
            };
            tb.stream(mutator_step_struct_ident_with_generic_params);
            tb.add(";");

            tb.add("type UnmutateToken = ");
            let unmutate_token_struct_ident_with_generic_params = {
                let mut tb = TokenBuilder::new();
                tb.extend_ident(unmutate_token_struct.ident.clone());
                let generic_args = {
                    let mut g = unmutate_token_struct.generics.clone();
                    for tp in g.type_params.iter_mut() {
                        let mut tb = TokenBuilder::new();
                        tb.add("<");
                        tb.stream(tp.type_ident.clone());
                        tb.add(" as fuzzcheck_mutators :: fuzzcheck_traits :: Mutator > :: UnmutateToken");
                        tp.type_ident = tb.end();
                    }
                    g.to_token_stream()
                };
                tb.stream(generic_args);
                tb.end()
            };
            tb.stream(unmutate_token_struct_ident_with_generic_params);
            tb.add(";");
        }

        {
            // max_complexity
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

        {
            // min_complexity
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

        {
            // complexity
            tb.add(
                "fn complexity ( & self , value : & Self :: Value , cache : & Self :: Cache ) -> f64 { cache . cplx }",
            );
        }

        {
            // cache_from_value
            tb.add("fn cache_from_value ( & self , value : & Self :: Value ) -> Self :: Cache {");
            let fields_iter = field_names.iter().zip(mutator_field_names.iter());
            for (field, mutator_field) in fields_iter.clone() {
                tb.add(&format!(
                    "let {m} = self . {m} . cache_from_value ( & value . {f} ) ; ",
                    f = field,
                    m = mutator_field
                ));
            }
            let mut fields_iter = fields_iter;
            tb.add("let cplx = ");
            if let Some((field, mutator_field)) = fields_iter.next() {
                tb.add(&format!(
                    "self . {m} . complexity ( & value . {f} , & {m} ) ",
                    f = field,
                    m = mutator_field
                ));
            }
            for (field, mutator_field) in fields_iter {
                tb.add(&format!(
                    "+ self . {m} . complexity ( & value . {f} , & {m} ) ",
                    f = field,
                    m = mutator_field
                ));
            }
            tb.add(";");

            tb.add("Self :: Cache {");
            for mutator_field in mutator_field_names.iter() {
                tb.add(mutator_field);
                tb.add(",");
            }
            tb.add("cplx , } }");
        }

        {
            // mutation_step_from_value
            tb.add("fn mutation_step_from_value ( & self , value : & Self :: Value  ) -> Self :: MutationStep { ");

            let fields_iter = field_names.iter().zip(mutator_field_names.iter());
            for (field, mutator_field) in fields_iter.clone() {
                tb.add(&format!(
                    "let {m} = self . {m} . mutation_step_from_value ( & value . {f} ) ; ",
                    f = field,
                    m = mutator_field
                ));
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

        {
            // arbitrary
            tb.add("fn arbitrary ( & mut self , seed : usize , max_cplx : f64 ) -> ( Self :: Value , Self :: Cache )");
            tb.push_group(Delimiter::Brace);

            // create option value for all fields
            for mutator_field in mutator_field_names.iter() {
                tb.add(&format!("let mut {m}_value : Option < _ > = None ;", m = mutator_field));
                tb.add(&format!("let mut {m}_cache : Option < _ > = None ;", m = mutator_field));
            }
            // create array of numbers, then shuffle it
            tb.add(&format!(
                "let mut indices = ( 0 .. {} ) . collect :: < Vec < _ > > ( ) ;",
                mutator_field_names.len()
            ));
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
                tb.add(&format!(
                    r#"
                {i} => {{ 
                    let ( value , cache ) = self . {m} . arbitrary ( seed , max_cplx - cplx ) ; 
                    
                    cplx += self . {m} . complexity ( & value , & cache ) ; 

                    {m}_value = Some ( value ) ;
                    {m}_cache = Some ( cache ) ;
                }} ,
                "#,
                    i = idx,
                    m = mutator_field
                ));
            }
            tb.add("_ => unreachable ! ( ) ");
            tb.pop_group(Delimiter::Brace);
            tb.pop_group(Delimiter::Brace);

            tb.push_group(Delimiter::Parenthesis);

            tb.add("Self :: Value ");
            tb.push_group(Delimiter::Brace);
            for (field, mutator_field) in field_names.iter().zip(mutator_field_names.iter()) {
                tb.add(&format!("{f} : {m}_value . unwrap ( ) ,", f = field, m = mutator_field));
            }
            tb.pop_group(Delimiter::Brace);
            tb.add(",");
            tb.add("Self :: Cache ");
            tb.push_group(Delimiter::Brace);
            for mutator_field in mutator_field_names.iter() {
                tb.add(&format!("{m} : {m}_cache . unwrap ( ) ,", m = mutator_field));
            }
            tb.add("cplx ,");
            tb.pop_group(Delimiter::Brace);
            tb.pop_group(Delimiter::Parenthesis);

            tb.pop_group(Delimiter::Brace);
        }

        {
            // mutate
            tb.add("fn mutate ( & mut self , value : & mut Self :: Value , cache : & mut Self :: Cache , step : & mut Self :: MutationStep , max_cplx : f64 ) -> Self :: UnmutateToken ");
            tb.push_group(Delimiter::Brace);
            tb.add("let orig_step = step . step ;");
            tb.add("step . step += 1 ; ");
            tb.add("let current_cplx = self . complexity ( value , cache ) ;");

            tb.add(&format!("match orig_step % {}", mutator_field_names.len()));
            tb.push_group(Delimiter::Brace);
            for (i, (mutator_field, field)) in mutator_field_names.iter().zip(field_names.iter()).enumerate() {
                tb.add(&format!(
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
                ));
            }
            tb.add("_ => unreachable ! ( ) ");
            tb.pop_group(Delimiter::Brace);

            tb.pop_group(Delimiter::Brace);
        }

        {
            // unmutate
            tb.add("fn unmutate ( & self , value : & mut Self :: Value , cache : & mut Self :: Cache , t : Self :: UnmutateToken ) ");
            tb.push_group(Delimiter::Brace);
            tb.add("cache . cplx = t . cplx ;");
            for (mutator_field, field) in mutator_field_names.iter().zip(field_names.iter()) {
                tb.add(&format!(
                    r#"
                    if let Some ( subtoken ) = t . {m} {{
                        self . {m} . unmutate ( & mut value . {f} , & mut cache . {m} , subtoken ) ;
                    }}
                "#,
                    f = field,
                    m = mutator_field
                ));
            }

            tb.pop_group(Delimiter::Brace);
        }

        tb.pop_group(Delimiter::Brace);
    }

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

        tb.add("impl")
            .stream(generics_stream.clone())
            .add(":: core :: default :: Default for")
            .extend_ident(mutator_struct.ident.clone())
            .stream(generics_stream.clone())
            .stream(where_clause.to_token_stream())
            .push_group(Delimiter::Brace);

        tb.add("fn default ( ) -> Self");
        tb.push_group(Delimiter::Brace);
        tb.add("Self ");

        tb.push_group(Delimiter::Brace);
        for field in mutator_struct.struct_fields.iter() {
            tb.extend_ident(field.identifier.clone().unwrap());
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

        tb.add("impl")
            .stream(generics_stream.clone())
            .add("fuzzcheck_mutators :: HasDefaultMutator for")
            .extend_ident(parsed_struct.ident.clone())
            .stream(generics_stream.clone())
            .stream(where_clause.to_token_stream())
            .push_group(Delimiter::Brace);

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
    }
}
