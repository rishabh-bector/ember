use std::sync::Arc;

use crate::renderer::buffer::{IndexBuffer, Mesh, Vertex2D, Vertex3D, VertexBuffer};

use super::registry::MeshBuilder;

pub enum PrimitiveMesh {
    UnitSquare,
    UnitCube,
}

impl MeshBuilder for PrimitiveMesh {
    fn build(&self, device: Arc<wgpu::Device>) -> Mesh {
        match &self {
            PrimitiveMesh::UnitSquare => unit_square(&device),
            PrimitiveMesh::UnitCube => unit_cube(&device),
        }
    }
}

pub fn unit_square(device: &wgpu::Device) -> Mesh {
    Mesh {
        vertices: VertexBuffer::new_2d(
            "unit_square",
            &[
                Vertex2D {
                    position: [-1.0, -1.0],
                    uvs: [0.0, 1.0],
                },
                Vertex2D {
                    position: [-1.0, 1.0],
                    uvs: [0.0, 0.0],
                },
                Vertex2D {
                    position: [1.0, 1.0],
                    uvs: [1.0, 0.0],
                },
                Vertex2D {
                    position: [1.0, -1.0],
                    uvs: [1.0, 1.0],
                },
            ],
            &device,
        ),
        indices: IndexBuffer::new(&[0, 2, 1, 3, 2, 0], &device),
    }
}

pub fn unit_cube(device: &wgpu::Device) -> Mesh {
    Mesh {
        vertices: VertexBuffer::new_3d(
            "unit_cube",
            &[
                // Back face //
                Vertex3D {
                    position: [0.5, 0.5, -0.5],
                    uvs: [0.0, 0.0],
                },
                Vertex3D {
                    position: [0.5, -0.5, -0.5],
                    uvs: [0.0, 1.0],
                },
                Vertex3D {
                    position: [-0.5, -0.5, -0.5],
                    uvs: [1.0, 1.0],
                },
                Vertex3D {
                    position: [-0.5, -0.5, -0.5],
                    uvs: [1.0, 1.0],
                },
                Vertex3D {
                    position: [-0.5, 0.5, -0.5],
                    uvs: [1.0, 0.0],
                },
                Vertex3D {
                    position: [0.5, 0.5, -0.5],
                    uvs: [0.0, 0.0],
                },
                // Front face //
                Vertex3D {
                    position: [-0.5, -0.5, 0.5],
                    uvs: [0.0, 1.0],
                },
                Vertex3D {
                    position: [0.5, -0.5, 0.5],
                    uvs: [1.0, 1.0],
                },
                Vertex3D {
                    position: [0.5, 0.5, 0.5],
                    uvs: [1.0, 0.0],
                },
                Vertex3D {
                    position: [0.5, 0.5, 0.5],
                    uvs: [1.0, 0.0],
                },
                Vertex3D {
                    position: [-0.5, 0.5, 0.5],
                    uvs: [0.0, 0.0],
                },
                Vertex3D {
                    position: [-0.5, -0.5, 0.5],
                    uvs: [0.0, 1.0],
                },
                // Left face //
                Vertex3D {
                    position: [-0.5, 0.5, 0.5],
                    uvs: [1.0, 0.0],
                },
                Vertex3D {
                    position: [-0.5, 0.5, -0.5],
                    uvs: [0.0, 0.0],
                },
                Vertex3D {
                    position: [-0.5, -0.5, -0.5],
                    uvs: [0.0, 1.0],
                },
                Vertex3D {
                    position: [-0.5, -0.5, -0.5],
                    uvs: [0.0, 1.0],
                },
                Vertex3D {
                    position: [-0.5, -0.5, 0.5],
                    uvs: [1.0, 1.0],
                },
                Vertex3D {
                    position: [-0.5, 0.5, 0.5],
                    uvs: [1.0, 0.0],
                },
                // Right face //
                Vertex3D {
                    position: [0.5, -0.5, -0.5],
                    uvs: [1.0, 1.0],
                },
                Vertex3D {
                    position: [0.5, 0.5, -0.5],
                    uvs: [1.0, 0.0],
                },
                Vertex3D {
                    position: [0.5, 0.5, 0.5],
                    uvs: [0.0, 0.0],
                },
                Vertex3D {
                    position: [0.5, 0.5, 0.5],
                    uvs: [0.0, 0.0],
                },
                Vertex3D {
                    position: [0.5, -0.5, 0.5],
                    uvs: [0.0, 1.0],
                },
                Vertex3D {
                    position: [0.5, -0.5, -0.5],
                    uvs: [1.0, 1.0],
                },
                // Bottom face //
                Vertex3D {
                    position: [-0.5, -0.5, -0.5],
                    uvs: [0.0, 1.0],
                },
                Vertex3D {
                    position: [0.5, -0.5, -0.5],
                    uvs: [1.0, 1.0],
                },
                Vertex3D {
                    position: [0.5, -0.5, 0.5],
                    uvs: [1.0, 0.0],
                },
                Vertex3D {
                    position: [0.5, -0.5, 0.5],
                    uvs: [1.0, 0.0],
                },
                Vertex3D {
                    position: [-0.5, -0.5, 0.5],
                    uvs: [0.0, 0.0],
                },
                Vertex3D {
                    position: [-0.5, -0.5, -0.5],
                    uvs: [0.0, 1.0],
                },
                // Top face //
                Vertex3D {
                    position: [0.5, 0.5, 0.5],
                    uvs: [1.0, 1.0],
                },
                Vertex3D {
                    position: [0.5, 0.5, -0.5],
                    uvs: [1.0, 0.0],
                },
                Vertex3D {
                    position: [-0.5, 0.5, -0.5],
                    uvs: [0.0, 0.0],
                },
                Vertex3D {
                    position: [-0.5, 0.5, -0.5],
                    uvs: [0.0, 0.0],
                },
                Vertex3D {
                    position: [-0.5, 0.5, 0.5],
                    uvs: [0.0, 1.0],
                },
                Vertex3D {
                    position: [0.5, 0.5, 0.5],
                    uvs: [1.0, 1.0],
                },
            ],
            device,
        ),
        indices: IndexBuffer::new(
            &[
                0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22,
                23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 33, 34, 35,
            ],
            device,
        ),
    }
}
