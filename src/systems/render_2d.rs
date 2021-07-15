use crate::component::{Base2D, Position2D};
use crate::render::{
    buffer::{IndexBuffer, VertexBuffer},
    uniform::UniformBuffer,
    GpuState,
};
use crate::resources::{
    camera::{Camera2D, Camera2DUniforms},
    store::TextureStore,
};

use legion::{world::SubWorld, IntoQuery};
use std::borrow::BorrowMut;
use std::{
    cell::RefCell,
    sync::{Arc, Mutex},
};

pub struct Render2DSystem {
    pub common_vertex_buffers: [VertexBuffer; 1],
    pub common_index_buffers: [IndexBuffer; 1],
}

#[system]
#[read_component(Position2D)]
#[read_component(Base2D)]
pub fn render_2d(
    world: &mut SubWorld,
    #[state] state: &Render2DSystem,
    #[resource] gpu: &Arc<Mutex<GpuState>>,
    #[resource] texture_store: &Arc<Mutex<TextureStore>>,
    #[resource] camera_uniforms: &Arc<Mutex<UniformBuffer<Camera2DUniforms>>>,
) {
    // Draw all Pipeline2D components //
    let gpu = gpu.lock().unwrap();
    let texture_store = texture_store.lock().unwrap();
    let camera_uniforms = camera_uniforms.lock().unwrap();

    // Begin render pass //

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
    render_pass.set_pipeline(&gpu.render_pipeline);

    let mut query = <(&Base2D, &Position2D)>::query();
    for (base_2d, _pos) in query.iter_mut(world) {
        // Set bind groups
        render_pass.set_bind_group(
            0,
            texture_store.bind_group(&base_2d.texture).expect(&format!(
                "failed to find referenced texture: {}",
                base_2d.texture
            )),
            &[],
        );
        camera_uniforms.bind_render_pass(&mut render_pass, 1, &gpu.queue);

        // Set buffers
        render_pass.set_vertex_buffer(
            0,
            state.common_vertex_buffers[base_2d.common_vertex_buffer]
                .buffer
                .slice(..),
        );
        render_pass.set_index_buffer(
            state.common_index_buffers[base_2d.common_index_buffer]
                .buffer
                .slice(..),
            wgpu::IndexFormat::Uint16,
        );

        // Run pipeline
        render_pass.draw_indexed(
            0..state.common_index_buffers[base_2d.common_index_buffer].size,
            0,
            0..1,
        );
    }

    // Finish render pass //

    drop(render_pass);
    gpu.queue.submit(std::iter::once(encoder.finish()));
}
