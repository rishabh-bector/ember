use std::{sync::Arc, time::Instant};

use crate::renderer::{graph::NodeState, systems::quad::Quad};

#[system]
pub fn render(
    #[state] state: &mut NodeState,
    #[resource] quad: &Quad,
    #[resource] device: &Arc<wgpu::Device>,
    #[resource] queue: &Arc<wgpu::Queue>,
) {
    debug!("running system render_chain (graph node)");
    let start_time = Instant::now();
    let node = Arc::clone(&state.node);

    let render_target = state.cycle_target();
    let render_target_mut = render_target.lock().unwrap();

    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("chain Encoder"),
    });

    let pass_res = render_target_mut.create_render_pass("chain_render", &mut encoder, true);
    if pass_res.is_err() {
        warn!("no target, aborting render pass: chain_channel");
        return;
    }

    let mut pass = pass_res.unwrap();
    pass.set_pipeline(&node.pipeline);

    pass.set_bind_group(1, &quad.uniform_group.bind_group, &[]);

    // NODE INPUT
    pass.set_bind_group(0, &state.inputs[0].bind_group_ref(), &[]);

    pass.set_vertex_buffer(0, quad.mesh.vertex_buffer.buffer.0.slice(..));
    pass.set_index_buffer(
        quad.mesh.index_buffer.buffer.0.slice(..),
        wgpu::IndexFormat::Uint32,
    );
    pass.draw_indexed(0..quad.mesh.index_buffer.buffer.1, 0, 0..1);

    debug!("done recording; submitting render pass");
    drop(pass);
    queue.submit(std::iter::once(encoder.finish()));

    debug!("chain_render pass submitted");
    state.reporter.update(start_time.elapsed().as_secs_f64());
}
