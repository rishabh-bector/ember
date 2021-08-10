use anyhow::{anyhow, Result};
use legion::Resources;
use std::{
    any::type_name,
    fmt::Debug,
    marker::PhantomData,
    num::NonZeroU64,
    sync::{Arc, Mutex},
};
use uuid::Uuid;
use wgpu::BindGroupEntry;

use crate::{
    constants::{
        DEFAULT_DYNAMIC_BUFFER_MIN_BINDING_SIZE, DEFAULT_MAX_DYNAMIC_ENTITIES_PER_PASS,
        DEFAULT_MAX_INSTANCES_PER_PASS,
    },
    renderer::uniform::generic::BufferState,
    sources::ResourceBuilder,
};

use super::UniformBuilder;

#[derive(Clone, Copy)]
pub enum BufferMode {
    Single,
    Dynamic(u32),
    Instance(u32),
}

pub struct DynamicOffsets {
    pub limits: Vec<u64>,
    pub sizes: Vec<u64>,
    pub state: Vec<u64>,
}

pub trait UniformSource:
    Copy + Clone + bytemuck::Pod + bytemuck::Zeroable + Debug + 'static
{
}

pub trait Group {
    fn into<N>() -> N;
}

pub struct UniformGroup<N> {
    pub buffers: Vec<wgpu::Buffer>,
    pub mode: BufferMode,
    pub bind_group: Arc<wgpu::BindGroup>,

    pub dynamic_offsets: DynamicOffsets,

    pub id: Uuid,
    pub queue: Arc<wgpu::Queue>,
    pub entity_count: Arc<Mutex<u64>>,

    _marker: PhantomData<N>,
}

impl<N> UniformGroup<N> {
    pub fn builder() -> UniformGroupBuilder<N> {
        UniformGroupBuilder::new()
    }

    pub fn load_buffer(&self, index: usize, source_bytes: &[u8]) {
        self.queue
            .write_buffer(&self.buffers[index], 0, source_bytes)
    }

    pub fn begin_dynamic_loading(&mut self) {
        self.dynamic_offsets.state.iter_mut().for_each(|i| *i = 0);
    }

    pub fn load_dynamic_uniform(&mut self, source_bytes: &[u8]) {
        for i in 0..self.buffers.len() {
            self.queue.write_buffer(
                &self.buffers[i],
                self.dynamic_offsets.state[i],
                source_bytes,
            );
            self.increase_offset(i);
        }
    }

    pub fn increase_offset(&mut self, index: usize) -> u32 {
        let old = self.dynamic_offsets.state[index];
        self.dynamic_offsets.state[index] += self.dynamic_offsets.sizes[index];
        old as u32
    }

    pub fn bind_group(&self) -> Arc<wgpu::BindGroup> {
        Arc::clone(&self.bind_group)
    }
}

pub trait GroupBuilder {
    fn build(
        &mut self,
        device: &wgpu::Device,
        resources: &mut Resources,
        queue: Arc<wgpu::Queue>,
    ) -> Result<wgpu::BindGroupLayout>;

    fn dynamic(&self) -> Option<(Arc<Mutex<u64>>, Vec<(u64, u64)>)>;
    fn binding(&self) -> (Uuid, Arc<wgpu::BindGroup>);
}

pub trait GroupResourceBuilder: GroupBuilder + ResourceBuilder {}
impl<N> GroupResourceBuilder for UniformGroupBuilder<N> where N: 'static {}

pub struct UniformGroupBuilder<N> {
    pub uniforms: Vec<Arc<Mutex<dyn UniformBuilder>>>,
    pub mode: BufferMode,

    pub bind_group_layout: Option<wgpu::BindGroupLayout>,
    pub bind_group: Option<Arc<wgpu::BindGroup>>,

    pub id: Uuid,
    pub state: Option<N>,
    pub dest: Option<Arc<Mutex<UniformGroup<N>>>>,

    pub dyn_offset_info: Vec<(u64, u64)>,
    pub entity_count: Arc<Mutex<u64>>,
}

impl<N> UniformGroupBuilder<N> {
    pub fn new() -> Self {
        Self {
            mode: BufferMode::Single,
            uniforms: vec![],
            bind_group_layout: None,
            bind_group: None,
            state: None,
            dest: None,
            id: Uuid::new_v4(),
            dyn_offset_info: vec![],
            entity_count: Arc::new(Mutex::new(0)),
        }
    }

    pub fn with_uniform<T: UniformBuilder + 'static>(mut self, uniform: T) -> Self {
        // if let Some(offset_info) = uniform.dynamic() {
        //     self.dyn_offset_info.push(offset_info);
        // }
        self.uniforms.push(Arc::new(Mutex::new(uniform)));
        self
    }

    pub fn with_id(mut self, id: Uuid) -> Self {
        self.id = id;
        self
    }

    pub fn mode_instance(mut self) -> Self {
        self.mode = BufferMode::Instance(DEFAULT_MAX_INSTANCES_PER_PASS);
        self
    }

    pub fn with_instance_limit(mut self, max: u32) -> Self {
        self.mode = BufferMode::Instance(max);
        self
    }

    pub fn mode_dynamic(mut self) -> Self {
        self.mode = BufferMode::Dynamic(DEFAULT_MAX_DYNAMIC_ENTITIES_PER_PASS);
        self
    }

    pub fn with_dynamic_entity_limit(mut self, max: u32) -> Self {
        self.mode = BufferMode::Dynamic(max);
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

        let mode = self.mode;
        let buffer_states: Vec<BufferState> = self
            .uniforms
            .iter_mut()
            .map(|builder| builder.lock().unwrap().build_buffer(device, mode))
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
            mode: self.mode,
            bind_group: Arc::clone(&self.bind_group.as_ref().unwrap()),
            dynamic_offsets: DynamicOffsets {
                state: std::iter::repeat(0).take(buffer_states.len()).collect(),
                sizes: buffer_states.iter().map(|s| s.element_size).collect(),
                limits: buffer_states.iter().map(|s| s.max_elements).collect(),
            },
            buffers: buffer_states.into_iter().map(|s| s.buffer).collect(),
            entity_count: Arc::clone(&self.entity_count),
            _marker: PhantomData,
        })));

        for builder in &self.uniforms {
            builder.lock().unwrap().build_to_resource(resources);
        }

        Ok(bind_group_layout)
    }

    fn dynamic(&self) -> Option<(Arc<Mutex<u64>>, Vec<(u64, u64)>)> {
        if self.dyn_offset_info.len() == 0 {
            return None;
        }
        Some((Arc::clone(&self.entity_count), self.dyn_offset_info.clone()))
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
        // If this group already exists, then keep the existing one
        let group_builder = resources
            .remove::<Arc<Mutex<UniformGroup<N>>>>()
            .unwrap_or_else(|| Arc::clone(self.dest.as_ref().unwrap()));
        resources.insert::<Arc<Mutex<UniformGroup<N>>>>(group_builder);
    }
}
