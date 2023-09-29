use proc_macro2::{Ident, Punct, Span, TokenStream, TokenTree};
use syn::punctuated::Punctuated;
use syn::token::Where;
use syn::{parse2, Field, Generics, Visibility, WhereClause};

use crate::token_builder::{ident, join_ts, ts};
use crate::{q, Common, MakeMutatorSettings};

// This file hosts the common code for generating default mutators for enums and structs

#[derive(Clone)]
pub struct FieldMutator {
    pub i: usize,
    pub j: Option<usize>,
    pub field: Field,
    pub kind: FieldMutatorKind,
}

#[derive(Clone)]
pub enum FieldMutatorKind {
    Generic,
    Prescribed(syn::Type, Option<TokenStream>),
    Ignore,
}

impl FieldMutatorKind {
    /// Returns `true` if the field mutator kind is [`Ignore`].
    ///
    /// [`Ignore`]: FieldMutatorKind::Ignore
    pub fn is_ignore(&self) -> bool {
        matches!(self, Self::Ignore)
    }
}

impl FieldMutator {
    pub(crate) fn mutator_stream(&self, cm: &Common) -> TokenStream {
        match &self.kind {
            FieldMutatorKind::Generic => {
                if let Some(j) = self.j {
                    ts!(cm.Mi_j.as_ref()(self.i, j))
                } else {
                    ts!(cm.Mi.as_ref()(self.i))
                }
            }
            FieldMutatorKind::Prescribed(m, _) => ts!(q!(m)),
            FieldMutatorKind::Ignore => ts!(),
        }
    }
}

#[allow(non_snake_case)]
pub(crate) struct CreateWrapperMutatorParams<'a> {
    pub(crate) cm: &'a Common,
    pub(crate) visibility: &'a Visibility,
    pub(crate) type_ident: &'a Ident,
    pub(crate) type_generics: &'a Generics,
    pub(crate) field_mutators: &'a Vec<Vec<FieldMutator>>,
    pub(crate) InnerMutator: &'a TokenStream,
    pub(crate) new_impl: &'a TokenStream,
    pub(crate) settings: &'a MakeMutatorSettings,
}

#[allow(non_snake_case)]
pub(crate) fn make_mutator_type_and_impl(params: CreateWrapperMutatorParams) -> TokenStream {
    let CreateWrapperMutatorParams {
        cm,
        visibility,
        type_ident,
        type_generics,
        field_mutators,
        InnerMutator,
        new_impl,
        settings,
    } = params;

    let NameMutator = if let Some(name) = &settings.name {
        name.clone()
    } else {
        ident!(type_ident "Mutator")
    };

    let field_generic_mutators = field_mutators
        .iter()
        .flatten()
        .filter(|m| match m.kind {
            FieldMutatorKind::Generic => true,
            FieldMutatorKind::Prescribed(_, _) | FieldMutatorKind::Ignore => false,
        })
        .collect::<Vec<_>>();

    let mut NameMutator_generics = type_generics.clone();
    for field_mutator in field_generic_mutators.iter() {
        NameMutator_generics
            .params
            .push(parse2(field_mutator.mutator_stream(cm)).unwrap());
    }
    if NameMutator_generics.where_clause.is_none() {
        NameMutator_generics.where_clause = Some(WhereClause {
            where_token: Where(Span::call_site()),
            predicates: Punctuated::new(),
        });
    }
    for tp in type_generics.type_params() {
        let where_clause = NameMutator_generics.where_clause.as_mut().unwrap();

        where_clause
            .predicates
            .push(parse2(ts!(tp.ident ":" cm.Clone)).unwrap());
        where_clause.predicates.push(parse2(ts!(tp.ident ": 'static")).unwrap());
    }
    for field_mutator in &field_generic_mutators {
        let where_clause = NameMutator_generics.where_clause.as_mut().unwrap();

        where_clause
            .predicates
            .push(parse2(
                ts!(field_mutator.mutator_stream(cm) ":" cm.fuzzcheck_traits_Mutator "<" q!(field_mutator.field.ty) ">"),
            )
            .unwrap()
        );
    }

    let field_prescribed_mutators = field_mutators
        .iter()
        .flatten()
        .filter_map(|m| match &m.kind {
            FieldMutatorKind::Generic | FieldMutatorKind::Ignore => None,
            FieldMutatorKind::Prescribed(mutator, init) => Some((m.clone(), mutator.clone(), init.clone())),
        })
        .collect::<Vec<_>>();

    let mut DefaultMutator_Mutator_generics = type_generics.clone();
    if DefaultMutator_Mutator_generics.where_clause.is_none() {
        DefaultMutator_Mutator_generics.where_clause = Some(WhereClause {
            where_token: Where(Span::call_site()),
            predicates: Punctuated::new(),
        });
    }

    let DefaultMutator_generic_args = ts!(
        "<"
            join_ts!(type_generics.params.iter(), p,
                match p {
                    syn::GenericParam::Type(tp) => {
                        ts!(tp.ident)
                    },
                    syn::GenericParam::Lifetime(lp) => {
                        ts!(TokenTree::Punct(Punct::new('\'', proc_macro2::Spacing::Alone)) lp.lifetime.ident)
                    },
                    syn::GenericParam::Const(cp) => {
                        ts!(cp.ident)
                    },
                }
                ","
            )
            join_ts!(field_generic_mutators, field_mutator,
                "<" q!(field_mutator.field.ty) "as" cm.DefaultMutator ">::Mutator ,"
            )
        ">"
    );

    for tp in type_generics.type_params() {
        let where_clause = DefaultMutator_Mutator_generics.where_clause.as_mut().unwrap();
        where_clause
            .predicates
            .push(parse2(ts!(tp.ident ":" cm.DefaultMutator)).unwrap());

        where_clause.predicates.push(parse2(ts!(tp.ident ": 'static")).unwrap());
    }
    for (_, mutator, _) in field_prescribed_mutators.iter().filter(|(_, _, init)| init.is_none()) {
        let where_clause = DefaultMutator_Mutator_generics.where_clause.as_mut().unwrap();
        where_clause
            .predicates
            .push(parse2(ts!(q!(mutator) ":" cm.Default)).unwrap());
    }

    let NameMutatorCache = ident!(NameMutator "Cache");
    let NameMutatorMutationStep = ident!(NameMutator "MutationStep");
    let NameMutatorArbitraryStep = ident!(NameMutator "ArbitraryStep");
    let NameMutatorUnmutateToken = ident!(NameMutator "UnmutateToken");

    let NameMutator_generics_split = NameMutator_generics.split_for_impl();
    let type_generics_split = type_generics.split_for_impl();

    let helper_type = |helper_type: &str, conformances: bool| {
        let helper_ty_ident = ident!(NameMutator helper_type);
        let InnerType = ts!(
            "<" InnerMutator " as " cm.fuzzcheck_traits_Mutator "<" type_ident q!(type_generics_split.1) "> >::" helper_type
        );

        ts!(
            "#[doc(hidden)]"
            q!(visibility) "struct" helper_ty_ident q!(NameMutator_generics_split.0) q!(NameMutator_generics_split.2) "{
                inner : " if settings.recursive { ts!(cm.Box "<") } else { ts!("") } InnerType if settings.recursive { ">" } else { "" } ",
            }
            impl " q!(NameMutator_generics_split.0) helper_ty_ident q!(NameMutator_generics_split.1) q!(NameMutator_generics_split.2) "{
                #[coverage(off)]
                fn new(inner: " InnerType ") -> Self {"
                    "Self {
                        inner: "  if settings.recursive { ts!(cm.Box "::new") } else { ts!("") }
                            "(inner)"
                        "
                    }"
                "}
            }"
            if conformances {
                ts!(
                    "impl" q!(NameMutator_generics_split.0) cm.Clone "for" helper_ty_ident q!(NameMutator_generics_split.1) q!(NameMutator_generics_split.2) "{
                        #[coverage(off)]
                        fn clone(&self) -> Self {
                            Self::new(self.inner " if settings.recursive { ".as_ref()" } else { "" } ".clone())
                        }
                    }
                    "
                )
            } else {
                ts!()
            }
        )
    };

    let selfty = ts!(type_ident q!(type_generics_split.1));

    let InnerMutator_as_Mutator =
        ts!("<" InnerMutator "as" cm.fuzzcheck_traits_Mutator "<" type_ident q!(type_generics_split.1) "> >" );

    let documentation = format!(
        "A mutator for [`{}`] 

Generated by a procedural macro of [`fuzzcheck`]",
        type_ident
    );
    ts!(
    "#[doc = " q!(documentation) " ]"
    q!(visibility) "struct" NameMutator q!(NameMutator_generics_split.0) q!(NameMutator_generics_split.2)
    "{
        mutator:" InnerMutator "
    }"
    helper_type("Cache", true)
    helper_type("MutationStep", true)
    helper_type("ArbitraryStep", true)
    helper_type("UnmutateToken", false)

    "impl " q!(NameMutator_generics_split.0) NameMutator q!(NameMutator_generics_split.1) q!(NameMutator_generics_split.2) "
    {"
        new_impl
    "}
    impl " q!(NameMutator_generics_split.0) cm.fuzzcheck_traits_Mutator "<" selfty "> 
        for " NameMutator q!(NameMutator_generics_split.1) q!(NameMutator_generics_split.2) "
        {
            #[doc(hidden)]
            type Cache =" NameMutatorCache q!(NameMutator_generics_split.1) ";
            #[doc(hidden)]
            type MutationStep =" NameMutatorMutationStep q!(NameMutator_generics_split.1) ";
            #[doc(hidden)]
            type ArbitraryStep =" NameMutatorArbitraryStep q!(NameMutator_generics_split.1) ";
            #[doc(hidden)]
            type UnmutateToken =" NameMutatorUnmutateToken q!(NameMutator_generics_split.1) ";

            #[doc(hidden)]
            #[coverage(off)]
            fn initialize(&self) {"
                InnerMutator_as_Mutator "::initialize(&self.mutator)
            }

            #[doc(hidden)]
            #[coverage(off)]
            fn default_arbitrary_step(&self) -> Self::ArbitraryStep {
                Self::ArbitraryStep::new(" InnerMutator_as_Mutator "::default_arbitrary_step(&self.mutator))
            }

            #[doc(hidden)]
            #[coverage(off)]
            fn is_valid(&self, value: &" selfty ") -> bool {"
                InnerMutator_as_Mutator "::is_valid(&self.mutator, value)
            }

            #[doc(hidden)]
            #[coverage(off)]
            fn validate_value(&self, value: &" selfty ") -> " cm.Option "<Self::Cache> {
                if let " cm.Some "(c) = " InnerMutator_as_Mutator "::validate_value(&self.mutator, value) {
                    " cm.Some "(Self::Cache::new(c))
                } else {
                    " cm.None "
                }
            }
            #[doc(hidden)]
            #[coverage(off)]
            fn default_mutation_step(&self, value: &" selfty ", cache: &Self::Cache) -> Self::MutationStep {
                Self::MutationStep::new(" InnerMutator_as_Mutator "::default_mutation_step(&self.mutator, value, &cache.inner))
            }

            #[doc(hidden)]
            #[coverage(off)]
            fn global_search_space_complexity(&self) -> f64 {
                " InnerMutator_as_Mutator "::global_search_space_complexity(&self.mutator)
            }

            #[doc(hidden)]
            #[coverage(off)]
            fn max_complexity(&self) -> f64 {
                " InnerMutator_as_Mutator "::max_complexity(&self.mutator)
            }

            #[doc(hidden)]
            #[coverage(off)]
            fn min_complexity(&self) -> f64 {
                " InnerMutator_as_Mutator "::min_complexity(&self.mutator)
            }

            #[doc(hidden)]
            #[coverage(off)]
            fn complexity(&self, value: &" selfty ", cache: &Self::Cache) -> f64 {
                " InnerMutator_as_Mutator "::complexity(&self.mutator, value, &cache.inner)
            }

            #[doc(hidden)]
            #[coverage(off)]
            fn ordered_arbitrary(&self, step: &mut Self::ArbitraryStep, max_cplx: f64) -> Option<(" selfty ", f64)> {
                if let " cm.Some "((value, cplx)) = " InnerMutator_as_Mutator "::ordered_arbitrary(&self.mutator, &mut step.inner, max_cplx) {"
                cm.Some "((value, cplx))"
            "} else {"
                cm.None
            "}
            }

            #[doc(hidden)]
            #[coverage(off)]
            fn random_arbitrary(&self, max_cplx: f64) -> (" selfty ", f64) {
                let (value, cplx) = " InnerMutator_as_Mutator "::random_arbitrary(&self.mutator, max_cplx) ;
                (value, cplx)
            }

            #[doc(hidden)]
            #[coverage(off)]
            fn ordered_mutate(
                &self,
                value: &mut " selfty ",
                cache: &mut Self::Cache,
                step: &mut Self::MutationStep,
                subvalue_provider: &dyn " cm.SubValueProvider ",
                max_cplx: f64,
            ) -> Option<(Self::UnmutateToken, f64)> {
                if let " cm.Some "((t, c)) = " InnerMutator_as_Mutator "::ordered_mutate(
                    &self.mutator,
                    value,
                    &mut cache.inner,
                    &mut step.inner,
                    subvalue_provider,
                    max_cplx,
                ) {
                    " cm.Some "((Self::UnmutateToken::new(t), c))
                } else {"
                    cm.None
                "}
            }

            #[doc(hidden)]
            #[coverage(off)]
            fn random_mutate(&self, value: &mut " selfty ", cache: &mut Self::Cache, max_cplx: f64) -> (Self::UnmutateToken, f64) {
                let (t, c) =" InnerMutator_as_Mutator "::random_mutate(&self.mutator, value, &mut cache.inner, max_cplx);
                (Self::UnmutateToken::new(t), c)
            }

            #[doc(hidden)]
            #[coverage(off)]
            fn unmutate(&self, value: &mut " selfty ", cache: &mut Self::Cache, t: Self::UnmutateToken) {
                " InnerMutator_as_Mutator "::unmutate(&self.mutator, value, &mut cache.inner," if settings.recursive {
                "*t.inner"
                } else {
                    "t.inner"
                }")
            }
            #[doc(hidden)]
            #[coverage(off)]
            fn visit_subvalues<'__fuzzcheck_derive_lt>(&self, value: &'__fuzzcheck_derive_lt " selfty ", cache: &'__fuzzcheck_derive_lt Self::Cache, visit: &mut dyn FnMut(&'__fuzzcheck_derive_lt dyn " cm.Any ", f64)) {
                " InnerMutator_as_Mutator "::visit_subvalues(&self.mutator, value, &cache.inner, visit);
            }
        }"
        if settings.default {
            ts!("impl" q!(type_generics_split.0) cm.DefaultMutator "for" selfty q!(DefaultMutator_Mutator_generics.where_clause) "{"
            if settings.recursive {
                ts!("type Mutator = " cm.RecursiveMutator "<" NameMutator q!(DefaultMutator_generic_args) ">;")
            } else {
                ts!("type Mutator = "  NameMutator q!(DefaultMutator_generic_args) ";")
            }
            "#[coverage(off)]
            fn default_mutator() -> Self::Mutator {"
                if settings.recursive {
                    format!("{}::new(|self_| {{", cm.RecursiveMutator)
                } else {
                    "".to_string()
                }
                NameMutator "::new("
                    join_ts!(field_mutators.iter().flatten().filter(|variant| {
                        !variant.kind.is_ignore()
                    }), field_mutator,
                        match &field_mutator.kind {
                            FieldMutatorKind::Generic => {
                                ts!("<" q!(field_mutator.field.ty) "as" cm.DefaultMutator ">::default_mutator()")
                            }
                            FieldMutatorKind::Prescribed(_, Some(init)) => {
                                ts!("{" init "}")
                            }
                            FieldMutatorKind::Prescribed(mutator, None) => {
                                ts!("<" q!(mutator) "as" cm.Default ">::default()")
                            }
                            // do not generate ignored variants
                            FieldMutatorKind::Ignore => {
                                unreachable!()
                            }
                        }
                    , separator: ",")
                ")"
                if settings.recursive {
                    "})"
                } else {
                    ""
                }
                "}
            }")
        } else {
            ts!()
        }
    )
}
