use std::sync::{Arc, Mutex};

use crate::renderer::{graph::RenderGraph, GpuState};

#[system]
pub fn begin_render_graph(
    #[resource] gpu: &Arc<Mutex<GpuState>>,
    #[resource] graph: &Arc<RenderGraph>,
) {
    debug!("running system begin_render_graph");
    let mut gpu = gpu.lock().unwrap();
    match gpu.swap_chain.get_current_frame() {
        Ok(frame) => graph
            .swap_chain_target
            .lock()
            .unwrap()
            .set_swap_chain(Arc::new(frame.output)),
        Err(err) => {
            warn!("failed to get swapchain frame: {}", err);
            warn!("cannot draw to any windows, attempting to recreate swapchain");
            // gpu.force_new_swap_chain();
        }
    }
}

#[system]
pub fn end_render_graph(#[resource] graph: &Arc<RenderGraph>) {
    debug!("running system end_render_graph");
    graph.swap_chain_target.lock().unwrap().release_swap_chain();
}
