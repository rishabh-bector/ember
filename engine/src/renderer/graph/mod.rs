use anyhow::{anyhow, Result};
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};
use uuid::Uuid;

use crate::{
    constants::{ID, METRICS_UI_IMGUI_ID},
    renderer::{graph::target::DepthBuffer, SCREEN_SIZE},
    sources::{
        metrics::{EngineMetrics, SystemReporter},
        registry::Registry,
        schedule::{StatelessSystem, SubSchedule},
        ui::{ImguiWindow, UIBuilder},
    },
    texture::Texture,
};

use super::{buffer::target::TargetBuffer, systems::graph::*};

use self::{
    node::{NodeBuilder, NodeBuilderTrait, NodeInput, RenderNode},
    target::RenderTarget,
};

pub mod node;
pub mod target;

pub enum UIMode {
    Disabled,
    Node(Uuid),
    Master,
}

#[derive(Clone)]
pub struct NodeState {
    pub node: Arc<RenderNode>,
    pub inputs: Vec<NodeInput>,

    // Currently, render_targets is a vector, allowing a node to have multiple render targets.
    // However, a single render target can already have multiple color attachments.
    // Therefore, multiple render targets is only useful if node outputs need to change within frame.
    // This is currently not the case, and so the Vec<> is redundant.
    pub render_targets: Vec<Arc<Mutex<RenderTarget>>>,

    // If chain_mode is enabled, chain_index will be greater than 0.
    pub chain_index: u32,

    // uniform group id -> [(element size, buffer size)]
    pub dyn_offset_state: HashMap<Uuid, (Arc<Mutex<u64>>, Vec<(u64, u64)>)>,
    // pub common_buffers: HashMap<Uuid, Arc<(wgpu::Buffer, u32)>>,
    pub reporter: SystemReporter,
}

impl NodeState {
    pub fn render_target(&self) -> Arc<Mutex<RenderTarget>> {
        Arc::clone(&self.render_targets[0])
    }

    // pub fn get_render_target(&self, index: u32) -> Arc<Mutex<RenderTarget>> {
    //     Arc::clone(&self.render_targets[index as usize])
    // }

    // pub fn get_chain_target(&mut self) -> Arc<Mutex<RenderTarget>> {
    //     let target = Arc::clone(&self.render_targets[self.chain_index as usize]);

    //     self.chain_index += 1;
    //     if self.chain_index >= self.render_targets.len() as u32 {
    //         self.chain_index = 0;
    //     }

    //     target
    // }
}

pub struct RenderGraph {
    // (source_node, source_channel, dest_node)
    pub channels: Vec<(Uuid, u32, Uuid)>,
    pub nodes: HashMap<Uuid, Arc<RenderNode>>,
    pub source_nodes: Vec<Uuid>,
    pub master_node: Uuid,

    pub swap_chain_target: Arc<Mutex<RenderTarget>>,
    pub ui_target: Arc<Mutex<RenderTarget>>,
    pub node_targets: TargetBuffer,
}

pub struct GraphBuilder {
    pub node_builders: HashMap<Uuid, Box<dyn NodeBuilderTrait>>,
    pub source_nodes: Vec<Uuid>,
    pub master_node: Option<Uuid>,

    pub channels: Vec<(Uuid, u32, Uuid)>,
    pub node_states: HashMap<Uuid, NodeState>,
    pub dest: Option<Arc<RenderGraph>>,
    pub ui_mode: UIMode,
}

pub struct MasterDepthBuffer(DepthBuffer);

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

    pub fn with_channel(mut self, input: Uuid, input_index: u32, output: Uuid) -> Self {
        self.channels.push((input, input_index, output));
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
        registry: &Registry,
        window: &winit::window::Window,
        mut metrics_ui: EngineMetrics,
    ) -> Result<(Arc<RenderGraph>, Arc<EngineMetrics>)> {
        if self.master_node.is_none() {
            return Err(anyhow!("render graph requires a master node"));
        }

        debug!("building render graph nodes");
        let nodes = self
            .node_builders
            .iter_mut()
            .map(|(id, builder)| {
                let node = builder.build(resources, &device, Arc::clone(&queue), registry)?;
                Ok((*id, node))
            })
            .collect::<Result<HashMap<Uuid, Arc<RenderNode>>>>()?;

        debug!("creating render graph node_targets");
        let screen_size = SCREEN_SIZE.read().unwrap();
        info!(
            "SCREEN_SIZE AT TARGET BUILD: {}, {}",
            screen_size.0, screen_size.1
        );

        let texture_registry = registry.textures.read().unwrap();
        let mut master = Uuid::default();
        let targets = nodes
            .iter()
            .map(|(id, node)| {
                let depth_buffers = match node.depth_buffer {
                    false => None,
                    true => {
                        debug!("building depth buffer for {}", node.name);
                        Some(
                            (0..node.render_outputs)
                                .map(|_| {
                                    Arc::new(DepthBuffer(Texture::depth_buffer(
                                        &format!("{}_depth_target", node.name),
                                        &device,
                                        (screen_size.0, screen_size.1),
                                        wgpu::TextureFormat::Depth32Float,
                                    )))
                                })
                                .collect::<Vec<Arc<DepthBuffer>>>(),
                        )
                    }
                };
                Ok((
                    *id,
                    match node.master {
                        true => {
                            master = node.id;
                            vec![Arc::new(Mutex::new(RenderTarget::empty_master(
                                depth_buffers
                                    .map_or_else(|| None, |bufs| Some(Arc::clone(&bufs[0]))),
                            )))]
                        }
                        false => {
                            //
                            // Multiple render targets even though render_outputs is 1 (loopback)
                            if node.loopback {
                                (0..2)
                                    .map(|out_index| {
                                        Arc::new(Mutex::new(RenderTarget::Texture {
                                            color_buffer: Arc::new(
                                                Texture::blank(
                                                    (screen_size.0, screen_size.1),
                                                    &device,
                                                    texture_registry.format,
                                                    &texture_registry.bind_layout,
                                                    Some(&format!("{}_render_target", node.name)),
                                                    true,
                                                )
                                                .unwrap(),
                                            ),
                                            depth_buffer: match &depth_buffers {
                                                Some(bufs) => {
                                                    Some(Arc::clone(&bufs[out_index as usize]))
                                                }
                                                None => None,
                                            },
                                        }))
                                    })
                                    .collect::<Vec<Arc<Mutex<RenderTarget>>>>()
                            //
                            // Multiple render targets for no reason (UNUSUAL)
                            } else {
                                if node.render_outputs > 1 {
                                    panic!("this will add multiple render TARGETs, but you probably want to add multiple ATTACHMENTS on the same TARGET");
                                }
                                (0..node.render_outputs)
                                    .map(|out_index| {
                                        Arc::new(Mutex::new(RenderTarget::Texture {
                                            color_buffer: Arc::new(
                                                Texture::blank(
                                                    // TODO: Make actual config (part of SHIP: EngineBuilder)
                                                    (screen_size.0, screen_size.1),
                                                    &device,
                                                    texture_registry.format,
                                                    &texture_registry.bind_layout,
                                                    Some(&format!("{}_render_target", node.name)),
                                                    true,
                                                )
                                                .unwrap(),
                                            ),
                                            depth_buffer: match &depth_buffers {
                                                Some(bufs) => {
                                                    Some(Arc::clone(&bufs[out_index as usize]))
                                                }
                                                None => None,
                                            },
                                        }))
                                    })
                                    .collect::<Vec<Arc<Mutex<RenderTarget>>>>()
                            }
                        }
                    },
                ))
            })
            .collect::<Result<HashMap<Uuid, Vec<Arc<Mutex<RenderTarget>>>>>>()?;

        let target_buffer = TargetBuffer::new(targets, master);
        let swap_chain_target = target_buffer.master();

        // Build UI if enabled
        let ui_target = match &self.ui_mode {
            UIMode::Disabled => Arc::new(Mutex::new(RenderTarget::Empty)),
            UIMode::Master => todo!(), // Arc::clone(&target_buffer.targets.get(id).unwrap()),
            UIMode::Node(id) => Arc::clone(&target_buffer.get_target(id, 0)),
        };

        // Build all NodeStates; each render node's system has this internal state,
        // allowing it to access the target bind groups of its inputs
        // as well as its own target texture
        debug!("building node states");
        let node_states: HashMap<Uuid, NodeState> = nodes
            .iter()
            .map(|(node_id, node)| {
                let mut input_channels = self
                    .input_targets_for_node(*node_id)
                    .iter()
                    .map(|(input_id, input_channel)| {
                        NodeInput::new_single(
                            target_buffer
                                .get_target(input_id, *input_channel as usize)
                                .lock()
                                .unwrap()
                                .get_bind_group()
                                .unwrap(),
                        )
                    })
                    .collect::<Vec<NodeInput>>();

                // If this is a loopback node, set own outputs as inputs
                if node.loopback {
                    input_channels.insert(
                        0,
                        NodeInput::new_ring(
                            target_buffer
                                .get(node_id)
                                .into_iter()
                                .map(|target| target.lock().unwrap().get_bind_group().unwrap())
                                .collect(),
                        ),
                    );
                }

                let render_targets = target_buffer
                    .get(&node_id)
                    .into_iter()
                    .map(Arc::clone)
                    .collect();

                let dyn_offset_state = nodes.get(node_id).unwrap().binder.dyn_offset_state.clone();

                (
                    *node_id,
                    NodeState {
                        node: Arc::clone(node),
                        inputs: input_channels,
                        render_targets,
                        // Cloned for now
                        // common_buffers: common_buffers.clone(),
                        dyn_offset_state,
                        // Register all node systems with metrics, and
                        // give them a system reporter
                        reporter: metrics_ui.register_system_id(&node.name, *node_id),
                        chain_index: 0,
                    },
                )
            })
            .collect();

        // let ui_reporter = metrics_ui.register_system_id("render_ui", ID(RENDER_UI_SYSTEM_ID));
        let metrics_ui = Arc::new(metrics_ui);
        let metrics_arc = Arc::clone(&metrics_ui);
        resources.insert(Arc::clone(&metrics_ui));
        let ui_builder =
            UIBuilder::new().with_imgui_window(metrics_ui.impl_imgui(), ID(METRICS_UI_IMGUI_ID));

        match self.ui_mode {
            UIMode::Node(_) | UIMode::Master | UIMode::Disabled => {
                ui_builder.build_to_resources(
                    resources,
                    Arc::clone(&ui_target),
                    window,
                    &device,
                    &queue,
                );
            } // _ => (debug!("ui is disabled")),
        }

        debug!("scheduling render systems");

        //////////////////////////////////
        // BEGIN RENDER GRAPH SCHEDULER //
        //////////////////////////////////

        // Request target from swap chain, store in graph
        sub_schedule.add_stateless(Arc::new(Box::new(StatelessSystem::new(
            begin_render_graph_system,
        ))));

        // --------------------------------------------------
        sub_schedule.flush();

        // Schedule the render systems such that they are processed in graph dependency
        // order, until the TargetBuffer runs out of render targets. Then, a flush()
        // is inserted, and the TargetBuffer count is started again. Once the master node
        // is reached, a flush() is inserted before it.

        // REQUIREMENT: Unique ID per node in the graph.

        // // First, schedule the source nodes

        // for source_id in &self.source_nodes {
        //     sub_schedule.add_node(
        //         Arc::clone(&nodes.get(source_id).unwrap().system),
        //         node_states.get(source_id).unwrap().to_owned(),
        //     );
        // }

        // sub_schedule.add_node(
        //     Arc::clone(&nodes.get(source_id).unwrap().system),
        //     node_states.get(source_id).unwrap().to_owned(),
        // );

        //

        // Schedule the source and channel nodes via the graph.
        // Recurse backwards from the master node to find these babies.

        let master_map = self.build_map(master);

        match master_map {
            Some(mm) => {
                let mut exec_order: Vec<(Uuid, u32)> = mm.clone().into_iter().flatten().collect();
                exec_order.reverse();

                for (node, out_index) in exec_order {
                    sub_schedule.add_node(
                        Arc::clone(&nodes.get(&node).unwrap().system),
                        node_states.get(&node).unwrap().to_owned(),
                    );
                }
            }
            // Single-node graph
            None => {}
        };

        // --------------------------------------------------

        // Then, schedule master node
        sub_schedule.flush();
        sub_schedule.add_node(
            Arc::clone(&nodes.get(&self.master_node.unwrap()).unwrap().system),
            node_states
                .get(&self.master_node.unwrap())
                .unwrap()
                .to_owned(),
        );

        // --------------------------------------------------
        // sub_schedule.flush();
        //
        // Run ui system
        // if let UIMode::Master = self.ui_mode {
        //     sub_schedule.add_single_threaded_reporter(
        //         Arc::new(Box::new(LocalReporterSystem::new(render_ui_system))),
        //         ui_reporter,
        //     );
        // }

        // --------------------------------------------------
        sub_schedule.flush();

        // Release lock on swap chain, end of frame

        sub_schedule.add_stateless(Arc::new(Box::new(StatelessSystem::new(
            end_render_graph_system,
        ))));

        ////////////////////////////////
        // END RENDER GRAPH SCHEDULER //
        ////////////////////////////////

        self.dest = Some(Arc::new(RenderGraph {
            nodes,
            node_targets: target_buffer,
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

    // Running this on the master node will return a map of all layers below the master node.
    //
    // [[l1, l1], [l2, l2, l2], [l3, l3]] etc. where master = l0 <- l1 <- l2 <- ...
    //
    fn build_map(&self, current_node: Uuid) -> Option<Vec<Vec<(Uuid, u32)>>> {
        let current_inputs: Vec<(Uuid, u32)> = self.input_targets_for_node(current_node);
        let mut dependency_layers: Vec<Vec<(Uuid, u32)>> = vec![];

        if current_inputs.len() > 0 {
            let current_layer = dependency_layers.len();
            dependency_layers.push(vec![]);
            dependency_layers[current_layer].extend(current_inputs);

            for (in_id, in_index) in dependency_layers[current_layer].clone() {
                match self.build_map(in_id) {
                    Some(input_map) => dependency_layers.extend(input_map),
                    None => (),
                }
            }
            Some(dependency_layers)
        } else {
            None
        }
    }

    fn input_targets_for_node(&self, node_id: Uuid) -> Vec<(Uuid, u32)> {
        let mut inputs = self
            .channels
            .iter()
            .filter_map(|(in_id, in_index, out_id)| {
                if *out_id == node_id {
                    Some((*in_id, *in_index))
                } else {
                    None
                }
            })
            .collect::<Vec<(Uuid, u32)>>();
        inputs.sort_unstable();
        inputs.dedup();
        inputs
    }
}
