use std::{borrow::Borrow, rc::Rc, sync::{Arc, Mutex}};

use anyhow::{anyhow, Result};

use crate::{render::shader::ShaderSource, resources::store::TextureStore};
use std::borrow::BorrowMut;
use super::shader::ShaderBuilder;

/// Builder for easily creating flexible wgpu render pipelines

#[derive(Default)]
pub struct PipelineBuilder {
    pub shader: Option<ShaderBuilder>,
    pub vertex_buffer_layouts: Vec<wgpu::VertexBufferLayout<'static>>,
    pub uniform_builders: Vec<&'static str>,
}

impl PipelineBuilder {
    pub fn new() -> Self {
        Default::default()
    }

    // pub fn uniform_group<T>(mut self) -> Self {}

    pub fn shader(mut self, shader: ShaderBuilder) -> Self {
        self.uniform_builders = shader.groups.clone();
        self.shader = Some(shader);
        self
    }

    pub fn vertex_buffer_layout(mut self, layout: wgpu::VertexBufferLayout<'static>) -> Self {
        self.vertex_buffer_layouts.push(layout);
        self
    }

    pub fn build(
        self,
        group_layouts: Vec<Rc<Option<wgpu::BindGroupLayout>>>,
        device: &wgpu::Device,
        chain_desc: &wgpu::SwapChainDescriptor,
    ) -> Result<wgpu::RenderPipeline> {
        let shader = self
            .shader
            .ok_or(anyhow!("PipelineBuilder: at least one shader required"))?
            .build(device);

        if self.vertex_buffer_layouts.len() == 0 {
            return Err(anyhow!(
                "PipelineBuilder: at least one vertex buffer required"
            ));
        }

        let bind_group_layouts: Vec<&wgpu::BindGroupLayout> = group_layouts.iter().map(|rc| rc.as_ref().as_ref().unwrap()).collect();

        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Render Pipeline Layout"),
                bind_group_layouts: bind_group_layouts.as_slice(),
                push_constant_ranges: &[],
            });

        Ok(
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("Render Pipeline"),
                layout: Some(&render_pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &shader.module,
                    entry_point: "main",
                    buffers: self.vertex_buffer_layouts.as_slice(),
                },
                fragment: Some(wgpu::FragmentState {
                    module: &shader.module,
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
            }),
        )
    }
}
