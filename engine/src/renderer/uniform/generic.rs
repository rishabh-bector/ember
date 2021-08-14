use legion::Resources;
use std::{
    any::type_name,
    fmt::Debug,
    mem::size_of,
    sync::{Arc, Mutex},
};
use wgpu::util::DeviceExt;

use super::{group::BufferMode, UniformBuilder};
use crate::sources::ResourceBuilder;

pub struct GenericUniformBuilder<U: Copy + Clone + bytemuck::Pod + bytemuck::Zeroable + Debug> {
    pub source: Option<U>,
    pub buffer: Option<wgpu::Buffer>,
    pub size: u32, // Size of one U
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
            size: size_of::<U>() as u32,
            dest: None,
        }
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
    fn build_buffer(&mut self, device: &wgpu::Device, mode: BufferMode) -> BufferState {
        let source = &[self.source.unwrap()];
        self.dest = Some(Arc::new(Mutex::new(GenericUniform {
            source: [self.source.unwrap()],
            mode: mode.clone(),
        })));

        return match mode {
            BufferMode::Single => {
                let source_bytes = bytemuck::cast_slice(source);
                let source_size = source_bytes.len();
                BufferState {
                    buffer: device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                        label: Some(&format!("Single Uniform Buffer: {}", type_name::<U>())),
                        contents: source_bytes,
                        usage: wgpu::BufferUsage::UNIFORM | wgpu::BufferUsage::COPY_DST,
                    }),
                    element_size: source_size as u64,
                    max_elements: 1,
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
            _ => {
                panic!("uniforms only support single and dynamic buffers")
            }
        };
    }

    fn single_buffer(&self, device: &wgpu::Device) -> BufferState {
        let source = &[self.source.unwrap()];
        let source_bytes = bytemuck::cast_slice(source);
        let source_size = source_bytes.len();
        BufferState {
            buffer: device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some(&format!("Single Uniform Buffer: {}", type_name::<U>())),
                contents: source_bytes,
                usage: wgpu::BufferUsage::UNIFORM | wgpu::BufferUsage::COPY_DST,
            }),
            element_size: source_size as u64,
            max_elements: 1,
        }
    }

    // Used by UniformGroupBuilder to store dynamic buffer info
    fn dynamic_size(&self) -> u64 {
        self.size as u64
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
