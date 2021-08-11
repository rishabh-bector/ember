#[macro_use]
extern crate proc_macro;
#[macro_use]
extern crate mem_macros;

use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::{quote, ToTokens};
use syn::{Data, Fields, Type};

#[proc_macro_attribute]
pub fn instance(start_location: TokenStream, item: TokenStream) -> TokenStream {
    layout(
        start_location,
        item,
        quote!(wgpu::InputStepMode::Instance).into(),
    )
}

#[proc_macro_attribute]
pub fn vertex(start_location: TokenStream, item: TokenStream) -> TokenStream {
    layout(
        start_location,
        item,
        quote!(wgpu::InputStepMode::Vertex).into(),
    )
}

fn layout(start_location: TokenStream, item: TokenStream, step_mode: TokenStream2) -> TokenStream {
    let struct_ast: syn::DeriveInput = syn::parse(item).unwrap();
    let struct_name = &struct_ast.ident.clone();
    let fields: &Fields;
    if let Data::Struct(struct_data) = &struct_ast.data {
        fields = &struct_data.fields;
    } else {
        panic!("vertex layouts can only be generated for structs");
    }

    let shader_loc_tokens: TokenStream2 = start_location.into();
    let mut location = quote!(#shader_loc_tokens.0 as u32);
    let mut attributes = quote!();
    let mut offset: usize = 0;

    match fields {
        Fields::Named(named) => {
            for field in named.named.iter() {
                let name = field.ty.clone();
                let (fmt, size) = type_info(name);

                let buf_offset = offset as wgpu::BufferAddress;
                let buf_offset_tokens = quote!(#buf_offset);
                attributes.extend(quote!(wgpu::VertexAttribute {
                            offset: #buf_offset_tokens,
                            shader_location: #location,
                            format: #fmt,
                        },));

                location.extend(quote!(+ 1));
                offset += size;
            }
        }
        _ => unimplemented!(),
    };

    let struct_tokens = struct_ast.into_token_stream();
    let layout_name: TokenStream2 =
        format!("{}_BUFFER_LAYOUT", struct_name.to_string().to_uppercase())
            .parse()
            .unwrap();
    let layout_tokens = quote!(
        pub const #layout_name: wgpu::VertexBufferLayout = wgpu::VertexBufferLayout {
            array_stride: #shader_loc_tokens.1 as wgpu::BufferAddress,
            step_mode: #step_mode,
            attributes: &[#attributes],
        };
    );

    quote!(
        // Original struct
        #struct_tokens
        // Generated vertex buffer layout
        #layout_tokens
    )
    .into()
}

// (vertex format, type size)
fn type_info(ty: Type) -> (TokenStream2, usize) {
    match quote!(#ty).to_string().as_str() {
        "u32" => (quote!(wgpu::VertexFormat::Uint32), size_of!(u32)),
        "f32" => (quote!(wgpu::VertexFormat::Float32), size_of!(f32)),
        "[f32 ; 2]" => (quote!(wgpu::VertexFormat::Float32x2), size_of!([f32; 2])),
        "[f32 ; 3]" => (quote!(wgpu::VertexFormat::Float32x3), size_of!([f32; 3])),
        "[f32 ; 4]" => (quote!(wgpu::VertexFormat::Float32x4), size_of!([f32; 4])),
        other => panic!("unsupported type in instance struct: {}", other),
    }
}
