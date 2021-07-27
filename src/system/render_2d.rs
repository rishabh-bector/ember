use legion::{world::SubWorld, IntoQuery};
use std::{
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
        GpuState, RenderPass,
    },
    system::{base_2d::*, camera_2d::*, lighting_2d::*},
};

pub struct Render2DSystem {
    pub common_vertex_buffers: [VertexBuffer; 1],
    pub common_index_buffers: [IndexBuffer; 1],
}

// Draw all Base2D components //

pub type Base2DRenderPass = ();

#[system]
pub fn forward_render_2d_NEW(
    #[state] state: &NodeState,
    #[resource] render_pass: &Arc<Mutex<RenderPass<Base2DRenderPass>>>,
) {
    let mut render_pass = render_pass.lock().unwrap();
    let node = Arc::clone(&render_pass.node);
    let master = Arc::clone(&render_pass.master);

    let num_dynamic = *render_pass
        .num_dynamic
        .get(&ID(BASE_2D_COMMON_TEXTURE_ID))
        .unwrap();

    let frame_view = match master.as_ref() {
        Some(swap_chain_texture) => &swap_chain_texture.view,
        None => &state.output_target.view,
    };

    let mut pass_handle = render_pass
        .encoder
        .begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Render2D Pass"),
            color_attachments: &[wgpu::RenderPassColorAttachment {
                view: frame_view,
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

    let dyn_offset_info = node
        .binder
        .dyn_offset_info
        .get(&ID(BASE_2D_BIND_GROUP_ID))
        .unwrap();

    let mut dyn_offset_state = std::iter::repeat(0)
        .take(dyn_offset_info.len())
        .collect::<Vec<u32>>();

    for _ in 0..num_dynamic {
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
            dyn_offset_state[i] += dyn_offset_info[i].0 as u32;
        }
    }

    // let mut query = <(&Base2D, &Position2D)>::query();
    // query.for_each(world, |(base_2d, _pos)| {
    //     pass_handle.set_bind_group(0, &state.bindings.groups[&base_2d.texture], &[]);

    //     pass_handle.set_bind_group(
    //         1,
    //         &base_2d_bind_group,
    //         &[base_2d_uniforms_group.increase_offset(0)],
    //     );

    //     debug!("Recording draw call");
    //     pass_handle.draw_indexed(
    //         0..state.common_index_buffers[base_2d.common_index_buffer].size,
    //         0,
    //         0..1,
    //     );
    // });

    // for (base_2d, _pos) in query.iter(world) {}

    // drop(pass_handle);
    // gpu.queue.submit(std::iter::once(encoder.finish()));
}

// #[system]
// #[read_component(Base2D)]
// #[read_component(Position2D)]
// pub fn forward_render_2d(
//     world: &SubWorld,
//     #[state] state: &Render2DSystem,
//     #[resource] gpu: &Arc<Mutex<GpuState>>,
//     //#[resource] base_2d_uniforms_group: &Arc<Mutex<UniformGroup<Base2DUniformGroup>>>,
//     //#[resource] camera_2d_uniforms_group: &Arc<Mutex<UniformGroup<Camera2DUniformGroup>>>,
//     //#[resource] lighting_2d_uniforms_group: &Arc<Mutex<UniformGroup<Lighting2DUniformGroup>>>,
// ) {
//     let gpu = gpu.lock().unwrap();
//     // let mut base_2d_uniforms_group = base_2d_uniforms_group.lock().unwrap();
//     // let camera_2d_uniforms_group = camera_2d_uniforms_group.lock().unwrap();
//     // let lighting_2d_uniforms_group = lighting_2d_uniforms_group.lock().unwrap();

//     // let base_2d_bind_group = base_2d_uniforms_group.bind_group();

//     let frame = gpu.swap_chain.get_current_frame().unwrap().output;
//     let mut encoder = gpu
//         .device
//         .create_command_encoder(&wgpu::CommandEncoderDescriptor {
//             label: Some("Render2D Encoder"),
//         });

//     let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
//         label: Some("Render2D Pass"),
//         color_attachments: &[wgpu::RenderPassColorAttachment {
//             view: &frame.view,
//             resolve_target: None,
//             ops: wgpu::Operations {
//                 load: wgpu::LoadOp::Clear(wgpu::Color {
//                     r: 0.0,
//                     g: 0.0,
//                     b: 0.0,
//                     a: 0.0,
//                 }),
//                 store: true,
//             },
//         }],
//         depth_stencil_attachment: None,
//     });

//     // Common bindings
//     // render_pass.set_pipeline(&gpu.pipelines[0].pipeline);

//     render_pass.set_bind_group(2, &camera_2d_uniforms_group.bind_group, &[]);
//     render_pass.set_bind_group(3, &lighting_2d_uniforms_group.bind_group, &[]);

//     render_pass.set_vertex_buffer(0, state.common_vertex_buffers[0].buffer.slice(..));
//     render_pass.set_index_buffer(
//         state.common_index_buffers[0].buffer.slice(..),
//         wgpu::IndexFormat::Uint16,
//     );

//     // Dynamic bindings
//     base_2d_uniforms_group.begin_dynamic_loading();

//     let mut query = <(&Base2D, &Position2D)>::query();
//     query.for_each(world, |(base_2d, _pos)| {
//         render_pass.set_bind_group(0, &state.bindings.groups[&base_2d.texture], &[]);

//         render_pass.set_bind_group(
//             1,
//             &base_2d_bind_group,
//             &[base_2d_uniforms_group.increase_offset(0)],
//         );

//         debug!("Recording draw call");
//         render_pass.draw_indexed(
//             0..state.common_index_buffers[base_2d.common_index_buffer].size,
//             0,
//             0..1,
//         );
//     });

//     for (base_2d, _pos) in query.iter(world) {}

//     drop(render_pass);
//     gpu.queue.submit(std::iter::once(encoder.finish()));
// }
