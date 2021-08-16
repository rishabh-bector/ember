use std::sync::{Arc, Mutex};

use crate::renderer::{graph::RenderGraph, GpuState};

#[system]
pub fn begin_render_graph(
    #[resource] gpu: &Arc<Mutex<GpuState>>,
    #[resource] graph: &Arc<RenderGraph>,
) {
    debug!("running system begin_render_graph");
    let gpu = gpu.lock().unwrap();
    graph
        .swap_chain_target
        .lock()
        .unwrap()
        .set_swap_chain(Arc::new(gpu.swap_chain.get_current_frame().unwrap().output));
}

#[system]
pub fn end_render_graph(#[resource] graph: &Arc<RenderGraph>) {
    debug!("running system end_render_graph");
    graph.swap_chain_target.lock().unwrap().release_swap_chain();
}
