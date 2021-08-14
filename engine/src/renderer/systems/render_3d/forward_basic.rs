use legion::{component, systems::CommandBuffer, world::SubWorld, Entity};
use std::{
    sync::{Arc, Mutex, MutexGuard},
    time::Instant,
};
use uuid::Uuid;

use crate::{
    components::Position3D,
    constants::{
        CAMERA_2D_BIND_GROUP_ID, CAMERA_3D_BIND_GROUP_ID, ID, LIGHTING_2D_BIND_GROUP_ID,
        RENDER_2D_COMMON_TEXTURE_ID, RENDER_3D_COMMON_TEXTURE_ID, UNIT_CUBE_IND_BUFFER_ID,
        UNIT_CUBE_VRT_BUFFER_ID, UNIT_SQUARE_IND_BUFFER_ID, UNIT_SQUARE_VRT_BUFFER_ID,
    },
    legion::IntoQuery,
    renderer::{
        graph::NodeState,
        uniform::{
            generic::GenericUniform,
            group::{
                GroupBuilder, GroupState, SingleStateBuilder, UniformGroup, UniformGroupBuilder,
            },
            Uniform,
        },
    },
};

// Todo: go through all todo comments and make tickets for them
// Todo: remove unnecessary builders

pub struct Render3D {
    pub name: String,

    // Todo: make these into a "BasicMaterial" component
    pub color: [f32; 4],
    pub texture: Uuid,
    pub mix: f32,

    // Todo: make these into a "Geometry" component,
    // with a global(?) store
    pub common_vertex_buffer: Uuid,
    pub common_index_buffer: Uuid,

    pub uniforms: Render3DUniforms,
}

impl Render3D {
    pub fn default(name: &str) -> Self {
        Self {
            name: name.to_owned(),
            color: [1.0, 1.0, 1.0, 1.0],
            texture: ID(RENDER_3D_COMMON_TEXTURE_ID),
            mix: 1.0,
            uniforms: Default::default(),
            common_vertex_buffer: ID(UNIT_CUBE_VRT_BUFFER_ID),
            common_index_buffer: ID(UNIT_CUBE_IND_BUFFER_ID),
        }
    }
}

#[repr(C)]
#[derive(Debug, Default, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Render3DUniforms {
    pub model: [[f32; 4]; 4],
    pub color: [f32; 4],
    pub mix: f32,
}

impl Uniform for Render3D {
    fn write_buffer(&self, queue: &wgpu::Queue, buffer: &wgpu::Buffer) {
        queue.write_buffer(
            buffer,
            0,
            bytemuck::cast_slice(&[Render3DUniforms::from(self)]),
        );
    }
}

impl From<&Render3D> for Render3DUniforms {
    fn from(render_3d: &Render3D) -> Self {
        Self {
            model: [
                [1.0, 0.0, 0.0, 0.0],
                [0.0, 1.0, 0.0, 0.0],
                [0.0, 0.0, 1.0, 0.0],
                [0.0, 0.0, 0.0, 1.0],
            ],
            color: render_3d.color,
            mix: render_3d.mix,
        }
    }
}

// Phantom type
pub struct Render3DForwardUniformGroup {}

#[system]
#[read_component(Render3D)]
#[read_component(Position3D)]
#[read_component(GroupState)]
pub fn load(
    world: &mut SubWorld,
    command_buffer: &mut CommandBuffer,
    #[resource] device: &Arc<wgpu::Device>,
    #[resource] group_builder: &Arc<Mutex<UniformGroupBuilder<Render3DForwardUniformGroup>>>,
) {
    // Add a GroupState to any Render3D component without one
    let group_builder = group_builder.lock().unwrap();
    let mut query = <(Entity, &Render3D, &Position3D)>::query().filter(!component::<GroupState>());
    query.for_each(world, |(entity, builder_3d, pos_3d)| {
        debug!("allocating uniform group state for new render_3d component");
        command_buffer.add_component(*entity, group_builder.single_state(&device).unwrap());
    });

    // Load all Render3D components into their GroupStates
    let mut query = <(&Render3D, &Position3D, &GroupState)>::query();
    query.par_for_each(world, |(render_3d, pos_3d, group_state)| {
        let source = &[Render3DUniforms::from(render_3d)];
        group_state.write_buffer(0, bytemuck::cast_slice(source));
    });
}

#[system]
#[read_component(Render3D)]
#[read_component(Position3D)]
#[read_component(GroupState)]
pub fn render(
    world: &mut SubWorld,
    #[state] state: &mut NodeState,
    #[resource] device: &Arc<wgpu::Device>,
    #[resource] queue: &Arc<wgpu::Queue>,
) {
    debug!("running system forward_render_3d (graph node)");
    let start_time = Instant::now();
    let node = Arc::clone(&state.node);

    let render_target = state.render_target.lock().unwrap();
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("Render3D Encoder"),
    });
    let mut pass = render_target
        .create_render_pass(&mut encoder, "forward_render_3d", true)
        .unwrap();
    pass.set_pipeline(&node.pipeline);

    pass.set_bind_group(
        2,
        &node.binder.uniform_groups[&ID(CAMERA_3D_BIND_GROUP_ID)],
        &[],
    );
    // pass.set_bind_group(
    //     3,
    //     &node.binder.uniform_groups[&ID(LIGHTING_2D_BIND_GROUP_ID)],
    //     &[],
    // );

    let mut query = <(&Render3D, &GroupState)>::query();
    for (render_3d, group_state) in query.iter(world) {
        pass.set_bind_group(0, &node.binder.texture_groups[&render_3d.texture], &[]);
        pass.set_bind_group(1, &group_state.bind_group, &[]);

        pass.set_vertex_buffer(
            0,
            state.common_buffers[&render_3d.common_vertex_buffer]
                .0
                .slice(..),
        );
        pass.set_index_buffer(
            state.common_buffers[&render_3d.common_index_buffer]
                .0
                .slice(..),
            wgpu::IndexFormat::Uint16,
        );

        pass.draw_indexed(
            0..state.common_buffers[&render_3d.common_index_buffer].1,
            0,
            0..1,
        );
    }

    debug!("done recording; submitting render pass");
    drop(pass);
    queue.submit(std::iter::once(encoder.finish()));

    debug!("forward_render_3d pass submitted");
    state.reporter.update(start_time.elapsed().as_secs_f64());
}
