use cgmath::Matrix2;
use legion::world::SubWorld;
use legion::IntoQuery;
use std::time::Instant;
use std::{
    str::FromStr,
    sync::{Arc, Mutex},
};
use uuid::Uuid;

use crate::{
    components::Position2D,
    constants::{
        CAMERA_2D_BIND_GROUP_ID, ID, LIGHTING_2D_BIND_GROUP_ID, RENDER_2D_BIND_GROUP_ID,
        RENDER_2D_COMMON_TEXTURE_ID, UNIT_SQUARE_IND_BUFFER_ID, UNIT_SQUARE_VRT_BUFFER_ID,
    },
    render::{
        buffer::{IndexBuffer, VertexBuffer},
        graph::NodeState,
        uniform::{generic::GenericUniform, group::UniformGroup, Uniform},
    },
    system::render_2d::Render2D,
};

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Render2DInstance {
    model: [f32; 4],
    color: [f32; 4],
}

impl Render2DInstance {
    pub fn new(x: f32, y: f32, w: f32, h: f32, color: [f32; 4]) -> Self {
        Self {
            model: [x, y, w, h],
            color,
        }
    }

    pub fn update_position(&mut self, pos: &Position2D) {
        self.model[0] = pos.x;
        self.model[1] = pos.y;
    }
}

// Phantom type
pub struct Render2DInstanceGroup {}

#[system]
#[write_component(Render2DInstance)]
#[read_component(Position2D)]
pub fn load(
    world: &mut SubWorld,
    #[resource] uniform_group: &Arc<Mutex<UniformGroup<Render2DInstanceGroup>>>,
) {
    debug!("running system render_2d_uniforms");

    <(&mut Render2DInstance, &Position2D)>::query().par_for_each_mut(world, |(instance, pos)| {
        instance.update_position(pos);
    });

    let instances: Vec<Render2DInstance> = <(&Render2DInstance, &Position2D)>::query()
        .iter(world)
        .map(|(instance, _)| instance.to_owned())
        .collect();

    uniform_group
        .lock()
        .unwrap()
        .load_buffer(0, bytemuck::cast_slice(&instances));
    *uniform_group.lock().unwrap().entity_count.lock().unwrap() = instances.len() as u64;

    debug!(
        "done loading render_2d instances with {} dynamic entities",
        instances.len()
    );
}

// Draw all Render2D components //

// PLAN FOR ARBITRARY NUMBER OF INSTANCE GROUPS:
//  - All instances are stored in the uniform groups
//  - Each group has a unique type, e.g. InstanceGroup<Render2DInstance>
//  - Use a rayon parallel iterator when modifying the instances
//  - Vec can then be cast to bytes without the additional copy,
//    while still being concurrent.
//  - The only con to this approach is that I cannot have individual entities also be instances
//    This seems like a pretty big con, no? Need to think about this more.
//
// If I want each instance to be a component:
//  - How do users access the components of their instance group? => phantom type
//  - How does render_2d::load_system access the render_2d component of each instance group?
//      It needs to: load each instance group's buffer (which is in that instance group's uniform group) with a
//      byte slice of a vector that holds every instance in that instance group
//  - How does render_2d::render_system access the instance buffer of each instance group?
//      It needs to bind a slice of each instance group's buffer before drawing that group
//
// REVELATION:
//
// the separation of load_system and render_system is only relevant for the dynamic node.
// for instancing, loading needs to happen before each instanced group is drawn; otherwise,
// we could only render 1 group of instanced entities per pass. In fact, right now the dynamic
// node can only render 1 group of dynamic entities per pass. Anyways, I'll fix the dynamic
// node later; for now, I'm focusing on instancing.

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
    let mut pass_handle = render_target
        .create_render_pass(&mut encoder, "forward_render_2d")
        .unwrap();

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

    let entity_count = 10;

    // let mut dyn_offset_state = std::iter::repeat(0)
    //     .take(group_info.len())
    //     .collect::<Vec<u32>>();

    for _ in 0..*entity_count.lock().unwrap() {
        pass_handle.set_bind_group(
            0,
            &node.binder.texture_groups[&ID(RENDER_2D_COMMON_TEXTURE_ID)],
            &[],
        );

        pass_handle.set_bind_group(
            1,
            &node.binder.uniform_groups[&ID(RENDER_2D_BIND_GROUP_ID)],
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

    debug!("forward_render_2d pass submitted");
    state.reporter.update(start_time.elapsed().as_secs_f64());
}
