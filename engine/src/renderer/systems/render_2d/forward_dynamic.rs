use legion::{world::SubWorld, IntoQuery};
use std::{
    sync::{Arc, Mutex},
    time::Instant,
};

use crate::{
    components::Position2D,
    constants::{
        CAMERA_2D_BIND_GROUP_ID, ID, LIGHTING_2D_BIND_GROUP_ID, RENDER_2D_BIND_GROUP_ID,
        RENDER_2D_COMMON_TEXTURE_ID, UNIT_SQUARE_IND_BUFFER_ID, UNIT_SQUARE_VRT_BUFFER_ID,
    },
    renderer::{
        graph::NodeState,
        systems::render_2d::Render2D,
        uniform::{generic::GenericUniform, group::UniformGroup, Uniform},
    },
};

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Render2DForwardDynamicUniforms {
    pub model: [f32; 4],
    pub color: [f32; 4],
    pub mix: f32,
    pub _padding: [f32; 32],
    pub __padding: [f32; 23],
}

// Phantom type
pub struct Render2DForwardDynamicGroup {}

// TODO: Make this a macro?
#[system]
#[read_component(Render2D)]
#[read_component(Position2D)]
pub fn load(
    world: &mut SubWorld,
    #[resource] base_uniforms: &Arc<Mutex<GenericUniform<Render2DForwardDynamicUniforms>>>,
    #[resource] base_uniforms_group: &Arc<Mutex<UniformGroup<Render2DForwardDynamicGroup>>>,
) {
    debug!("running system render_2d_uniforms");

    let mut base_uniforms = base_uniforms.lock().unwrap();
    let mut base_uniforms_group = base_uniforms_group.lock().unwrap();

    let mut query = <(&Render2D, &Position2D)>::query();

    base_uniforms_group.begin_dynamic_loading();
    let mut count: u64 = 0;
    for (render_2d, pos) in query.iter_mut(world) {
        base_uniforms.mut_ref().model = [pos.x, pos.y, render_2d.width, render_2d.height];
        base_uniforms.mut_ref().color = render_2d.color;
        base_uniforms.mut_ref().mix = render_2d.mix;
        // base_uniforms_group.load_dynamic_uniform(base_uniforms.as_bytes());
        count += 1;
    }
    *base_uniforms_group.entity_count.lock().unwrap() = count;
    debug!(
        "done loading render_2d uniforms with {} dynamic entities",
        count
    );
}

// Draw all Render2D components //

#[system]
pub fn render(
    #[state] state: &mut NodeState,
    #[resource] device: &Arc<wgpu::Device>,
    #[resource] queue: &Arc<wgpu::Queue>,
) {
    let start_time = Instant::now();
    debug!("running system forward_render_2d (graph node)");
    let node = Arc::clone(&state.node);

    let render_target = state.render_target.lock().unwrap();
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("Render2D Encoder"),
    });
    let mut pass = render_target
        .create_render_pass(&mut encoder, "forward_render_2d", true)
        .unwrap();

    pass.set_pipeline(&node.pipeline);

    pass.set_bind_group(
        2,
        &node.binder.uniform_groups[&ID(CAMERA_2D_BIND_GROUP_ID)],
        &[],
    );
    pass.set_bind_group(
        3,
        &node.binder.uniform_groups[&ID(LIGHTING_2D_BIND_GROUP_ID)],
        &[],
    );

    pass.set_vertex_buffer(
        0,
        state.common_buffers[&ID(UNIT_SQUARE_VRT_BUFFER_ID)]
            .0
            .slice(..),
    );
    pass.set_index_buffer(
        state.common_buffers[&ID(UNIT_SQUARE_IND_BUFFER_ID)]
            .0
            .slice(..),
        wgpu::IndexFormat::Uint16,
    );

    // Dynamic bindings

    let (entity_count, group_info) = node
        .binder
        .dyn_offset_state
        .get(&ID(RENDER_2D_BIND_GROUP_ID))
        .unwrap();

    let mut dyn_offset_state = std::iter::repeat(0)
        .take(group_info.len())
        .collect::<Vec<u32>>();

    for _ in 0..*entity_count.lock().unwrap() {
        pass.set_bind_group(
            0,
            &node.binder.texture_groups[&ID(RENDER_2D_COMMON_TEXTURE_ID)],
            &[],
        );

        pass.set_bind_group(
            1,
            &node.binder.uniform_groups[&ID(RENDER_2D_BIND_GROUP_ID)],
            &dyn_offset_state,
        );

        pass.draw_indexed(
            0..state.common_buffers[&ID(UNIT_SQUARE_IND_BUFFER_ID)].1,
            0,
            0..1,
        );

        for i in 0..dyn_offset_state.len() {
            dyn_offset_state[i] += group_info[i].0 as u32;
        }
    }

    debug!("done recording; submitting render pass");
    drop(pass);
    queue.submit(std::iter::once(encoder.finish()));

    debug!("forward_render_2d pass submitted");
    state.reporter.update(start_time.elapsed().as_secs_f64());
}
