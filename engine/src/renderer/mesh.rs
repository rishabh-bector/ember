use std::sync::Arc;

use uuid::Uuid;

use crate::sources::registry::MeshBuilder;

use super::buffer::{IndexBuffer, Vertex3D, VertexBuffer};

pub struct Mesh {
    pub vertices: Vec<f32>,
    pub indices: Vec<u16>,
    pub vertex_buffer: VertexBuffer,
    pub index_buffer: IndexBuffer,
}

impl Mesh {
    pub fn vertex_buffer(&self) -> Arc<(wgpu::Buffer, u32)> {
        Arc::clone(&self.vertex_buffer.buffer)
    }

    pub fn index_buffer(&self) -> Arc<(wgpu::Buffer, u32)> {
        Arc::clone(&self.index_buffer.buffer)
    }
}

pub struct ObjLoader {
    pub id: Uuid,
    pub path: String,
}

impl ObjLoader {
    pub fn new(path: String) -> Self {
        Self {
            path,
            id: Uuid::new_v4(),
        }
    }
}

impl ObjLoader {
    pub fn arc_dyn(self) -> Arc<dyn MeshBuilder> {
        Arc::new(self)
    }
}

impl MeshBuilder for ObjLoader {
    fn build(&self, device: Arc<wgpu::Device>) -> Mesh {
        let (models, _) = tobj::load_obj(&self.path, &tobj::LoadOptions::default()).unwrap();
        let mesh = &models[0].mesh;

        // Only load one face for now
        let face_size = mesh.face_arities[0] as usize;

        let flat_vertices: &[f32] = &mesh.positions[0..face_size];
        let flat_uvs: &[f32] = &mesh.texcoords[0..face_size];

        let (vertex_buffer, vertices) =
            VertexBuffer::from_flat_slices(&self.path, flat_vertices, flat_uvs, &device);
        let indices: Vec<u16> = mesh.indices[0..face_size]
            .iter()
            .map(|i| *i as u16)
            .collect();

        Mesh {
            index_buffer: IndexBuffer::new(&indices, &device),
            indices,
            vertices,
            vertex_buffer,
        }
    }
}

// pub fn buffer_flat_slices_3d(flat_vertices: &[f32], flat_uvs: &[f32]) -> Vec<Vertex3D> {
//     let num_vertices = flat_vertices.len() / 3;
//     assert_eq!(num_vertices, flat_uvs.len() / 3);

//     let mut vec: Vec<Vertex3D> = vec![];
//     for i in 0..num_vertices {
//         vec.push(Vertex3D {
//             position: [
//                 flat_vertices[i * 3],
//                 flat_vertices[i * 3 + 1],
//                 flat_vertices[i * 3 + 2],
//             ],
//             uvs: [flat_uvs[i * 2], flat_uvs[i * 2 + 1]],
//         });
//     }
//     vec
// }
