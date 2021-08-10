use legion::{world::SubWorld, IntoQuery};
use std::{
    sync::{Arc, Mutex},
    time::Instant,
};
use uuid::Uuid;

use crate::{
    components::Position2D,
    constants::{
        CAMERA_2D_BIND_GROUP_ID, ID, LIGHTING_2D_BIND_GROUP_ID, RENDER_2D_COMMON_TEXTURE_ID,
        UNIT_SQUARE_IND_BUFFER_ID, UNIT_SQUARE_VRT_BUFFER_ID,
    },
    renderer::{
        graph::NodeState,
        instance::{Instance, InstanceGroup, InstanceGroupBinder},
        uniform::group::UniformGroup,
    },
};
use vertex_traits::*;

#[repr(C)]
#[derive(VertexLayout, Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Render2DInstance {
    pub model: [f32; 4],
    pub color: [f32; 4],
    pub id: u32,
}

impl Render2DInstance {
    pub fn new(x: f32, y: f32, w: f32, h: f32, color: [f32; 4]) -> Self {
        Self {
            model: [x, y, w, h],
            color,
            id: 0,
        }
    }

    pub fn default_group() -> InstanceGroup<Render2DInstance> {
        InstanceGroup::new(
            ID(RENDER_2D_COMMON_TEXTURE_ID),
            (ID(UNIT_SQUARE_VRT_BUFFER_ID), ID(UNIT_SQUARE_IND_BUFFER_ID)),
        )
    }

    pub fn update_position(&mut self, pos: &Position2D) {
        self.model[0] = pos.x;
        self.model[1] = pos.y;
    }
}

impl Instance for Render2DInstance {
    fn get_id(&self) -> u32 {
        self.id
    }

    fn set_id(&mut self, id: u32) {
        self.id = id
    }
}

// Phantom type
pub struct Render2DUniformGroup {}

#[system]
#[write_component(Render2DInstance)]
#[read_component(Position2D)]
pub fn load(world: &mut SubWorld) {
    debug!("running system render_2d_uniforms");

    <(&mut Render2DInstance, &Position2D)>::query().par_for_each_mut(world, |(instance, pos)| {
        instance.update_position(pos);
    });
}

// Draw all Render2D components //
//
// REVELATION:
// the separation of load_system and render_system is only relevant for the dynamic node.
// for instancing, loading needs to happen before each instanced group is drawn; otherwise,
// we could only render 1 group of instanced entities per pass. In fact, right now the dynamic
// node can only render 1 group of dynamic entities per pass. Anyways, I'll fix the dynamic
// node later; for now, I'm focusing on instancing.
//
// So, the render system needs to:
//  - Go through each instance group, load the uniform group's buffer with all the instances
//  - Bind the instance group-specific things such as textures and v/i buffers
//  - Bind a slice of each instance group's buffer before drawing that group
//
// Render system needs access to:
//  - (ONE) uniform group instance buffer (from NodeState)
//  - (MANY) instance group struct (from registry resource)
//  - (MANY) !!vector of every single instance!! (from instance group struct)
//
// So, on init, users should request all their necessary instance groups (they'll give or receive IDs)
//  (and pass in the texture + common v/i buffers)
//  - For each one requested, we need to:
//      - create a new instance group struct
//      - add this struct to some master registry resource
//    !!! only one instance buffer should be created per node/pipeline !!!
//
// Adding instances: AUTOMATIC ADDING CAN COME LATER
//   Users should be able to request "Render2DInstanceRef" components from the Render2D InstanceGroup within the registry resource at any time
//   The function takes ownership of the user's Render2DInstance builder, building the instance and inserting it into a vector.
//   The user is given a Render2DInstanceRef whose u64 ID which matches the instance's ID in the instance group's vector.
//
// Modifying instances:
//   In order to modify instances, users can request use a concurrent_update closure,
//   which uses some rayon parallelism to mutably iterate over all the instances in the group.
//
// Deleting instances:
//   - Instances can be deleted as part of parallelism, via some mutable bool ref passed in the concurrent closure?
//   - Instances can be deleted by ID via the instance group (less efficient obviously)
//   In both cases, Vec.swap_remove is used for O(1) performance, although when deleting by ID, the vector must be searched.
//
#[system]
#[read_component(InstanceGroup<Render2DInstance>)]
pub fn render(
    world: &mut SubWorld,
    #[state] state: &mut NodeState,
    #[resource] uniform_group: &Arc<Mutex<UniformGroup<Render2DUniformGroup>>>,
    #[resource] device: &Arc<wgpu::Device>,
    #[resource] queue: &Arc<wgpu::Queue>,
) {
    let start_time = Instant::now();
    debug!("running system render_2d_forward_instance (graph node)");
    let node = Arc::clone(&state.node);

    let render_target = state.render_target.lock().unwrap();
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("render_2d_forward_instance_encoder"),
    });
    let mut pass = render_target
        .create_render_pass(&mut encoder, "render_2d_forward_instance_pass")
        .unwrap();
    pass.set_pipeline(&node.pipeline);

    // Global bindings
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

    // Instance group bindings
    for group in <&InstanceGroup<Render2DInstance>>::query().iter(world) {
        debug!(
            "loading render_2d instance group {}, size: {}",
            "",
            group.num_instances()
        );

        uniform_group
            .lock()
            .unwrap()
            .load_buffer(0, group.buffer_bytes());

        pass.set_bind_group(0, &node.binder.texture_groups[&group.texture()], &[]);
        pass.set_bind_group(
            1,
            &node.binder.uniform_groups[&uniform_group.lock().unwrap().id],
            &[],
        );

        pass.set_vertex_buffer(0, state.common_buffers[&group.geometry().0].0.slice(..));
        pass.set_index_buffer(
            state.common_buffers[&group.geometry().1].0.slice(..),
            wgpu::IndexFormat::Uint16,
        );

        pass.draw_indexed(
            0..state.common_buffers[&group.geometry().1].1,
            0,
            0..group.num_instances() as _,
        );
    }

    debug!("done recording; submitting render pass");
    drop(pass);
    queue.submit(std::iter::once(encoder.finish()));

    debug!("render_2d_forward_instance pass submitted");
    state.reporter.update(start_time.elapsed().as_secs_f64());
}
