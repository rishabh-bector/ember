use std::fmt::Debug;

use self::{
    generic::{BufferState, GenericUniform},
    group::BufferMode,
};
use crate::sources::ResourceBuilder;

pub mod generic;
pub mod group;

pub trait Uniform {
    fn write_buffer(&self, queue: &wgpu::Queue, buffer: &wgpu::Buffer);
}

impl<U> Uniform for GenericUniform<U>
where
    U: Copy + Clone + bytemuck::Pod + bytemuck::Zeroable + Debug,
{
    fn write_buffer(&self, queue: &wgpu::Queue, buffer: &wgpu::Buffer) {
        queue.write_buffer(buffer, 0, bytemuck::cast_slice(&self.source));
    }
}
pub trait UniformBuilder: ResourceBuilder {
    // -> (buffer, source size, max dynamic offsets per render pass)
    fn build_buffer(&mut self, device: &wgpu::Device, mode: BufferMode) -> BufferState;
    fn single_buffer(&self, device: &wgpu::Device) -> BufferState;
    fn dynamic_size(&self) -> u64;
}
