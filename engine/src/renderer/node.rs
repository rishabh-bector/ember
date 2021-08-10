use anyhow::{anyhow, Result};
use legion::{systems::ParallelRunnable, Resources};
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};
use uuid::Uuid;

use super::{graph::NodeState, uniform::group::GroupResourceBuilder};
use crate::sources::{
    schedule::{NodeSystem, SubSchedulable},
    store::{TextureGroup, TextureStore},
};

pub struct RenderNode {
    pub id: Uuid,
    pub name: String,
    pub graph_inputs: u32,
    pub master: bool,

    pub pipeline: wgpu::RenderPipeline,
    pub shader_module: wgpu::ShaderModule,
    pub binder: PipelineBinder,

    pub system: Arc<Box<dyn SubSchedulable>>,
}

pub struct PipelineBinder {
    pub texture_groups: HashMap<Uuid, Arc<wgpu::BindGroup>>,
    pub uniform_groups: HashMap<Uuid, Arc<wgpu::BindGroup>>,

    // uniform group id -> (dyn_entity_count, [(dyn uniform size, max count)])
    // Todo: should deprecate or improve this
    pub dyn_offset_state: HashMap<Uuid, (Arc<Mutex<u64>>, Vec<(u64, u64)>)>,
}

pub enum ShaderSource {
    WGSL(String),
    _SPIRV(String),
}

pub enum BindIndex {
    Uniform(usize),
    Texture(TextureGroup),
}

/// RenderGraph node builder.
pub struct NodeBuilder {
    pub name: String,
    pub graph_inputs: u32,
    pub master: bool,

    pub shader_source: ShaderSource,
    pub bind_groups: Vec<BindIndex>,
    pub vertex_buffer_layouts: Vec<wgpu::VertexBufferLayout<'static>>,
    pub uniform_group_builders: Vec<Arc<Mutex<dyn GroupResourceBuilder>>>,

    pub dest: Option<Arc<RenderNode>>,
    pub dest_name: String,
    pub dest_id: Uuid,

    pub system: Option<Arc<Box<dyn SubSchedulable>>>,
}

impl NodeBuilder {
    pub fn new(name: String, graph_inputs: u32, shader: ShaderSource) -> Self {
        Self {
            name: format!("{}_builder", &name),
            graph_inputs,
            master: false,
            shader_source: shader,
            bind_groups: vec![],
            vertex_buffer_layouts: vec![],
            uniform_group_builders: vec![],
            dest: None,
            dest_name: name,
            dest_id: Uuid::new_v4(),
            system: None,
        }
    }

    pub fn with_uniform_group<T: GroupResourceBuilder + 'static>(
        mut self,
        group_builder: T,
    ) -> Self {
        self.bind_groups
            .push(BindIndex::Uniform(self.uniform_group_builders.len()));
        self.uniform_group_builders
            .push(Arc::new(Mutex::new(group_builder)));
        self
    }

    pub fn with_shared_uniform_group<T: GroupResourceBuilder + 'static>(
        mut self,
        group_builder: Arc<Mutex<T>>,
    ) -> Self {
        self.bind_groups
            .push(BindIndex::Uniform(self.uniform_group_builders.len()));
        self.uniform_group_builders.push(group_builder);
        self
    }

    pub fn with_texture_group(mut self, group: TextureGroup) -> Self {
        self.bind_groups.push(BindIndex::Texture(group));
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
        texture_format: wgpu::TextureFormat,
        texture_bind_group_layout: &wgpu::BindGroupLayout,
        texture_store: Arc<Mutex<TextureStore>>,
    ) -> Result<Arc<RenderNode>> {
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
                    BindIndex::Texture(_) => None,
                    BindIndex::Uniform(i) => {
                        Some(self.uniform_group_builders[i].lock().unwrap().build(
                            device,
                            resources,
                            Arc::clone(&queue),
                        )?)
                    }
                })
            })
            .collect::<Result<Vec<Option<wgpu::BindGroupLayout>>>>()?;

        let layout_refs = bind_group_layouts
            .into_iter()
            .map(|opt_uniform| match opt_uniform {
                Some(u) => &u,
                None => texture_bind_group_layout,
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
                    format: texture_format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrite::ALL,
                }],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back),
                polygon_mode: wgpu::PolygonMode::Fill,
                clamp_depth: false,
                conservative: false,
            },
            depth_stencil: None,
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

        let texture_groups_needed: Vec<TextureGroup> = self
            .bind_groups
            .iter()
            .filter_map(|bind| match bind {
                &BindIndex::Texture(group) => Some(group),
                _ => None,
            })
            .collect();

        let uniform_groups_needed: Vec<usize> = self
            .bind_groups
            .iter()
            .filter_map(|bind| match bind {
                &BindIndex::Uniform(i) => Some(i),
                _ => None,
            })
            .collect();

        let mut texture_groups: HashMap<Uuid, Arc<wgpu::BindGroup>> = HashMap::new();
        for group in &texture_groups_needed {
            texture_groups.extend(texture_store.lock().unwrap().bind_group(group));
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
            system: Arc::clone(&self.system.as_ref().unwrap()),
            master: self.master,
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
        texture_format: wgpu::TextureFormat,
        texture_bind_group_layout: &wgpu::BindGroupLayout,
        texture_store: Arc<Mutex<TextureStore>>,
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
