use proc_macro::TokenStream;
use syn::{DeriveInput};

fn impl_binrepr_trait(ast: DeriveInput) -> TokenStream {
    let ident = ast.ident;
    
    let fields: Vec<(syn::Ident, syn::Type)> = match ast.data {
        syn::Data::Struct(data) => data
            .fields
            .into_iter()
            .filter_map(|f| {
                let name = f.ident?;
                let ty = f.ty;
                Some((name, ty))
            })
            .collect(),
        syn::Data::Enum(_) => panic!("Enums are not supported yet!"),
        syn::Data::Union(_) => panic!("Unions are not supported yet!"),
    };

    let field_idents: Vec<&syn::Ident> = fields.iter().map(|(name, _)| name).collect();
    let field_types: Vec<&syn::Type> = fields.iter().map(|(_, ty)| ty).collect();

    quote::quote! {
        impl ::bin2::BinRepr for #ident {
            fn write_to<W: std::io::Write>(
                &self,
                writer: &mut W,
            ) -> std::io::Result<()> {
                #(
                    self.#field_idents.write_to(writer)?;
                )*
                Ok(())
            }

            fn read_from(reader: &mut ::bin2::Reader<'_>) -> std::io::Result<Self> {
                Ok(Self {
                    #(
                        #field_idents: <#field_types>::read_from(reader)?,
                    )*
                })
            }
        }
    }
    .into()
}


#[proc_macro_derive(BinRepr)]
pub fn binrepr_derive_macro(item: TokenStream) -> TokenStream {
    let ast: DeriveInput = syn::parse(item).unwrap();

    impl_binrepr_trait(ast)
}
