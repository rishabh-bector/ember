use std::sync::Arc;
use uuid::Uuid;

use crate::sources::registry::MeshBuilder;

use super::buffer::{IndexBuffer, VertexBuffer};

pub struct Mesh {
    pub vertices: Vec<f32>,
    pub indices: Vec<u16>,
    pub vertex_buffer: VertexBuffer,
    pub index_buffer: IndexBuffer,
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
        debug!("building obj meshes from file: {}", &self.path);

        let options = tobj::LoadOptions {
            triangulate: true,
            single_index: true,
            ignore_lines: true,
            ignore_points: true,
            ..Default::default()
        };
        let (models, _) = tobj::load_obj(&self.path, &options).unwrap();
        debug!("obj contains {} meshes which will be merged", models.len());

        let mut flat_vertices: Vec<f32> = vec![];
        let mut flat_uvs: Vec<f32> = vec![];
        let mut flat_normals: Vec<f32> = vec![];

        let mut indices: Vec<u16> = vec![];
        let mut mesh_index_offset: u16 = 0;
        for i in 0..models.len() {
            // if i != 0 {
            //     continue;
            // }

            let mesh = &models[i].mesh;
            debug!(
                "building mesh {} with {} triangles and {} indices (faces: {})",
                i,
                mesh.positions.len() / 3,
                mesh.indices.len(),
                mesh.face_arities.len(),
            );

            let mesh = &models[i].mesh;
            for index in 0..mesh.positions.len() / 3 {
                flat_vertices.push(mesh.positions[3 * index]);
                flat_vertices.push(mesh.positions[3 * index + 1]);
                flat_vertices.push(mesh.positions[3 * index + 2]);

                flat_uvs.push(mesh.texcoords[2 * index]);
                flat_uvs.push(mesh.texcoords[2 * index + 1]);

                flat_normals.push(mesh.normals[3 * index]);
                flat_normals.push(mesh.normals[3 * index + 1]);
                flat_normals.push(mesh.normals[3 * index + 2]);
            }

            indices.extend(mesh.indices.iter().map(|i| mesh_index_offset + *i as u16));
            mesh_index_offset = indices.len() as u16 + 1;
        }

        let (vertex_buffer, vertices) = VertexBuffer::from_flat_slices(
            &self.path,
            flat_vertices.as_slice(),
            flat_uvs.as_slice(),
            flat_normals.as_slice(),
            &device,
        );

        Mesh {
            index_buffer: IndexBuffer::new(&indices, &device),
            indices,
            vertices,
            vertex_buffer,
        }
    }
}
