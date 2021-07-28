use anyhow::Result;
use legion::systems::ParallelRunnable;
use std::{
    collections::HashMap,
    str::FromStr,
    sync::{Arc, Mutex},
};
use uuid::Uuid;
use wgpu::SwapChainTexture;

use crate::{
    buffer::{IndexBuffer, Vertex2D, VertexBuffer},
    constants::{
        BASE_2D_RENDER_NODE_ID, DEFAULT_SCREEN_HEIGHT, DEFAULT_SCREEN_WIDTH, ID,
        UNIT_SQUARE_IND_BUFFER_ID, UNIT_SQUARE_VRT_BUFFER_ID,
    },
    render::{
        node::{NodeBuilder, RenderNode},
        GpuState,
    },
    resource::{
        schedule::{LocalSystem, NodeSystem, Schedulable, StatelessSystem, SubSchedule},
        store::TextureStore,
        ui::UI,
    },
    system::{physics_2d::*, render_graph::*, render_ui::*},
    texture::Texture,
};

use super::node::NodeBuilderTrait;

// Example graph: BASE_2D_FORWARD_RENDERER
// NODES:
//      0: Dynamic Draw (source, draws all Base2D components)
//      1: Post Process (color filter)
//      2: Post Process (blur or bloom idk)
//      3: Assembler    (master node, combines 1 and 2)
// EDGES:
//      0 -> 1
//      0 -> 2
//      1 -> 3
//      2 -> 3
// SOURCES:
//      0
// MASTER:
//      3
//
// Render graph should:
//  - For now, create one texture target per node (excluding master, so 3 here: T0, T1, T2)
//  - Add to the schedule:
//      1. begin_render_graph: [rsrc] gpu -> creates all encoders and RenderPass resources
//      2. --flush--
//      3. forward_render_2d_NEW: [rsrc] encoder -> draws to internal state, which should be T0
//      4. --flush--
//      5. post_process_0: [rsrc] encoder -> draws to internal state, should be T1
//      6. post_process_1: [rsrc] encoder -> draws to internal state, should be T2
//      7. --flush--
//      8. assembly_0: [rsrc] encoder -> draws to internal state, should be swap_chain output
//
//      Problem: post_process_0 needs the Arc<BindGroup> of T0 (+ the TextureView of T1)
//      Solution: mutability not required, so set inputs as system state on init.
//
//  RenderGraph Configuration
//  - For now, this should be code, later could look at YAML
//  Init tasks:
//  - create one texture target per node (Arc owned by node I guess)
//  - create one NodeState per node:
//      NodeState represents the state of a render_pass system (aka a node).
//      It includes the Arc<target> to draw to, as well as one Arc<BindGroup> for each input.
//  Therefore, what info is needed per node config?
//  - number of inputs
//  - type or id of pipeline (need Arc<pipeline> for binding to the render pass)
//

pub enum RenderTarget {
    Texture(Texture),
    Master(Option<wgpu::SwapChainTexture>),
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
    pub master: Arc<Mutex<Option<wgpu::SwapChainTexture>>>,

    // uniform group id -> [(element size, buffer size)]
    pub dyn_offset_state: HashMap<Uuid, (Arc<Mutex<u64>>, Vec<(u64, u64)>)>,
    pub common_buffers: HashMap<Uuid, Arc<(wgpu::Buffer, u32)>>,
}

pub struct RenderGraph {
    pub swap_chain_target: Arc<Mutex<Option<wgpu::SwapChainTexture>>>,
    pub targets: HashMap<Uuid, Arc<Texture>>,
    pub nodes: HashMap<Uuid, Arc<RenderNode>>,
    pub source_nodes: Vec<Uuid>,
    pub master_node: Uuid,
    pub channels: Vec<(Uuid, Uuid)>,
    pub ui_target: Option<Arc<Texture>>,
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

        debug!("creating render graph targets");
        let targets = nodes
            .iter()
            .map(|(id, node)| {
                Ok((
                    *id,
                    RenderTarget::Texture(Texture::blank(
                        // TODO: Make actual config (I will, part of SHIP: EngineBuilder)
                        (DEFAULT_SCREEN_WIDTH as u32, DEFAULT_SCREEN_HEIGHT as u32),
                        &device,
                        &queue,
                        texture_bind_group_layout,
                        Some(&format!("{}_render_target", node.name)),
                        true,
                    )?),
                ))
            })
            .collect::<Result<HashMap<Uuid, RenderTarget>>>()?;

        // Build UI if enabled
        let swap_chain_target = Arc::new(Mutex::new(None));
        let ui_target = match &self.ui_mode {
            UIMode::Disabled => Arc::new(Mutex::new(None)),
            UIMode::Master => Arc::clone(&swap_chain_target),
            UIMode::Node(id) => Arc::clone(targets.get(id).unwrap()),
        };
        match self.ui_mode {
            UIMode::Target(_) | UIMode::Master => {
                debug!("building ui");
                resources.insert(UI::new(Arc::clone(&ui_target), window, &device, &queue))
            }
            _ => (),
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
                            .map(|input_id| Arc::clone(&targets.get(input_id).unwrap().bind_group))
                            .collect::<Vec<Arc<wgpu::BindGroup>>>(),
                        output_target: Arc::clone(&targets.get(&node_id).unwrap()),
                        master: if node.master {
                            Arc::clone(&swap_chain_target)
                        } else {
                            Arc::new(Mutex::new(None))
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

        // Build all render pass resources; these are static containers for the
        // command encoders which only exist for the lifetime of the pass

        // TODO:
        // receive mutable subschedule from EngineBuilder
        // add NodeSystem to NodeBuilder, so that each node can have a system builder func
        // schedule full render graph onto subschedule

        // 1. RUN BEGIN_RENDER_GRAPH, which will:
        //  - create all RenderPass command encoders
        debug!("scheduling render systems");

        sub_schedule.add_boxed_stateless(Arc::new(Box::new(StatelessSystem::new(
            begin_render_graph_system,
        ))));

        sub_schedule.flush();

        // 2. RUN NODE SYSTEMS EXCEPT MASTER
        // sub_schedule.add_boxed(Arc::clone(
        //     &nodes.get(&self.node_builders[0].dest_id).unwrap().system,
        // ));

        sub_schedule.add_boxed(
            Arc::clone(&nodes.get(&ID(BASE_2D_RENDER_NODE_ID)).unwrap().system),
            node_states
                .get(&ID(BASE_2D_RENDER_NODE_ID))
                .unwrap()
                .to_owned(),
        );

        sub_schedule.flush();

        // 3. RUN MASTER NODE SYSTEM (should be set to gpu swap chain view)

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
            targets,
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
            .filter_map(|(in_id, out_id)| match out_id {
                node_id => Some(*in_id),
                _ => None,
            })
            .collect::<Vec<Uuid>>()
    }
}
