use legion::{world::SubWorld, IntoQuery};
use std::{
    borrow::Borrow,
    marker::PhantomData,
    str::FromStr,
    sync::{Arc, Mutex},
};
use uuid::Uuid;

use crate::{
    components::Position2D,
    constants::{
        BASE_2D_BIND_GROUP_ID, BASE_2D_COMMON_TEXTURE_ID, CAMERA_2D_BIND_GROUP_ID, ID,
        LIGHTING_2D_BIND_GROUP_ID, UNIT_SQUARE_IND_BUFFER_ID, UNIT_SQUARE_VRT_BUFFER_ID,
    },
    render::{
        buffer::{IndexBuffer, VertexBuffer},
        graph::{NodeState, RenderGraph},
        texture::Texture,
        uniform::UniformGroup,
        GpuState,
    },
    system::{base_2d::*, camera_2d::*, lighting_2d::*},
};

pub struct Render2DSystem {
    pub common_vertex_buffers: [VertexBuffer; 1],
    pub common_index_buffers: [IndexBuffer; 1],
}

// Draw all Base2D components //

pub type Base2DRenderNode = ();

#[system]
pub fn forward_render_2d(
    #[state] state: &NodeState,
    #[resource] device: &Arc<wgpu::Device>, // read only
    #[resource] queue: &Arc<wgpu::Queue>,   // read only
) {
    debug!("running system forward_render_2d (graph node)");

    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("Render2D Encoder"),
    });

    let mut pass_handle = state
        .render_target
        .create_render_pass(&mut encoder)
        .unwrap();

    let node = Arc::clone(&state.node);
    pass_handle.set_pipeline(&node.pipeline);
    pass_handle.set_bind_group(
        2,
        &node.binder.uniform_groups[&ID(CAMERA_2D_BIND_GROUP_ID)],
        &[],
    );
    pass_handle.set_bind_group(
        3,
        &node.binder.uniform_groups[&ID(LIGHTING_2D_BIND_GROUP_ID)],
        &[],
    );

    pass_handle.set_vertex_buffer(
        0,
        state.common_buffers[&ID(UNIT_SQUARE_VRT_BUFFER_ID)]
            .0
            .slice(..),
    );
    pass_handle.set_index_buffer(
        state.common_buffers[&ID(UNIT_SQUARE_IND_BUFFER_ID)]
            .0
            .slice(..),
        wgpu::IndexFormat::Uint16,
    );

    // Dynamic bindings

    let (entity_count, group_info) = node
        .binder
        .dyn_offset_state
        .get(&ID(BASE_2D_BIND_GROUP_ID))
        .unwrap();

    let mut dyn_offset_state = std::iter::repeat(0)
        .take(group_info.len())
        .collect::<Vec<u32>>();

    for _ in 0..*entity_count.lock().unwrap() {
        pass_handle.set_bind_group(
            0,
            &node.binder.texture_groups[&ID(BASE_2D_COMMON_TEXTURE_ID)],
            &[],
        );

        pass_handle.set_bind_group(
            1,
            &node.binder.uniform_groups[&ID(BASE_2D_BIND_GROUP_ID)],
            &dyn_offset_state,
        );

        pass_handle.draw_indexed(
            0..state.common_buffers[&ID(UNIT_SQUARE_IND_BUFFER_ID)].1,
            0,
            0..1,
        );

        for i in 0..dyn_offset_state.len() {
            dyn_offset_state[i] += group_info[i].0 as u32;
        }
    }

    debug!("done recording; submitting render pass");
    drop(pass_handle);
    queue.submit(std::iter::once(encoder.finish()));
    debug!("pass submitted");
}

pub fn create_render_pass<'a>(
    target: &'a wgpu::TextureView,
    encoder: &'a mut wgpu::CommandEncoder,
) -> wgpu::RenderPass<'a> {
    encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: Some("Render2D Pass"),
        color_attachments: &[wgpu::RenderPassColorAttachment {
            view: target,
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
    })
}
