use legion::{world::SubWorld, IntoQuery};
use std::{
    marker::PhantomData,
    sync::{Arc, Mutex},
};

use crate::{
    components::Position2D,
    render::{
        buffer::{IndexBuffer, VertexBuffer},
        texture::Texture,
        uniform::UniformGroup,
        GpuState, RenderPass,
    },
    systems::{base_2d::*, camera_2d::*, lighting_2d::*},
};

pub struct Render2DSystem {
    pub common_vertex_buffers: [VertexBuffer; 1],
    pub common_index_buffers: [IndexBuffer; 1],
}

// This system starts the render graph on each frame, by creating all the
// RenderPass{} entities. This system starts running AFTER init--all shared
// targets should already be held by RenderGraph.
//
// Thus, each render system will have a state with an Arc<Target>, and will access
// a resource with:
//  - encoder
//  - pipeline (including binder w/ uniform and texture bind groups)
#[system]
pub fn begin_render_graph(#[resource] gpu: &Arc<Mutex<GpuState>>) {
    let gpu = gpu.lock().unwrap();

    let frame = gpu.swap_chain.get_current_frame().unwrap().output;

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

    for pipeline in gpu.pipelines {
        let pass = RenderPass::<Base2DRenderPass> {
            pipeline: Arc::clone(&pipeline),
            encoder: gpu
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("Render2D Encoder"),
                }),
            _marker: PhantomData,
        };
    }
}

// Draw all Base2D components //

pub type Base2DRenderPass = ();

#[system]
pub fn forward_render_2d_NEW(#[resource] render_pass: &Arc<Mutex<RenderPass<Base2DRenderPass>>>) {
    let mut render_pass = render_pass.lock().unwrap();

    let mut pass_handle = render_pass
        .encoder
        .begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Render2D Pass"),
            color_attachments: &[wgpu::RenderPassColorAttachment {
                view: &frame.view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color {
                        r: 0.0,
                        g: 0.0,
                        b: 0.0,
                        a: 0.0,
                    }),
                    store: true,
                },
            }],
            depth_stencil_attachment: None,
        });

    // Common bindings
    render_pass.set_pipeline(&gpu.pipelines[0].pipeline);

    render_pass.set_bind_group(2, &camera_2d_uniforms_group.bind_group, &[]);
    render_pass.set_bind_group(3, &lighting_2d_uniforms_group.bind_group, &[]);

    render_pass.set_vertex_buffer(0, state.common_vertex_buffers[0].buffer.slice(..));
    render_pass.set_index_buffer(
        state.common_index_buffers[0].buffer.slice(..),
        wgpu::IndexFormat::Uint16,
    );

    // Dynamic bindings
    base_2d_uniforms_group.begin_dynamic_loading();

    let mut query = <(&Base2D, &Position2D)>::query();
    query.for_each(world, |(base_2d, _pos)| {
        render_pass.set_bind_group(0, &state.bindings.groups[&base_2d.texture], &[]);

        render_pass.set_bind_group(
            1,
            &base_2d_bind_group,
            &[base_2d_uniforms_group.increase_offset(0)],
        );

        debug!("Recording draw call");
        render_pass.draw_indexed(
            0..state.common_index_buffers[base_2d.common_index_buffer].size,
            0,
            0..1,
        );
    });

    for (base_2d, _pos) in query.iter(world) {}

    drop(render_pass);
    gpu.queue.submit(std::iter::once(encoder.finish()));
}

#[system]
#[read_component(Base2D)]
#[read_component(Position2D)]
pub fn forward_render_2d(
    world: &SubWorld,
    #[state] state: &Render2DSystem,
    #[resource] gpu: &Arc<Mutex<GpuState>>,
    //#[resource] base_2d_uniforms_group: &Arc<Mutex<UniformGroup<Base2DUniformGroup>>>,
    //#[resource] camera_2d_uniforms_group: &Arc<Mutex<UniformGroup<Camera2DUniformGroup>>>,
    //#[resource] lighting_2d_uniforms_group: &Arc<Mutex<UniformGroup<Lighting2DUniformGroup>>>,
) {
    let gpu = gpu.lock().unwrap();
    // let mut base_2d_uniforms_group = base_2d_uniforms_group.lock().unwrap();
    // let camera_2d_uniforms_group = camera_2d_uniforms_group.lock().unwrap();
    // let lighting_2d_uniforms_group = lighting_2d_uniforms_group.lock().unwrap();

    // let base_2d_bind_group = base_2d_uniforms_group.bind_group();

    let frame = gpu.swap_chain.get_current_frame().unwrap().output;
    let mut encoder = gpu
        .device
        .create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Render2D Encoder"),
        });

    let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: Some("Render2D Pass"),
        color_attachments: &[wgpu::RenderPassColorAttachment {
            view: &frame.view,
            resolve_target: None,
            ops: wgpu::Operations {
                load: wgpu::LoadOp::Clear(wgpu::Color {
                    r: 0.0,
                    g: 0.0,
                    b: 0.0,
                    a: 0.0,
                }),
                store: true,
            },
        }],
        depth_stencil_attachment: None,
    });

    // Common bindings
    // render_pass.set_pipeline(&gpu.pipelines[0].pipeline);

    render_pass.set_bind_group(2, &camera_2d_uniforms_group.bind_group, &[]);
    render_pass.set_bind_group(3, &lighting_2d_uniforms_group.bind_group, &[]);

    render_pass.set_vertex_buffer(0, state.common_vertex_buffers[0].buffer.slice(..));
    render_pass.set_index_buffer(
        state.common_index_buffers[0].buffer.slice(..),
        wgpu::IndexFormat::Uint16,
    );

    // Dynamic bindings
    base_2d_uniforms_group.begin_dynamic_loading();

    let mut query = <(&Base2D, &Position2D)>::query();
    query.for_each(world, |(base_2d, _pos)| {
        render_pass.set_bind_group(0, &state.bindings.groups[&base_2d.texture], &[]);

        render_pass.set_bind_group(
            1,
            &base_2d_bind_group,
            &[base_2d_uniforms_group.increase_offset(0)],
        );

        debug!("Recording draw call");
        render_pass.draw_indexed(
            0..state.common_index_buffers[base_2d.common_index_buffer].size,
            0,
            0..1,
        );
    });

    for (base_2d, _pos) in query.iter(world) {}

    drop(render_pass);
    gpu.queue.submit(std::iter::once(encoder.finish()));
}
