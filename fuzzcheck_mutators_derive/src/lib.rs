#![allow(dead_code)]
mod macro_lib;
mod macro_lib_test;

//mod struct_derive;

use crate::macro_lib::*;
use proc_macro::{Delimiter, Ident, Span, TokenStream, TokenTree};

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

    let safe_field_names = fields
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

    // let mut mutator_field_types = vec![];
    // let mut mutator_cache_field_types = vec![];
    // let mut mutator_step_field_types = vec![];
    // let mut mutator_unmutate_token_field_types = vec![];

    // for ty in field_types.iter() {
    //     let mut tb_i = TokenBuilder::new();
    //     tb_i.punct("<")
    //         .stream(ty.clone())
    //         .add("as fuzzcheck_mutators :: HasDefaultMutator > :: Mutator");
    //     mutator_field_types.push(tb_i.end());

    //     let mut tb_i = TokenBuilder::new();
    //     tb_i.add("< <")
    //         .stream(ty.clone())
    //         .add("as fuzzcheck_mutators :: HasDefaultMutator > :: Mutator as fuzzcheck_traits :: Mutator > :: Cache");
    //     mutator_cache_field_types.push(tb_i.end());

    //     let mut tb_i = TokenBuilder::new();
    //     tb_i.add("< <").stream(ty.clone()).add(
    //         "as fuzzcheck_mutators :: HasDefaultMutator > :: Mutator as fuzzcheck_traits :: Mutator > :: MutationStep",
    //     );
    //     mutator_step_field_types.push(tb_i.end());

    //     let mut tb_i = TokenBuilder::new();
    //     tb_i.add("< <").stream(ty.clone()).add(
    //         "as fuzzcheck_mutators :: HasDefaultMutator > :: Mutator as fuzzcheck_traits :: Mutator > :: UnmutateToken",
    //     );
    //     mutator_unmutate_token_field_types.push(tb_i.end());
    // }

    /*
    generics: existing generics without the bounds + generic mutator type params
    where_clause_items: existing where_clause + existing generic bounds + “Mutator” conditions
    mutator_field_types: the generic types for the sub-mutators, one for each field
    */
    let (generics, where_clause_items, mutator_field_types) = { 
        /*
        1. remove the bounds from the existing generics
        2. put those bounds on new where_clause_items
        */
        let (mut generics, mut where_clause_items) = if let Some(generics) = &parsed_struct.generics {
            let (generics, bounds) = generics.removing_bounds_and_eq_type();
            let where_clause_items = 
                generics.lifetime_params.iter().map(|lp| lp.ident.clone())
                .chain(generics.type_params.iter().map(|tp| TokenTree::Ident(tp.ident.clone()).into()))
                .zip(bounds.iter())
                .filter_map(|(lhs, rhs)| {
                    if let Some(rhs) = rhs {
                        Some((lhs, rhs))
                    } else {
                        None
                    }
                })
                .map(|(lhs, rhs)| {
                    WhereClauseItem {
                        for_lifetimes: None,
                        lhs,
                        rhs: rhs.clone(), 
                    }
                })
                .collect();

            (generics, where_clause_items)
        } else {
            (Generics { lifetime_params: Vec::new(), type_params: Vec::new() }, Vec::new())
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
        let mut mutator_field_types = Vec::new();
        for (field, field_name) in parsed_struct.struct_fields.iter().zip(safe_field_names.iter()) {
            let field_mutator_type_ident = Ident::new(&format!("{}Mutator", field_name), Span::call_site());
            mutator_field_types.push(field_mutator_type_ident.clone());
            let ty_param = TypeParam {
                attributes: Vec::new(),
                ident: field_mutator_type_ident.clone(),
                bounds: None,
                equal_ty: None,
            };
            generics.type_params.push(ty_param);
            where_clause_items.push(WhereClauseItem {
                for_lifetimes: None,
                lhs: TokenTree::Ident(field_mutator_type_ident).into(),
                rhs: {
                    let mut tb = TokenBuilder::new();
                    tb.add("fuzzcheck_traits :: Mutator < Value = ")
                        .stream(field.ty.clone())
                        .punct(">");
                    tb.end()
                },
            });
        }
        (generics, where_clause_items, mutator_field_types)
    };

    let where_clause = {
        WhereClause {
            items: where_clause_items, 
        }
    };

    let mutator_struct_fields = {
        let mut r = safe_field_names.iter()
        .zip(mutator_field_types.iter())
        .map(|(name, ty)| 
            StructField {
                attributes: Vec::new(),
                visibility: None,
                identifier: Some(Ident::new(name, Span::call_site())),
                ty: TokenTree::Ident(ty.clone()).into(),
            }
        ).collect::<Vec<_>>();
        r.push(
            StructField {
                attributes: Vec::new(),
                visibility: None,
                identifier: Some(Ident::new("rng", Span::call_site())),
                ty: { 
                    let mut tb = TokenBuilder::new();
                    tb.add("fuzzcheck_mutators :: fastrand :: Rng");
                    tb.end()
                },
                
            }
        );
        r
    };

    // main mutator struct
    let mutator_struct = Struct {
        visibility: parsed_struct.visibility.clone(),
        ident: Ident::new(&format!("{}Mutator", parsed_struct.ident), Span::call_site()),
        generics: Some(generics),
        kind: StructKind::Struct,
        where_clause: Some(where_clause),
        struct_fields: mutator_struct_fields,
    };

    tb.stream(mutator_struct.to_token_stream());
    // let name_mutator = format!("{}Mutator", parsed_struct.ident);
    // add_struct(&name_mutator, &mutator_field_types, {
    //     let mut tb = TokenBuilder::new();
    //     tb.add("rng : fuzzcheck_mutators :: fastrand :: Rng");
    //     tb.end()
    // });

    // let name_mutator_cache = format!("{}MutatorCache", parsed_struct.ident);
    // add_struct(&name_mutator_cache, &mutator_cache_field_types, {
    //     let mut tb = TokenBuilder::new();
    //     tb.add("cplx : f64 ,");
    //     tb.end()
    // });

    // let name_mutator_step = format!("{}MutatorStep", parsed_struct.ident);
    // add_struct(&name_mutator_step, &mutator_step_field_types, {
    //     let mut tb = TokenBuilder::new();
    //     tb.add("step : usize ,");
    //     tb.end()
    // });

    // let name_unmutate_token = format!("{}UnmutateToken", parsed_struct.ident);
    // add_struct(
    //     &name_unmutate_token,
    //     &mutator_unmutate_token_field_types,
    //     TokenStream::new(),
    // );

    // {
    //     // implementation of Default for Mutator
    //     tb.ident("impl");
    //     tb.stream_opt(generic_params.clone());
    //     tb.add("core :: default :: Default for");
    //     tb.ident(&name_mutator);
    //     tb.stream_opt(generic_params.clone());
    //     todo!();
    //     // tb.stream_opt(parsed_struct.data.where_clause.clone());
    //     tb.add("{");
    //     tb.add("fn default ( ) -> Self {");
    //     tb.add("Self {");

    //     for (field, ty) in safe_field_names.iter().zip(mutator_field_types.iter()) {
    //         tb.add(&format!(
    //             "{} : < {} as  core :: default :: Default > :: default ( ) ,",
    //             field, ty
    //         ));
    //     }
    //     tb.add("rng : fuzzcheck_mutators :: fastrand :: Rng :: new ( )");

    //     tb.add("} } }");
    // }

    // {
    //     // implementation of Mutator trait
    //     tb.ident("impl");
    //     tb.stream_opt(generic_params.clone());
    //     tb.add("fuzzcheck_traits :: Mutator for");
    //     tb.ident(&name_mutator);
    //     tb.stream_opt(generic_params.clone());
    //     //tb.stream_opt(parsed_struct.data.where_clause.clone());
    //     if let Some(generics) = &parsed_struct.generics {
    //         if !generics.type_params.is_empty() {
    //             let mut condition = TokenBuilder::new();
    //             for type_param in generics.type_params.iter() {
    //                 if let Some(bounds) = &type_param.bounds {
    //                     condition.extend_ident(type_param.ident.clone());
    //                     condition.punct(":");
    //                     condition.stream(bounds.clone());
    //                     condition.punct(",");
    //                 }
    //                 condition.extend_ident(type_param.ident.clone());
    //                 condition.add(": fuzzcheck_traits :: Mutator , ");
    //             }
    //             todo!();
    //             // let where_clause = add_condition_to_where_clause(parsed_struct.data.where_clause.clone(), condition.end());
    //             // tb.stream(where_clause);
    //         }
    //     }
    //     tb.add("{ }");
    // }
}

// fn add_condition_to_where_clause(where_clause: Option<TokenStream>, condition: TokenStream) -> TokenStream {
//     let mut tb = TokenBuilder::new();
//     if let Some(where_clause) = where_clause {
//         tb.stream(where_clause);
//         tb.add(", ");
//         tb.stream(condition);
//         tb.end()
//     } else if !condition.is_empty() {
//         tb.ident("where");
//         tb.stream(condition);
//         tb.end()
//     } else {
//         TokenStream::new()
//     }
// } 