use std::{
    collections::HashMap,
    marker::PhantomData,
    sync::{Arc, Mutex},
};

use crate::render::{graph::RenderGraph, GpuState, RenderPass};

use super::render_2d::Base2DRenderPass;

// This system starts the render graph on each frame, by creating all the
// RenderPass{} entities. This system starts running AFTER init--all shared
// targets should already be held by RenderGraph.
//
// Thus, each render system will have a state with an Arc<Target>, and will access
// a resource with:
//  - encoder
//  - pipeline (including binder w/ uniform and texture bind groups)
#[system]
pub fn begin_render_graph(
    #[resource] gpu: &Arc<Mutex<GpuState>>,
    #[resource] graph: &Arc<RenderGraph>,
) {
    let gpu = gpu.lock().unwrap();
    let frame = Arc::new(Some(gpu.swap_chain.get_current_frame().unwrap().output));

    let mut encoder = gpu
        .device
        .create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Render2D Encoder"),
        });

    // TODO AFTER SMOK:
    //  - Instead of "for pipeline in gpu.pipelines", this should
    //    be "for render_pass in graph.nodes"
    //  - The amount of targets created in mod.rs should equal
    //    the number of source passes + the number of assembly passes
    //  - A graph node can be one of ""

    // .add_system(source),     targets = post_1.0, post_2.0    resource:
    // .add_system(post_1),     targets = merge.0               resource:
    // .add_system(post_2),     targets = merge.1               resource:
    // .add_system(merge),      targets = master.0              resource: merge.0, merge.1
    // .add_system(master),     targets = output_target.0       resource:

    // WHAT I'LL HAVE TO DO:
    //      EACH RENDER PASS SYSTEM HAS A STATE OF Arc<TextureView> aka a target
    //      targets are created on startup based on the render graph
    //      render graph schedules all pass systems by determining
    //          - how many targets are needed in total
    //          - where flushes() need to be inserted
    //      render graph therefore needs a complete representation of the graph.
    //      YAML? Nah not for now, but all the other shit is top notch
    //
    // info in one render node:
    //  - input node refs, to access texture bind groups to use as input uniforms
    //  - render pass, aka:
    //  // a way to schedule a system which takes some input texture bind groups as state
    //
    // In the end, I want the render graph to add a sequence of multiple systems and flushes
    // to the schedule, which in total will:
    //  ON INIT:
    //  - create all target textures required for a single graph run
    //  ON FRAME:
    //  - one system to start the render graph by creating all the RenderPass{} entities which
    //    have encoders
    //  - these Arc<Texture>(s) will be created beforehand, and RenderGraph will also
    //    assign each node a target

    for (id, node) in &graph.nodes {
        let pass = RenderPass::<Base2DRenderPass> {
            node: Arc::clone(node),
            num_dynamic: HashMap::new(),
            encoder: gpu
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("Render2D Encoder"),
                }),
            master: match node.master {
                false => Arc::new(None),
                true => Arc::clone(&frame),
            },
            _marker: PhantomData,
        };
    }
}
