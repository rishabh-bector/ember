use anyhow::{anyhow, Result};
use regex::Regex;
use std::{
    borrow::Borrow,
    collections::HashMap,
    rc::Rc,
    sync::{Arc, Mutex},
};
use wgpu::RenderPipeline;
use winit::window::Window;

pub mod buffer;
pub mod pipeline;
pub mod texture;
pub mod uniform;

use pipeline::PipelineBuilder;

use crate::{
    render::{pipeline::Pipeline, texture::TextureUniformGroup},
    resources::store::TextureStoreBuilder,
};

use self::uniform::GroupResourceBuilder;

pub struct GpuState {
    pub surface: wgpu::Surface,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub chain_descriptor: wgpu::SwapChainDescriptor,
    pub swap_chain: wgpu::SwapChain,
    pub screen_size: (u32, u32),
    pub pipelines: Vec<Pipeline>,
}

#[derive(Default)]
pub struct GpuStateBuilder {
    pub screen_size: (u32, u32),
    pub instance: Option<wgpu::Instance>,
    pub surface: Option<wgpu::Surface>,
    pub pipeline_builders: Vec<PipelineBuilder>,
}

impl GpuStateBuilder {
    pub fn winit(window: &Window) -> Self {
        let size = window.inner_size();

        // Instance is a handle to the GPU
        // BackendBit::PRIMARY => Vulkan + Metal + DX12 + Browser WebGPU
        let instance = wgpu::Instance::new(wgpu::BackendBit::PRIMARY);

        // Surface is used to create a swap chain
        let surface = unsafe { instance.create_surface(window) };

        Self {
            screen_size: (size.width, size.height),
            instance: Some(instance),
            surface: Some(surface),
            ..Default::default()
        }
    }

    pub fn pipeline(mut self, pipeline: PipelineBuilder) -> Self {
        self.pipeline_builders.push(pipeline);
        self
    }

    // Depends on TextureStore being in resources
    pub async fn build(
        mut self,
        store_builder: &mut TextureStoreBuilder,
        resources: &mut legion::Resources,
    ) -> Result<GpuState> {
        if self.pipeline_builders.len() == 0 {
            return Err(anyhow!(
                "GpuStateBuilder: must provide at least one pipeline builder"
            ));
        }

        let surface = self
            .surface
            .ok_or_else(|| anyhow!("GpuStateBuilder: must provide a surface"))?;
        let instance = self
            .instance
            .ok_or_else(|| anyhow!("GpuStateBuilder: must provide an instance"))?;

        // Adapter is used to request a device and queue
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                compatible_surface: Some(&surface),
            })
            .await
            .ok_or(anyhow!("GpuStateBuilder: failed to request adapter"))?;

        // Device is an open connection to the GPU
        // Queue is a handle to the GPU's command buffer executor
        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: None,
                    features: wgpu::Features::empty(),
                    limits: wgpu::Limits::default(),
                },
                None,
            )
            .await?;

        // Swap chain is used to store rendered textures which
        // are synced with the display
        let chain_descriptor = wgpu::SwapChainDescriptor {
            usage: wgpu::TextureUsage::RENDER_ATTACHMENT,
            format: adapter
                .get_swap_chain_preferred_format(&surface)
                .ok_or(anyhow!("failed to get preferred swap chain format"))?,
            width: self.screen_size.0,
            height: self.screen_size.1,
            present_mode: wgpu::PresentMode::Fifo,
        };
        let swap_chain = device.create_swap_chain(&surface, &chain_descriptor);

        // Build all render pipelines
        debug!("Building render pipelines");
        let pipelines = self
            .pipeline_builders
            .into_iter()
            .map(|builder| {
                builder.build(
                    resources,
                    &device,
                    &chain_descriptor,
                    store_builder.build(&device, &queue)?,
                )
            })
            .collect::<Result<Vec<Pipeline>>>()?;

        // Add TextureStore to system resources
        store_builder.build_to_resource(resources);

        Ok(GpuState {
            screen_size: self.screen_size,
            surface,
            device,
            queue,
            chain_descriptor,
            swap_chain,
            pipelines: pipelines,
        })
    }
}

impl GpuState {
    pub fn resize(&mut self, new_size: (u32, u32)) {
        self.screen_size = new_size;
        self.chain_descriptor.width = new_size.0;
        self.chain_descriptor.height = new_size.1;
        self.swap_chain = self
            .device
            .create_swap_chain(&self.surface, &self.chain_descriptor);
    }
}

pub fn type_key<T>() -> &'static str {
    let re = Regex::new(".*::(.*)>?");
    let type_path = std::any::type_name::<T>();
    let mut type_key = re
        .unwrap()
        .captures(type_path)
        .unwrap()
        .get(1)
        .unwrap()
        .as_str();
    if type_key.ends_with(">") {
        type_key = &type_key[..type_key.len() - 1];
    }
    type_key
}
