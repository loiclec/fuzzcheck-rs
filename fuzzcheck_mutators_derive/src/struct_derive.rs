
// use proc_macro::{TokenStream};
// use crate::macro_lib::*;

// #[proc_macro_derive(HasDefaultMutator)]
// pub fn derive_struct_mutator(input: TokenStream) -> TokenStream {

//     let mut parser = TokenParser::new(input);
//     let mut tb = TokenBuilder::new();
    

//     let input = parse_macro_input!(input as DeriveInput);

//     let vis = input.vis;
//     let name = input.ident;
//     let data = input.data;

//     match data {
//         Data::Struct(data) => {
//             proc_macro::TokenStream::from(make_struct_mutator(name, vis, data))
//         }
//         Data::Enum(_) => {
//             panic!("Deriving mutators for enums is not supported yet")
//         }
//         Data::Union(_) => {
//             panic!("Deriving mutators for unions is not supported yet")
//         }
//     }

//     let mut field_idents = vec![];

//     let mut mutator_fields = vec![];
//     let mut mutator_cache_fields = vec![];
//     let mut mutator_step_fields = vec![];
//     let mut unmutate_token_fields = vec![];

//     let mut field_arbitrary_return_tys = vec![];

//     let mut mutator_field_defaults = vec![];

//     let field_idents = if matches!(data.fields, Fields::Named(_)) {
//         Some(data.fields.map(|field| {
//             field.ident.unwrap()
//         }))
//     } else {
//         None
//     };
//     let fields_tys = data.fields.map(|field| {
//         field.ty
//     });

//     let name_mutator = format_ident!("{}Mutator", &name);

//     let mutator_field_tys = fields_tys.map(|ty| {
//         quote! { <#ty as fuzzcheck_mutators::HasDefaultMutator>::Mutator }
//     });

//     let declaration_mutator = quote! {
//         #visibility struct #name_mutator {
//             #(#mutator_fields),*,
//             rng: fastrand::Rng,
//         }
//     };

//     for field in data.fields {
//         let mutator_field = quote! {
//             #name: #mutator_field_ty
//         };
//         mutator_fields.push(mutator_field);
        
//         let mutator_field_default = quote! {
//             #name: <#mutator_field_ty as core::default::Default>::default()
//         };
//         mutator_field_defaults.push(mutator_field_default);

//         let mutator_cache_field_ty = quote! {
//             <<#ty as fuzzcheck_mutators::HasDefaultMutator>::Mutator as fuzzcheck_traits::Mutator>::Cache
//         };
//         field_arbitrary_return_tys.push(quote! { (#ty, #mutator_cache_field_ty) });
    
//         let mutator_cache_field = quote! {
//             #name: #mutator_cache_field_ty
//         };
//         mutator_cache_fields.push(mutator_cache_field);

//         let mutator_step_field = quote! {
//             #name: <<#ty as fuzzcheck_mutators::HasDefaultMutator>::Mutator as fuzzcheck_traits::Mutator>::MutationStep
//         };
//         mutator_step_fields.push(mutator_step_field);

//         let unmutate_token_field = quote! {
//             #name: Option<#ty>
//         };
//         unmutate_token_fields.push(unmutate_token_field);
//     }



//     let name_mutator_cache = format_ident!("{}MutatorCache", &name);
//     let declaration_mutator_cache = quote! {
//         #[derive(Clone)]
//         #visibility struct #name_mutator_cache {
//             #(#mutator_cache_fields),*,
//             cplx: f64,
//         }
//     };

//     let name_mutator_step = format_ident!("{}MutatorStep", &name);
//     let declaration_mutator_step = quote! {
//         #[derive(Clone)]
//         #visibility struct #name_mutator_step {
//             #(#mutator_step_fields),*,
//             pick_step: usize,
//         }
//     };

//     let name_unmutate_token = format_ident!("{}UnmutateToken", &name);
//     let declaration_unmutate_token = quote! {
//         #[derive(Default)]
//         #visibility struct #name_unmutate_token {
//             #(#unmutate_token_fields),*,
//         }
//     };

//     let default_impl_mutator = quote! {
//         impl core::default::Default for #name_mutator {
//             fn default() -> Self {
//                 Self {
//                     #(#mutator_field_defaults),*,
//                     rng: fastrand::Rng::new(),
//                 }
//             }
//         }
//     };

//     let (cache_from_value_field_caches_and_cplxs, cplx_idents) = {
//         let (mut cache_from_value_field_caches_and_cplxs, mut cplx_idents) = (vec![], vec![]);

//         for field_ident in field_idents.iter() {
//             let cplx_ident = format_ident!("{}_cplx", field_ident);

//             cplx_idents.push(cplx_ident.clone());

//             let tokens = quote! {
//                 let #field_ident = self.#field_ident.cache_from_value(&value.#field_ident);
//                 let #cplx_ident = self.#field_ident.complexity(&value.#field_ident, &#field_ident);

//             };
//             cache_from_value_field_caches_and_cplxs.push(tokens);
//         }
//         (cache_from_value_field_caches_and_cplxs, cplx_idents)
//     };

//     let mutation_step_from_value_field_steps = {
//         let mut mutation_step_from_value_field_steps = vec![];

//         for ident in field_idents.iter() {
//             let tokens = quote! {
//                 let #ident = self.#ident.mutation_step_from_value(&value.#ident);
//             };
//             mutation_step_from_value_field_steps.push(tokens);
//         }
//         mutation_step_from_value_field_steps
//     };

//     let array_of_mutator_indices = {
//         let idcs = (0 .. field_idents.len()).collect::<Vec<_>>();
//         quote! {
//             [#(#idcs),*]
//         }
//     };

//     let arbitrary_partial = {
//         let mut arbitrary_partial = vec![];
//         for (idx, ident) in field_idents.iter().enumerate() {
//             let idx = Index::from(idx);
//             let tokens = quote! {
//                 #idx => {
//                     sum_of_remaining_min_cplxs -= self.#ident.min_complexity();
//                     let partial_value = self.#ident.arbitrary(self.rng.usize(..), cplx_budget - sum_of_remaining_min_cplxs);
//                     cplx_budget -= self.#ident.complexity(&partial_value.0, &partial_value.1);
//                     partial_values.#idx = Some(partial_value);
                    
//                 }
//             };
//             arbitrary_partial.push(tokens);
//         }
//         arbitrary_partial
//     };

//     let (arbitrary_partial_values_unwrap, arbitrary_partial_caches_unwrap) = {
//         let mut arbitrary_partial_values_unwrap = vec![];
//         let mut arbitrary_partial_caches_unwrap = vec![];
//         for (idx, ident) in field_idents.iter().enumerate() {
//             let idx = Index::from(idx);
//             let tokens = quote! {
//                 #ident: partial_values.#idx.unwrap().0
//             };
//             arbitrary_partial_values_unwrap.push(tokens);
//             let tokens = quote! {
//                 #ident: partial_values.#idx.unwrap().1
//             };
//             arbitrary_partial_caches_unwrap.push(tokens);
//         }
//         (arbitrary_partial_values_unwrap, arbitrary_partial_caches_unwrap)
//     };

//     let number_of_fields = field_idents.len();
//     let mutate_partial = {
//         let mut mutate_partial = vec![];
//         for (idx, ident) in field_idents.iter().enumerate() {
//             let idx = Index::from(idx);
//             let tokens = quote! {
//                 #idx => {
//                     sum_of_cplxs -= self.#ident.complexity(&value.#ident, &cache.#ident);
//                     let max_cplx = max_cplx - sum_of_cplxs;
//                     let token = self.#ident.mutate(&mut value.#ident, &mut cache.#ident, &mut step.#ident, max_cplx);
//                     #name_unmutate_token {
//                         #ident: Some(token),
//                         ..<#name_unmutate_token as core::default::Default>::default()
//                     }
//                 }
//             };
//             mutate_partial.push(tokens);
//         }
//         mutate_partial
//     };

//     let mutator_impl_mutator = quote! {
//         impl fuzzcheck_traits::Mutator for #name_mutator {
//             type Value = #name;
//             type Cache = #name_mutator_cache;
//             type MutationStep = #name_mutator_step;
//             type UnmutateToken = #name_unmutate_token;

//             fn max_complexity(&self) -> f64 {
//                 #(self.#field_idents.max_complexity())+*
//             }

//             fn min_complexity(&self) -> f64 {
//                 #(self.#field_idents.min_complexity())+*
//             }

//             fn complexity(&self, value: &Self::Value, cache: &Self::Cache) -> f64 {
//                 cache.cplx
//             }

//             fn cache_from_value(&self, value: &Self::Value) -> Self::Cache {
//                 #(#cache_from_value_field_caches_and_cplxs)*
//                 let cplx = #(#cplx_idents)+*;

//                 Self::Cache {
//                     #(#field_idents),*,
//                     cplx,
//                 }
//             }
            
//             fn mutation_step_from_value(&self, value: &Self::Value) -> Self::MutationStep {
//                 #(#mutation_step_from_value_field_steps)*

//                 #name_mutator_step {
//                     #(#field_idents),*,
//                     pick_step: 0,
//                 }
//             }

//             fn arbitrary(&mut self, seed: usize, max_cplx: f64) -> (Self::Value, Self::Cache) {
//                 let mut cplx_budget = self.max_complexity();
//                 let mut sum_of_remaining_min_cplxs = self.min_complexity();

//                 let mut mutators_indices = #array_of_mutator_indices;
//                 let mut partial_values = (#(Option::<(#field_arbitrary_return_tys)>::None),*);

//                 self.rng.shuffle(&mut mutators_indices);

//                 for idx in &mutators_indices {
//                     match idx {
//                         #(#arbitrary_partial),*,
//                         _ => unreachable!()
//                     }
//                 }

//                 (
//                     Self::Value { 
//                         #(#arbitrary_partial_values_unwrap),* 
//                     },
//                     Self::Cache { 
//                         #(#arbitrary_partial_caches_unwrap),*,
//                         cplx: self.max_complexity() - cplx_budget
//                     }
//                 )
//             }

//             fn mutate(
//                 &mut self,
//                 value: &mut Self::Value,
//                 cache: &mut Self::Cache,
//                 step: &mut Self::MutationStep,
//                 max_cplx: f64,
//             ) -> Self::UnmutateToken {
//                 let mut sum_of_cplxs = self.complexity(&value, &cache);
//                 match self.rng.usize(0 .. #number_of_fields) {
//                     #(#mutate_partial),*,
//                     _ => unreachable!()
//                 }
//             }

//             fn unmutate(&self, value: &mut Self::Value, cache: &mut Self::Cache, t: Self::UnmutateToken) {
//                 #(
//                 if let Some(u) = t.#field_idents {
//                     self.#field_idents.unmutate(&mut value.#field_idents, &mut cache.#field_idents, u); 
//                 }
//                 )*
//             }
//         }
//     };

//     let has_default_mutator_impl_data = quote! {
//         impl HasDefaultMutator for #name {
//             type Mutator = #name_mutator;
//             fn default_mutator() -> Self::Mutator {
//                 <#name_mutator as core::default::Default>::default()
//             }
//         }
//     };

//     quote! {
//         #declaration_mutator

//         #declaration_mutator_cache

//         #declaration_mutator_step

//         #declaration_unmutate_token

//         #default_impl_mutator

//         #mutator_impl_mutator

//         #has_default_mutator_impl_data
//     }
// }