use anyhow::Result;
use std::{
    collections::HashMap,
    str::FromStr,
    sync::{Arc, Mutex},
};
use uuid::Uuid;

use crate::{
    buffer::{IndexBuffer, Vertex2D, VertexBuffer},
    constants::{
        DEFAULT_SCREEN_HEIGHT, DEFAULT_SCREEN_WIDTH, ID, INSTANCE_2D_NODE_ID, METRICS_UI_IMGUI_ID,
        RENDER_UI_SYSTEM_ID, UNIT_SQUARE_IND_BUFFER_ID, UNIT_SQUARE_VRT_BUFFER_ID,
    },
    sources::{
        metrics::{EngineMetrics, SystemReporter},
        schedule::{LocalReporterSystem, StatelessSystem, SubSchedule},
        store::TextureStore,
        ui::{ImguiWindow, UIBuilder},
    },
    texture::Texture,
};

use super::{
    node::{NodeBuilder, NodeBuilderTrait, RenderNode},
    systems::{graph::*, ui::*},
    target::RenderTarget,
};

pub enum UIMode {
    Disabled,
    Node(Uuid),
    Master,
}

#[derive(Clone)]
pub struct NodeState {
    pub node: Arc<RenderNode>,
    pub input_channels: Vec<Arc<wgpu::BindGroup>>,
    pub render_target: Arc<Mutex<RenderTarget>>,

    // uniform group id -> [(element size, buffer size)]
    pub dyn_offset_state: HashMap<Uuid, (Arc<Mutex<u64>>, Vec<(u64, u64)>)>,
    pub common_buffers: HashMap<Uuid, Arc<(wgpu::Buffer, u32)>>,

    pub reporter: SystemReporter,
}

pub struct RenderGraph {
    pub channels: Vec<(Uuid, Uuid)>,
    pub nodes: HashMap<Uuid, Arc<RenderNode>>,
    pub source_nodes: Vec<Uuid>,
    pub master_node: Uuid,

    pub swap_chain_target: Arc<Mutex<RenderTarget>>,
    pub ui_target: Arc<Mutex<RenderTarget>>,
    pub node_targets: HashMap<Uuid, Arc<Mutex<RenderTarget>>>,
}

pub struct GraphBuilder {
    pub node_builders: HashMap<Uuid, Box<dyn NodeBuilderTrait>>,
    pub source_nodes: Vec<Uuid>,
    pub master_node: Option<Uuid>,
    pub channels: Vec<(Uuid, Uuid)>,
    pub node_states: HashMap<Uuid, NodeState>,
    pub dest: Option<Arc<RenderGraph>>,
    pub ui_mode: UIMode,
}

impl GraphBuilder {
    pub fn new() -> GraphBuilder {
        Self {
            node_builders: HashMap::new(),
            source_nodes: vec![],
            master_node: None,
            channels: vec![],
            node_states: HashMap::new(),
            dest: None,
            ui_mode: UIMode::Disabled,
        }
    }

    pub fn with_node<T: NodeBuilderTrait + 'static>(mut self, node: T) -> Self {
        self.node_builders.insert(node.id(), Box::new(node));
        self
    }

    pub fn with_source_node(mut self, node: NodeBuilder) -> Self {
        self.source_nodes.push(node.dest_id.to_owned());
        self.with_node(node)
    }

    pub fn with_master_node(mut self, mut node: NodeBuilder) -> Self {
        node.master = true;
        self.master_node = Some(node.dest_id.to_owned());
        self.with_node(node)
    }

    pub fn with_channel(mut self, input: Uuid, output: Uuid) -> Self {
        self.channels.push((input, output));
        self
    }

    pub fn with_ui_master(mut self) -> Self {
        self.ui_mode = UIMode::Master;
        self
    }

    // TODO: distil this into several functions
    pub fn build(
        &mut self,
        device: Arc<wgpu::Device>,
        queue: Arc<wgpu::Queue>,
        resources: &mut legion::Resources,
        sub_schedule: &mut SubSchedule,
        texture_format: wgpu::TextureFormat,
        texture_bind_group_layout: &wgpu::BindGroupLayout,
        texture_store: Arc<Mutex<TextureStore>>,
        window: &winit::window::Window,
        mut metrics_ui: EngineMetrics,
    ) -> Result<(Arc<RenderGraph>, Arc<EngineMetrics>)> {
        debug!("building render graph nodes");
        let nodes = self
            .node_builders
            .iter_mut()
            .map(|(id, builder)| {
                debug!("building node: {}", builder.id());
                let node = builder.build(
                    resources,
                    &device,
                    Arc::clone(&queue),
                    texture_format,
                    &texture_bind_group_layout,
                    Arc::clone(&texture_store),
                )?;
                Ok((*id, node))
            })
            .collect::<Result<HashMap<Uuid, Arc<RenderNode>>>>()?;

        debug!("creating render graph node_targets");
        let node_targets = nodes
            .iter()
            .map(|(id, node)| {
                Ok((
                    *id,
                    Arc::new(Mutex::new(RenderTarget::Texture(Arc::new(Texture::blank(
                        // TODO: Make actual config (I will, part of SHIP: EngineBuilder)
                        (DEFAULT_SCREEN_WIDTH as u32, DEFAULT_SCREEN_HEIGHT as u32),
                        &device,
                        texture_format,
                        texture_bind_group_layout,
                        Some(&format!("{}_render_target", node.name)),
                        true,
                    )?)))),
                ))
            })
            .collect::<Result<HashMap<Uuid, Arc<Mutex<RenderTarget>>>>>()?;

        // Build UI if enabled
        let swap_chain_target = Arc::new(Mutex::new(RenderTarget::Empty));
        let ui_target = match &self.ui_mode {
            UIMode::Disabled => Arc::new(Mutex::new(RenderTarget::Empty)),
            UIMode::Master => Arc::clone(&swap_chain_target),
            UIMode::Node(id) => Arc::clone(&node_targets.get(id).unwrap()),
        };

        let unit_square_buffers = (
            VertexBuffer::new_2d(
                &[
                    Vertex2D {
                        position: [-1.0, -1.0],
                        uvs: [0.0, 1.0],
                    },
                    Vertex2D {
                        position: [-1.0, 1.0],
                        uvs: [0.0, 0.0],
                    },
                    Vertex2D {
                        position: [1.0, 1.0],
                        uvs: [1.0, 0.0],
                    },
                    Vertex2D {
                        position: [1.0, -1.0],
                        uvs: [1.0, 1.0],
                    },
                ],
                &device,
            ),
            IndexBuffer::new(&[0, 2, 1, 3, 2, 0], &device),
        );

        debug!("loading common buffers");
        let mut common_buffers: HashMap<Uuid, Arc<(wgpu::Buffer, u32)>> = HashMap::new();
        common_buffers.insert(
            Uuid::from_str(UNIT_SQUARE_VRT_BUFFER_ID).unwrap(),
            Arc::clone(&unit_square_buffers.0.buffer),
        );
        common_buffers.insert(
            Uuid::from_str(UNIT_SQUARE_IND_BUFFER_ID).unwrap(),
            Arc::clone(&unit_square_buffers.1.buffer),
        );

        // Build all NodeStates; each render node's system has this internal state,
        // allowing it to access the target bind groups of its inputs
        // as well as its own target texture
        debug!("building node states");
        let node_states: HashMap<Uuid, NodeState> = nodes
            .iter()
            .map(|(node_id, node)| {
                (
                    *node_id,
                    NodeState {
                        node: Arc::clone(node),
                        input_channels: self
                            .input_nodes_for_node(*node_id)
                            .iter()
                            .map(|input_id| {
                                Arc::clone(
                                    &node_targets
                                        .get(input_id)
                                        .unwrap()
                                        .lock()
                                        .unwrap()
                                        .bind_group()
                                        .unwrap(),
                                )
                            })
                            .collect::<Vec<Arc<wgpu::BindGroup>>>(),
                        render_target: if node.master {
                            Arc::clone(&swap_chain_target)
                        } else {
                            Arc::clone(&node_targets.get(&node_id).unwrap())
                        },
                        // Cloned for now
                        common_buffers: common_buffers.clone(),
                        dyn_offset_state: nodes
                            .get(node_id)
                            .unwrap()
                            .binder
                            .dyn_offset_state
                            .clone(),
                        reporter: metrics_ui.register_system_id(&node.name, *node_id),
                    },
                )
            })
            .collect();

        // let reporter_forward_2d =
        //     metrics_ui.register_system_id("forward_2d", ID(FORWARD_2D_NODE_ID));
        // let reporter_render_ui =
        //     metrics_ui.register_system_id("render_ui", ID(RENDER_UI_SYSTEM_ID));

        let ui_reporter = metrics_ui.register_system_id("render_ui", ID(RENDER_UI_SYSTEM_ID));
        let metrics_ui = Arc::new(metrics_ui);
        let metrics_arc = Arc::clone(&metrics_ui);
        resources.insert(Arc::clone(&metrics_ui));
        let ui_builder =
            UIBuilder::new().with_imgui_window(metrics_ui.impl_imgui(), ID(METRICS_UI_IMGUI_ID));

        match self.ui_mode {
            UIMode::Node(_) | UIMode::Master => {
                ui_builder.build_to_resources(
                    resources,
                    Arc::clone(&ui_target),
                    window,
                    &device,
                    &queue,
                );
            }
            _ => (debug!("ui is disabled")),
        }

        debug!("scheduling render systems");

        // Request target from swap chain, store in graph
        sub_schedule.add_boxed_stateless(Arc::new(Box::new(StatelessSystem::new(
            begin_render_graph_system,
        ))));
        sub_schedule.flush();

        // Run all node systems except master
        // sub_schedule.add_boxed(Arc::clone(
        //     &nodes.get(&self.node_builders[0].dest_id).unwrap().system,
        // ));

        sub_schedule.add_boxed(
            Arc::clone(&nodes.get(&ID(INSTANCE_2D_NODE_ID)).unwrap().system),
            node_states
                .get(&ID(INSTANCE_2D_NODE_ID))
                .unwrap()
                .to_owned(),
        );

        sub_schedule.flush();

        // Run ui system
        if let UIMode::Master = self.ui_mode {
            sub_schedule.add_single_threaded_reporter(
                Arc::new(Box::new(LocalReporterSystem::new(render_ui_system))),
                ui_reporter,
            );
        }

        // Release lock on swap chain, end of frame
        sub_schedule.flush();
        sub_schedule.add_boxed_stateless(Arc::new(Box::new(StatelessSystem::new(
            end_render_graph_system,
        ))));

        self.dest = Some(Arc::new(RenderGraph {
            nodes,
            node_targets,
            swap_chain_target,
            channels: self.channels.clone(),
            source_nodes: self.source_nodes.clone(),
            master_node: self
                .master_node
                .expect("RenderGraphBuilder: master node required"),
            ui_target,
        }));

        debug!("done building render graph!");

        Ok((Arc::clone(&self.dest.as_ref().unwrap()), metrics_arc))
    }

    fn input_nodes_for_node(&self, node_id: Uuid) -> Vec<Uuid> {
        self.channels
            .iter()
            .filter_map(|(in_id, out_id)| {
                if *out_id == node_id {
                    Some(*in_id)
                } else {
                    None
                }
            })
            .collect::<Vec<Uuid>>()
    }
}
