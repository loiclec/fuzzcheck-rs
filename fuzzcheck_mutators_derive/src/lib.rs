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

    let safe_field_idents = safe_field_names.iter().map(|name| {
        Ident::new(name, Span::call_site())
    }).collect::<Vec<_>>();

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

    let generic_types_for_field = safe_field_names.iter().map(|name| { Ident::new(&format!("{}Type", name), Span::call_site()) });

    let basic_generics = Generics {
        lifetime_params: Vec::new(),
        type_params: generic_types_for_field.clone().map(|ident| {
            TypeParam {
                attributes: Vec::new(),
                ident,
                bounds: None,
                equal_ty: None,
                
            }
        }).collect(),
    };

    let basic_fields = safe_field_idents.iter()
        .zip(generic_types_for_field.clone())
        .map(|(identifier, ty)| 
            StructField {
                attributes: Vec::new(),
                visibility: None,
                identifier: Some(identifier.clone()),
                ty: TokenTree::Ident(ty.clone()).into(),
            }
        ).collect::<Vec<_>>();

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
        for (field, generic_ty_for_field) in parsed_struct.struct_fields.iter().zip(generic_types_for_field) {
            let ty_param = TypeParam {
                attributes: Vec::new(),
                ident: generic_ty_for_field.clone(),
                bounds: None,
                equal_ty: None,
            };
            generics.type_params.push(ty_param);
            where_clause_items.push(WhereClauseItem {
                for_lifetimes: None,
                lhs: TokenTree::Ident(generic_ty_for_field).into(),
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
        mutator_struct_fields.push(
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
        cache_fields.push(
            StructField {
                attributes: Vec::new(),
                visibility: None,
                identifier: Some(Ident::new("cplx", Span::call_site())),
                ty: TokenTree::Ident(Ident::new("f64", Span::call_site())).into(),
            }
        );
        Struct {
            visibility: parsed_struct.visibility.clone(),
            ident: Ident::new(&format!("{}MutatorCache", parsed_struct.ident), Span::call_site()),
            generics: Some(basic_generics.clone()),
            kind: StructKind::Struct,
            where_clause: None,
            struct_fields: cache_fields.clone(),
        }
    };

    tb.stream(mutator_cache_struct.to_token_stream());

    let mutator_step_struct = {
        let mut step_fields = basic_fields.clone();
        step_fields.push(
            StructField {
                attributes: Vec::new(),
                visibility: None,
                identifier: Some(Ident::new("step", Span::call_site())),
                ty: TokenTree::Ident(Ident::new("u64", Span::call_site())).into(),
            }
        );
        Struct {
            visibility: parsed_struct.visibility.clone(),
            ident: Ident::new(&format!("{}MutationStep", parsed_struct.ident), Span::call_site()),
            generics: Some(basic_generics.clone()),
            kind: StructKind::Struct,
            where_clause: None,
            struct_fields: step_fields,
        }
    };

    tb.stream(mutator_step_struct.to_token_stream());

    let unmutate_token_struct = {
        let step_fields = basic_fields.iter().map(|field| {
            let mut tb = TokenBuilder::new();
            tb.add(":: std :: option :: Option <")
                .stream(field.ty.clone())
                .add(">");
            StructField {
                ty: tb.end(),
                .. field.clone()
            }
        }).collect();
        Struct {
            visibility: parsed_struct.visibility.clone(),
            ident: Ident::new(&format!("{}UnmutateToken", parsed_struct.ident), Span::call_site()),
            generics: Some(basic_generics),
            kind: StructKind::Struct,
            where_clause: None,
            struct_fields: step_fields,
        }
    };

    tb.stream(unmutate_token_struct.to_token_stream());

    { // implementation of Mutator trait
        
        let generics = mutator_struct.generics.clone().map(|g| g.to_token_stream());
        tb
            .add("impl")
            .stream_opt(generics.clone())
            .add("fuzzcheck_mutators :: fuzzcheck_traits :: Mutator for")
            .extend_ident(mutator_struct.ident.clone())
            .stream_opt(generics)
            .stream_opt(mutator_struct.where_clause.clone().map(|wc| wc.to_token_stream()))
            .push_group(Delimiter::Brace)
            .pop_group(Delimiter::Brace);

        
    }
}
