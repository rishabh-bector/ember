use cgmath::{Angle, InnerSpace, Rad, Vector2};
use legion::{world::SubWorld, IntoQuery};
use rayon::{
    iter::{IntoParallelRefIterator, IntoParallelRefMutIterator, ParallelIterator},
    slice::ParallelSliceMut,
};
use std::{
    sync::{Arc, RwLock},
    time::Instant,
};
use uuid::Uuid;

use crate::{
    components::{Position2D, Velocity2D},
    constants::{
        CAMERA_2D_BIND_GROUP_ID, ID, LIGHTING_2D_BIND_GROUP_ID, PRIMITIVE_MESH_GROUP_ID,
        RENDER_2D_COMMON_TEXTURE_ID,
    },
    renderer::{
        buffer::instance::{
            Instance, InstanceBuffer, InstanceGroup, InstanceGroupBinder, InstanceId,
        },
        graph::NodeState,
        mesh::Mesh,
    },
    sources::{
        primitives::unit_square,
        registry::{MeshRegistry, TextureRegistry},
    },
};

#[instance((4, 44usize))]
#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Render2DInstance {
    pub model: [f32; 4],
    pub color: [f32; 4],
    pub mix: f32,
    pub group_id: u32,
    pub id: u32,
}

impl Render2DInstance {
    pub fn new(x: f32, y: f32, w: f32, h: f32, color: [f32; 4]) -> Self {
        Self {
            model: [x, y, w, h],
            color,
            mix: 1.0,
            group_id: 0,
            id: 0,
        }
    }

    pub fn new_default_group() -> InstanceGroup<Render2DInstance> {
        InstanceGroup::new(0, ID(RENDER_2D_COMMON_TEXTURE_ID))
    }

    pub fn update_position(&mut self, pos: &Position2D) {
        self.model[0] = pos.x;
        self.model[1] = pos.y;
    }
}

impl Default for Render2DInstance {
    fn default() -> Self {
        Self::new(0.0, 0.0, 10.0, 10.0, [1.0, 1.0, 1.0, 1.0])
    }
}

impl Instance for Render2DInstance {
    fn id(&self) -> (u32, u32) {
        (self.group_id, self.id)
    }

    fn set_id(&mut self, group_id: u32, inst_id: u32) {
        self.group_id = group_id;
        self.id = inst_id;
    }

    fn size() -> usize {
        36
    }
}

pub struct Attractor2D {
    pub force: f32,
}

pub struct Render2DUniformGroup {}

#[system]
#[write_component(InstanceGroup<Render2DInstance>)]
#[write_component(Mesh)]
pub fn load(world: &mut SubWorld) {
    debug!("running system render_2d_instance_loader");

    <(&mut InstanceGroup<Render2DInstance>, &Mesh)>::query().par_for_each_mut(
        world,
        |(group, _)| {
            let components = Arc::clone(&group.components);
            let mutators = components.read().unwrap();
            group.instances.par_iter_mut().for_each(|instance| {
                for component in &mutators[instance.id as usize] {
                    component.mutate(instance);
                }
            })
        },
    );

    // <(&InstanceId, &Position2D)>::query().par_for_each(world, |(inst_id, pos_2d)| {
    //     groups[inst_id.0 as usize].instances[inst_id.1 as usize].model[0] = pos_2d.x;
    //     groups[inst_id.0 as usize].instances[inst_id.1 as usize].model[1] = pos_2d.y;
    // });
}

#[system]
#[read_component(InstanceGroup<Render2DInstance>)]
#[read_component(Mesh)]
pub fn render(
    world: &SubWorld,
    #[state] state: &mut NodeState,
    #[resource] mesh_registry: &Arc<RwLock<MeshRegistry>>,
    #[resource] instance_buffer: &InstanceBuffer<Render2DInstance>,
    #[resource] device: &Arc<wgpu::Device>,
    #[resource] queue: &Arc<wgpu::Queue>,
) {
    let start_time = Instant::now();
    debug!("running system render_2d_forward_instance (graph node)");
    let node = Arc::clone(&state.node);
    let mesh_registry = mesh_registry.read().unwrap();

    let render_target = state.render_target.lock().unwrap();
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("render_2d_forward_instance_encoder"),
    });
    let mut pass = render_target
        .create_render_pass("render_2d_forward_instance_pass", &mut encoder, true)
        .unwrap();
    pass.set_pipeline(&node.pipeline);

    // Global bindings
    pass.set_bind_group(
        1,
        &node.binder.uniform_groups[&ID(CAMERA_2D_BIND_GROUP_ID)],
        &[],
    );
    pass.set_bind_group(
        2,
        &node.binder.uniform_groups[&ID(LIGHTING_2D_BIND_GROUP_ID)],
        &[],
    );

    for (group, mesh) in <(&InstanceGroup<Render2DInstance>, &Mesh)>::query().iter(world) {
        debug!(
            "rendering instance group => type: render_2d, name: {}, size: {}",
            "",
            group.num_instances()
        );

        // One instance buffer is managed per group type
        // (in this case: InstanceBuffer<Render2DInstance>)
        instance_buffer.load_group(group.buffer_bytes());

        // Every instance in a group shares the same texture and mesh
        pass.set_bind_group(0, &node.binder.texture_groups[&group.texture()], &[]);
        pass.set_vertex_buffer(0, mesh.vertex_buffer.buffer.0.slice(..));
        pass.set_index_buffer(
            mesh.index_buffer.buffer.0.slice(..),
            wgpu::IndexFormat::Uint16,
        );

        // Load and draw all instances in the group
        pass.set_vertex_buffer(1, instance_buffer.state.buffer.slice(..));
        pass.draw_indexed(
            0..mesh.index_buffer.buffer.1,
            0,
            0..group.num_instances() as _,
        );
    }

    debug!("done recording; submitting render pass");
    drop(pass);
    drop(mesh_registry);
    queue.submit(std::iter::once(encoder.finish()));

    debug!("render_2d_forward_instance pass submitted");
    state.reporter.update(start_time.elapsed().as_secs_f64());
}

// #[system]
// #[read_component(InstanceId)]
// #[read_component(Position2D)]
// #[read_component(Attractor2D)]
// #[write_component(Velocity2D)]
// pub fn attractor(world: &mut SubWorld) {
//     debug!("running system render_2d_instance_attractor");

//     let attractors: Vec<(f32, (f32, f32))> = <(&Attractor2D, &Position2D)>::query()
//         .iter(world)
//         .map(|(a, p)| (a.force, (p.x, p.y)))
//         .collect();

//     let mut query = <(&InstanceId, &Position2D, &mut Velocity2D)>::query();
//     query.par_for_each_mut(world, |(_inst_id, pos_2d, vel_2d)| {
//         for attractor in &attractors {
//             attractor_2d(attractor, pos_2d, vel_2d);
//         }
//     });
// }

// fn attractor_2d(attractor: &(f32, (f32, f32)), pos: &Position2D, vel: &mut Velocity2D) {
//     let line = Vector2::<f32>::new((attractor.1 .0) - pos.x, (attractor.1 .1) - pos.y);
//     let power = attractor.0 / line.magnitude2();
//     let theta: Rad<f32> = Angle::atan2(line.y, line.x);
//     let dvx = power * Angle::cos(theta);
//     let dvy = power * Angle::sin(theta);
//     // info!("DVVVVV {} {}", dvx, dvy);
//     vel.dx += dvx;
//     vel.dy += dvy;
// }

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
