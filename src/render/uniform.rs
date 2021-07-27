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
use uuid::Uuid;
use wgpu::{util::DeviceExt, BindGroupEntry};

use crate::constants::{
    DEFAULT_DYNAMIC_BUFFER_MIN_BINDING_SIZE, DEFAULT_MAX_DYNAMIC_ENTITIES_PER_PASS,
};

pub trait UniformSource:
    Copy + Clone + bytemuck::Pod + bytemuck::Zeroable + Debug + 'static
{
}

pub trait Group {
    fn into<N>() -> N;
}

pub struct UniformGroup<N> {
    pub buffers: Vec<wgpu::Buffer>,
    pub bind_group: Arc<wgpu::BindGroup>,

    pub dynamic_offset_limits: Vec<u64>,
    pub dynamic_offset_sizes: Vec<u64>,
    pub dynamic_offset_state: Vec<u64>,

    pub id: Uuid,
    pub queue: Arc<wgpu::Queue>,
    _marker: PhantomData<N>,
}

impl<N> UniformGroup<N> {
    pub fn builder() -> UniformGroupBuilder<N> {
        UniformGroupBuilder::new()
    }

    pub fn load_uniform(&self, index: usize, source_bytes: &[u8]) {
        self.queue
            .write_buffer(&self.buffers[index], 0, source_bytes)
    }

    pub fn begin_dynamic_loading(&mut self) {
        self.dynamic_offset_state.iter_mut().for_each(|i| *i = 0);
    }

    pub fn load_dynamic_uniform(&mut self, index: usize, source_bytes: &[u8]) {
        for i in 0..self.buffers.len() {
            self.queue
                .write_buffer(&self.buffers[i], self.dynamic_offset_state[i], source_bytes);
            self.increase_offset(i);
        }
    }

    pub fn increase_offset(&mut self, index: usize) -> u32 {
        let old = self.dynamic_offset_state[index];
        self.dynamic_offset_state[index] += self.dynamic_offset_sizes[index];
        old as u32
    }

    pub fn bind_group(&self) -> Arc<wgpu::BindGroup> {
        Arc::clone(&self.bind_group)
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
        queue: Arc<wgpu::Queue>,
    ) -> Result<wgpu::BindGroupLayout>;

    fn dynamic(&self) -> Option<Vec<(u64, u64)>>;
    fn binding(&self) -> (Uuid, Arc<wgpu::BindGroup>);
}

pub trait GroupResourceBuilder: GroupBuilder + ResourceBuilder {}
impl<N> GroupResourceBuilder for UniformGroupBuilder<N> where N: 'static {}

pub struct UniformGroupBuilder<N> {
    pub uniforms: Vec<Arc<Mutex<dyn UniformBuilder>>>,

    pub bind_group_layout: Option<wgpu::BindGroupLayout>,
    pub bind_group: Option<Arc<wgpu::BindGroup>>,

    pub id: Uuid,
    pub state: Option<N>,
    pub dest: Option<Arc<Mutex<UniformGroup<N>>>>,

    pub dyn_offset_info: Vec<(u64, u64)>,
}

impl<N> UniformGroupBuilder<N> {
    pub fn new() -> Self {
        Self {
            uniforms: vec![],
            bind_group_layout: None,
            bind_group: None,
            state: None,
            dest: None,
            id: Uuid::new_v4(),
            dyn_offset_info: vec![],
        }
    }

    pub fn with_uniform<T: UniformBuilder + 'static>(mut self, uniform: T) -> Self {
        if let Some(offset_info) = uniform.dynamic() {
            self.dyn_offset_info.push(offset_info);
        }
        self.uniforms.push(Arc::new(Mutex::new(uniform)));
        self
    }

    pub fn with_id(mut self, id: Uuid) -> Self {
        self.id = id;
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
        queue: Arc<wgpu::Queue>,
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
            .map(|i| {
                let has_dynamic_offset = buffer_states[i].max_elements != 1;
                let min_binding_size = NonZeroU64::new(match has_dynamic_offset {
                    false => buffer_states[i].element_size as u64,
                    true => DEFAULT_DYNAMIC_BUFFER_MIN_BINDING_SIZE,
                })
                .unwrap();

                wgpu::BindGroupLayoutEntry {
                    binding: i as u32,
                    visibility: wgpu::ShaderStage::VERTEX_FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset,
                        min_binding_size: Some(min_binding_size),
                    },
                    count: None,
                }
            })
            .collect();

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &entries,
            label: Some(&format!("uniform_bind_group_layout: {}", type_name::<N>())),
        });

        self.bind_group = Some(Arc::new(
            device.create_bind_group(&wgpu::BindGroupDescriptor {
                layout: &bind_group_layout,
                entries: &(0..buffer_states.len())
                    .map(|i| {
                        let mut buffer_binding = buffer_states[i].buffer.as_entire_buffer_binding();
                        let has_dynamic_offset = buffer_states[i].max_elements != 1;

                        buffer_binding.size = Some(
                            NonZeroU64::new(match has_dynamic_offset {
                                false => buffer_states[i].element_size as u64,
                                true => DEFAULT_DYNAMIC_BUFFER_MIN_BINDING_SIZE,
                            })
                            .unwrap(),
                        );

                        wgpu::BindGroupEntry {
                            binding: i as u32,
                            resource: wgpu::BindingResource::Buffer(buffer_binding),
                        }
                    })
                    .collect::<Vec<BindGroupEntry>>(),
                label: Some(&format!("uniform_bind_group: {}", type_name::<N>())),
            }),
        ));

        self.dest = Some(Arc::new(Mutex::new(UniformGroup {
            queue,
            id: self.id,
            bind_group: Arc::clone(&self.bind_group.as_ref().unwrap()),
            dynamic_offset_state: std::iter::repeat(0).take(buffer_states.len()).collect(),
            dynamic_offset_sizes: buffer_states.iter().map(|s| s.element_size).collect(),
            dynamic_offset_limits: buffer_states.iter().map(|s| s.max_elements).collect(),
            buffers: buffer_states.into_iter().map(|s| s.buffer).collect(),
            _marker: PhantomData,
        })));

        for builder in &self.uniforms {
            builder.lock().unwrap().build_to_resource(resources);
        }

        Ok(bind_group_layout)
    }

    fn dynamic(&self) -> Option<Vec<(u64, u64)>> {
        if self.dyn_offset_info.len() == 0 {
            return None;
        }
        Some(self.dyn_offset_info.clone())
    }

    fn binding(&self) -> (Uuid, Arc<wgpu::BindGroup>) {
        (self.id, Arc::clone(&self.bind_group.as_ref().unwrap()))
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
    fn dynamic(&self) -> Option<(u64, u64)>;
}

pub struct GenericUniformBuilder<U: Copy + Clone + bytemuck::Pod + bytemuck::Zeroable + Debug> {
    pub source: Option<U>,
    pub buffer: Option<wgpu::Buffer>,

    // Max sources to fit in one buffer
    pub dynamic: bool,
    pub max_size: Option<u32>,

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
            max_size: None,
            size: size_of::<U>() as u32,
            dest: None,
            dynamic: false,
        }
    }

    pub fn enable_dynamic_buffering(mut self) -> Self {
        self.dynamic = true;
        self
    }

    pub fn with_dynamic_entity_limit(mut self, max: u32) -> Self {
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
    element_size: u64,
    max_elements: u64,
}

impl<U> UniformBuilder for GenericUniformBuilder<U>
where
    U: Copy + Clone + bytemuck::Pod + bytemuck::Zeroable + Debug,
{
    fn build_buffer(&mut self, device: &wgpu::Device) -> BufferState {
        let source = &[self.source.unwrap()];

        let max_elements = match self.dynamic {
            false => 1,
            true => self
                .max_size
                .unwrap_or(DEFAULT_MAX_DYNAMIC_ENTITIES_PER_PASS),
        };

        let source_bytes = bytemuck::cast_slice(source);
        let source_size = source_bytes.len();
        let source_bytes = source_bytes.repeat(max_elements as usize);

        self.dest = Some(Arc::new(Mutex::new(GenericUniform {
            source: [self.source.unwrap()],
            dynamic: self.dynamic,
        })));

        BufferState {
            buffer: device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some(&format!("Uniform Buffer: {}", type_name::<U>())),
                contents: &source_bytes,
                usage: wgpu::BufferUsage::UNIFORM | wgpu::BufferUsage::COPY_DST,
            }),
            element_size: source_size as u64,
            max_elements: max_elements as u64,
        }
    }

    fn dynamic(&self) -> Option<(u64, u64)> {
        match self.dynamic {
            false => None,
            true => Some((
                self.size as u64,
                self.max_size
                    .unwrap_or(DEFAULT_MAX_DYNAMIC_ENTITIES_PER_PASS) as u64,
            )),
        }
    }
}

#[derive(Clone)]
pub struct GenericUniform<U: Copy + Clone + bytemuck::Pod + bytemuck::Zeroable + Debug> {
    pub source: [U; 1],
    pub dynamic: bool,
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

pub trait Uniform {
    fn as_bytes(&self) -> &[u8];
}

impl<U> Uniform for GenericUniform<U>
where
    U: Copy + Clone + bytemuck::Pod + bytemuck::Zeroable + Debug,
{
    fn as_bytes(&self) -> &[u8] {
        bytemuck::cast_slice(&self.source)
    }
}
