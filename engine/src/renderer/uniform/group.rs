use anyhow::{anyhow, Result};
use legion::Resources;
use std::{
    any::type_name,
    fmt::Debug,
    marker::PhantomData,
    num::NonZeroU64,
    sync::{Arc, Mutex, MutexGuard},
};
use uuid::Uuid;
use wgpu::BindGroupEntry;

use crate::{
    constants::{
        DEFAULT_DYNAMIC_BUFFER_MIN_BINDING_SIZE, DEFAULT_MAX_DYNAMIC_ENTITIES_PER_PASS,
        DEFAULT_MAX_INSTANCES_PER_BUFFER,
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

impl BufferMode {
    pub fn is_instance(&self) -> bool {
        if let BufferMode::Instance(_) = &self {
            return true;
        }
        false
    }
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

// Ways to draw multiple Render3D components:
//  - Instancing (each batch must share a texture, v/i buffers, etc.)
//      - For each group: all instance uniforms are loaded into a vertex buffer (per group type)
//        by the render system before the group draw call is submitted.
//  - Singleton (each can have its own texture, etc. which also req)
//      - One UniformGroup (e.g. Render3DForwardUniformGroup) can have many GroupStates
//      - A GroupState contains the data needed for a unique object in a render pass:
//          - buffers (one for each uniform in the group)
//          - bind group
//      - The GroupState should be owned by the component, e.g. Render3D
//        (The group can have one default GroupState to allow for dynamic uniforms in the future...)
//      - Uniforms are loaded by a par_for_each which iterates all Render3D components (does this work?)
//      - This way, a node builder can still take one UniformGroup, which defines
//        the overall layout. Then, each time the user submits a Render3D component,
//        it needs to be given a BufferState. How? Options:
//          - 1. User submits a Render3DBuilder component, which contains all the data for a Render3D,
//            then a system continously consumes all Render3DBuilder(s) and turns them into Render3Ds
//          - 2. User submits an "incomplete" Render3D with an Option<GroupState>
//          - Leaning towards option 1 atm.

#[derive(Clone, Debug)]
pub struct GroupState {
    pub buffers: Arc<Vec<wgpu::Buffer>>,
    pub bind_group: Arc<wgpu::BindGroup>,
    pub queue: Arc<wgpu::Queue>,
}

impl GroupState {
    pub fn write_buffer(&self, index: usize, source_bytes: &[u8]) {
        self.queue
            .write_buffer(&self.buffers[index], 0, source_bytes)
    }
}

pub struct UniformGroup<N> {
    pub default_state: GroupState,
    pub states: Vec<Arc<GroupState>>,
    pub mode: BufferMode,

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

    pub fn default_buffer(&self, index: usize) -> &wgpu::Buffer {
        &self.default_state.buffers[index]
    }

    pub fn write_buffer(&self, index: usize, source_bytes: &[u8]) {
        self.queue
            .write_buffer(&self.default_state.buffers[index], 0, source_bytes)
    }

    pub fn begin_dynamic_loading(&mut self) {
        self.dynamic_offsets.state.iter_mut().for_each(|i| *i = 0);
    }

    pub fn load_dynamic_uniform(&mut self, source_bytes: &[u8]) {
        for i in 0..self.default_state.buffers.len() {
            self.queue.write_buffer(
                &self.default_state.buffers[i],
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

    pub fn default_bind_group(&self) -> Arc<wgpu::BindGroup> {
        Arc::clone(&self.default_state.bind_group)
    }
}

pub trait GroupBuilder {
    fn mode(&self) -> BufferMode;
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
    pub entries: Option<Vec<wgpu::BindGroupLayoutEntry>>,

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
            entries: None,
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
        self.mode = BufferMode::Instance(DEFAULT_MAX_INSTANCES_PER_BUFFER);
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
    fn mode(&self) -> BufferMode {
        self.mode
    }

    fn build(
        &mut self,
        device: &wgpu::Device,
        resources: &mut Resources,
        queue: Arc<wgpu::Queue>,
    ) -> Result<wgpu::BindGroupLayout> {
        debug!(
            "UniformGroupBuilder: building {} with {} bind entries",
            type_name::<N>(),
            self.uniforms.len()
        );

        if let Some(entries) = &self.entries {
            debug!("This uniform group has already been built; reusing");
            return Ok(
                device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    entries: &entries,
                    label: Some(&format!("uniform_bind_group_layout: {}", type_name::<N>())),
                }),
            );
        }

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
        self.entries = Some(entries.clone());

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
            dynamic_offsets: DynamicOffsets {
                state: std::iter::repeat(0).take(buffer_states.len()).collect(),
                sizes: buffer_states.iter().map(|s| s.element_size).collect(),
                limits: buffer_states.iter().map(|s| s.max_elements).collect(),
            },
            default_state: GroupState {
                buffers: Arc::new(buffer_states.into_iter().map(|s| s.buffer).collect()),
                bind_group: Arc::clone(&self.bind_group.as_ref().unwrap()),
                queue: Arc::clone(&queue),
            },
            entity_count: Arc::clone(&self.entity_count),
            states: vec![],
            queue,
            id: self.id,
            mode: self.mode,
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

pub trait SingleStateBuilder {
    fn single_state(&self, device: &wgpu::Device) -> Result<GroupState>;
}

impl<N> SingleStateBuilder for UniformGroupBuilder<N>
where
    N: Send + Sync + 'static,
{
    fn single_state(&self, device: &wgpu::Device) -> Result<GroupState> {
        debug!(
            "UniformGroupBuilder: new state {} with {} bind entries",
            type_name::<N>(),
            self.uniforms.len()
        );

        if self.uniforms.len() == 0 {
            return Err(anyhow!(
                "GroupBuilder: must provide at least one uniform builder"
            ));
        }

        let buffer_states: Vec<BufferState> = self
            .uniforms
            .iter()
            .map(|builder| builder.lock().unwrap().single_buffer(device))
            .collect();

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &self
                .entries
                .as_ref()
                .expect("group must be built before group states"),
            label: Some(&format!("uniform_bind_group_layout: {}", type_name::<N>())),
        });

        let bind_group = Some(Arc::new(
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

        Ok(GroupState {
            buffers: Arc::new(buffer_states.into_iter().map(|s| s.buffer).collect()),
            bind_group: Arc::clone(&self.bind_group.as_ref().unwrap()),
            queue: Arc::clone(&self.dest.as_ref().unwrap().lock().unwrap().queue),
        })
    }
}

impl<N> SingleStateBuilder for MutexGuard<'_, UniformGroupBuilder<N>>
where
    N: 'static,
{
    fn single_state(&self, device: &wgpu::Device) -> Result<GroupState> {
        self.single_state(device)
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
        let group = resources
            .remove::<Arc<Mutex<UniformGroup<N>>>>()
            .unwrap_or_else(|| Arc::clone(self.dest.as_ref().unwrap()));
        resources.insert::<Arc<Mutex<UniformGroup<N>>>>(group);
    }
}
