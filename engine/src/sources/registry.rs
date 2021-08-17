use anyhow::{anyhow, Result};
use image::io::Reader as ImageReader;
use legion::Resources;
use std::{
    collections::HashMap,
    rc::Rc,
    sync::{Arc, Mutex, RwLock},
};
use uuid::Uuid;
use wgpu::BindGroup;

use crate::{
    constants::{ID, PRIMITIVE_MESH_GROUP_ID, UNIT_CUBE_MESH_ID, UNIT_SQUARE_MESH_ID},
    renderer::buffer::{texture::Texture, Mesh, VertexBuffer},
};

use super::primitives::{unit_cube, unit_square, PrimitiveMesh};

pub struct Registry {
    pub textures: Arc<RwLock<TextureRegistry>>,
    pub meshes: Arc<RwLock<MeshRegistry>>,
}

impl Registry {
    pub fn build(
        device: Arc<wgpu::Device>,
        queue: &wgpu::Queue,
        texture_format: wgpu::TextureFormat,
        texture_builder: TextureRegistryBuilder,
        mesh_builder: MeshRegistryBuilder,
    ) -> Result<Registry> {
        Ok(Registry {
            textures: Arc::new(RwLock::new(texture_builder.build(
                &device,
                queue,
                texture_format,
            )?)),
            meshes: Arc::new(RwLock::new(mesh_builder.build(device))),
        })
    }
}

pub struct TextureRegistry {
    pub textures: HashMap<Uuid, HashMap<Uuid, Texture>>,
    pub bind_layout: wgpu::BindGroupLayout,
    pub format: wgpu::TextureFormat,
}

impl TextureRegistry {
    pub fn texture_group(&self, group_id: &Uuid) -> HashMap<Uuid, Arc<BindGroup>> {
        self.textures[group_id]
            .iter()
            .map(|(id, tex)| (*id, Arc::clone(tex.bind_group.as_ref().unwrap())))
            .collect()
    }
}

pub struct TextureRegistryBuilder {
    pub to_load: HashMap<Uuid, Vec<(Uuid, String)>>,
    pub bind_group_layout: Rc<Option<wgpu::BindGroupLayout>>,
}

impl TextureRegistryBuilder {
    pub fn new() -> Self {
        Self {
            to_load: HashMap::new(),
            bind_group_layout: Rc::new(None),
        }
    }

    pub fn load(&mut self, path: &str, group_id: Uuid) -> Uuid {
        let id = Uuid::new_v4();
        match self.to_load.get_mut(&group_id) {
            Some(paths) => paths.push((id, path.to_owned())),
            None => {
                self.to_load.insert(group_id, vec![(id, path.to_owned())]);
            }
        }
        id
    }

    pub fn load_id(&mut self, id: Uuid, path: &str, group_id: Uuid) {
        match self.to_load.get_mut(&group_id) {
            Some(paths) => paths.push((id, path.to_owned())),
            None => {
                self.to_load.insert(group_id, vec![(id, path.to_owned())]);
            }
        }
    }

    pub fn build(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        format: wgpu::TextureFormat,
    ) -> Result<TextureRegistry> {
        let bind_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStage::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        multisampled: false,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStage::FRAGMENT,
                    ty: wgpu::BindingType::Sampler {
                        comparison: false,
                        filtering: true,
                    },
                    count: None,
                },
            ],
            label: Some("texture_bind_group_layout"),
        });

        let mut textures: HashMap<Uuid, HashMap<Uuid, Texture>> = HashMap::new();
        for (group, tex) in &self.to_load {
            let group_textures = tex
                .into_iter()
                .map(|(id, path)| {
                    let rgba = ImageReader::open(&path)
                        .map_err(|err| anyhow!("error loading texture {}: - {}", path, err))?
                        .decode()?
                        .into_rgba8();
                    Ok((
                        *id,
                        Texture::load_image(device, queue, format, &rgba, &bind_layout, None)?,
                    ))
                })
                .collect::<Result<HashMap<Uuid, Texture>>>()?;
            textures.insert(*group, group_textures);
        }

        Ok(TextureRegistry {
            textures,
            bind_layout,
            format,
        })
    }
}

pub trait MeshBuilder: Send + Sync {
    fn build(&self, device: Arc<wgpu::Device>) -> Mesh;
}

pub struct MeshRegistry {
    pub builders: HashMap<Uuid, Arc<dyn MeshBuilder>>,
    pub device: Arc<wgpu::Device>,
}

impl MeshRegistry {
    pub fn register<M: MeshBuilder + 'static>(&mut self, builder: M) -> Uuid {
        let id = Uuid::new_v4();
        self.builders.insert(id, Arc::new(builder));
        id
    }

    pub fn register_id<M: MeshBuilder + 'static>(&mut self, id: Uuid, builder: M) {
        self.builders.insert(id, Arc::new(builder));
    }

    pub fn clone_mesh(&self, id: Uuid) -> Mesh {
        self.builders[&id].build(Arc::clone(&self.device))
    }
}

pub struct MeshRegistryBuilder {
    pub to_load: HashMap<Uuid, Vec<(Uuid, String)>>,
}

impl MeshRegistryBuilder {
    pub fn new() -> Self {
        Self {
            to_load: HashMap::new(),
        }
    }

    pub fn load(&mut self, path: &str) -> Uuid {
        let id = Uuid::new_v4();
        match self.to_load.get_mut(&ID(PRIMITIVE_MESH_GROUP_ID)) {
            Some(paths) => paths.push((id, path.to_owned())),
            None => {
                self.to_load
                    .insert(ID(PRIMITIVE_MESH_GROUP_ID), vec![(id, path.to_owned())]);
            }
        }
        id
    }

    pub fn load_id(&mut self, id: Uuid, path: &str) {
        match self.to_load.get_mut(&ID(PRIMITIVE_MESH_GROUP_ID)) {
            Some(paths) => paths.push((id, path.to_owned())),
            None => {
                self.to_load
                    .insert(ID(PRIMITIVE_MESH_GROUP_ID), vec![(id, path.to_owned())]);
            }
        }
    }

    pub fn build(&self, device: Arc<wgpu::Device>) -> MeshRegistry {
        let mut builders: HashMap<Uuid, Arc<dyn MeshBuilder>> = HashMap::new();
        builders.insert(ID(UNIT_SQUARE_MESH_ID), Arc::new(PrimitiveMesh::UnitSquare));
        builders.insert(ID(UNIT_CUBE_MESH_ID), Arc::new(PrimitiveMesh::UnitCube));

        MeshRegistry {
            builders,
            device: Arc::clone(&device),
        }
    }
}
