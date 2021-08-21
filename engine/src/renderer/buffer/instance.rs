use std::{
    any::type_name,
    marker::PhantomData,
    rc::Rc,
    sync::{Arc, Mutex, RwLock},
};
use uuid::Uuid;
use wgpu::util::DeviceExt;

use crate::{
    constants::ID,
    renderer::{
        mesh::Mesh,
        uniform::{generic::BufferState, group::BufferMode},
    },
};

pub struct InstanceBuffer<I: Instance> {
    pub state: BufferState,
    pub queue: Arc<wgpu::Queue>,
    marker: PhantomData<I>,
}

impl<I> InstanceBuffer<I>
where
    I: Instance,
{
    pub fn new(device: &wgpu::Device, queue: Arc<wgpu::Queue>, max_elements: u32) -> Self {
        let source = &[I::default()];
        let source_bytes = bytemuck::cast_slice(source);
        let source_size = source_bytes.len();
        let source_bytes = source_bytes.repeat(max_elements as usize);
        Self {
            state: BufferState {
                buffer: device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some(&format!("Instance Buffer: {}", type_name::<I>())),
                    contents: &source_bytes,
                    usage: wgpu::BufferUsage::VERTEX | wgpu::BufferUsage::COPY_DST,
                }),
                element_size: source_size as u64,
                mode: BufferMode::Dynamic(max_elements),
            },
            queue,
            marker: PhantomData,
        }
    }

    pub fn load_group(&self, bytes: &[u8]) {
        self.queue.write_buffer(&self.state.buffer, 0, bytes);
    }
}

// A group of components which can be rendered with one instanced draw call.
// These share textures and meshes.
pub struct InstanceGroup<I: Instance> {
    pub id: u32,
    pub instances: Vec<I>,
    pub components: Arc<RwLock<Vec<Vec<Arc<dyn InstanceMutator<I>>>>>>,
    pub texture: Uuid,
    next_id: InstanceId,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct InstanceId(pub u32, pub u32);

pub trait Instance: bytemuck::Pod + bytemuck::Zeroable + Clone + Default {
    fn id(&self) -> (u32, u32);
    fn size() -> usize;
    fn set_id(&mut self, group_id: u32, inst_id: u32);
}

pub trait InstanceMutator<I: Instance>: Send + Sync {
    fn mutate(&self, instance: &mut I);
}

impl<I> InstanceGroup<I>
where
    I: Instance,
{
    pub fn new(id: u32, texture: Uuid) -> Self {
        Self {
            next_id: InstanceId(id, 0),
            instances: vec![],
            components: Arc::new(RwLock::new(vec![])),
            texture,
            id,
        }
    }

    pub fn push(
        &mut self,
        mut instance: I,
        instance_components: Vec<Arc<dyn InstanceMutator<I>>>,
    ) -> InstanceId {
        instance.set_id(self.id, self.next_id.1);
        self.instances.push(instance);
        self.components.write().unwrap().push(instance_components);

        let old_id = self.next_id;
        self.next_id.1 += 1;
        old_id
    }

    pub fn delete(&mut self, id: u32) {
        if let Some(index) = self.instances.iter().position(|inst| inst.id().1 == id) {
            self.instances.swap_remove(index);
        }
    }
}

pub trait InstanceGroupBinder {
    fn num_instances(&self) -> usize;
    fn buffer_bytes(&self) -> &[u8];
    fn texture(&self) -> Uuid;
    fn bind_mesh<'rp>(&self, render_pass: wgpu::RenderPass<'rp>);
}

impl<I> InstanceGroupBinder for InstanceGroup<I>
where
    I: Instance + bytemuck::Pod,
{
    fn num_instances(&self) -> usize {
        self.instances.len()
    }

    fn buffer_bytes(&self) -> &[u8] {
        bytemuck::cast_slice(self.instances.as_slice())
    }

    fn texture(&self) -> Uuid {
        self.texture
    }

    fn bind_mesh<'rp>(&self, render_pass: wgpu::RenderPass<'rp>) {}
}
