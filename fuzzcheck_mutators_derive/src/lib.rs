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

macro_rules! ident {
    ($x:expr) => {
        {
            Ident::new($x, Span::call_site())
        }
    };
}

#[proc_macro_derive(HasDefaultMutator)]
pub fn derive_mutator(input: TokenStream) -> TokenStream {
    let mut parser = TokenParser::new(input);
    let mut tb = TokenBuilder::new();

    if let Some(s) = parser.eat_struct() {
        derive_struct_mutator(s, &mut tb);
    } else if let Some(e) = parser.eat_enumeration() {
        derive_enum_mutator(e, &mut tb)
    } else {
        extend_token_builder!(&mut tb,
            "compile_error ! ("
            Literal::string("fuzzcheck_mutators_derive could not parse the structure")
            ") ;"
        )
    }
    // tb.eprint();
    tb.end()
}

fn derive_struct_mutator(parsed_struct: Struct, tb: &mut TokenBuilder) {
    if !parsed_struct.struct_fields.is_empty() {
        derive_struct_mutator_with_fields(&parsed_struct, tb)
    } else {
        derive_unit_mutator(parsed_struct, tb);
    }
}

fn derive_struct_mutator_with_fields(parsed_struct: &Struct, tb: &mut TokenBuilder) {

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

    let mutator_field_idents = field_names
        .iter()
        .map(|x| {
            ident!(&format!("_{}", x))
        })
        .collect::<Vec<_>>();

    let generic_types_for_field = mutator_field_idents
        .iter()
        .map(|name| ident!(&format!("{}Type", name)));

    let generics_without_bounds = parsed_struct.generics.removing_bounds_and_eq_type();

    let basic_generics = Generics {
        lifetime_params: Vec::new(),
        type_params: generic_types_for_field
            .clone()
            .map(|ident| TypeParam {
                type_ident: token_stream!(ident),
                ..<_>::default()
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
                ..<_>::default()
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
            identifier: Some(ident!("rng")),
            ty: token_stream!("fuzzcheck_mutators :: fastrand :: Rng"),
        });

        Struct {
            visibility: parsed_struct.visibility.clone(),
            ident: ident!(&format!("{}Mutator", parsed_struct.ident)),
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
            identifier: Some(ident!("cplx")),
            ty: token_stream!("f64"),
        });
        Struct {
            visibility: parsed_struct.visibility.clone(),
            ident: ident!(&format!("{}MutatorCache", parsed_struct.ident)),
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
            identifier: Some(ident!("step")),
            ty: token_stream!("u64"),
        });
        Struct {
            visibility: parsed_struct.visibility.clone(),
            ident: ident!(&format!("{}MutationStep", parsed_struct.ident)),
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
            identifier: Some(ident!("cplx")),
            ty: token_stream!("f64"),
        });
        Struct {
            visibility: parsed_struct.visibility.clone(),
            ident: ident!(&format!("{}UnmutateToken", parsed_struct.ident)),
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

    let fields_iter = field_names.iter().zip(mutator_field_idents.iter());

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
                    ..<_>::default()
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

fn derive_enum_mutator(parsed_enum: Enum, tb: &mut TokenBuilder) {
    if !parsed_enum.items.is_empty() {
        derive_enum_mutator_with_items(&parsed_enum, tb)
    } else {
        todo!("Build mutator for empty enum");
    }
}

struct EnumItemDataForMutatorDerive {
    item: EnumItem, // Aa
    fields: Vec<EnumItemDataFieldForMutatorDerive>, // (u8, _Aa_0, _Aa_0_Type) or (pub x: u16, _Aa_x, _Aa_x_Type),  }
}
struct EnumItemDataFieldForMutatorDerive {
    field: StructField,
    name: Ident,
    mutator_ty: Ident,
}

fn derive_enum_mutator_with_items(parsed_enum: &Enum, tb: &mut TokenBuilder) { 
    let item_idents = parsed_enum
        .items
        .iter()
        .map(|f| f.ident.clone())
        .collect::<Vec<_>>();

    // let item_names = item_idents;

    let (basic_generics, items_for_derive, mutator_struct) = { // mutator struct
        /*
        generics: existing generics + generic mutator type params
        where_clause_items: existing where_clause + “Mutator” conditions
        mutator_field_types: the generic types for the sub-mutators, one for each field
        */
        let mut generics = parsed_enum.generics.removing_bounds_and_eq_type();
        let mut where_clause = parsed_enum.where_clause.clone().unwrap_or_default();
    
        /*
            items_for_derive contains the items of the enum plus some information about
            their fields such as the name and type of the submutators associated with them
        */
        let items_for_derive = parsed_enum.items.iter().map(|item| {
            EnumItemDataForMutatorDerive {
                item: item.clone(),
                fields: if let Some(EnumItemData::Struct(_, fields)) = &item.data {
                    fields.iter().enumerate().map(|(i, field)| {
                        let submutator_name = ident!(
                            &format!(
                                "_{}_{}", 
                                item.ident, 
                                field.identifier
                                    .as_ref().map(<_>::to_string)
                                    .unwrap_or(format!("{}", i))
                            )
                        );
                        let submutator_type_ident = ident!(&format!("{}_Type", submutator_name));
                        EnumItemDataFieldForMutatorDerive {
                            field: field.clone(), 
                            name: submutator_name, 
                            mutator_ty: submutator_type_ident
                        }
                    }).collect::<Vec<_>>()
                } else {
                    vec![]
                }
            }
        }).collect::<Vec<_>>();

        let fields_iter = items_for_derive.iter().flat_map(|item| item.fields.iter());

        // the generic types corresponding to each field in each item
        let basic_generics = Generics {
            lifetime_params: Vec::new(),
            type_params: fields_iter.clone().map(|x| &x.mutator_ty)
                .map(|ident| TypeParam {
                    type_ident: token_stream!(ident),
                    ..<_>::default()
                })
                .collect(),
        };

        let basic_fields = fields_iter.clone().map(|x| (&x.name, &x.mutator_ty))
            .map(|(identifier, ty)| StructField {
                attributes: Vec::new(),
                visibility: Some(token_stream!("pub")),
                identifier: Some(identifier.clone()),
                ty: token_stream!(ty),
            })
            .collect::<Vec<_>>();

        /*
           for each field, add a generic parameter for its mutator as well as a where_clause_item
           ensuring it impls the Mutator trait and that it impls the Clone trait
        */
        for EnumItemDataFieldForMutatorDerive { field, name: _, mutator_ty: submutator_ty } in fields_iter.clone() {
            let ty_param = TypeParam {
                type_ident: token_stream!(submutator_ty),
                ..<_>::default()
            };
            generics.type_params.push(ty_param);
            where_clause.items.push(WhereClauseItem {
                for_lifetimes: None,
                lhs: field.ty.clone(),
                rhs: token_stream!(":: core :: clone :: Clone"),
            });
            where_clause.items.push(WhereClauseItem {
                for_lifetimes: None,
                lhs: token_stream!(submutator_ty),
                rhs: token_stream!("fuzzcheck_mutators :: fuzzcheck_traits :: Mutator < Value = " field.ty ">"),
            });
        }
        /* also add requirement that the whole value is Clone */
        where_clause.items.push(WhereClauseItem {
            for_lifetimes: None,
            lhs: token_stream!(parsed_enum.ident parsed_enum.generics.removing_bounds_and_eq_type()),
            rhs: token_stream!(":: core :: clone :: Clone"),
        });

        let mut mutator_struct_fields = basic_fields.clone();
        mutator_struct_fields.push(StructField {
            attributes: Vec::new(),
            visibility: Some(token_stream!("pub")),
            identifier: Some(ident!("rng")),
            ty: token_stream!("fuzzcheck_mutators :: fastrand :: Rng"),
        });

        (basic_generics, items_for_derive, Struct {
            visibility: parsed_enum.visibility.clone(),
            ident: ident!(&format!("{}Mutator", parsed_enum.ident)),
            generics,
            kind: StructKind::Struct,
            where_clause: Some(where_clause),
            struct_fields: mutator_struct_fields,
        })
    };
    extend_token_builder!(tb, mutator_struct);

    /*
        the enum items to use for “Inner” version of mutator types
        e.g. if the original enum is:
        enum X {
            Aa(u8, u16)
            Bb { x: bool }
            Cc
        }
        then the inner items are:
            Aa { _0: _Aa_0_Type, _1: _Aa_1_Type }
            Bb { _x: _Bb_x_Type }
            Cc
    */
    let inner_items = items_for_derive.iter().map(|item| {
        EnumItem {
            attributes: Vec::new(),
            ident: item.item.ident.clone(),
            data: if item.fields.is_empty() { None } else { 
                Some(EnumItemData::Struct(
                    StructKind::Struct, 
                    item.fields.iter().map(|field| {
                        StructField {
                            identifier: Some(field.name.clone()),
                            ty: token_stream!(field.mutator_ty),
                            ..<_>::default()
                        }
                    }).collect()
                ))
            },
        }
    }).collect::<Vec<_>>();

    let (cache_enum, cache_struct) = { // mutator cache
        let cache_enum = Enum {
            visibility: parsed_enum.visibility.clone(),
            ident: ident!(&format!("{}InnerMutatorCache", parsed_enum.ident.clone())),
            generics: basic_generics.clone(),
            where_clause: None,
            items: inner_items.clone(),
         };

         let cache_struct = Struct {
             visibility: parsed_enum.visibility.clone(),
             ident: ident!(&format!("{}MutatorCache", parsed_enum.ident.clone())),
             generics: basic_generics.clone(),
             kind: StructKind::Struct,
             where_clause: None,
             struct_fields: vec![
                 StructField { 
                     identifier: Some(ident!("inner")),
                     ty: token_stream!(cache_enum.ident cache_enum.generics),
                     ..<_>::default()
                },
                StructField { 
                    identifier: Some(ident!("cplx")),
                    ty: token_stream!("f64"),
                    ..<_>::default()
               }
             ],
            
         };

         (cache_enum, cache_struct)
    };

    extend_token_builder!(tb,
        cache_enum
        cache_struct
    );

    let (step_enum, step_struct) = { // mutation step
        let step_enum = Enum {
            visibility: parsed_enum.visibility.clone(),
            ident: ident!(&format!("{}InnerMutationStep", parsed_enum.ident)),
            generics: basic_generics.clone(),
            where_clause: None,
            items: inner_items.clone(),
         };

         let field_inner = StructField {
             identifier: Some(ident!("inner")),
             ty: token_stream!(step_enum.ident step_enum.generics),
             ..<_>::default()
         };
         let field_step = StructField {
            identifier: Some(ident!("step")),
            ty: token_stream!("u64"),
            ..<_>::default()
        };

         let step_struct = Struct {
             visibility: parsed_enum.visibility.clone(),
             ident: ident!(&format!("{}MutationStep", parsed_enum.ident)),
             generics: basic_generics.clone(),
             kind: StructKind::Struct,
             where_clause: None,
             struct_fields: vec![field_inner, field_step],
         };

         (step_enum, step_struct)
    };

    extend_token_builder!(tb,
        step_enum
        step_struct
    );


    let unmutate_enum = {
        let unmutate_enum = Enum {
            visibility: parsed_enum.visibility.clone(),
            ident: ident!(&format!("{}UnmutateToken", parsed_enum.ident)),
            generics: basic_generics.clone(),
            where_clause: None,
            items: inner_items.clone().into_iter().map(|inner_item| {
                EnumItem {
                    data: match inner_item.data {
                        Some(EnumItemData::Struct(kind, fields)) => {
                            Some(EnumItemData::Struct(kind, fields.into_iter().map(|field| {
                                StructField {
                                    ty: token_stream!(":: std :: option :: Option :: <" field.ty ">"),
                                    .. field
                                }
                            }).collect()))
                        }
                        data @ Some(EnumItemData::Discriminant(_)) | data @ None => { 
                            data
                        }
                    },
                    .. inner_item
                }
            }).collect(),
         };
         unmutate_enum
    };

    tb.extend(&unmutate_enum);

    { // impl Mutator
        let parsed_enum_generics_without_bounds = parsed_enum.generics.removing_bounds_and_eq_type();
        let mutator_struct_generics_without_bounds = mutator_struct.generics.removing_bounds_and_eq_type();
        
        let cplx_choose_item = ((parsed_enum.items.len() as f64).log2() * 100.0).round() / 100.0;
        let enum_has_fields = items_for_derive.iter().find(|item| !item.fields.is_empty()).is_some();

        let items_with_fields_iter = items_for_derive.iter().filter(|item| !item.fields.is_empty());

        extend_token_builder!(tb,
            "impl" mutator_struct.generics "fuzzcheck_mutators :: fuzzcheck_traits :: Mutator for" 
                mutator_struct.ident mutator_struct_generics_without_bounds mutator_struct.where_clause
            "{
                type Value = " parsed_enum.ident parsed_enum_generics_without_bounds ";
                type Cache = " 
                    cache_struct.ident cache_struct.generics.mutating_type_params(|tp| 
                        tp.type_ident = token_stream!("<" tp.type_ident "as fuzzcheck_mutators :: fuzzcheck_traits :: Mutator > :: Cache")
                    )
                    ";
                type MutationStep = " 
                    step_struct.ident step_struct.generics.mutating_type_params(|tp| 
                        tp.type_ident = token_stream!("<" tp.type_ident "as fuzzcheck_mutators :: fuzzcheck_traits :: Mutator > :: MutationStep")
                    )
                    ";
                type UnmutateToken = " 
                    unmutate_enum.ident unmutate_enum.generics.mutating_type_params(|tp|
                        tp.type_ident = token_stream!("<" tp.type_ident "as fuzzcheck_mutators :: fuzzcheck_traits :: Mutator > :: UnmutateToken")
                    )
                ";

                fn max_complexity ( & self ) -> f64 {"
                    cplx_choose_item 
                    if enum_has_fields {
                        token_stream!(
                            "+ [ "
                                joined_token_streams!(items_with_fields_iter.clone().map(|item| {
                                    joined_token_streams!(item.fields.iter().map(|field|
                                        token_stream!("self ." field.name ". max_complexity ( ) ")
                                    ), "+")
                                }), ",")
                            "] . iter ( ) . max ( )"
                        )
                    } else {
                        token_stream!()
                    }
                "}
                fn min_complexity ( & self ) -> f64 {"
                    cplx_choose_item 
                    if enum_has_fields {
                        token_stream!(
                            "+ ["
                                joined_token_streams!(items_with_fields_iter.clone().map(|item| {
                                    joined_token_streams!(item.fields.iter().map(|field|
                                        token_stream!("self ." field.name ". min_complexity ( ) ")
                                    ), "+")
                                }), ",")
                            "] . iter ( ) . min ( )"
                        )
                    } else {
                        token_stream!()
                    }
                "}
                fn complexity ( & self , value : & Self :: Value , cache : & Self :: Cache ) -> f64 { 
                    cache . cplx 
                }

                fn cache_from_value ( & self , value : & Self :: Value ) -> Self :: Cache {
                    match value {"
                        items_for_derive.iter().map(|item| {
                            if let Some(EnumItemData::Struct(kind, _)) = &item.item.data {
                                token_stream!(
                                    parsed_enum.ident "::" item.item.ident 
                                    kind.open()
                                        joined_token_streams!(item.fields.iter().map(|f| {
                                            if let Some(ident) = &f.field.identifier {
                                                token_stream!(ident ":" f.name)
                                            } else {
                                                token_stream!(f.name)
                                            }
                                        }), ",")
                                    kind.close()
                                    "=> {
                                        let inner = XInnerMutatorCache :: " item.item.ident 
                                        "{"
                                            joined_token_streams!(item.fields.iter().map(|f| 
                                                token_stream!(f.name ": self ." f.name ". cache_from_value ( &" f.name ")" )
                                            ),",")
                                        "} ;
                                        let cplx = " cplx_choose_item 
                                            item.fields.iter().map(|f| 
                                                token_stream!("+ self ." f.name ". complexity ( &" f.name ", & inner . " f.name " )")
                                            ).collect::<Vec<_>>()
                                        ";
                                        XMutatorCache {
                                            inner ,
                                            cplx ,
                                        }
                                    }"
                                )
                            } else {
                                token_stream!(
                                    parsed_enum.ident "::" item.item.ident "=> {
                                        XMutatorCache {
                                            inner : XInnerMutatorCache ::" item.item.ident ",
                                            cplx : " cplx_choose_item "
                                        }
                                    }"
                                )
                            }
                        }).collect::<Vec<_>>()
                    "}
                }

                fn initial_step_from_value ( & self , value : & Self :: Value ) -> Self :: MutationStep {
                    match value {"
                        items_for_derive.iter().map(|item| {
                            if let Some(EnumItemData::Struct(kind, _)) = &item.item.data {
                                token_stream!(
                                    parsed_enum.ident "::" item.item.ident 
                                    kind.open() 
                                        joined_token_streams!(item.fields.iter().map(|f| {
                                            if let Some(ident) = &f.field.identifier {
                                                token_stream!(ident ":" f.name)
                                            } else {
                                                token_stream!(f.name)
                                            }
                                        }), ",")
                                    kind.close()
                                    "=> {
                                        let inner = XInnerMutationStep :: " item.item.ident 
                                        "{"
                                            joined_token_streams!(item.fields.iter().map(|f| 
                                                token_stream!(f.name ": self ." f.name ". initial_step_from_value ( &" f.name ")" )
                                            ),",")
                                        "} ;
                                        let step = 0 ;
                                        XMutationStep {
                                            inner ,
                                            step ,
                                        }
                                    }"
                                )
                            } else {
                                token_stream!(
                                    parsed_enum.ident "::" item.item.ident "=> {
                                        XMutationStep {
                                            inner : XInnerMutationStep ::" item.item.ident ",
                                            step : 0
                                        }
                                    }"
                                )
                            }
                        }).collect::<Vec<_>>()
                    "}
                }

                fn random_step_from_value ( & self , value : & Self :: Value ) -> Self :: MutationStep {
                    match value {"
                        items_for_derive.iter().map(|item| {
                            if let Some(EnumItemData::Struct(kind, _)) = &item.item.data {
                                token_stream!(
                                    parsed_enum.ident "::" item.item.ident 
                                    kind.open() 
                                        joined_token_streams!(item.fields.iter().map(|f| {
                                            if let Some(ident) = &f.field.identifier {
                                                token_stream!(ident ":" f.name)
                                            } else {
                                                token_stream!(f.name)
                                            }
                                        }), ",")
                                    kind.close()
                                    "=> {
                                        let inner = XInnerMutationStep :: " item.item.ident 
                                        "{"
                                            joined_token_streams!(item.fields.iter().map(|f| 
                                                token_stream!(f.name ": self ." f.name ". random_step_from_value ( &" f.name ")" )
                                            ),",")
                                        "} ;
                                        let step = self . rng . u64 ( .. ) ;
                                        XMutationStep {
                                            inner ,
                                            step ,
                                        }
                                    }"
                                )
                            } else {
                                token_stream!(
                                    parsed_enum.ident "::" item.item.ident "=> {
                                        XMutationStep {
                                            inner : XInnerMutationStep ::" item.item.ident ",
                                            step : self . rng . u64 ( .. )
                                        }
                                    }"
                                )
                            }
                        }).collect::<Vec<_>>()
                    "}
                }
            }"
            // + arbitraries
            // + mutate
        )
    }
}

fn derive_unit_mutator(parsed_struct: Struct, tb: &mut TokenBuilder) {
    
    let generics_without_bounds = parsed_struct.generics.clone().removing_bounds_and_eq_type();
    let mutator_ident = format!("{}Mutator", parsed_struct.ident);

    extend_token_builder!(tb, 
    "type" mutator_ident generics_without_bounds parsed_struct.where_clause
        "= fuzzcheck_mutators :: unit :: UnitMutator < " parsed_struct.ident generics_without_bounds "> ;"
    
    "impl" parsed_struct.generics "HasDefaultMutator for" 
        parsed_struct.ident generics_without_bounds parsed_struct.where_clause
    "{
        type Mutator = " mutator_ident generics_without_bounds ";
        fn default_mutator ( ) -> Self :: Mutator {
            Self :: Mutator :: new ( " parsed_struct.ident " { } )
        }
    }"
    );
}
