#[macro_use]
extern crate proc_macro;

use proc_macro::TokenStream;
use quote::quote;
use syn::{Data, Fields};

#[proc_macro_derive(VertexLayout)]
pub fn derive_vertex_layout(input: TokenStream) -> TokenStream {
    let ast: syn::DeriveInput = syn::parse(input).unwrap();
    let name = &ast.ident;

    let fields: &Fields;
    if let Data::Struct(struct_data) = &ast.data {
        fields = &struct_data.fields;
    } else {
        panic!("only structs can be vertex layouts");
    }

    let mut attributes = quote!();
    let mut total_offsets = quote!(0);
    let mut location: u32 = 0;

    match fields {
        Fields::Named(named) => {
            for field in named.named.iter() {
                let name = field.ty.clone();

                attributes.extend(quote!(wgpu::VertexAttribute {
                    offset: #total_offsets,
                    shader_location: #location,
                    format: <#name>::vertex_format(),
                },));

                total_offsets.extend(quote!(+ <#name>::attribute_size()));
                location += 1;
            }
        }
        _ => unimplemented!(),
    };

    let gen = quote! {
        impl VertexLayout for #name {
            fn layout_builder() -> VertexLayoutBuilder {
                VertexLayoutBuilder::new(vec![#attributes])
            }
        }
    };
    gen.into()
}
