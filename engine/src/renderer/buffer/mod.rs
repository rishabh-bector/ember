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

#[vertex((0, 32usize))]
#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct Vertex3D {
    pub position: [f32; 3],
    pub uvs: [f32; 2],
    pub normal: [f32; 3],
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

    pub fn raw(name: &str, data: &[f32], num_vertices: u32, device: &wgpu::Device) -> Self {
        VertexBuffer {
            buffer: Arc::new((
                device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some(&format!("Raw Vertex Buffer: {}", name)),
                    contents: bytemuck::cast_slice(data),
                    usage: wgpu::BufferUsage::VERTEX,
                }),
                num_vertices,
            )),
            size: num_vertices,
        }
    }

    pub fn from_flat_slices(
        name: &str,
        vertices_flat: &[f32],
        uvs_flat: &[f32],
        normals_flat: &[f32],
        device: &wgpu::Device,
    ) -> (Self, Vec<f32>) {
        let num_vertices = vertices_flat.len() / 3;
        assert_eq!(num_vertices, uvs_flat.len() / 2);
        assert_eq!(num_vertices, normals_flat.len() / 3);

        let mut buf: Vec<f32> = vec![];
        for i in 0..num_vertices {
            buf.push(vertices_flat[i * 3]);
            buf.push(vertices_flat[i * 3 + 1]);
            buf.push(vertices_flat[i * 3 + 2]);

            buf.push(uvs_flat[i * 2]);
            buf.push(uvs_flat[i * 2 + 1]);

            buf.push(normals_flat[i * 3]);
            buf.push(normals_flat[i * 3 + 1]);
            buf.push(normals_flat[i * 3 + 2]);
        }

        (
            VertexBuffer {
                buffer: Arc::new((
                    device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                        label: Some(&format!("3D Vertex Buffer: {}", name)),
                        contents: bytemuck::cast_slice(buf.as_slice()),
                        usage: wgpu::BufferUsage::VERTEX,
                    }),
                    buf.len() as u32,
                )),
                size: buf.len() as u32,
            },
            buf,
        )
    }
}

pub struct IndexBuffer {
    pub buffer: Arc<(wgpu::Buffer, u32)>,
    pub size: u32,
}

impl IndexBuffer {
    pub fn new(indices: &[u32], device: &wgpu::Device) -> Self {
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
