use cgmath::{Matrix, SquareMatrix};
use legion::{component, systems::CommandBuffer, world::SubWorld, Entity};
use std::{
    sync::{Arc, Mutex},
    time::Instant,
};
use uuid::Uuid;
use wgpu::BindGroup;

use crate::{
    components::Transform3D,
    constants::{
        CAMERA_3D_BIND_GROUP_ID, ID, IDENTITY_MATRIX_4, RENDER_3D_BIND_GROUP_ID,
        RENDER_3D_COMMON_TEXTURE_ID,
    },
    legion::IntoQuery,
    renderer::{
        buffer::texture::Texture,
        graph::NodeState,
        mesh::Mesh,
        uniform::{
            generic::GenericUniformBuilder,
            group::{
                GroupState, GroupStateBuilder, UniformGroup, UniformGroupBuilder, UniformGroupType,
            },
        },
    },
    sources::camera::Camera3D,
    systems::camera_3d::matrix2array_4d,
};

use super::render_3d::forward_basic::{Render3D, Render3DUniforms};

// Sky is a SINGLETON (one Render3D/Mesh/GroupState which is stored as a resource)

pub struct Sky {
    pub mesh: Mesh,
    pub r3d: Render3D,
    pub t3d: Transform3D,
    pub r3d_group: GroupState,
    pub cubemap: Arc<BindGroup>,
}

#[system]
pub fn update(#[resource] sky: &mut Sky, #[resource] camera: &Arc<Mutex<Camera3D>>) {
    debug!("running system update_sky");

    let cam_pos = camera.lock().unwrap().pos;
    sky.t3d.position = [cam_pos.x, cam_pos.y, cam_pos.z];

    let source = &[Render3DUniforms::from((&sky.r3d, &sky.t3d))];
    sky.r3d_group.write_buffer(0, bytemuck::cast_slice(source));
}

#[system]
pub fn render(
    #[state] state: &mut NodeState,
    #[resource] sky: &mut Sky,
    #[resource] device: &Arc<wgpu::Device>,
    #[resource] queue: &Arc<wgpu::Queue>,
) {
    debug!("running system render_sky (graph node)");
    let start_time = Instant::now();
    let node = Arc::clone(&state.node);

    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("Sky Encoder"),
    });

    let render_target = state.render_target();
    let render_target_mut = render_target.lock().unwrap();

    let pass_res = render_target_mut.create_render_pass("render_sky", &mut encoder, true);
    if pass_res.is_err() {
        warn!("no target, aborting render pass: render_sky");
        return;
    }

    let mut pass = pass_res.unwrap();
    pass.set_pipeline(&node.pipeline);

    pass.set_bind_group(0, &sky.r3d_group.bind_group, &[]);
    pass.set_bind_group(
        1,
        &node.binder.uniform_groups[&ID(CAMERA_3D_BIND_GROUP_ID)],
        &[],
    );
    pass.set_bind_group(2, &sky.cubemap, &[]);

    pass.set_vertex_buffer(0, sky.mesh.vertex_buffer.buffer.0.slice(..));
    pass.set_index_buffer(
        sky.mesh.index_buffer.buffer.0.slice(..),
        wgpu::IndexFormat::Uint32,
    );

    pass.draw_indexed(0..sky.mesh.index_buffer.buffer.1, 0, 0..1);

    debug!("done recording; submitting render pass");
    drop(pass);
    queue.submit(std::iter::once(encoder.finish()));

    debug!("render_sky pass submitted");
    state.reporter.update(start_time.elapsed().as_secs_f64());
}
