use syn;
use quote::ToTokens;

use proc_macro2::TokenStream;
use field::{Field, FieldParent, Fields, FieldsKind};

#[derive(Debug)]
pub struct Derived<'p, T: 'p> {
    pub derive_input: &'p syn::DeriveInput,
    pub value: &'p T,
}

pub type Variant<'v> = Derived<'v, syn::Variant>;

pub type Struct<'v> = Derived<'v, syn::DataStruct>;

pub type Enum<'v> = Derived<'v, syn::DataEnum>;

impl<'p, T> Derived<'p, T> {
    pub fn from(derive_input: &'p syn::DeriveInput, value: &'p T) -> Self {
        Derived { derive_input, value }
    }
}

impl<'p, T> ::std::ops::Deref for Derived<'p, T> {
    type Target = T;

    fn deref(&self) -> &T {
        self.value
    }
}

impl<'p, T: ToTokens> ToTokens for Derived<'p, T> {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        self.value.to_tokens(tokens)
    }
}

impl<'p, T> Copy for Derived<'p, T> { }

impl<'p, T> Clone for Derived<'p, T> {
    fn clone(&self) -> Derived<'p, T> {
        *self
    }
}

impl<'f> Variant<'f> {
    pub fn builder<F: Fn(Field) -> TokenStream>(&self, f: F) -> TokenStream {
        let variant = &self.ident;
        let expression = self.fields().iter().map(f);
        let enum_name = &self.derive_input.ident;
        match self.fields().kind {
            FieldsKind::Named(..) => {
                let field_name = self.fields.iter()
                    .map(|f| f.ident.as_ref().unwrap());
                quote! {
                    #enum_name::#variant { #(#field_name: #expression),* }
                }
            },
            FieldsKind::Unnamed(..) => {
                quote!( #enum_name::#variant(#(#expression),*) )
            }
            FieldsKind::Unit => quote!(#enum_name::#variant),
        }
    }

    pub fn fields(self) -> Fields<'f> {
        FieldParent::Variant(self).fields()
    }
}

impl<'p> Enum<'p> {
    pub fn variants(self) -> impl Iterator<Item = Variant<'p>> {
        self.value.variants.iter()
            .map(move |v| Derived::from(&self.derive_input, v))
    }
}

impl<'p> Struct<'p> {
    pub fn fields(self) -> Fields<'p> {
        FieldParent::Struct(self).fields()
    }

    pub fn builder<F: Fn(Field) -> TokenStream>(&self, f: F) -> TokenStream {
        let expression = self.fields().iter().map(f);
        let struct_name = &self.derive_input.ident;
        match self.fields().kind {
            FieldsKind::Named(..) => {
                let field_name = self.fields.iter()
                    .map(|f| f.ident.as_ref().unwrap());

                quote!(#struct_name { #(#field_name: #expression),* })
            },
            FieldsKind::Unnamed(..) => {
                quote!(#struct_name ( #(#expression),* ))
            }
            FieldsKind::Unit => quote!(#struct_name),
        }
    }
}
