use legion::{world::SubWorld, IntoQuery};
use rayon::iter::{IntoParallelRefMutIterator, ParallelIterator};
use std::{
    sync::{Arc, RwLock},
    time::Instant,
};

use crate::{
    components::{FrameMetrics, Position2D},
    constants::{
        CAMERA_2D_BIND_GROUP_ID, DEFAULT_SCREEN_HEIGHT, DEFAULT_SCREEN_WIDTH, ID,
        LIGHTING_2D_BIND_GROUP_ID, QUAD_BIND_GROUP_ID, RENDER_2D_COMMON_TEXTURE_ID,
    },
    renderer::{
        buffer::instance::{Instance, InstanceBuffer, InstanceGroup, InstanceGroupBinder},
        graph::NodeState,
        mesh::Mesh,
        uniform::{
            generic::GenericUniformBuilder,
            group::{GroupState, UniformGroup, UniformGroupBuilder, UniformGroupType},
        },
    },
    sources::registry::MeshRegistry,
};

// Resource
pub struct Quad {
    pub mesh: Mesh,
    pub uniforms: QuadUniforms,
    pub uniform_group: GroupState,
}

#[repr(C)]
#[derive(Debug, Default, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct QuadUniforms {
    pub dimensions: [f32; 2],
    pub time: f32,
    pub nut: f32,
}

pub struct QuadUniformGroup {}

impl UniformGroupType<Self> for QuadUniformGroup {
    fn builder() -> UniformGroupBuilder<QuadUniformGroup> {
        UniformGroup::<QuadUniformGroup>::builder()
            .with_uniform(GenericUniformBuilder::from_source(QuadUniforms {
                dimensions: [DEFAULT_SCREEN_WIDTH as f32, DEFAULT_SCREEN_HEIGHT as f32],
                time: 0.0,
                nut: 0.0,
            }))
            .with_id(ID(QUAD_BIND_GROUP_ID))
    }
}

#[system]
pub fn load(#[resource] quad: &mut Quad) {
    debug!("running system render_quad_uniform_loader (graph node)");
    quad.uniform_group
        .write_buffer(0, bytemuck::cast_slice(&[quad.uniforms]));
}

#[system]
pub fn render(
    #[state] state: &mut NodeState,
    #[resource] quad: &Quad,
    #[resource] device: &Arc<wgpu::Device>,
    #[resource] queue: &Arc<wgpu::Queue>,
) {
    debug!("running system render_quad (graph node)");
    let start_time = Instant::now();
    let node = Arc::clone(&state.node);

    let render_target = state.render_target.lock().unwrap();
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("Quad Encoder"),
    });
    let mut pass = render_target
        .create_render_pass("quad_render", &mut encoder, true)
        .unwrap();
    pass.set_pipeline(&node.pipeline);

    pass.set_bind_group(0, &quad.uniform_group.bind_group, &[]);
    pass.set_vertex_buffer(0, quad.mesh.vertex_buffer.buffer.0.slice(..));
    pass.set_index_buffer(
        quad.mesh.index_buffer.buffer.0.slice(..),
        wgpu::IndexFormat::Uint32,
    );
    pass.draw_indexed(0..quad.mesh.index_buffer.buffer.1, 0, 0..1);

    debug!("done recording; submitting render pass");
    drop(pass);
    queue.submit(std::iter::once(encoder.finish()));

    debug!("quad_render pass submitted");
    state.reporter.update(start_time.elapsed().as_secs_f64());
}
