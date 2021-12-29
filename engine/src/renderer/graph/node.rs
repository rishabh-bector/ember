use anyhow::{anyhow, Result};
use legion::{systems::ParallelRunnable, Resources};
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};
use uuid::Uuid;
use wgpu::BindGroup;

use crate::{
    renderer::uniform::group::GroupResourceBuilder,
    sources::{
        registry::Registry,
        schedule::{NodeSystem, SubSchedulable},
    },
};

use super::NodeState;

pub struct RenderNode {
    pub id: Uuid,
    pub name: String,

    pub master: bool,       //  Is this the master node?
    pub loopback: bool,     //  Should this node alternate targets and inputs?
    pub depth_buffer: bool, //  Should this node have a depth buffer attached?

    // Pipeline settings
    pub reverse_cull: bool, //  Should front faces be culled instead of back faces?

    // pub blend: bool, //  Should this node render/blend into another node's target?
    //
    // Currently, each render graph node has its own outputs, because it is assumed
    // that some later node(s) in the graph will use said outputs as inputs.
    //
    // However, source passes often need to render into the same render target with blending.
    // Example: draw sky, then draw objects with different render nodes but into same target as sky.
    //
    // // How should RenderGraph provide this functionality? // //
    //
    // Ideas:
    // - with_channel() specifies "dependency" links while with_chain() specifies shared targets
    // - nodes which are to share a target are added altogether? e.g. chain(vec![n1, n2, n3])
    //

    // Number of output textures (RenderTargets)
    pub render_outputs: u32,
    pub graph_inputs: u32,

    pub pipeline: wgpu::RenderPipeline,
    pub shader_module: wgpu::ShaderModule,
    pub binder: PipelineBinder,

    pub system: Arc<Box<dyn SubSchedulable>>,
}

pub enum NodeOutput {
    Single,
    Ring,
}

// If the input node renders to different targets per-frame,
// it will be represented as a "Ring" (increments every frame).
pub enum NodeInput {
    Single {
        target: Arc<BindGroup>,
    },
    Ring {
        targets: Vec<Arc<BindGroup>>,
        last: usize,
    },
}

impl NodeInput {
    pub fn new_single(target: Arc<BindGroup>) -> Self {
        Self::Single { target }
    }

    pub fn new_ring(targets: Vec<Arc<BindGroup>>) -> Self {
        Self::Ring {
            last: targets.len(),
            targets,
        }
    }

    pub fn bind_group_ref(&mut self) -> &BindGroup {
        match self {
            NodeInput::Single { target } => target,
            NodeInput::Ring { targets, last } => {
                *last += 1;
                if *last >= targets.len() {
                    *last = 0;
                }
                &targets[*last]
            }
        }
    }

    // pub fn bind_group(&mut self) -> Option<Arc<BindGroup>> {
    //     match self {
    //         NodeInput::Single { target } => Some(Arc::clone(target)),
    //         NodeInput::Ring { .. } => self.pick_from_ring(),
    //     }
    // }

    // pub fn pick_from_ring(&mut self) -> Option<Arc<BindGroup>> {
    //     match self {
    //         NodeInput::Ring { targets, last } => {
    //             *last += 1;
    //             if *last >= targets.len() {
    //                 *last = 0;
    //             }
    //             Some(Arc::clone(&targets[*last]))
    //         }
    //         _ => None,
    //     }
    // }

    pub fn arc(&self) -> Self {
        match self {
            NodeInput::Single { target } => NodeInput::Single {
                target: Arc::clone(target),
            },
            NodeInput::Ring { targets, last } => NodeInput::Ring {
                targets: targets.into_iter().map(Arc::clone).collect(),
                last: *last,
            },
        }
    }
}

impl Clone for NodeInput {
    fn clone(&self) -> Self {
        self.arc()
    }
}

pub struct PipelineBinder {
    pub texture_groups: HashMap<Uuid, Arc<wgpu::BindGroup>>,
    pub uniform_groups: HashMap<Uuid, Arc<wgpu::BindGroup>>,

    // uniform group id -> (dyn_entity_count, [(dyn uniform size, max count)])
    // Todo: should deprecate or improve this
    pub dyn_offset_state: HashMap<Uuid, (Arc<Mutex<u64>>, Vec<(u64, u64)>)>,
}

#[derive(Clone)]
pub enum ShaderSource {
    WGSL(String),
    _SPIRV(String),
}

pub enum BindIndex {
    Uniform { node_index: usize },
    Texture { group_id: Uuid, cubemap: bool },
    NodeInput,
}

/// RenderGraph node builder.
pub struct NodeBuilder {
    pub name: String,
    pub master: bool,

    pub graph_inputs: u32,
    pub loopback: bool,

    pub render_outputs: u32,
    pub depth_buffer: bool,

    pub reverse_cull: bool,

    pub shader_source: ShaderSource,
    pub bind_groups: Vec<BindIndex>,
    pub vertex_buffer_layouts: Vec<wgpu::VertexBufferLayout<'static>>,
    pub uniform_group_builders: Vec<Arc<Mutex<dyn GroupResourceBuilder>>>,

    // The final product, a RenderNode
    pub dest: Option<Arc<RenderNode>>,
    pub dest_name: String,
    pub dest_id: Uuid,

    pub system: Option<Arc<Box<dyn SubSchedulable>>>,
}

impl NodeBuilder {
    pub fn new(name: String, graph_inputs: u32, render_outputs: u32, shader: ShaderSource) -> Self {
        Self {
            name: format!("{}_builder", &name),
            dest_id: Uuid::new_v4(),
            depth_buffer: false,
            master: false,
            loopback: false,
            reverse_cull: false,
            uniform_group_builders: vec![],
            vertex_buffer_layouts: vec![],
            bind_groups: vec![],
            system: None,
            dest: None,
            shader_source: shader,
            dest_name: name,
            render_outputs,
            graph_inputs,
        }
    }

    pub fn with_uniform_group<T: GroupResourceBuilder + 'static>(
        mut self,
        group_builder: T,
    ) -> Self {
        self.bind_groups.push(BindIndex::Uniform {
            node_index: self.uniform_group_builders.len(),
        });
        self.uniform_group_builders
            .push(Arc::new(Mutex::new(group_builder)));
        self
    }

    pub fn with_shared_uniform_group<T: GroupResourceBuilder + 'static>(
        mut self,
        group_builder: Arc<Mutex<T>>,
    ) -> Self {
        self.bind_groups.push(BindIndex::Uniform {
            node_index: self.uniform_group_builders.len(),
        });
        self.uniform_group_builders.push(group_builder);
        self
    }

    pub fn with_texture_group(mut self, group_id: Uuid, cubemap: bool) -> Self {
        self.bind_groups
            .push(BindIndex::Texture { group_id, cubemap });
        self
    }

    pub fn with_node_input(mut self) -> Self {
        self.bind_groups.push(BindIndex::NodeInput);
        self
    }

    pub fn with_vertex_layout(mut self, layout: wgpu::VertexBufferLayout<'static>) -> Self {
        self.vertex_buffer_layouts.push(layout);
        self
    }

    pub fn with_system<
        S: ParallelRunnable + 'static,
        F: Fn(NodeState) -> S + Send + Sync + 'static,
    >(
        mut self,
        system: F,
    ) -> Self {
        self.system = Some(Arc::new(Box::new(NodeSystem::new(system))));
        self
    }

    pub fn with_id(mut self, id: Uuid) -> Self {
        self.dest_id = id;
        self
    }

    pub fn with_depth_buffer(mut self) -> Self {
        self.depth_buffer = true;
        self
    }

    pub fn with_loopback(mut self) -> Self {
        self.loopback = true;
        self
    }

    pub fn with_reverse_culling(mut self) -> Self {
        self.reverse_cull = true;
        self
    }
}

impl NodeBuilderTrait for NodeBuilder {
    fn id(&self) -> Uuid {
        self.dest_id
    }

    fn build(
        &mut self,
        resources: &mut Resources,
        device: &wgpu::Device,
        queue: Arc<wgpu::Queue>,
        registry: &Registry,
    ) -> Result<Arc<RenderNode>> {
        debug!("building node: {}", self.dest_id);

        if let Some(node) = &self.dest {
            warn!("{}: this node has already been built; it is probably being referenced more than once in the graph; the existing node will be reused", &self.name);
            return Ok(Arc::clone(&node));
        }

        if self.vertex_buffer_layouts.len() == 0 {
            return Err(anyhow!(
                "{}: render nodes require at least one vertex buffer"
            ));
        }

        let shader_module = build_shader(
            &self.shader_source,
            &format!("shader_{}", &self.name),
            device,
        );

        let bind_group_layouts = &self
            .bind_groups
            .iter()
            .map(|bind_index| {
                Ok(match *bind_index {
                    BindIndex::Texture { cubemap, .. } => (None, cubemap),
                    BindIndex::Uniform { node_index } => (
                        Some(
                            self.uniform_group_builders[node_index]
                                .lock()
                                .unwrap()
                                .build(device, resources, Arc::clone(&queue))?,
                        ),
                        false,
                    ),
                    BindIndex::NodeInput {} => (None, false),
                })
            })
            .collect::<Result<Vec<(Option<wgpu::BindGroupLayout>, bool)>>>()?;

        let texture_registry = registry.textures.read().unwrap();
        let layout_refs = bind_group_layouts
            .into_iter()
            .map(|(opt_uniform, is_cubemap)| match opt_uniform {
                Some(u) => &u,
                None => {
                    if *is_cubemap {
                        &texture_registry.cube_bind_layout
                    } else {
                        &texture_registry.bind_layout
                    }
                }
            })
            .collect::<Vec<&wgpu::BindGroupLayout>>();

        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some(&format!("render_pipeline_layout_{}", &self.name)),
                bind_group_layouts: layout_refs.as_slice(),
                push_constant_ranges: &[],
            });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some(&format!("render_pipeline_{}", &self.name)),
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader_module,
                entry_point: "main",
                buffers: self.vertex_buffer_layouts.as_slice(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader_module,
                entry_point: "main",
                targets: &[wgpu::ColorTargetState {
                    format: registry.textures.read().unwrap().format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrite::ALL,
                }],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(match self.reverse_cull {
                    true => wgpu::Face::Front,
                    false => wgpu::Face::Back,
                }),
                polygon_mode: wgpu::PolygonMode::Fill,
                clamp_depth: false,
                conservative: false,
            },
            depth_stencil: match self.depth_buffer {
                false => None,
                true => {
                    debug!("adding depth buffer to pipeline: {}", self.name);
                    Some(wgpu::DepthStencilState {
                        format: wgpu::TextureFormat::Depth32Float,
                        depth_write_enabled: true,
                        depth_compare: wgpu::CompareFunction::Less,
                        stencil: wgpu::StencilState::default(),
                        bias: wgpu::DepthBiasState::default(),
                    })
                }
            },
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
        });

        // Move registered uniform groups and sources into system resources
        for builder in &self.uniform_group_builders {
            builder.lock().unwrap().build_to_resource(resources);
        }

        let texture_groups_needed: Vec<Uuid> = self
            .bind_groups
            .iter()
            .filter_map(|bind| match bind {
                &BindIndex::Texture { group_id, .. } => Some(group_id),
                _ => None,
            })
            .collect();

        let uniform_groups_needed: Vec<usize> = self
            .bind_groups
            .iter()
            .filter_map(|bind| match bind {
                &BindIndex::Uniform { node_index } => Some(node_index),
                _ => None,
            })
            .collect();

        let mut texture_groups: HashMap<Uuid, Arc<wgpu::BindGroup>> = HashMap::new();
        for group_id in &texture_groups_needed {
            texture_groups.extend(registry.textures.read().unwrap().texture_group(group_id));
        }

        let mut uniform_groups: HashMap<Uuid, Arc<wgpu::BindGroup>> = HashMap::new();
        let mut dyn_offset_state: HashMap<Uuid, (Arc<Mutex<u64>>, Vec<(u64, u64)>)> =
            HashMap::new();

        for group in uniform_groups_needed {
            let needed_group = self.uniform_group_builders[group].lock().unwrap();
            let (id, bind_group) = needed_group.binding();
            uniform_groups.insert(id, bind_group);

            if let Some(dyn_offsets) = needed_group.dynamic() {
                dyn_offset_state.insert(id, dyn_offsets);
            }
        }

        let binder = PipelineBinder {
            texture_groups,
            uniform_groups,
            dyn_offset_state,
        };

        self.dest = Some(Arc::new(RenderNode {
            id: self.dest_id,
            name: self.dest_name.to_owned(),
            graph_inputs: self.graph_inputs,
            render_outputs: self.render_outputs,
            system: Arc::clone(&self.system.as_ref().unwrap()),
            master: self.master,
            depth_buffer: self.depth_buffer,
            loopback: self.loopback,
            reverse_cull: self.reverse_cull,
            binder,
            pipeline,
            shader_module,
        }));

        Ok(Arc::clone(&self.dest.as_ref().unwrap()))
    }
}

pub trait NodeBuilderTrait {
    fn id(&self) -> Uuid;
    fn build(
        &mut self,
        resources: &mut Resources,
        device: &wgpu::Device,
        queue: Arc<wgpu::Queue>,
        registry: &Registry,
    ) -> Result<Arc<RenderNode>>;
}

fn build_shader(source: &ShaderSource, label: &str, device: &wgpu::Device) -> wgpu::ShaderModule {
    device.create_shader_module(&wgpu::ShaderModuleDescriptor {
        label: Some(label),
        flags: wgpu::ShaderFlags::all(),
        source: match source {
            ShaderSource::WGSL(src) => wgpu::ShaderSource::Wgsl(src.clone().into()),
            _ => panic!(
                "Error building shader {}: only WGSL shaders are supported currently",
                label
            ),
        },
    })
}
