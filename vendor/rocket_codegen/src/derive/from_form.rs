use devise::{*, ext::{TypeExt, Split3, SpanDiagnosticExt}};

use crate::proc_macro2::{Span, TokenStream};
use crate::syn_ext::NameSource;

#[derive(FromMeta)]
pub struct Form {
    pub field: FormField,
}

pub struct FormField {
    pub span: Span,
    pub name: NameSource,
}

fn is_valid_field_name(s: &str) -> bool {
    // The HTML5 spec (4.10.18.1) says 'isindex' is not allowed.
    if s == "isindex" || s.is_empty() {
        return false
    }

    // We allow all visible ASCII characters except '&', '=', and '?' since we
    // use those as control characters for parsing.
    s.chars().all(|c| (c >= ' ' && c <= '~') && c != '&' && c != '=' && c != '?')
}

impl FromMeta for FormField {
    fn from_meta(meta: MetaItem<'_>) -> Result<Self> {
        let name = NameSource::from_meta(meta)?;
        if !is_valid_field_name(name.name()) {
            return Err(meta.value_span().error("invalid form field name"));
        }

        Ok(FormField { span: meta.value_span(), name })
    }
}

fn validate_struct(_: &DeriveGenerator, data: Struct<'_>) -> Result<()> {
    if data.fields().is_empty() {
        return Err(data.fields.span().error("at least one field is required"));
    }

    let mut names = ::std::collections::HashMap::new();
    for field in data.fields().iter() {
        let id = field.ident.as_ref().expect("named field");
        let field = match Form::from_attrs("form", &field.attrs) {
            Some(result) => result?.field,
            None => FormField { span: Spanned::span(&id), name: id.clone().into() }
        };

        if let Some(span) = names.get(&field.name) {
            return Err(field.span.error("duplicate field name")
                       .span_note(*span, "previous definition here"));
        }

        names.insert(field.name, field.span);
    }

    Ok(())
}

pub fn derive_from_form(input: proc_macro::TokenStream) -> TokenStream {
    let form_error = quote!(::rocket::request::FormParseError);
    DeriveGenerator::build_for(input, quote!(impl<'__f> ::rocket::request::FromForm<'__f>))
        .generic_support(GenericSupport::Lifetime | GenericSupport::Type)
        .replace_generic(0, 0)
        .data_support(DataSupport::NamedStruct)
        .map_type_generic(|_, ident, _| quote! {
            #ident : ::rocket::request::FromFormValue<'__f>
        })
        .validate_generics(|_, generics| match generics.lifetimes().enumerate().last() {
            Some((i, lt)) if i >= 1 => Err(lt.span().error("only one lifetime is supported")),
            _ => Ok(())
        })
        .validate_struct(validate_struct)
        .function(|_, inner| quote! {
            type Error = ::rocket::request::FormParseError<'__f>;

            fn from_form(
                __items: &mut ::rocket::request::FormItems<'__f>,
                __strict: bool,
            ) -> ::std::result::Result<Self, Self::Error> {
                #inner
            }
        })
        .try_map_fields(move |_, fields| {
            define_vars_and_mods!(_None, _Some, _Ok, _Err);
            let (constructors, matchers, builders) = fields.iter().map(|field| {
                let (ident, span) = (&field.ident, field.span());
                let default_name = NameSource::from(ident.clone().expect("named"));
                let name = Form::from_attrs("form", &field.attrs)
                    .map(|result| result.map(|form| form.field.name))
                    .unwrap_or_else(|| Ok(default_name))?;

                let ty = field.ty.with_stripped_lifetimes();
                let ty = quote_spanned! {
                    span => <#ty as ::rocket::request::FromFormValue>
                };

                let constructor = quote_spanned!(span => let mut #ident = #_None;);

                let name = name.name();
                let matcher = quote_spanned! { span =>
                    #name => { #ident = #_Some(#ty::from_form_value(__v)
                                .map_err(|_| #form_error::BadValue(__k, __v))?); },
                };

                let builder = quote_spanned! { span =>
                    #ident: #ident.or_else(#ty::default)
                        .ok_or_else(|| #form_error::Missing(#name.into()))?,
                };

                Ok((constructor, matcher, builder))
            }).collect::<Result<Vec<_>>>()?.into_iter().split3();

            Ok(quote! {
                #(#constructors)*

                for (__k, __v) in __items.map(|item| item.key_value()) {
                    match __k.as_str() {
                        #(#matchers)*
                        _ if __strict && __k != "_method" => {
                            return #_Err(#form_error::Unknown(__k, __v));
                        }
                        _ => { /* lenient or "method"; let it pass */ }
                    }
                }

                #_Ok(Self { #(#builders)* })
            })
        })
        .to_tokens2()
}
