use anyhow::{anyhow, Result};
use once_cell::sync::Lazy;
use std::sync::{Arc, RwLock};
use winit::window::Window;

use crate::constants::{
    DEFAULT_SCREEN_HEIGHT, DEFAULT_SCREEN_WIDTH, DEFAULT_TEXTURE_BUFFER_FORMAT,
};

pub mod buffer;
pub mod graph;
pub mod mesh;
pub mod systems;
pub mod uniform;

// Global Mutable Variable
pub static SCREEN_SIZE: Lazy<RwLock<(u32, u32)>> =
    Lazy::new(|| RwLock::new((DEFAULT_SCREEN_WIDTH, DEFAULT_SCREEN_HEIGHT)));

pub struct GpuState {
    pub device: Arc<wgpu::Device>,
    pub queue: Arc<wgpu::Queue>,
    pub adapter: Arc<wgpu::Adapter>,

    pub surface: wgpu::Surface,
    pub chain_descriptor: wgpu::SwapChainDescriptor,
    pub swap_chain: wgpu::SwapChain,
    pub first_resize: bool,
}

pub struct GpuStateBuilder {
    pub window: Arc<Window>,
    pub screen_size: (u32, u32),
    pub instance: Option<wgpu::Instance>,
    pub surface: Option<wgpu::Surface>,
}

impl GpuStateBuilder {
    pub fn winit(window: Arc<Window>) -> Self {
        let size = window.inner_size();

        // Instance is a handle to the GPU
        // BackendBit::PRIMARY => Vulkan + Metal + DX12 + Browser WebGPU
        let instance = wgpu::Instance::new(wgpu::BackendBit::VULKAN | wgpu::BackendBit::METAL);

        // Surface is used to create a swap chain
        let surface = unsafe { instance.create_surface(window.as_ref()) };

        Self {
            window,
            screen_size: (size.width, size.height),
            instance: Some(instance),
            surface: Some(surface),
        }
    }

    // Depends on TextureStore being in resources
    pub async fn build(self, resources: &mut legion::Resources) -> Result<GpuState> {
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

        let device = Arc::new(device);
        let queue = Arc::new(queue);

        resources.insert(Arc::clone(&device));
        resources.insert(Arc::clone(&queue));

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

        Ok(GpuState {
            adapter: Arc::new(adapter),
            surface,
            device,
            queue,
            chain_descriptor,
            swap_chain,
            first_resize: false,
        })
    }
}

impl GpuState {
    pub fn resize(&mut self, new_size: (u32, u32)) {
        let current_size = SCREEN_SIZE.read().unwrap();
        if current_size.0 == new_size.0 && current_size.1 == new_size.1 {
            debug!(
                "gpu_state resize() called but new_size the same: {:?}",
                new_size
            );
            if self.first_resize {
                return;
            } else {
                self.first_resize = true;
            }
        } else {
        }
        drop(current_size);

        *SCREEN_SIZE.write().unwrap() = new_size;
        self.chain_descriptor.width = new_size.0;
        self.chain_descriptor.height = new_size.1;

        self.swap_chain = self
            .device
            .create_swap_chain(&self.surface, &self.chain_descriptor);

        info!("SCREEN_SIZE CHANGED TO: {}, {}", new_size.0, new_size.1);
    }

    pub fn force_new_swap_chain(&mut self) {
        warn!("running force_new_swap_chain; something might be wrong");
        warn!("> chain descriptor: {:?}", self.chain_descriptor);

        let current_size = SCREEN_SIZE.read().unwrap();
        warn!(
            "> current screen size: {}, {}",
            current_size.0, current_size.1
        );
        drop(current_size);

        self.swap_chain = self
            .device
            .create_swap_chain(&self.surface, &self.chain_descriptor);
    }

    pub fn device_preferred_format(&mut self) -> wgpu::TextureFormat {
        let fmt = self
            .adapter
            .get_swap_chain_preferred_format(&self.surface)
            .unwrap_or(DEFAULT_TEXTURE_BUFFER_FORMAT);
        debug!("device preferred texture format: {:?}", fmt);
        fmt
    }
}

// -----------------------------------------------------------

// pub struct RenderPass<N> {
//     pub _marker: PhantomData<N>,
// }

// impl<N> RenderPass<N> {
//     pub fn new(gpu: Arc<Mutex<GpuState>>, pipeline: usize) -> Self {

//     }

//     // pub fn new() -> Self {
//     //     Self {
//     //         groups: HashMap::new(),
//     //         pass: vec![],
//     //     }
//     // }

//     // pub fn add_texture_group(&mut self, textures: &HashMap<Uuid, Texture>) {
//     //     self.groups.extend(
//     //         textures
//     //             .iter()
//     //             .map(|(id, tex)| (*id, Arc::clone(&tex.bind_group))),
//     //     );
//     // }

//     // pub fn add_uniform_group(&mut self, id: Uuid, group: Arc<wgpu::BindGroup>) {
//     //     self.groups.insert(id, group);
//     // }

//     // pub fn configure_pass(&mut self, indices: Vec<Uuid>) {
//     //     self.pass = indices;
//     // }
// }
