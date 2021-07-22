use crate::component::Position2D;
use crate::render::{
    buffer::{IndexBuffer, VertexBuffer},
    uniform::{GenericUniform, Uniform, UniformGroup},
    GpuState,
};
use crate::resources::{camera::Camera2D, store::TextureStore};
use crate::systems::{camera_2d::*, lighting_2d::*};

use legion::{world::SubWorld, IntoQuery};
use std::borrow::BorrowMut;
use std::{
    cell::RefCell,
    sync::{Arc, Mutex, MutexGuard},
};

pub const BASE_2D_COMMON_TEXTURE: &str = "test";
pub const BASE_2D_COMMON_VERTEX_BUFFER: usize = 0;
pub const BASE_2D_COMMON_INDEX_BUFFER: usize = 0;

#[derive(Clone, Debug, PartialEq)]
pub struct Base2D {
    pub name: String,

    pub color: [f32; 4],
    pub texture: String,
    pub mix: f32,

    pub width: f32,
    pub height: f32,
    pub common_vertex_buffer: usize,
    pub common_index_buffer: usize,
}

impl Base2D {
    pub fn test(name: &str, width: f32, height: f32) -> Self {
        Base2D::solid_rect(name, width, height, [1.0, 1.0, 1.0, 1.0])
    }

    pub fn solid_rect(name: &str, width: f32, height: f32, color: [f32; 4]) -> Self {
        Base2D {
            name: name.to_owned(),
            color,
            mix: 1.0,
            width,
            height,
            texture: BASE_2D_COMMON_TEXTURE.to_string(),
            common_vertex_buffer: BASE_2D_COMMON_VERTEX_BUFFER,
            common_index_buffer: BASE_2D_COMMON_INDEX_BUFFER,
        }
    }
}

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Base2DUniforms {
    pub model: [f32; 4],
    pub color: [f32; 4],
    pub mix: f32,
    pub _padding: [f32; 32],
    pub __padding: [f32; 23],
}

pub struct Base2DUniformGroup {}

pub struct Render2DSystem {
    pub common_vertex_buffers: [VertexBuffer; 1],
    pub common_index_buffers: [IndexBuffer; 1],
}

// Draw all Base2D components //

#[system]
#[read_component(Position2D)]
#[read_component(Base2D)]
pub fn render_2d(
    world: &mut SubWorld,
    #[state] state: &Render2DSystem,
    #[resource] gpu: &Arc<Mutex<GpuState>>,
    #[resource] texture_store: &Arc<Mutex<TextureStore>>,
    #[resource] base_2d_uniforms_group: &Arc<Mutex<UniformGroup<Base2DUniformGroup>>>,
    #[resource] camera_2d_uniforms_group: &Arc<Mutex<UniformGroup<Camera2DUniformGroup>>>,
    #[resource] lighting_2d_uniforms_group: &Arc<Mutex<UniformGroup<Lighting2DUniformGroup>>>,
    #[resource] base_2d_uniforms: &Arc<Mutex<GenericUniform<Base2DUniforms>>>,
    #[resource] camera_2d_uniforms: &Arc<Mutex<GenericUniform<Camera2DUniforms>>>,
    #[resource] lighting_2d_uniforms: &Arc<Mutex<GenericUniform<Lighting2DUniforms>>>,
) {
    let gpu = gpu.lock().unwrap();
    let texture_store = texture_store.lock().unwrap();

    let base_2d_uniforms_group: MutexGuard<UniformGroup<Base2DUniformGroup>> =
        base_2d_uniforms_group.lock().unwrap();
    let mut base_2d_uniforms = base_2d_uniforms.lock().unwrap();

    let camera_2d_uniforms_group = camera_2d_uniforms_group.lock().unwrap();
    let camera_2d_uniforms = camera_2d_uniforms.lock().unwrap();

    let mut lighting_2d_uniforms = lighting_2d_uniforms.lock().unwrap();
    let lighting_2d_uniforms_group = lighting_2d_uniforms_group.lock().unwrap();

    // Begin render pass //

    // Per-pass logic //

    let mut b2doffset = 0 as u32;
    let mut camoffset = 0 as u32;

    lighting_2d_uniforms.load_buffer(&lighting_2d_uniforms_group.buffers[0], &gpu.queue, 0);

    let frame = gpu.swap_chain.get_current_frame().unwrap().output;
    let mut encoder = gpu
        .device
        .create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Render2D Encoder"),
        });

    let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: Some("Render2D Pass"),
        color_attachments: &[wgpu::RenderPassColorAttachment {
            view: &frame.view,
            resolve_target: None,
            ops: wgpu::Operations {
                load: wgpu::LoadOp::Clear(wgpu::Color {
                    r: 0.1,
                    g: 0.2,
                    b: 0.3,
                    a: 1.0,
                }),
                store: true,
            },
        }],
        depth_stencil_attachment: None,
    });
    render_pass.set_pipeline(&gpu.pipelines.get("base_2d").unwrap());

    render_pass.set_bind_group(3, &lighting_2d_uniforms_group.bind_group, &[0]);

    // Set buffers
    debug!("Setting common vertex buffer");
    render_pass.set_vertex_buffer(0, state.common_vertex_buffers[0].buffer.slice(..));

    debug!("Setting common index buffer");
    render_pass.set_index_buffer(
        state.common_index_buffers[0].buffer.slice(..),
        wgpu::IndexFormat::Uint16,
    );

    // Per-entity logic //

    let mut query = <(&Base2D, &Position2D)>::query();
    for (base_2d, pos) in query.iter_mut(world) {
        debug!("Loading uniforms for pipeline: Base2D:");
        base_2d_uniforms.source.model = [pos.x, pos.y, base_2d.width, base_2d.height];
        debug!("  - model: {:?}", base_2d_uniforms.source.model);
        base_2d_uniforms.source.color = base_2d.color;
        debug!("  - color: {:?}", base_2d_uniforms.source.color);
        base_2d_uniforms.source.mix = base_2d.mix;
        debug!("  - mix: {:?}", base_2d_uniforms.source.mix);

        debug!("Loading buffer base_2d_uniforms");
        base_2d_uniforms.load_buffer(
            &base_2d_uniforms_group.buffers[0],
            &gpu.queue,
            b2doffset as u64,
        );

        debug!("Loading buffer camera_2d_uniforms");
        camera_2d_uniforms.load_buffer(
            &camera_2d_uniforms_group.buffers[0],
            &gpu.queue,
            camoffset as u64,
        );

        // Set bind groups
        debug!("Setting bind group texture");
        render_pass.set_bind_group(
            0,
            texture_store.bind_group(&base_2d.texture).expect(&format!(
                "failed to find referenced texture: {}",
                base_2d.texture
            )),
            &[],
        );
        debug!("Setting bind group base_2d_uniforms_group");
        render_pass.set_bind_group(1, &base_2d_uniforms_group.bind_group, &[b2doffset]);
        debug!("Setting bind group camera_2d_uniforms_group");
        render_pass.set_bind_group(2, &camera_2d_uniforms_group.bind_group, &[camoffset]);

        debug!("Updating offsets");
        b2doffset += base_2d_uniforms.buffer_size();
        camoffset += camera_2d_uniforms.buffer_size();

        // Run pipeline
        debug!("Recording draw call");
        render_pass.draw_indexed(
            0..state.common_index_buffers[base_2d.common_index_buffer].size,
            0,
            0..1,
        );
    }

    // Submit render pass //

    drop(render_pass);
    gpu.queue.submit(std::iter::once(encoder.finish()));
}
