mod macro_lib;
mod macro_lib_test;

//mod struct_derive;

use crate::macro_lib::*;
use proc_macro::{Delimiter, TokenStream, TokenTree};

#[proc_macro_derive(HasDefaultMutator)]
pub fn derive_mutator(input: TokenStream) -> TokenStream {
    let mut parser = TokenParser::new(input);
    let mut tb = TokenBuilder::new();
    //tb.add("mod X { ");
    if let Some(s) = parser.eat_struct() {
        derive_struct_mutator(s, &mut tb);
    } else if let Some(e) = parser.eat_enumeration() {
        //tb.stream(e.whole);
    }
    // tb.add("}");
    tb.eprint();

    tb.end()
}

fn derive_struct_mutator(parsed_struct: Struct, tb: &mut TokenBuilder) {
    if let Some(_) = parsed_struct.data.struct_fields.as_ref() {
        derive_struct_mutator_with_fields(&parsed_struct, tb)
    } else {
        todo!("Build mutator for empty struct");
    }
}

fn derive_struct_mutator_with_fields(parsed_struct: &Struct, tb: &mut TokenBuilder) {
    let fields = parsed_struct.data.struct_fields.as_ref().unwrap();

    let field_types = fields.fields.iter().map(|f| f.ty.clone()).collect::<Vec<_>>();

    let safe_field_names = fields
        .fields
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

    let mut mutator_field_types = vec![];
    let mut mutator_cache_field_types = vec![];
    let mut mutator_step_field_types = vec![];
    let mut mutator_unmutate_token_field_types = vec![];

    for ty in field_types.iter() {
        let mut tb_i = TokenBuilder::new();
        tb_i.punct("<")
            .stream(ty.clone())
            .add("as fuzzcheck_mutators :: HasDefaultMutator > :: Mutator");
        mutator_field_types.push(tb_i.end());

        let mut tb_i = TokenBuilder::new();
        tb_i.add("< <")
            .stream(ty.clone())
            .add("as fuzzcheck_mutators :: HasDefaultMutator > :: Mutator as fuzzcheck_traits :: Mutator > :: Cache");
        mutator_cache_field_types.push(tb_i.end());

        let mut tb_i = TokenBuilder::new();
        tb_i.add("< <").stream(ty.clone()).add(
            "as fuzzcheck_mutators :: HasDefaultMutator > :: Mutator as fuzzcheck_traits :: Mutator > :: MutationStep",
        );
        mutator_step_field_types.push(tb_i.end());

        let mut tb_i = TokenBuilder::new();
        tb_i.add("< <").stream(ty.clone()).add(
            "as fuzzcheck_mutators :: HasDefaultMutator > :: Mutator as fuzzcheck_traits :: Mutator > :: UnmutateToken",
        );
        mutator_unmutate_token_field_types.push(tb_i.end());
    }

    let generic_params = {
        if let Some(params_vec) = parsed_struct.data.generics.as_ref().map(|generics| {
            let mut params = generics
                .lifetime_params
                .iter()
                .map(|lp| lp.ident.clone())
                .collect::<Vec<TokenStream>>();
            params.extend(
                generics
                    .type_params
                    .iter()
                    .map(|tp| TokenTree::Ident(tp.ident.clone()).into()),
            );
            params
        }) {
            let mut tb = TokenBuilder::new();
            tb.add("<");
            for param in params_vec {
                tb.stream(param);
                tb.add(",");
            }
            tb.add(">");
            Some(tb.end())
        } else {
            None
        }
    };
    let where_clause_extended_with_generic_params_bounds = {
        let mut tb = TokenBuilder::new();
        if let Some(generics) = &parsed_struct.data.generics {
            let mut condition = TokenBuilder::new();
            if !generics.lifetime_params.is_empty() {
                for lifetime_param in generics.lifetime_params.iter() {
                    if let Some(bounds) = &lifetime_param.bounds {
                        condition.stream(lifetime_param.ident.clone());
                        condition.punct(":");
                        condition.stream(bounds.clone());
                        condition.punct(",");
                    }

                }
            }
            if !generics.type_params.is_empty() {
                for type_param in generics.type_params.iter() {
                    if let Some(bounds) = &type_param.bounds {
                        condition.extend_ident(type_param.ident.clone());
                        condition.punct(":");
                        condition.stream(bounds.clone());
                        condition.punct(",");
                    }
                    condition.extend_ident(type_param.ident.clone());
                    condition.add(": fuzzcheck_traits :: Mutator , ");
                }
            }
            todo!();
            // let where_clause = add_condition_to_where_clause(parsed_struct.data.where_clause.clone(), condition.end());
            // tb.stream(where_clause);
        }
    };

    let mut add_struct = |ident: &str, ty_fields: &[TokenStream], add_fields: TokenStream| {
        // Main Mutator
        tb.ident("struct")
            .ident(ident)
            .stream_opt(generic_params.clone()); todo!();
            // .stream_opt(parsed_struct.data.where_clause.as_ref().map(|wc| wc.clone()));

        tb.push_group(Delimiter::Brace);

        for (ty_field, field_name) in ty_fields.into_iter().zip(safe_field_names.iter()) {
            tb.ident(&field_name).add(":");
            tb.stream(ty_field.clone());
            tb.add(",");
        }
        tb.stream(add_fields);

        tb.pop_group(Delimiter::Brace);
    };

    // TODO: add where clause condition on generic type parameters (or field types) having a default mutator?

    let name_mutator = format!("{}Mutator", parsed_struct.data.ident);
    add_struct(&name_mutator, &mutator_field_types, {
        let mut tb = TokenBuilder::new();
        tb.add("rng : fuzzcheck_mutators :: fastrand :: Rng");
        tb.end()
    });

    let name_mutator_cache = format!("{}MutatorCache", parsed_struct.data.ident);
    add_struct(&name_mutator_cache, &mutator_cache_field_types, {
        let mut tb = TokenBuilder::new();
        tb.add("cplx : f64 ,");
        tb.end()
    });

    let name_mutator_step = format!("{}MutatorStep", parsed_struct.data.ident);
    add_struct(&name_mutator_step, &mutator_step_field_types, {
        let mut tb = TokenBuilder::new();
        tb.add("step : usize ,");
        tb.end()
    });

    let name_unmutate_token = format!("{}UnmutateToken", parsed_struct.data.ident);
    add_struct(
        &name_unmutate_token,
        &mutator_unmutate_token_field_types,
        TokenStream::new(),
    );

    {
        // implementation of Default for Mutator
        tb.ident("impl");
        tb.stream_opt(generic_params.clone());
        tb.add("core :: default :: Default for");
        tb.ident(&name_mutator);
        tb.stream_opt(generic_params.clone());
        todo!();
        // tb.stream_opt(parsed_struct.data.where_clause.clone());
        tb.add("{");
        tb.add("fn default ( ) -> Self {");
        tb.add("Self {");

        for (field, ty) in safe_field_names.iter().zip(mutator_field_types.iter()) {
            tb.add(&format!(
                "{} : < {} as  core :: default :: Default > :: default ( ) ,",
                field, ty
            ));
        }
        tb.add("rng : fuzzcheck_mutators :: fastrand :: Rng :: new ( )");

        tb.add("} } }");
    }

    {
        // implementation of Mutator trait
        tb.ident("impl");
        tb.stream_opt(generic_params.clone());
        tb.add("fuzzcheck_traits :: Mutator for");
        tb.ident(&name_mutator);
        tb.stream_opt(generic_params.clone());
        //tb.stream_opt(parsed_struct.data.where_clause.clone());
        if let Some(generics) = &parsed_struct.data.generics {
            if !generics.type_params.is_empty() {
                let mut condition = TokenBuilder::new();
                for type_param in generics.type_params.iter() {
                    if let Some(bounds) = &type_param.bounds {
                        condition.extend_ident(type_param.ident.clone());
                        condition.punct(":");
                        condition.stream(bounds.clone());
                        condition.punct(",");
                    }
                    condition.extend_ident(type_param.ident.clone());
                    condition.add(": fuzzcheck_traits :: Mutator , ");
                }
                todo!();
                // let where_clause = add_condition_to_where_clause(parsed_struct.data.where_clause.clone(), condition.end());
                // tb.stream(where_clause);
            }
        }
        tb.add("{ }");
    }
}

fn add_condition_to_where_clause(where_clause: Option<TokenStream>, condition: TokenStream) -> TokenStream {
    let mut tb = TokenBuilder::new();
    if let Some(where_clause) = where_clause {
        tb.stream(where_clause);
        tb.add(", ");
        tb.stream(condition);
        tb.end()
    } else if !condition.is_empty() {
        tb.ident("where");
        tb.stream(condition);
        tb.end()
    } else {
        TokenStream::new()
    }
} 