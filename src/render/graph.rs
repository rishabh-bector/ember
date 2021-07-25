use std::{collections::HashMap, sync::Arc};
use uuid::Uuid;

use super::pipeline::Pipeline;

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

pub struct RenderGraph {
    pub nodes: HashMap<Uuid, Arc<RenderNode>>,
    pub channels: Vec<(Uuid, Uuid)>,
    pub sources: Vec<Uuid>,
    pub master: Uuid,
}

impl RenderGraph {
    pub fn new() -> Self {
        Self {
            nodes: HashMap::new(),
            channels: vec![],
            sources: vec![],
            master: Uuid::new_v4(),
        }
    }

    pub fn node(&mut self, node: Arc<RenderNode>) -> Uuid {
        let id = Uuid::new_v4();
        self.nodes.insert(id, node);
        id
    }

    pub fn build(&mut self) {
        // Build all render nodes
        debug!("Building render nodes");
        let queue = Arc::new(queue);
        let nodes = self
            .pipeline_builders
            .into_iter()
            .map(|builder| {
                builder.build(
                    resources,
                    &device,
                    Arc::clone(&queue),
                    &chain_descriptor,
                    &texture_bind_group_layout,
                    Arc::clone(&texture_store),
                )
            })
            .collect::<Result<Vec<RenderNode>>>()?;
    }
}

pub struct GraphBuilder {
    pub node_builders: HashMap<Uuid, NodeBuilder>,
    pub dest: RenderGraph,
}
