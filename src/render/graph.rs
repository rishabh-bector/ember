use anyhow::Result;
use legion::Schedule;
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};
use uuid::Uuid;

use crate::{
    constants,
    render::{
        node::{NodeBuilder, RenderNode},
        GpuState,
    },
    resources::store::TextureStore,
};

use super::texture::Texture;

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

pub struct NodeState {
    pub input_channels: Vec<Arc<wgpu::BindGroup>>,
    pub output_target: Arc<Texture>,
}

pub struct RenderGraph {
    pub targets: HashMap<Uuid, Arc<Texture>>,
    pub nodes: HashMap<Uuid, Arc<RenderNode>>,
    pub source_nodes: Vec<Uuid>,
    pub master_node: Uuid,
    pub channels: Vec<(Uuid, Uuid)>,
}

pub struct GraphBuilder {
    pub node_builders: Vec<NodeBuilder>,
    pub source_nodes: Vec<Uuid>,
    pub master_node: Option<Uuid>,
    pub channels: Vec<(Uuid, Uuid)>,
    pub node_states: HashMap<Uuid, NodeState>,
    pub dest: Option<Arc<RenderGraph>>,
}

impl GraphBuilder {
    pub fn new() -> GraphBuilder {
        Self {
            node_builders: Vec::new(),
            source_nodes: Vec::new(),
            master_node: None,
            channels: Vec::new(),
            node_states: HashMap::new(),
            dest: None,
        }
    }

    pub fn with_node(mut self, node: NodeBuilder) -> Self {
        self.node_builders.push(node);
        self
    }

    pub fn with_source_node(mut self, node: NodeBuilder) -> Self {
        self.source_nodes.push(node.dest_id.to_owned());
        self.with_node(node)
    }

    pub fn with_master_node(mut self, node: NodeBuilder) -> Self {
        self.master_node = Some(node.dest_id.to_owned());
        self.with_node(node)
    }

    pub fn with_channel(mut self, input: Uuid, output: Uuid) -> Self {
        self.channels.push((input, output));
        self
    }

    pub fn build(
        &mut self,
        gpu: Arc<Mutex<GpuState>>,
        resources: &mut legion::Resources,
        texture_bind_group_layout: &wgpu::BindGroupLayout,
        texture_store: Arc<Mutex<TextureStore>>,
    ) -> Result<Arc<RenderGraph>> {
        let gpu = gpu.lock().unwrap();

        // Build all render nodes; a render node holds data for
        // running a render pipeline such as bind group refs,
        // shader modules, uniform group builders, etc.
        let nodes = self
            .node_builders
            .iter_mut()
            .map(|builder| {
                debug!("render_graph_builder: running {}", &builder.name);
                let node = builder.build(
                    resources,
                    &gpu.device,
                    Arc::clone(&gpu.queue),
                    &gpu.chain_descriptor,
                    &texture_bind_group_layout,
                    Arc::clone(&texture_store),
                )?;
                Ok((node.id, node))
            })
            .collect::<Result<HashMap<Uuid, Arc<RenderNode>>>>()?;

        // Build all render targets; one for each render node (for now)
        let targets = self
            .node_builders
            .iter()
            .map(|builder| {
                Ok((
                    builder.dest_id,
                    Arc::new(Texture::blank(
                        // TODO: Make actual config (I will, part of SHIP: EngineBuilder)
                        (
                            constants::DEFAULT_SCREEN_WIDTH as u32,
                            constants::DEFAULT_SCREEN_HEIGHT as u32,
                        ),
                        &gpu.device,
                        &gpu.queue,
                        texture_bind_group_layout,
                        Some(&format!("render_target_{}", builder.dest_name)),
                    )?),
                ))
            })
            .collect::<Result<HashMap<Uuid, Arc<Texture>>>>()?;

        // Build all NodeStates; each render node's system has this internal state,
        // allowing it to access the target bind groups of its inputs
        // as well as its own target texture (to bind both in a pass)
        let node_states: HashMap<Uuid, NodeState> = self
            .node_builders
            .iter()
            .map(|builder| {
                let node_id = &builder.dest.as_ref().unwrap().id;
                (
                    *node_id,
                    NodeState {
                        input_channels: self
                            .input_nodes_for_node(*node_id)
                            .iter()
                            .map(|input_id| Arc::clone(&targets.get(input_id).unwrap().bind_group))
                            .collect::<Vec<Arc<wgpu::BindGroup>>>(),
                        output_target: Arc::clone(&targets.get(&node_id).unwrap()),
                    },
                )
            })
            .collect();

        self.dest = Some(Arc::new(RenderGraph {
            nodes,
            targets,
            channels: self.channels.clone(),
            source_nodes: self.source_nodes.clone(),
            master_node: self
                .master_node
                .expect("RenderGraphBuilder: master node required"),
        }));

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

#[system]
pub fn testing() {}
