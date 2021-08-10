use anyhow::Result;
use uuid::Uuid;

use crate::constants::{ID, RENDER_2D_COMMON_TEXTURE_ID};

use super::ResourceBuilder;

// A group of components which can be rendered with one instanced draw call.
// These share textures and vertex/index buffers.
pub struct InstanceGroup<T: Instance> {
    pub id: Uuid,
    instances: Vec<T>,

    pub texture: Uuid,
    pub common_vertex_buffer: usize,
    pub common_index_buffer: usize,
}

pub trait Instance {
    fn get_id(&self) -> u64;
    fn set_id(&mut self, id: u64);
}

impl<T> InstanceGroup<T>
where
    T: Instance,
{
    pub fn new(texture: Uuid, common_vertex_buffer: usize, common_index_buffer: usize) -> Self {
        Self {
            id: Uuid::new_v4(),
            instances: Vec::new(),
            texture,
            common_index_buffer,
            common_vertex_buffer,
        }
    }

    pub fn insert(&mut self, instance: T) {
        self.instances.push(instance);
    }

    pub fn delete(&mut self, id: u64) {
        if let Some(index) = self.instances.iter().position(|inst| inst.get_id() == id) {
            self.instances.swap_remove(index);
        }
    }
}

pub trait InstanceGroupBinder {
    fn buffer_bytes(&self) -> &[u8];
    fn group_texture(&self) -> Uuid;
    fn group_geometry(&self) -> (usize, usize);
}

impl<T> InstanceGroupBinder for InstanceGroup<T>
where
    T: Instance + bytemuck::Pod,
{
    fn buffer_bytes(&self) -> &[u8] {
        bytemuck::cast_slice(self.instances.as_slice())
    }

    fn group_texture(&self) -> Uuid {
        self.texture
    }

    fn group_geometry(&self) -> (usize, usize) {
        (self.common_vertex_buffer, self.common_index_buffer)
    }
}
