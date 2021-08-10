use anyhow::Result;
use uuid::Uuid;

use crate::constants::{ID, RENDER_2D_COMMON_TEXTURE_ID};

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

pub trait Instance {
    fn get_id(&self) -> u32;
    fn set_id(&mut self, id: u32);
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
