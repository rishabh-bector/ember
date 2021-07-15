use std::fmt::Debug;

use wgpu::util::DeviceExt;

pub type ShaderStage = wgpu::ShaderStage;

pub struct UniformBuffer<U: Copy + Clone + bytemuck::Pod + bytemuck::Zeroable + Debug> {
    pub source: U,
    pub buffer: wgpu::Buffer,
    pub visibility: ShaderStage,
    pub bind_group_layout: wgpu::BindGroupLayout,
    pub bind_group: wgpu::BindGroup,
}

impl<U> UniformBuffer<U>
where
    U: Copy + Clone + bytemuck::Pod + bytemuck::Zeroable + Debug,
{
    pub fn generic(source: U, visibility: ShaderStage, device: &wgpu::Device) -> Self {
        let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Uniform Buffer"),
            contents: bytemuck::cast_slice(&[source]),
            usage: wgpu::BufferUsage::UNIFORM | wgpu::BufferUsage::COPY_DST,
        });

        let bind_group_layout = UniformBuffer::<U>::bind_group_layout(device);

        UniformBuffer {
            visibility,
            source,
            bind_group: device.create_bind_group(&wgpu::BindGroupDescriptor {
                layout: &bind_group_layout,
                entries: &[wgpu::BindGroupEntry {
                    binding: 0,
                    resource: buffer.as_entire_binding(),
                }],
                label: Some("uniform_bind_group"),
            }),
            buffer,
            bind_group_layout,
        }
    }

    pub fn bind_render_pass<'a>(
        &'a self,
        render_pass: &mut wgpu::RenderPass<'a>,
        bind_index: u32,
        queue: &wgpu::Queue,
    ) {
        println!("Binding: {:?}", self.source);
        queue.write_buffer(&self.buffer, 0, bytemuck::cast_slice(&[self.source]));
        render_pass.set_bind_group(bind_index, &self.bind_group, &[]);
    }

    pub fn bind_group_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStage::VERTEX_FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
            label: Some("uniform_bind_group_layout"),
        })
    }
}
