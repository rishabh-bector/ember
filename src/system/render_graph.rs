use std::{
    collections::HashMap,
    marker::PhantomData,
    sync::{Arc, Mutex},
};

use crate::render::{graph::RenderGraph, GpuState};

use super::render_2d::Base2DRenderNode;

#[system]
pub fn begin_render_graph(
    #[resource] gpu: &Arc<Mutex<GpuState>>,
    #[resource] graph: &Arc<RenderGraph>,
) {
    debug!("running system begin_render_graph");
    let gpu = gpu.lock().unwrap();
    *graph.swap_chain_target.lock().unwrap() =
        Some(gpu.swap_chain.get_current_frame().unwrap().output);
}

#[system]
pub fn end_render_graph(#[resource] graph: &Arc<RenderGraph>) {
    debug!("running system end_render_graph");
    // release lock on swap chain so that buffer can
    // be drawn to window
    *graph.swap_chain_target.lock().unwrap() = None;
}
