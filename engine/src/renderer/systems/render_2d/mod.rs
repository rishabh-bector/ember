use std::str::FromStr;
use uuid::Uuid;

use crate::constants::{
    ID, RENDER_2D_COMMON_TEXTURE_ID, UNIT_SQUARE_IND_BUFFER_ID, UNIT_SQUARE_VRT_BUFFER_ID,
};

pub mod forward_dynamic;
pub mod forward_instance;

#[derive(Clone, Debug, PartialEq)]
pub struct Render2D {
    pub name: String,

    pub color: [f32; 4],
    pub texture: Uuid,
    pub mix: f32,

    pub width: f32,
    pub height: f32,

    pub common_vertex_buffer: Uuid,
    pub common_index_buffer: Uuid,
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
            common_vertex_buffer: ID(UNIT_SQUARE_VRT_BUFFER_ID),
            common_index_buffer: ID(UNIT_SQUARE_IND_BUFFER_ID),
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
            common_vertex_buffer: ID(UNIT_SQUARE_VRT_BUFFER_ID),
            common_index_buffer: ID(UNIT_SQUARE_IND_BUFFER_ID),
        }
    }
}

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

// pub fn _flatten(mat: Matrix2<f32>) -> [f32; 4] {
//     [mat.x[0], mat.y[0], mat.x[1], mat.y[1]]
// }
