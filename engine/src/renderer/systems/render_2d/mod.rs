use std::str::FromStr;
use uuid::Uuid;

use crate::constants::{
    RENDER_2D_COMMON_INDEX_BUFFER, RENDER_2D_COMMON_TEXTURE_ID, RENDER_2D_COMMON_VERTEX_BUFFER,
};

pub mod forward_dynamic;
pub mod forward_instance;

// Notes: Automatic instancing of Render2D components
//
//  - currently, Render2D components use dynamic buffering by default
//    therefore, even a single component should use a (tiny) dynamic buffer
//
//  - once instancing is available, we need to create one instance
//    buffer for each "group" of Render2D components. So, we need to decide:
//      - How to form the groups
//      - How/where to store the instance buffers
//
//    Groups:
//      Render2D components should be sorted into groups which share the
//      same texture, common_vertex_buffer, and common_index_buffer
//
//    Storing the instance buffers:
//      One instance buffer is needed per group; the user may add an arbitrary
//      number of different Render2D components, each with different "shared" properties,
//      consisting of an arbitrary total of groups.
//
//    Requirements for the instance buffer:
//      - Instance buffers must have a constant "num max entities", like dynamic
//        buffers, to be allocated on init
//      - Instance buffers are a per-uniform concept: thanks to my macros, we should be able
//        to use any generic uniform struct as an instance layout.
//      - Instance buffers are a per-uniform group concept: we will make a Render2DInstance
//        struct which is consumed by the render_2d_instance shader (and/or others) as vertex input.

#[derive(Clone, Debug, PartialEq)]
pub struct Render2D {
    pub name: String,

    pub color: [f32; 4],
    pub texture: Uuid,
    pub mix: f32,

    pub width: f32,
    pub height: f32,

    pub common_vertex_buffer: usize,
    pub common_index_buffer: usize,
}

impl Render2D {
    pub fn _test(name: &str, width: f32, height: f32) -> Self {
        Render2D::solid_rect(name, width, height, [1.0, 1.0, 1.0, 1.0])
    }

    pub fn solid_rect(name: &str, width: f32, height: f32, color: [f32; 4]) -> Self {
        Render2D {
            name: name.to_owned(),
            color,
            mix: 1.0,
            width,
            height,
            texture: Uuid::from_str(RENDER_2D_COMMON_TEXTURE_ID).unwrap(),
            common_vertex_buffer: RENDER_2D_COMMON_VERTEX_BUFFER,
            common_index_buffer: RENDER_2D_COMMON_INDEX_BUFFER,
        }
    }

    pub fn texture(name: &str, texture: Uuid, width: f32, height: f32) -> Self {
        Render2D {
            name: name.to_owned(),
            color: [1.0, 1.0, 1.0, 1.0],
            mix: 0.0,
            width,
            height,
            texture,
            common_vertex_buffer: RENDER_2D_COMMON_VERTEX_BUFFER,
            common_index_buffer: RENDER_2D_COMMON_INDEX_BUFFER,
        }
    }
}

// pub fn _flatten(mat: Matrix2<f32>) -> [f32; 4] {
//     [mat.x[0], mat.y[0], mat.x[1], mat.y[1]]
// }

pub fn create_render_pass<'a>(
    target: &'a wgpu::TextureView,
    encoder: &'a mut wgpu::CommandEncoder,
    label: &'a str,
) -> wgpu::RenderPass<'a> {
    encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: Some(label),
        color_attachments: &[wgpu::RenderPassColorAttachment {
            view: target,
            resolve_target: None,
            ops: wgpu::Operations {
                load: wgpu::LoadOp::Load,
                // load: wgpu::LoadOp::Clear(wgpu::Color {
                //     r: 0.0,
                //     g: 0.0,
                //     b: 0.0,
                //     a: 0.0,
                // }),
                store: true,
            },
        }],
        depth_stencil_attachment: None,
    })
}
