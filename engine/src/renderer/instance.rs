use anyhow::Result;
use std::{any::type_name, marker::PhantomData};
use uuid::Uuid;
use wgpu::util::DeviceExt;

use crate::constants::{ID, RENDER_2D_COMMON_TEXTURE_ID};

use super::uniform::generic::BufferState;

pub struct InstanceBuffer<I: Instance> {
    pub state: BufferState,
    _marker: PhantomData<I>,
}

impl<I> InstanceBuffer<I>
where
    I: Instance,
{
    pub fn new(device: &wgpu::Device, max_elements: u32) -> Self {
        let source = &[I::default()];
        let source_bytes = bytemuck::cast_slice(source);
        let source_size = source_bytes.len();
        let source_bytes = source_bytes.repeat(max_elements as usize);
        Self {
            state: BufferState {
                buffer: device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some(&format!("Instance Buffer: {}", type_name::<I>())),
                    contents: &source_bytes,
                    usage: wgpu::BufferUsage::VERTEX,
                }),
                element_size: source_size as u64,
                max_elements: max_elements as u64,
            },
            _marker: PhantomData,
        }
    }
}

// A group of components which can be rendered with one instanced draw call.
// These share textures and vertex/index buffers.
pub struct InstanceGroup<T: Instance> {
    pub id: Uuid,

    instances: Vec<T>,
    next_id: InstanceId,

    pub texture: Uuid,
    pub geometry: (Uuid, Uuid),
}

#[derive(Clone, Copy)]
pub struct InstanceId(u32);

pub trait Instance: bytemuck::Pod + bytemuck::Zeroable + Clone + Default {
    fn get_id(&self) -> u32;
    fn set_id(&mut self, id: u32);
    fn size() -> usize;
}

impl<T> InstanceGroup<T>
where
    T: Instance,
{
    pub fn new(texture: Uuid, geometry: (Uuid, Uuid)) -> Self {
        Self {
            id: Uuid::new_v4(),
            instances: Vec::new(),
            next_id: InstanceId(1),
            texture,
            geometry,
        }
    }

    pub fn insert(&mut self, mut instance: T) -> InstanceId {
        instance.set_id(self.next_id.0);
        self.instances.push(instance);

        let old_id = self.next_id;
        self.next_id.0 += 1;
        old_id
    }

    pub fn delete(&mut self, id: u32) {
        if let Some(index) = self.instances.iter().position(|inst| inst.get_id() == id) {
            self.instances.swap_remove(index);
        }
    }
}

pub trait InstanceGroupBinder {
    fn num_instances(&self) -> usize;
    fn buffer_bytes(&self) -> &[u8];
    fn texture(&self) -> Uuid;
    fn geometry(&self) -> (Uuid, Uuid);
}

impl<T> InstanceGroupBinder for InstanceGroup<T>
where
    T: Instance + bytemuck::Pod,
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

    fn geometry(&self) -> (Uuid, Uuid) {
        self.geometry
    }
}
