use proc_macro::TokenStream;
use syn::{Data, DeriveInput, Fields, LitBool, Meta};
use proc_macro2::TokenStream as TokenStream2;
use std::collections::HashSet;

fn fnv1a_64(s: &str) -> u64 {
    let mut hash = 0xcbf29ce484222325u64;
    for byte in s.bytes() {
        hash ^= byte as u64;
        hash = hash.wrapping_mul(0x100000001b3u64);
    }
    hash
}

fn parse_hash_attribute(ast: &DeriveInput) -> bool {
    for attr in &ast.attrs {
        if attr.path().is_ident("hash") {
            if let Meta::List(meta_list) = &attr.meta {
                if let Ok(LitBool { value, .. }) = meta_list.parse_args::<LitBool>() {
                    return value;
                }
            }
        }
    }
    // Hashing is ENABLED by default
    true
}

#[proc_macro_derive(BinRepr, attributes(hash))]
pub fn binrepr_derive_macro(item: TokenStream) -> TokenStream {
    let ast: DeriveInput = syn::parse(item).unwrap();
    let ident = &ast.ident;

    let use_hashing = parse_hash_attribute(&ast);

    let (write_body, read_body) = match &ast.data {
        Data::Struct(data) => impl_struct(data),
        Data::Enum(data) => impl_enum(data, use_hashing),
        Data::Union(_) => panic!("Unions are not supported!"),
    };

    quote::quote! {
        impl ::bin2::BinRepr for #ident {
            fn write_to<W: std::io::Write>(
                &self,
                writer: &mut W,
            ) -> std::io::Result<()> {
                #write_body
                Ok(())
            }

            fn read_from(reader: &mut ::bin2::Reader<'_>) -> std::io::Result<Self> {
                #read_body
            }
        }
    }
    .into()
}

fn impl_struct(data: &syn::DataStruct) -> (TokenStream2, TokenStream2) {
    let fields = match &data.fields {
        Fields::Named(fields) => &fields.named,
        _ => panic!("Only structs with named fields are supported!"),
    };

    let field_idents: Vec<&syn::Ident> = fields.iter().map(|f| f.ident.as_ref().unwrap()).collect();
    let field_types: Vec<&syn::Type> = fields.iter().map(|f| &f.ty).collect();

    let write_body = quote::quote! {
        #( self.#field_idents.write_to(writer)?; )*
    };

    let read_body = quote::quote! {
        Ok(Self {
            #( #field_idents: <#field_types>::read_from(reader)?, )*
        })
    };

    (write_body, read_body)
}

fn impl_enum(data: &syn::DataEnum, use_hashing: bool) -> (TokenStream2, TokenStream2) {
    let mut write_arms = vec![];
    let mut read_arms = vec![];
    
    let mut used_tags = HashSet::new();

    for (index, variant) in data.variants.iter().enumerate() {
        let variant_ident = &variant.ident;
        
        let tag: u64 = if use_hashing {
            let hash_value = fnv1a_64(&variant_ident.to_string());
            
            if !used_tags.insert(hash_value) {
                panic!(
                    "Collision detected! Variant '{}' generated an identical hash key ({}) to another variant.",
                    variant_ident, hash_value
                );
            }
            hash_value
        } else {
            index as u64
        };

        match &variant.fields {
            Fields::Unit => {
                write_arms.push(quote::quote! {
                    Self::#variant_ident => { #tag.write_to(writer)?; }
                });
                read_arms.push(quote::quote! {
                    #tag => Ok(Self::#variant_ident),
                });
            }
            Fields::Unnamed(fields) => {
                let idents: Vec<_> = (0..fields.unnamed.len())
                    .map(|i| syn::Ident::new(&format!("__f{}", i), proc_macro2::Span::call_site()))
                    .collect();
                let types: Vec<&syn::Type> = fields.unnamed.iter().map(|f| &f.ty).collect();

                write_arms.push(quote::quote! {
                    Self::#variant_ident( #( #idents ),* ) => {
                        #tag.write_to(writer)?;
                        #( #idents.write_to(writer)?; )*
                    }
                });
                read_arms.push(quote::quote! {
                    #tag => Ok(Self::#variant_ident(
                        #( <#types>::read_from(reader)? ),*
                    )),
                });
            }
            Fields::Named(fields) => {
                let idents: Vec<&syn::Ident> = fields.named.iter().map(|f| f.ident.as_ref().unwrap()).collect();
                let types: Vec<&syn::Type> = fields.named.iter().map(|f| &f.ty).collect();

                write_arms.push(quote::quote! {
                    Self::#variant_ident { #( #idents ),* } => {
                        #tag.write_to(writer)?;
                        #( #idents.write_to(writer)?; )*
                    }
                });
                read_arms.push(quote::quote! {
                    #tag => Ok(Self::#variant_ident {
                        #( #idents: <#types>::read_from(reader)?, )*
                    }),
                });
            }
        }
    }

    let write_body = quote::quote! {
        match self {
            #( #write_arms )*
        }
    };

    let read_body = quote::quote! {
        let tag = u64::read_from(reader)?;
        match tag {
            #( #read_arms )*
            _ => Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("Unknown enum tag hash: {}", tag)
            )),
        }
    };

    (write_body, read_body)
}
