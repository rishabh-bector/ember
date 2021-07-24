use legion::{world::SubWorld, IntoQuery};
use std::sync::{Arc, Mutex};

use crate::{
    component::Position2D,
    render::{
        buffer::{IndexBuffer, VertexBuffer},
        uniform::UniformGroup,
        GpuState,
    },
    resources::store::TextureStore,
    systems::{base_2d::*, camera_2d::*, lighting_2d::*},
};

pub struct Render2DSystem {
    pub common_vertex_buffers: [VertexBuffer; 1],
    pub common_index_buffers: [IndexBuffer; 1],
    pub bind_map: 
}

// Draw all Base2D components //

#[system]
#[read_component(Base2D)]
#[read_component(Position2D)]
pub fn forward_render_2d(
    world: &mut SubWorld,
    #[state] state: &Render2DSystem,
    #[resource] gpu: &Arc<Mutex<GpuState>>,
    #[resource] texture_store: &Arc<Mutex<TextureStore>>,
    #[resource] base_2d_uniforms_group: &Arc<Mutex<UniformGroup<Base2DUniformGroup>>>,
    #[resource] camera_2d_uniforms_group: &Arc<Mutex<UniformGroup<Camera2DUniformGroup>>>,
    #[resource] lighting_2d_uniforms_group: &Arc<Mutex<UniformGroup<Lighting2DUniformGroup>>>,
) {
    let gpu = gpu.lock().unwrap();
    let texture_store = texture_store.lock().unwrap();
    let mut base_2d_uniforms_group = base_2d_uniforms_group.lock().unwrap();
    let camera_2d_uniforms_group = camera_2d_uniforms_group.lock().unwrap();
    let lighting_2d_uniforms_group = lighting_2d_uniforms_group.lock().unwrap();

    let base_2d_bind_group = base_2d_uniforms_group.bind_group();

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
                    r: 0.0,
                    g: 0.0,
                    b: 0.0,
                    a: 0.0,
                }),
                store: true,
            },
        }],
        depth_stencil_attachment: None,
    });

    // Common bindings
    render_pass.set_pipeline(&gpu.pipelines[0].pipeline);

    // render_pass.set_bind_group(
    //     0,
    //     texture_store.bind_group("test").expect(&format!(
    //         "failed to find referenced texture: {}",
    //         "test" // base_2d.texture
    //     )),
    //     &[],
    // );
    render_pass.set_bind_group(2, &camera_2d_uniforms_group.bind_group, &[]);
    render_pass.set_bind_group(3, &lighting_2d_uniforms_group.bind_group, &[]);

    render_pass.set_vertex_buffer(0, state.common_vertex_buffers[0].buffer.slice(..));
    render_pass.set_index_buffer(
        state.common_index_buffers[0].buffer.slice(..),
        wgpu::IndexFormat::Uint16,
    );

    // Dynamic bindings
    base_2d_uniforms_group.begin_dynamic_loading();

    let mut query = <(&Base2D, &Position2D)>::query();
    for (base_2d, _pos) in query.iter_mut(world) {
        render_pass.set_bind_group(
            1,
            &base_2d_bind_group,
            &[base_2d_uniforms_group.increase_offset(0)],
        );

        debug!("Recording draw call");
        render_pass.draw_indexed(
            0..state.common_index_buffers[base_2d.common_index_buffer].size,
            0,
            0..1,
        );
    }

    drop(render_pass);
    gpu.queue.submit(std::iter::once(encoder.finish()));
}
