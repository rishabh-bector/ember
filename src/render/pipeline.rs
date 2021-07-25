use anyhow::{anyhow, Result};
use legion::Resources;
use std::sync::{Arc, Mutex};

use crate::resources::store::{BindMap, TextureGroup, TextureStore};

use super::uniform::GroupResourceBuilder;

pub enum ShaderSource {
    WGSL(String),
    _SPIRV(String),
}

pub struct RenderNode {
    pub pipeline: wgpu::RenderPipeline,
    pub shader_module: wgpu::ShaderModule,
    pub texture_binds: BindMap,
}

pub enum BindIndex {
    Uniform(usize),
    Texture(TextureGroup),
}
/// Builder for easily creating flexible wgpu render pipelines

pub struct NodeBuilder {
    pub shader_source: ShaderSource,
    pub bind_groups: Vec<BindIndex>,
    pub vertex_buffer_layouts: Vec<wgpu::VertexBufferLayout<'static>>,
    pub uniform_group_builders: Vec<Arc<Mutex<dyn GroupResourceBuilder>>>,
}

impl NodeBuilder {
    pub fn new(shader: ShaderSource) -> Self {
        Self {
            shader_source: shader,
            bind_groups: vec![],
            vertex_buffer_layouts: vec![],
            uniform_group_builders: vec![],
        }
    }

    pub fn uniform_group<T: GroupResourceBuilder + 'static>(mut self, group_builder: T) -> Self {
        self.bind_groups
            .push(BindIndex::Uniform(self.uniform_group_builders.len()));
        self.uniform_group_builders
            .push(Arc::new(Mutex::new(group_builder)));
        self
    }

    pub fn texture_group(mut self, group: TextureGroup) -> Self {
        self.bind_groups.push(BindIndex::Texture(group));
        self
    }

    pub fn vertex_buffer_layout(mut self, layout: wgpu::VertexBufferLayout<'static>) -> Self {
        self.vertex_buffer_layouts.push(layout);
        self
    }

    pub fn build(
        self,
        resources: &mut Resources,
        device: &wgpu::Device,
        queue: Arc<wgpu::Queue>,
        chain_desc: &wgpu::SwapChainDescriptor,
        texture_bind_group_layout: &wgpu::BindGroupLayout,
        texture_store: Arc<Mutex<TextureStore>>,
    ) -> Result<RenderNode> {
        if self.vertex_buffer_layouts.len() == 0 {
            return Err(anyhow!(
                "PipelineBuilder: at least one vertex buffer required"
            ));
        }

        let shader_module = build_shader(&self.shader_source, device);

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
                label: Some("Render Pipeline Layout"),
                bind_group_layouts: layout_refs.as_slice(),
                push_constant_ranges: &[],
            });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
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
                    format: chain_desc.format,
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
            .into_iter()
            .filter_map(|bind| match bind {
                BindIndex::Texture(group) => Some(group),
                _ => None,
            })
            .collect();

        let bind_map = texture_store
            .lock()
            .unwrap()
            .build_bind_map(texture_groups_needed.as_slice());

        Ok(RenderNode {
            texture_binds: bind_map,
            pipeline,
            shader_module,
        })
    }
}

fn build_shader(source: &ShaderSource, device: &wgpu::Device) -> wgpu::ShaderModule {
    device.create_shader_module(&wgpu::ShaderModuleDescriptor {
        label: Some("Shader"),
        flags: wgpu::ShaderFlags::all(),
        source: match source {
            ShaderSource::WGSL(src) => wgpu::ShaderSource::Wgsl(src.clone().into()),
            _ => panic!("ShaderSource: only WGSL shaders are supported currently"),
        },
    })
}
