use std::fmt::Debug;

use self::{
    generic::{BufferState, GenericUniform},
    group::BufferMode,
};
use crate::sources::ResourceBuilder;

pub mod generic;
pub mod group;

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
pub trait UniformBuilder: ResourceBuilder {
    // -> (buffer, source size, max dynamic offsets per render pass)
    fn build_buffer(&mut self, device: &wgpu::Device, mode: BufferMode) -> BufferState;
    fn dynamic_size(&self) -> u64;
}
