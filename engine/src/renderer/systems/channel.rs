use legion::{world::SubWorld, IntoQuery};
use rayon::iter::{IntoParallelRefMutIterator, ParallelIterator};
use std::{
    sync::{Arc, RwLock},
    time::Instant,
};

use crate::{
    components::{FrameMetrics, Position2D},
    constants::{
        CAMERA_2D_BIND_GROUP_ID, CAMERA_3D_BIND_GROUP_ID, ID, LIGHTING_2D_BIND_GROUP_ID,
        QUAD_BIND_GROUP_ID, RENDER_2D_COMMON_TEXTURE_ID,
    },
    renderer::{
        buffer::instance::{Instance, InstanceBuffer, InstanceGroup, InstanceGroupBinder},
        graph::NodeState,
        mesh::Mesh,
        systems::quad::Quad,
        uniform::{
            generic::GenericUniformBuilder,
            group::{GroupState, UniformGroup, UniformGroupBuilder, UniformGroupType},
        },
    },
    sources::{registry::MeshRegistry, WindowSize},
};

#[system]
pub fn render(
    #[state] state: &mut NodeState,
    #[resource] quad: &Quad,
    #[resource] device: &Arc<wgpu::Device>,
    #[resource] queue: &Arc<wgpu::Queue>,
) {
    debug!("running system render_channel (graph node)");
    let start_time = Instant::now();
    let node = Arc::clone(&state.node);

    let render_target = state.render_target.lock().unwrap();
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("Channel Encoder"),
    });

    let pass_res = render_target.create_render_pass("channel_render", &mut encoder, false);
    if pass_res.is_err() {
        warn!("no target, aborting render pass: render_channel");
        return;
    }

    let mut pass = pass_res.unwrap();
    pass.set_pipeline(&node.pipeline);

    pass.set_bind_group(1, &quad.uniform_group.bind_group, &[]);
    pass.set_bind_group(
        2,
        &node.binder.uniform_groups[&ID(CAMERA_3D_BIND_GROUP_ID)],
        &[],
    );

    // NODE INPUT
    pass.set_bind_group(0, &state.input_channels[0], &[]);

    pass.set_vertex_buffer(0, quad.mesh.vertex_buffer.buffer.0.slice(..));
    pass.set_index_buffer(
        quad.mesh.index_buffer.buffer.0.slice(..),
        wgpu::IndexFormat::Uint32,
    );
    pass.draw_indexed(0..quad.mesh.index_buffer.buffer.1, 0, 0..1);

    debug!("done recording; submitting render pass");
    drop(pass);
    queue.submit(std::iter::once(encoder.finish()));

    debug!("channel_render pass submitted");
    state.reporter.update(start_time.elapsed().as_secs_f64());
}
