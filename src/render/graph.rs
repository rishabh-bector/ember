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
        DEFAULT_SCREEN_HEIGHT, DEFAULT_SCREEN_WIDTH, FORWARD_2D_NODE_ID, ID,
        UNIT_SQUARE_IND_BUFFER_ID, UNIT_SQUARE_VRT_BUFFER_ID,
    },
    resource::{
        metrics::SystemReporter,
        schedule::{LocalSystem, StatelessSystem, SubSchedule},
        store::TextureStore,
        ui::UIBuilder,
    },
    system::{render_2d::create_render_pass, render_graph::*, render_ui::*},
    texture::Texture,
};

use super::node::{NodeBuilder, NodeBuilderTrait, RenderNode};

pub enum RenderTarget {
    Empty,
    Texture(Arc<Texture>),
    Master(Arc<wgpu::SwapChainTexture>),
}

impl RenderTarget {
    pub fn begin_render_pass(&self) -> Option<&wgpu::TextureView> {
        None
    }

    pub fn create_render_pass<'a>(
        &'a self,
        encoder: &'a mut wgpu::CommandEncoder,
        label: &'a str,
    ) -> Option<wgpu::RenderPass<'a>> {
        match self {
            RenderTarget::Empty => None,
            RenderTarget::Texture(tex) => Some(create_render_pass(&tex.view, encoder, label)),
            RenderTarget::Master(opt) => Some(create_render_pass(&opt.view, encoder, label)),
        }
    }

    pub fn bind_group(&self) -> Option<Arc<wgpu::BindGroup>> {
        match self {
            RenderTarget::Empty => None,
            RenderTarget::Texture(tex) => Some(Arc::clone(&tex.bind_group)),
            RenderTarget::Master(_) => None, // Master node cannot be used as input
        }
    }

    pub fn borrow_if_master(&self) -> Option<Arc<wgpu::SwapChainTexture>> {
        match self {
            RenderTarget::Empty => None,
            RenderTarget::Texture(_) => None,
            RenderTarget::Master(opt) => Some(Arc::clone(opt)),
        }
    }

    pub fn arc(&self) -> Self {
        match self {
            RenderTarget::Empty => RenderTarget::Empty,
            RenderTarget::Texture(tex) => RenderTarget::Texture(Arc::clone(&tex)),
            RenderTarget::Master(opt) => RenderTarget::Master(Arc::clone(&opt)),
        }
    }
}

impl Clone for RenderTarget {
    fn clone(&self) -> Self {
        self.arc()
    }
}

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
            source_nodes: Vec::new(),
            master_node: None,
            channels: Vec::new(),
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
        ui_builder: UIBuilder,
    ) -> Result<Arc<RenderGraph>> {
        debug!("building render graph nodes");
        let nodes = self
            .node_builders
            .iter_mut()
            .map(|(id, builder)| {
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
                    },
                )
            })
            .collect();

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
            Arc::clone(&nodes.get(&ID(FORWARD_2D_NODE_ID)).unwrap().system),
            node_states.get(&ID(FORWARD_2D_NODE_ID)).unwrap().to_owned(),
        );

        sub_schedule.flush();

        // Run ui system
        if let UIMode::Master = self.ui_mode {
            sub_schedule
                .add_single_threaded(Arc::new(Box::new(LocalSystem::new(render_ui_system))));
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

        Ok(Arc::clone(&self.dest.as_ref().unwrap()))
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
