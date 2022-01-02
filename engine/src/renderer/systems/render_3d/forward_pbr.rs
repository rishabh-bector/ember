use cgmath::{Matrix, SquareMatrix};
use legion::{component, systems::CommandBuffer, world::SubWorld, Entity};
use std::{
    sync::{Arc, Mutex},
    time::Instant,
};
use uuid::Uuid;

use crate::{
    components::Transform3D,
    constants::{
        CAMERA_3D_BIND_GROUP_ID, ID, IDENTITY_MATRIX_4, RENDER_3D_BIND_GROUP_ID,
        RENDER_3D_COMMON_TEXTURE_ID,
    },
    legion::IntoQuery,
    renderer::{
        graph::NodeState,
        mesh::Mesh,
        systems::sky::Sky,
        uniform::{
            generic::GenericUniformBuilder,
            group::{
                GroupState, GroupStateBuilder, UniformGroup, UniformGroupBuilder, UniformGroupType,
            },
        },
    },
    systems::camera_3d::matrix2array_4d,
};

// Todo: go through all todo comments and make tickets for them
// Todo: remove unnecessary builders

pub struct RenderPBR {
    pub name: String,

    // Todo: make these into a "BasicMaterial" component
    pub color: [f32; 4],
    pub texture: Uuid,
    pub mix: f32,
}

impl RenderPBR {
    pub fn default(name: &str) -> Self {
        Self {
            name: name.to_owned(),
            color: [1.0, 1.0, 1.0, 1.0],
            texture: ID(RENDER_3D_COMMON_TEXTURE_ID),
            mix: 0.0,
        }
    }

    pub fn colored(name: &str, color: [f32; 4]) -> Self {
        Self {
            name: name.to_owned(),
            color,
            texture: ID(RENDER_3D_COMMON_TEXTURE_ID),
            mix: 0.0,
        }
    }
}

#[repr(C)]
#[derive(Debug, Default, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct RenderPBRUniforms {
    pub model_mat: [[f32; 4]; 4],
    pub normal_mat: [[f32; 4]; 4],
    pub color: [f32; 4],
    pub mix: f32,
}

impl From<(&RenderPBR, &Transform3D)> for RenderPBRUniforms {
    fn from(entity: (&RenderPBR, &Transform3D)) -> Self {
        let model_mat = cgmath::Matrix4::from_translation(
            (
                entity.1.position[0],
                entity.1.position[1],
                entity.1.position[2],
            )
                .into(),
        ) * cgmath::Matrix4::from_angle_x(cgmath::Deg(entity.1.rotation[0]))
            * cgmath::Matrix4::from_angle_y(cgmath::Deg(entity.1.rotation[1]))
            * cgmath::Matrix4::from_angle_z(cgmath::Deg(entity.1.rotation[2]))
            * cgmath::Matrix4::from_nonuniform_scale(
                entity.1.scale[0],
                entity.1.scale[1],
                entity.1.scale[2],
            );

        let normal_mat = model_mat.invert().unwrap().transpose();

        Self {
            model_mat: matrix2array_4d(model_mat),
            normal_mat: matrix2array_4d(normal_mat),
            color: entity.0.color,
            mix: entity.0.mix,
        }
    }
}

pub struct RenderPBRForwardUniformGroup {}

impl UniformGroupType<Self> for RenderPBRForwardUniformGroup {
    fn builder() -> UniformGroupBuilder<RenderPBRForwardUniformGroup> {
        UniformGroup::<RenderPBRForwardUniformGroup>::builder()
            .with_uniform(GenericUniformBuilder::from_source(RenderPBRUniforms {
                model_mat: IDENTITY_MATRIX_4,
                normal_mat: IDENTITY_MATRIX_4,
                color: [1.0, 1.0, 1.0, 1.0],
                mix: 1.0,
            }))
            .with_id(ID(RENDER_3D_BIND_GROUP_ID))
    }
}

#[system]
#[read_component(RenderPBR)]
#[read_component(Transform3D)]
#[read_component(GroupState)]
pub fn load(
    world: &mut SubWorld,
    command_buffer: &mut CommandBuffer,
    #[resource] device: &Arc<wgpu::Device>,
    #[resource] queue: &Arc<wgpu::Queue>,
    #[resource] group_builder: &Arc<Mutex<GroupStateBuilder<RenderPBRForwardUniformGroup>>>,
) {
    debug!("running system render_3d_forward_basic_uniform_loader (graph node)");

    // Add a GroupState to any RenderPBR component without one
    let group_builder = group_builder.lock().unwrap();
    let mut query =
        <(Entity, &RenderPBR, &Transform3D)>::query().filter(!component::<GroupState>());
    query.for_each(world, |(entity, builder_3d, _)| {
        debug!(
            "allocating buffers for new render_3d component: {}",
            builder_3d.name
        );
        command_buffer.add_component(*entity, group_builder.single_state(device, queue).unwrap());
    });

    // Load all RenderPBR components into their GroupStates
    let mut query = <(&RenderPBR, &Transform3D, &GroupState)>::query();
    query.par_for_each(world, |(render_3d, transform_3d, group_state)| {
        debug!(
            "loading uniform group state for existing render_3d component: {}",
            render_3d.name
        );
        let source = &[RenderPBRUniforms::from((render_3d, transform_3d))];
        group_state.write_buffer(0, bytemuck::cast_slice(source));
    });
}

#[system]
#[read_component(RenderPBR)]
#[read_component(Mesh)]
#[read_component(GroupState)]
pub fn render(
    world: &mut SubWorld,
    #[state] state: &mut NodeState,
    #[resource] device: &Arc<wgpu::Device>,
    #[resource] queue: &Arc<wgpu::Queue>,
    #[resource] sky: &Sky,
) {
    debug!("running system render_forward_pbr (graph node)");
    let start_time = Instant::now();
    let node = Arc::clone(&state.node);

    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("RenderPBR Encoder"),
    });

    let render_target = state.render_target();
    let render_target_mut = render_target.lock().unwrap();

    let pass_res = render_target_mut.create_render_pass("forward_render_pbr", &mut encoder, false);
    if pass_res.is_err() {
        warn!("no target, aborting render pass: render_forward_pbr");
        return;
    }

    let mut pass = pass_res.unwrap();
    pass.set_pipeline(&node.pipeline);

    pass.set_bind_group(
        2,
        &node.binder.uniform_groups[&ID(CAMERA_3D_BIND_GROUP_ID)],
        &[],
    );
    pass.set_bind_group(3, &sky.cubemap, &[]);

    let cubemap_blur = sky.cubemap_blur.as_ref().unwrap();
    pass.set_bind_group(4, cubemap_blur, &[]);

    let mut query = <(&RenderPBR, &Mesh, &GroupState)>::query();
    for (render_pbr, mesh, group_state) in query.iter(world) {
        pass.set_bind_group(0, &node.binder.texture_groups[&render_pbr.texture], &[]);
        pass.set_bind_group(1, &group_state.bind_group, &[]);

        pass.set_vertex_buffer(0, mesh.vertex_buffer.buffer.0.slice(..));
        pass.set_index_buffer(
            mesh.index_buffer.buffer.0.slice(..),
            wgpu::IndexFormat::Uint32,
        );

        // info!(
        //     "RENDER 3D drawing entity with {} triangles",
        //     mesh.indices.len() / 3
        // );
        pass.draw_indexed(0..mesh.index_buffer.buffer.1, 0, 0..1);
    }

    debug!("done recording; submitting render pass");
    drop(pass);
    queue.submit(std::iter::once(encoder.finish()));

    debug!("forward_render_pbr pass submitted");
    state.reporter.update(start_time.elapsed().as_secs_f64());
}
