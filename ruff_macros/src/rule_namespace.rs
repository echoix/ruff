use proc_macro2::{Ident, Span};
use quote::quote;
use syn::spanned::Spanned;
use syn::{Data, DataEnum, DeriveInput, Error, Lit, Meta, MetaNameValue};

pub fn derive_impl(input: DeriveInput) -> syn::Result<proc_macro2::TokenStream> {
    let DeriveInput { ident, data: Data::Enum(DataEnum {
        variants, ..
    }), .. } = input else {
        return Err(Error::new(input.ident.span(), "only named fields are supported"));
    };

    let mut parsed = Vec::new();

    let mut prefix_match_arms = quote!();
    let mut name_match_arms = quote!(Self::Ruff => "Ruff-specific rules",);
    let mut url_match_arms = quote!(Self::Ruff => None,);

    for variant in variants {
        let prefix_attrs: Vec<_> = variant
            .attrs
            .iter()
            .filter(|a| a.path.is_ident("prefix"))
            .collect();

        if prefix_attrs.is_empty() {
            return Err(Error::new(
                variant.span(),
                r#"Missing [#prefix = "..."] attribute"#,
            ));
        }

        let Some(doc_attr) = variant.attrs.iter().find(|a| a.path.is_ident("doc")) else {
            return Err(Error::new(variant.span(), r#"expected a doc comment"#))
        };

        let variant_ident = variant.ident;

        if variant_ident != "Ruff" {
            let Ok(Meta::NameValue(MetaNameValue{lit: Lit::Str(doc_lit), ..})) = doc_attr.parse_meta() else {
                return Err(Error::new(doc_attr.span(), r#"expected doc attribute to be in the form of [#doc = "..."]"#))
            };
            let doc_lit = doc_lit.value();
            let Some((name, url)) = parse_markdown_link(doc_lit.trim()) else {
                return Err(Error::new(doc_attr.span(), r#"expected doc comment to be in the form of `/// [name](https://example.com/)`"#))
            };
            name_match_arms.extend(quote! {Self::#variant_ident => #name,});
            url_match_arms.extend(quote! {Self::#variant_ident => Some(#url),});
        }

        let mut prefix_literals = Vec::new();

        for attr in prefix_attrs {
            let Ok(Meta::NameValue(MetaNameValue{lit: Lit::Str(lit), ..})) = attr.parse_meta() else {
                return Err(Error::new(attr.span(), r#"expected attribute to be in the form of [#prefix = "..."]"#))
            };
            parsed.push((lit.clone(), variant_ident.clone()));
            prefix_literals.push(lit);
        }

        prefix_match_arms.extend(quote! {
            Self::#variant_ident => &[#(#prefix_literals),*],
        });
    }

    parsed.sort_by_key(|(prefix, _)| prefix.value().len());

    let mut if_statements = quote!();
    let mut into_iter_match_arms = quote!();

    for (prefix, field) in parsed {
        if_statements.extend(quote! {if let Some(rest) = code.strip_prefix(#prefix) {
            return Some((#ident::#field, rest));
        }});

        let prefix_ident = Ident::new(&prefix.value(), Span::call_site());

        if field != "Pycodestyle" {
            into_iter_match_arms.extend(quote! {
                #ident::#field => RuleSelector::#prefix_ident.into_iter(),
            });
        }
    }

    into_iter_match_arms.extend(quote! {
        #ident::Pycodestyle => {
            let rules: Vec<_> = (&RuleSelector::E).into_iter().chain(&RuleSelector::W).collect();
            rules.into_iter()
        }
    });

    Ok(quote! {
        impl crate::registry::RuleNamespace for #ident {
            fn parse_code(code: &str) -> Option<(Self, &str)> {
                #if_statements
                None
            }


            fn prefixes(&self) -> &'static [&'static str] {
                match self { #prefix_match_arms }
            }

            fn name(&self) -> &'static str {
                match self { #name_match_arms }
            }

            fn url(&self) -> Option<&'static str> {
                match self { #url_match_arms }
            }
        }

        impl IntoIterator for &#ident {
            type Item = Rule;
            type IntoIter = ::std::vec::IntoIter<Self::Item>;

            fn into_iter(self) -> Self::IntoIter {
                use colored::Colorize;

                match self {
                    #into_iter_match_arms
                }
            }
        }
    })
}

fn parse_markdown_link(link: &str) -> Option<(&str, &str)> {
    link.strip_prefix('[')?.strip_suffix(')')?.split_once("](")
}
