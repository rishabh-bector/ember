use std::{
    collections::HashMap,
    rc::Rc,
    sync::{Arc, Mutex},
};

use anyhow::{anyhow, Result};
use legion::Resources;

use super::{shader::ShaderBuilder, type_key, uniform::GroupResourceBuilder};

pub enum ShaderSource {
    WGSL(String),
    SPIRV(String),
}

pub struct Pipeline {
    pipeline: wgpu::RenderPipeline,
    shader_module: wgpu::ShaderModule,
}

/// Builder for easily creating flexible wgpu render pipelines

pub struct PipelineBuilder {
    pub shader_source: ShaderSource,
    pub vertex_buffer_layouts: Vec<wgpu::VertexBufferLayout<'static>>,
    pub uniform_group_builders: HashMap<&'static str, Arc<Mutex<dyn GroupResourceBuilder>>>,
}

impl PipelineBuilder {
    pub fn new(shader: ShaderSource) -> Self {
        Self {
            shader_source: shader,
            vertex_buffer_layouts: vec![],
            uniform_group_builders: HashMap::new(),
        }
    }

    pub fn uniform_group<T: GroupResourceBuilder + 'static>(mut self, group_builder: T) -> Self {
        self.uniform_group_builders
            .insert(type_key::<T>(), Arc::new(Mutex::new(group_builder)));
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
        chain_desc: &wgpu::SwapChainDescriptor,
    ) -> Result<Pipeline> {
        // Validate pipelne

        if self.vertex_buffer_layouts.len() == 0 {
            return Err(anyhow!(
                "PipelineBuilder: at least one vertex buffer required"
            ));
        }

        // Build pipeline

        let shader_module = build_shader(self.shader_source, device);

        let bind_group_layouts: Vec<&wgpu::BindGroupLayout> = self
            .uniform_group_builders
            .iter()
            .map(|(name, builder)| {
                builder.lock().unwrap().build(device, resources)?;
                Ok(builder
                    .lock()
                    .unwrap()
                    .group_layout()
                    .as_ref()
                    .as_ref()
                    .unwrap())
            })
            .collect();

        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Render Pipeline Layout"),
                bind_group_layouts: bind_group_layouts.as_slice(),
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

        Ok(Pipeline {
            pipeline,
            shader_module,
        })
    }
}

fn build_shader(source: ShaderSource, device: &wgpu::Device) -> wgpu::ShaderModule {
    device.create_shader_module(&wgpu::ShaderModuleDescriptor {
        label: Some("Shader"),
        flags: wgpu::ShaderFlags::all(),
        source: match source {
            ShaderSource::WGSL(src) => wgpu::ShaderSource::Wgsl(src.clone().into()),
            _ => panic!("ShaderSource: only wgsl shaders are supported currently"),
        },
    })
}
