use std::sync::Arc;
use wgpu::util::DeviceExt;

pub mod instance;
pub mod texture;

// Vertex Layout Builder
// - Automatically generate vertex buffer layouts
//   for structs based on macros! (MAKE THIS A SEPARATE CRATE PROOOOOJECT)
//
// - Then I can automatically generate the buffer layouts
//   need to make any generic uniform struct instancable!

// Vertex and index buffers

#[vertex((0, 20usize))]
#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct Vertex3D {
    pub position: [f32; 3],
    pub uvs: [f32; 2],
}

unsafe impl bytemuck::Pod for Vertex3D {}
unsafe impl bytemuck::Zeroable for Vertex3D {}

#[vertex((0, 16usize))]
#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct Vertex2D {
    pub position: [f32; 2],
    pub uvs: [f32; 2],
}

unsafe impl bytemuck::Pod for Vertex2D {}
unsafe impl bytemuck::Zeroable for Vertex2D {}

pub struct VertexBuffer {
    pub buffer: Arc<(wgpu::Buffer, u32)>,
    pub size: u32,
}

impl VertexBuffer {
    pub fn new_2d(name: &str, vertices: &[Vertex2D], device: &wgpu::Device) -> Self {
        VertexBuffer {
            buffer: Arc::new((
                device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some(&format!("2D Vertex Buffer: {}", name)),
                    contents: bytemuck::cast_slice(vertices),
                    usage: wgpu::BufferUsage::VERTEX,
                }),
                vertices.len() as u32,
            )),
            size: vertices.len() as u32,
        }
    }

    pub fn new_3d(name: &str, vertices: &[Vertex3D], device: &wgpu::Device) -> Self {
        VertexBuffer {
            buffer: Arc::new((
                device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some(&format!("3D Vertex Buffer: {}", name)),
                    contents: bytemuck::cast_slice(vertices),
                    usage: wgpu::BufferUsage::VERTEX,
                }),
                vertices.len() as u32,
            )),
            size: vertices.len() as u32,
        }
    }
    pub fn layout_2d<'a>() -> wgpu::VertexBufferLayout<'a> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Vertex2D>() as wgpu::BufferAddress,
            step_mode: wgpu::InputStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x2,
                },
                wgpu::VertexAttribute {
                    offset: std::mem::size_of::<[f32; 2]>() as wgpu::BufferAddress,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x2,
                },
            ],
        }
    }

    pub fn layout_3d<'a>() -> wgpu::VertexBufferLayout<'a> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Vertex3D>() as wgpu::BufferAddress,
            step_mode: wgpu::InputStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x3,
                },
                wgpu::VertexAttribute {
                    offset: std::mem::size_of::<[f32; 3]>() as wgpu::BufferAddress,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x2,
                },
            ],
        }
    }
}

pub struct IndexBuffer {
    pub buffer: Arc<(wgpu::Buffer, u32)>,
    pub size: u32,
}

impl IndexBuffer {
    pub fn new(indices: &[u16], device: &wgpu::Device) -> Self {
        IndexBuffer {
            buffer: Arc::new((
                device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("Index Buffer"),
                    contents: bytemuck::cast_slice(indices),
                    usage: wgpu::BufferUsage::INDEX,
                }),
                indices.len() as u32,
            )),
            size: indices.len() as u32,
        }
    }
}
