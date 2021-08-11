#[macro_use]
extern crate proc_macro;
#[macro_use]
extern crate mem_macros;

use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::{quote, ToTokens};
use syn::{Data, Fields, Type};

// This procedural macro generates static vertex buffer layouts
// for structs built with the supported primitives.

#[proc_macro_attribute]
pub fn layout(start_location: TokenStream, item: TokenStream) -> TokenStream {
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
            step_mode: wgpu::InputStepMode::Vertex,
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

// #[proc_macro_derive(VertexLayout)]
// pub fn derive_vertex_layout(input: TokenStream) -> TokenStream {
//     let ast: syn::DeriveInput = syn::parse(input).unwrap();
//     let name = &ast.ident;

//     let fields: &Fields;
//     if let Data::Struct(struct_data) = &ast.data {
//         fields = &struct_data.fields;
//     } else {
//         panic!("only structs can be vertex layouts");
//     }

//     let mut attributes = quote!();
//     let mut total_offsets = quote!(0);
//     let mut location: u32 = 0;

//     match fields {
//         Fields::Named(named) => {
//             for field in named.named.iter() {
//                 let name = field.ty.clone();

//                 attributes.extend(quote!(wgpu::VertexAttribute {
//                     offset: 0,                             //#total_offsets,
//                     shader_location: #location,
//                     format: <#name>::vertex_format(),
//                 },));

//                 total_offsets.extend(quote!(+ <#name>::attribute_size()));
//                 location += 1;
//             }
//         }
//         _ => unimplemented!(),
//     };

//     let gen = quote! {
//         impl VertexLayout for #name {
//             fn layout_builder() -> VertexLayoutBuilder {
//                 VertexLayoutBuilder::new(vec![#attributes])
//             }

//             fn layout() -> wgpu::VertexBufferLayout<'static> {
//                 // wgpu::VertexBufferLayout {
//                 //     array_stride: 0,
//                 //     step_mode: wgpu::InputStepMode::Vertex,
//                 //     attributes: &[#attributes],
//                 // }
//                 wgpu::VertexBufferLayout {
//                     array_stride: 0,// std::mem::size_of::<Vertex2D>() as wgpu::BufferAddress,
//                     step_mode: wgpu::InputStepMode::Vertex,
//                     attributes: &[
//                         #attributes
//                         // wgpu::VertexAttribute {
//                         //     offset: 0,
//                         //     shader_location: 0,
//                         //     format: wgpu::VertexFormat::Float32x2,
//                         // },
//                         // wgpu::VertexAttribute {
//                         //     offset: std::mem::size_of::<[f32; 2]>() as wgpu::BufferAddress,
//                         //     shader_location: 1,
//                         //     format: wgpu::VertexFormat::Float32x2,
//                         // },
//                     ],
//                 }
//             }
//         }
//     };
//     gen.into()
// }
