use legion::Resources;
use std::{
    any::type_name,
    fmt::Debug,
    mem::size_of,
    sync::{Arc, Mutex},
};
use wgpu::util::DeviceExt;

use super::UniformBuilder;
use crate::{constants::DEFAULT_MAX_DYNAMIC_ENTITIES_PER_PASS, resource::ResourceBuilder};

#[derive(Clone)]
pub enum BufferMode {
    Single,
    Dynamic(u32),
    Instance(u32),
}

pub struct GenericUniformBuilder<U: Copy + Clone + bytemuck::Pod + bytemuck::Zeroable + Debug> {
    pub source: Option<U>,
    pub buffer: Option<wgpu::Buffer>,
    pub mode: BufferMode,

    // Size of one U
    pub size: u32,

    pub dest: Option<Arc<Mutex<GenericUniform<U>>>>,
}

impl<U> GenericUniformBuilder<U>
where
    U: Copy + Clone + bytemuck::Pod + bytemuck::Zeroable + Debug,
{
    pub fn from_source(source: U) -> Self {
        Self {
            source: Some(source),
            buffer: None,
            mode: BufferMode::Single,
            size: size_of::<U>() as u32,
            dest: None,
        }
    }

    pub fn enable_dynamic_buffering(mut self) -> Self {
        self.mode = BufferMode::Dynamic(DEFAULT_MAX_DYNAMIC_ENTITIES_PER_PASS);
        self
    }

    pub fn with_dynamic_entity_limit(mut self, max: u32) -> Self {
        self.mode = BufferMode::Dynamic(max);
        self
    }
}

impl<U> ResourceBuilder for GenericUniformBuilder<U>
where
    U: Copy + Clone + bytemuck::Pod + bytemuck::Zeroable + Debug + 'static,
{
    fn build_to_resource(&self, resources: &mut Resources) {
        resources.insert(Arc::clone(&self.dest.as_ref().as_ref().unwrap()));
    }
}

pub struct BufferState {
    pub buffer: wgpu::Buffer,
    pub element_size: u64,
    pub max_elements: u64,
}

impl<U> UniformBuilder for GenericUniformBuilder<U>
where
    U: Copy + Clone + bytemuck::Pod + bytemuck::Zeroable + Debug,
{
    fn build_buffer(&mut self, device: &wgpu::Device) -> BufferState {
        let source = &[self.source.unwrap()];
        self.dest = Some(Arc::new(Mutex::new(GenericUniform {
            source: [self.source.unwrap()],
            mode: self.mode.clone(),
        })));

        return match self.mode {
            BufferMode::Single => {
                let source_bytes = bytemuck::cast_slice(source);
                BufferState {
                    buffer: device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                        label: Some("Uniform Buffer"),
                        contents: source_bytes,
                        usage: wgpu::BufferUsage::UNIFORM | wgpu::BufferUsage::COPY_DST,
                    }),
                    element_size: 0,
                    max_elements: 0,
                }
            }
            BufferMode::Dynamic(max_elements) => {
                let source_bytes = bytemuck::cast_slice(source);
                let source_size = source_bytes.len();
                let source_bytes = source_bytes.repeat(max_elements as usize);

                BufferState {
                    buffer: device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                        label: Some(&format!("Dynamic Uniform Buffer: {}", type_name::<U>())),
                        contents: &source_bytes,
                        usage: wgpu::BufferUsage::UNIFORM | wgpu::BufferUsage::COPY_DST,
                    }),
                    element_size: source_size as u64,
                    max_elements: max_elements as u64,
                }
            }
            BufferMode::Instance(max_elements) => {
                let source_bytes = bytemuck::cast_slice(source);
                let source_size = source_bytes.len();
                let source_bytes = source_bytes.repeat(max_elements as usize);

                BufferState {
                    buffer: device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                        label: Some(&format!("Instance Buffer: {}", type_name::<U>())),
                        contents: &source_bytes,
                        usage: wgpu::BufferUsage::VERTEX,
                    }),
                    element_size: source_size as u64,
                    max_elements: max_elements as u64,
                }
            }
        };
    }

    // Used by UniformGroupBuilder to store dynamic buffer info
    fn dynamic(&self) -> Option<(u64, u64)> {
        if let BufferMode::Dynamic(max_size) = self.mode {
            return Some((self.size as u64, max_size as u64));
        }
        None
    }
}

#[derive(Clone)]
pub struct GenericUniform<U: Copy + Clone + bytemuck::Pod + bytemuck::Zeroable + Debug> {
    pub source: [U; 1],
    pub mode: BufferMode,
}

impl<U> GenericUniform<U>
where
    U: Copy + Clone + bytemuck::Pod + bytemuck::Zeroable + Debug,
{
    pub fn mut_ref(&mut self) -> &mut U {
        &mut self.source[0]
    }

    pub fn buffer_size(&self) -> u32 {
        size_of::<U>() as u32
    }

    pub fn to_bytes(source: &[U]) -> &[u8] {
        bytemuck::cast_slice(source)
    }
}
