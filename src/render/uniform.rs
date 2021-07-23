use anyhow::{anyhow, Result};
use legion::Resources;
use std::{
    any::type_name,
    fmt::Debug,
    marker::PhantomData,
    mem::size_of,
    num::NonZeroU64,
    sync::{Arc, Mutex},
};
use wgpu::{util::DeviceExt, BindGroupEntry};

pub const DEFAULT_MAX_DYNAMIC_ENTITIES_PER_PASS: u32 = 64;
pub const DEFAULT_DYNAMIC_BUFFER_MIN_BINDING_SIZE: u64 = 128;

pub trait UniformSource:
    Copy + Clone + bytemuck::Pod + bytemuck::Zeroable + Debug + 'static
{
}

pub trait Group {
    fn into<N>() -> N;
}

pub struct UniformGroup<N> {
    pub buffers: Vec<wgpu::Buffer>,
    pub bind_group: wgpu::BindGroup,

    pub dynamic_offset_limits: Vec<u32>,
    pub dynamic_offset_sizes: Vec<u32>,

    _marker: PhantomData<N>,
}

impl<N> UniformGroup<N> {
    pub fn builder() -> UniformGroupBuilder<N> {
        UniformGroupBuilder::new()
    }
}

pub trait ResourceBuilder {
    fn build_to_resource(&self, resources: &mut Resources);
}

pub trait GroupBuilder {
    fn build(
        &mut self,
        device: &wgpu::Device,
        resources: &mut Resources,
    ) -> Result<wgpu::BindGroupLayout>;
}

pub trait GroupResourceBuilder: GroupBuilder + ResourceBuilder {}
impl<N> GroupResourceBuilder for UniformGroupBuilder<N> where N: 'static {}

pub struct UniformGroupBuilder<N> {
    pub uniforms: Vec<Arc<Mutex<dyn UniformBuilder>>>,

    pub bind_group_layout: Option<wgpu::BindGroupLayout>,
    pub bind_group: Option<wgpu::BindGroup>,

    pub state: Option<N>,
    pub dest: Option<Arc<Mutex<UniformGroup<N>>>>,
}

impl<N> UniformGroupBuilder<N> {
    pub fn new() -> Self {
        Self {
            uniforms: vec![],
            bind_group_layout: None,
            bind_group: None,
            state: None,
            dest: None,
        }
    }

    pub fn uniform<T: UniformBuilder + 'static>(mut self, uniform: T) -> Self {
        self.uniforms.push(Arc::new(Mutex::new(uniform)));
        self
    }

    pub fn _state(mut self, state: N) -> Self {
        self.state = Some(state);
        self
    }
}

impl<N> GroupBuilder for UniformGroupBuilder<N> {
    fn build(
        &mut self,
        device: &wgpu::Device,
        resources: &mut Resources,
    ) -> Result<wgpu::BindGroupLayout> {
        debug!("UniformGroupBuilder: building {}", type_name::<N>());

        if self.uniforms.len() == 0 {
            return Err(anyhow!(
                "GroupBuilder: must provide at least one uniform builder"
            ));
        }

        let buffer_states: Vec<BufferState> = self
            .uniforms
            .iter_mut()
            .map(|builder| builder.lock().unwrap().build_buffer(device))
            .collect();

        let entries: Vec<wgpu::BindGroupLayoutEntry> = (0..buffer_states.len())
            .map(|i| wgpu::BindGroupLayoutEntry {
                binding: i as u32,
                visibility: wgpu::ShaderStage::VERTEX_FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: true,
                    min_binding_size: Some(
                        NonZeroU64::new(DEFAULT_DYNAMIC_BUFFER_MIN_BINDING_SIZE).unwrap(),
                    ),
                },
                count: None,
            })
            .collect();

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &entries,
            label: Some(&format!("uniform_bind_group_layout: {}", type_name::<N>())),
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &bind_group_layout,
            entries: &(0..buffer_states.len())
                .map(|i| {
                    let mut buffer_binding = buffer_states[i].buffer.as_entire_buffer_binding();
                    buffer_binding.size =
                        Some(NonZeroU64::new(DEFAULT_DYNAMIC_BUFFER_MIN_BINDING_SIZE).unwrap());
                    wgpu::BindGroupEntry {
                        binding: i as u32,
                        resource: wgpu::BindingResource::Buffer(buffer_binding),
                    }
                })
                .collect::<Vec<BindGroupEntry>>(),
            label: Some(&format!("uniform_bind_group: {}", type_name::<N>())),
        });

        self.dest = Some(Arc::new(Mutex::new(UniformGroup {
            dynamic_offset_sizes: buffer_states.iter().map(|s| s.element_size).collect(),
            dynamic_offset_limits: buffer_states.iter().map(|s| s.max_elements).collect(),
            buffers: buffer_states.into_iter().map(|s| s.buffer).collect(),
            bind_group,
            _marker: PhantomData,
        })));

        for builder in &self.uniforms {
            builder.lock().unwrap().build_to_resource(resources);
        }

        Ok(bind_group_layout)
    }
}

impl<N> ResourceBuilder for UniformGroupBuilder<N>
where
    N: 'static,
{
    fn build_to_resource(&self, resources: &mut Resources) {
        for uniform in &self.uniforms {
            resources.insert(Arc::clone(uniform));
        }
        resources.insert::<Arc<Mutex<UniformGroup<N>>>>(Arc::clone(self.dest.as_ref().unwrap()));
    }
}

pub trait UniformBuilder: ResourceBuilder {
    // -> (buffer, source size, max dynamic offsets per render pass)
    fn build_buffer(&mut self, device: &wgpu::Device) -> BufferState;
}

pub trait Uniform {
    fn load_buffer(&self, buffer: &wgpu::Buffer, queue: &wgpu::Queue, offset: wgpu::BufferAddress);
}

pub struct GenericUniformBuilder<U: Copy + Clone + bytemuck::Pod + bytemuck::Zeroable + Debug> {
    pub source: Option<U>,
    pub buffer: Option<wgpu::Buffer>,

    // Max sources to fit in one buffer
    pub max_size: Option<u32>,

    // Size of one U
    pub size: u32,

    pub dest: Option<Arc<Mutex<GenericUniform<U>>>>,
}

impl<U> GenericUniformBuilder<U>
where
    U: Copy + Clone + bytemuck::Pod + bytemuck::Zeroable + Debug,
{
    pub fn source(source: U) -> Self {
        Self {
            source: Some(source),
            buffer: None,
            max_size: None,
            size: size_of::<U>() as u32,
            dest: Some(Arc::new(Mutex::new(GenericUniform { source }))),
        }
    }

    pub fn max_dynamic_entities(mut self, max: u32) -> Self {
        self.max_size = Some(max);
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
    buffer: wgpu::Buffer,
    element_size: u32,
    max_elements: u32,
}

impl<U> UniformBuilder for GenericUniformBuilder<U>
where
    U: Copy + Clone + bytemuck::Pod + bytemuck::Zeroable + Debug,
{
    fn build_buffer(&mut self, device: &wgpu::Device) -> BufferState {
        let source = &[self.source.unwrap()];

        let max_elements = self
            .max_size
            .unwrap_or(DEFAULT_MAX_DYNAMIC_ENTITIES_PER_PASS);

        let source_bytes = bytemuck::cast_slice(source);
        let source_size = source_bytes.len();
        let source_bytes = source_bytes.repeat(max_elements as usize);

        BufferState {
            buffer: device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some(&format!("Uniform Buffer: {}", type_name::<U>())),
                contents: &source_bytes,
                usage: wgpu::BufferUsage::UNIFORM | wgpu::BufferUsage::COPY_DST,
            }),
            element_size: source_size as u32,
            max_elements,
        }
    }
}

impl<U> Uniform for GenericUniform<U>
where
    U: Copy + Clone + bytemuck::Pod + bytemuck::Zeroable + Debug,
{
    fn load_buffer(&self, buffer: &wgpu::Buffer, queue: &wgpu::Queue, offset: wgpu::BufferAddress) {
        queue.write_buffer(buffer, offset, bytemuck::cast_slice(&[self.source]));
    }
}

#[derive(Copy, Clone)]
pub struct GenericUniform<U: Copy + Clone + bytemuck::Pod + bytemuck::Zeroable + Debug> {
    pub source: U,
}

impl<U> GenericUniform<U>
where
    U: Copy + Clone + bytemuck::Pod + bytemuck::Zeroable + Debug,
{
    pub fn buffer_size(&self) -> u32 {
        size_of::<U>() as u32
    }
}
