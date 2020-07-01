#[macro_use]
extern crate quote;

use proc_macro2::TokenStream;
use quote::{quote, quote_spanned};
use syn::spanned::Spanned;
use syn::{parse_macro_input, Data, DataStruct, DeriveInput, Ident};

#[proc_macro_derive(HasDefaultMutator)]
pub fn derive_heap_size(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    let name = input.ident;
    let data = input.data;

    match data {
        Data::Struct(data) => {
            proc_macro::TokenStream::from(make_struct_mutator(name, data))
        }
        Data::Enum(_) => {
            panic!("Deriving mutators for enums is not supported yet")
        }
        Data::Union(_) => {
            panic!("Deriving mutators for unions is not supported yet")
        }
    }
}

fn make_struct_mutator(name: Ident, data: DataStruct) -> TokenStream {

    let mut mutator_fields = vec![];
    let mut mutator_cache_fields = vec![];
    let mut mutator_step_fields = vec![];
    let mut unmutate_token_fields = vec![];

    let mut mutator_field_defaults = vec![];


    for field in data.fields {
        let span = field.span();
        // TODO: support tuple structs
        let name = field.ident.unwrap();
        let ty = field.ty;

        // TODO: check that the type has an attribute specifying which mutator to use

        // TODO: assert that the type has a default mutator

        let mutator_field_ident = format_ident!("{}_mutator", &name);
        let mutator_field_ty = quote_spanned! {span=>
            <#ty as fuzzcheck_mutators::HasDefaultMutator>::Mutator
        };

        let mutator_field = quote! {
            #mutator_field_ident: #mutator_field_ty
        };
        mutator_fields.push(mutator_field);
       
        let mutator_field_default = quote! {
            #mutator_field_ident: <#mutator_field_ty as core::default::Default>::default()
        };
        mutator_field_defaults.push(mutator_field_default);

        let mutator_cache_field = quote! {
            #name: <<#ty as fuzzcheck_mutators::HasDefaultMutator>::Mutator as fuzzcheck_traits::Mutator>::Cache
        };
        mutator_cache_fields.push(mutator_cache_field);

        let mutator_step_field = quote! {
            #name: <<#ty as fuzzcheck_mutators::HasDefaultMutator>::Mutator as fuzzcheck_traits::Mutator>::MutationStep
        };
        mutator_step_fields.push(mutator_step_field);

        let unmutate_token_field = quote! {
            #name: Option<#ty>
        };
        unmutate_token_fields.push(unmutate_token_field);
    }

    let name_mutator = format_ident!("{}Mutator", &name);

    let declaration_mutator = quote! {
        pub struct #name_mutator {
            #(#mutator_fields),*,
            rng: fastrand::Rng,
        }
    };

    let name_mutator_cache = format_ident!("{}MutatorCache", &name);
    let declaration_mutator_cache = quote! {
        #[derive(Clone)]
        struct #name_mutator_cache {
            #(#mutator_cache_fields),*,
            cplx: f64,
        }
    };

    let name_mutator_step = format_ident!("{}MutatorStep", &name);
    let declaration_mutator_step = quote! {
        #[derive(Clone)]
        struct #name_mutator_step {
            #(#mutator_step_fields),*,
            pick_step: usize,
        }
    };

    let name_unmutate_token = format_ident!("{}UnmutateToken", &name);
    let declaration_unmutate_token = quote! {
        struct #name_unmutate_token {
            #(#unmutate_token_fields),*,
        }
    };

    let default_impl_mutator = 
    quote! {
        impl core::default::Default for #name_mutator {
            fn default() -> Self {
                Self {
                    #(#mutator_field_defaults),*,
                    rng: fastrand::Rng::new(),
                }
            }
        }
    };

    let mutator_impl_mutator = quote! {
        impl fuzzcheck_traits::Mutator for #name_mutator {
            type Value = #name;
            type Cache = #name_mutator_cache;
            type MutationStep = #name_mutator_step;
        }
    };

    let has_default_mutator_impl_data = quote! {
        impl HasDefaultMutator for #name {
            type Mutator = #name_mutator;
            fn default_mutator() -> Self::Mutator {
                <#name_mutator as core::default::Default>::default()
            }
        }
    };

    quote! {
        #declaration_mutator

        #declaration_mutator_cache

        #declaration_mutator_step

        #declaration_unmutate_token

        #default_impl_mutator

        #mutator_impl_mutator

        #has_default_mutator_impl_data
    }
}