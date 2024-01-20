use proc_macro::TokenStream;
use quote::ToTokens;
use syn::{Data, DataEnum, DataStruct, DataUnion, DeriveInput, Index};

pub fn expand(input: DeriveInput) -> TokenStream {
    let name = input.ident;
    let inner = match input.data {
        Data::Struct(data) => expand_struct(data).to_token_stream(),
        Data::Enum(data) => expand_enum(data).to_token_stream(),
        Data::Union(data) => expand_union(data).to_token_stream(),
    };
    TokenStream::from(quote::quote! {
        impl ::asn1rs::prelude::ProtobufEq for #name {
            fn protobuf_eq(&self, other: &Self) -> bool {
                #inner
            }
        }
    })
}

fn expand_struct(data: DataStruct) -> impl ToTokens {
    let fields = data.fields.iter().enumerate().map(|(index, field)| {
        field
            .ident
            .as_ref()
            .map(|i| i.to_token_stream())
            .unwrap_or_else(|| Index::from(index).to_token_stream())
    });
    quote::quote! {
       #(::asn1rs::prelude::ProtobufEq::protobuf_eq(&self.#fields, &other.#fields) &&)* true
    }
}

fn expand_enum(data: DataEnum) -> impl ToTokens {
    let data_enum = data.variants.iter().any(|d| !d.fields.is_empty());
    let rows = data.variants.iter().map(|variant| &variant.ident);

    if data_enum {
        quote::quote! {
           match &self {
               #(
                   Self::#rows(me) => if let Self::#rows(other) = &other {
                       ::asn1rs::prelude::ProtobufEq::protobuf_eq(me, other)
                   } else {
                       false
                   }
               ),*
           }
        }
    } else {
        quote::quote! {
           match &self {
               #(
                   Self::#rows => matches!(other, Self::#rows),
               )*
               _ => false,
           }
        }
    }
}

fn expand_union(_data: DataUnion) -> impl ToTokens {
    quote::quote! { unimplemented!() }
}
