use anyhow::{anyhow, Result};
use image::{io::Reader as ImageReader, ImageBuffer, Rgba};

use rayon::iter::{IntoParallelIterator, ParallelIterator};
use std::{
    collections::HashMap,
    rc::Rc,
    sync::{Arc, RwLock},
};
use uuid::Uuid;
use wgpu::BindGroup;

use crate::{
    constants::{
        ID, PRIMITIVE_MESH_GROUP_ID, SCREEN_QUAD_MESH_ID, UNIT_CUBE_MESH_ID, UNIT_SQUARE_MESH_ID,
    },
    renderer::{
        buffer::texture::Texture,
        mesh::{Mesh, ObjLoader},
    },
};

use super::primitives::PrimitiveMesh;

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
    pub format: wgpu::TextureFormat,

    bind_layout: wgpu::BindGroupLayout,
    cube_bind_layouts: HashMap<usize, wgpu::BindGroupLayout>,
}

impl TextureRegistry {
    pub fn texture_group(&self, group_id: &Uuid) -> HashMap<Uuid, Arc<BindGroup>> {
        self.textures[group_id]
            .iter()
            .map(|(id, tex)| (*id, Arc::clone(tex.bind_group.as_ref().unwrap())))
            .collect()
    }

    pub fn bind_group_layout(&self, tex_type: TextureType) -> &wgpu::BindGroupLayout {
        match tex_type {
            TextureType::Image => &self.bind_layout,
            TextureType::Cubemap => &self.cube_bind_layouts[&1usize],
            TextureType::CubemapN { n } => &self.cube_bind_layouts[&n],
        }
    }
}

#[derive(PartialEq, Eq, Clone, Copy)]
pub enum TextureType {
    Image,
    Cubemap,
    CubemapN { n: usize },
}

pub struct TextureDescriptor {
    id: Uuid,
    path: String,
    texture_group: Uuid,
    texture_type: TextureType,

    bind_group: Option<Uuid>,
}

pub struct TextureRegistryBuilder {
    // group_id -> (tex_id, tex_path)
    // pub to_load: HashMap<Uuid, Vec<(Uuid, String)>>,
    pub to_load: HashMap<Uuid, Vec<TextureDescriptor>>,
    // pub to_load_cube: HashMap<Uuid, Vec<(Uuid, String)>>,
}

impl TextureRegistryBuilder {
    pub fn new() -> Self {
        Self {
            to_load: HashMap::new(),
            // to_load_cube: HashMap::new(),
        }
    }

    // pub fn load_cube(&mut self, path: &str, group_id: Uuid) -> Uuid {
    //     let id = Uuid::new_v4();
    //     self.load_cube_id(id, path, &group_id);
    //     id
    // }

    // pub fn load_cube_id(&mut self, id: Uuid, path: &str, group_id: &Uuid) {
    //     let descriptor = TextureDescriptor {
    //         id,
    //         path: path.to_owned(),
    //         texture_type: TextureType::Cubemap,
    //         texture_group: *group_id,
    //         bind_group: None,
    //     };

    //     match self.to_load_cube.get_mut(group_id) {
    //         Some(paths) => paths.push((id, path.to_owned())),
    //         None => {
    //             self.to_load_cube
    //                 .insert(*group_id, vec![(id, path.to_owned())]);
    //         }
    //     }
    // }

    pub fn load(&mut self, path: &str, tex_type: TextureType, group_id: Uuid) -> Uuid {
        let id = Uuid::new_v4();
        self.load_id(id, path, tex_type, &group_id);
        id
    }

    pub fn load_id(&mut self, id: Uuid, path: &str, tex_type: TextureType, group_id: &Uuid) {
        let descriptor = TextureDescriptor {
            id,
            path: path.to_owned(),
            texture_type: tex_type,
            texture_group: *group_id,
            bind_group: None,
        };

        match self.to_load.get_mut(group_id) {
            Some(descriptors) => descriptors.push(descriptor),
            None => {
                self.to_load.insert(*group_id, vec![descriptor]);
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

        let cube_bind_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStage::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        multisampled: false,
                        view_dimension: wgpu::TextureViewDimension::Cube,
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
            label: Some("cube_bind_group_layout"),
        });

        let mut cubemap_Ns: Vec<usize> = vec![0];
        let mut cube_bind_layouts: HashMap<usize, wgpu::BindGroupLayout> = HashMap::new();
        for (group_id, group) in &self.to_load {
            let group_textures = group.iter().for_each(|desc| match desc.texture_type {
                TextureType::CubemapN { n } => {
                    if n != 0 {
                        if !cubemap_Ns.contains(&n) {
                            cubemap_Ns.push(n)
                        }
                    }
                }
                _ => {}
            });
        }

        // two cubemaps, one bind group
        let cube_2_bind_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStage::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            multisampled: false,
                            view_dimension: wgpu::TextureViewDimension::Cube,
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
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
                        visibility: wgpu::ShaderStage::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            multisampled: false,
                            view_dimension: wgpu::TextureViewDimension::Cube,
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 3,
                        visibility: wgpu::ShaderStage::FRAGMENT,
                        ty: wgpu::BindingType::Sampler {
                            comparison: false,
                            filtering: true,
                        },
                        count: None,
                    },
                ],
                label: Some("cube_2_bind_group_layout"),
            });

        cube_bind_layouts.insert(1usize, cube_bind_layout);
        cube_bind_layouts.insert(2usize, cube_2_bind_layout);

        let file_ext = "png";
        let dirs = vec!["px", "nx", "py", "ny", "pz", "nz"];

        let mut textures: HashMap<Uuid, HashMap<Uuid, Texture>> = HashMap::new();

        for (group_id, group) in &self.to_load {
            let group_textures = group
                .into_par_iter()
                .map(|descriptor| {
                    match descriptor.texture_type {
                        TextureType::Image => {
                            let rgba = ImageReader::open(&descriptor.path)
                                .map_err(|err| {
                                    anyhow!("error loading texture {}: - {}", descriptor.path, err)
                                })?
                                .decode()?
                                .into_rgba8();
                            Ok((
                                descriptor.id,
                                Texture::load_image(
                                    device,
                                    queue,
                                    format,
                                    &rgba,
                                    &bind_layout,
                                    None,
                                )?,
                            ))
                        }
                        TextureType::Cubemap => {
                            let faces: Vec<ImageBuffer<Rgba<u8>, Vec<u8>>> = dirs
                                .iter()
                                .map(|dir| {
                                    // dir is direction not directory
                                    let img_path =
                                        format!("{}/{}.{}", descriptor.path, dir, file_ext);
                                    debug!("loading cubemap at {}", img_path);
                                    image::io::Reader::open(img_path)
                                        .unwrap()
                                        .decode()
                                        .unwrap()
                                        .into_rgba8()
                                })
                                .collect();

                            Ok((
                                descriptor.id,
                                Texture::load_cubemap(
                                    &device,
                                    &queue,
                                    wgpu::TextureFormat::Rgba8UnormSrgb,
                                    &faces,
                                    &cube_bind_layouts[&1usize],
                                    None,
                                )?,
                            ))
                        }
                        TextureType::CubemapN { n } => {
                            let faces: Vec<ImageBuffer<Rgba<u8>, Vec<u8>>> = dirs
                                .iter()
                                .map(|dir| {
                                    // dir is direction not directory
                                    let img_path =
                                        format!("{}/{}.{}", descriptor.path, dir, file_ext);
                                    debug!("loading cubemap at {}", img_path);
                                    image::io::Reader::open(img_path)
                                        .unwrap()
                                        .decode()
                                        .unwrap()
                                        .into_rgba8()
                                })
                                .collect();

                            Ok((
                                descriptor.id,
                                Texture::load_cubemap(
                                    &device,
                                    &queue,
                                    wgpu::TextureFormat::Rgba8UnormSrgb,
                                    &faces,
                                    &cube_bind_layouts[&n],
                                    None,
                                )?,
                            ))
                        }
                    }
                })
                .collect::<Result<HashMap<Uuid, Texture>>>()?;
            textures.insert(*group_id, group_textures);
        }

        // CUBEMAPS

        // let dirs = vec!["back", "back", "up", "down", "back", "front"];
        // let dirs = vec!["right", "left", "up", "down", "back", "front"];

        // for (group_id, group) in &self.to_load_cube {
        //     let group_textures = group
        //         .into_par_iter()
        //         .map(|(id, path)| {})
        //         .collect::<Result<HashMap<Uuid, Texture>>>()?;

        //     if textures.contains_key(group_id) {
        //         let existing = textures.get_mut(group_id).unwrap();
        //         for (i, t) in group_textures {
        //             existing.insert(i, t);
        //         }
        //     } else {
        //         textures.insert(*group_id, group_textures);
        //     }
        // }

        Ok(TextureRegistry {
            textures,
            bind_layout,
            cube_bind_layouts,
            format,
        })
    }
}

pub trait MeshBuilder: Send + Sync {
    fn build(&self, device: Arc<wgpu::Device>) -> Mesh;
}

pub struct MeshRegistry {
    pub groups: HashMap<Uuid, HashMap<Uuid, Arc<dyn MeshBuilder>>>,
    pub device: Arc<wgpu::Device>,
}

impl MeshRegistry {
    pub fn register<M: MeshBuilder + 'static>(&mut self, builder: M, group_id: &Uuid) -> Uuid {
        let id = Uuid::new_v4();
        match self.groups.get_mut(group_id) {
            Some(group) => {
                group.insert(id, Arc::new(builder));
            }
            None => {
                self.groups.insert(*group_id, HashMap::new());
                self.register(builder, group_id);
            }
        }
        id
    }

    pub fn register_id<M: MeshBuilder + 'static>(&mut self, id: Uuid, builder: M, group_id: &Uuid) {
        match self.groups.get_mut(group_id) {
            Some(group) => {
                group.insert(id, Arc::new(builder));
            }
            None => {
                self.groups.insert(*group_id, HashMap::new());
                self.register_id(id, builder, group_id);
            }
        }
    }

    pub fn clone_mesh(&self, mesh_id: &Uuid, group_id: &Uuid) -> Mesh {
        self.groups[group_id][mesh_id].build(Arc::clone(&self.device))
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

    pub fn load(&mut self, path: &str, group_id: &Uuid) -> Uuid {
        let id = Uuid::new_v4();
        match self.to_load.get_mut(group_id) {
            Some(paths) => paths.push((id, path.to_owned())),
            None => {
                self.to_load.insert(*group_id, vec![(id, path.to_owned())]);
            }
        }
        id
    }

    pub fn load_id(&mut self, id: Uuid, path: &str, group_id: &Uuid) {
        match self.to_load.get_mut(group_id) {
            Some(paths) => paths.push((id, path.to_owned())),
            None => {
                self.to_load.insert(*group_id, vec![(id, path.to_owned())]);
            }
        }
    }

    pub fn build(&self, device: Arc<wgpu::Device>) -> MeshRegistry {
        let mut num_meshes = 0;
        let _ = &self
            .to_load
            .iter()
            .for_each(|(_, mesh)| mesh.iter().for_each(|_| num_meshes += 1));
        debug!(
            "building mesh registry: {} groups, {} meshes",
            self.to_load.len(),
            num_meshes
        );

        let base_path = std::env::current_dir().unwrap();

        let mut groups: HashMap<Uuid, HashMap<Uuid, Arc<dyn MeshBuilder>>> = self
            .to_load
            .to_owned()
            .into_par_iter()
            .map(|(group_id, group)| {
                (
                    group_id,
                    group
                        .into_par_iter()
                        .map(|(mesh_id, path)| {
                            (
                                mesh_id,
                                ObjLoader::new(base_path.join(&path).to_str().unwrap().to_owned())
                                    .arc_dyn(),
                            )
                        })
                        .collect(),
                )
            })
            .collect();

        // Common shapes
        let mut primitive_group: HashMap<Uuid, Arc<dyn MeshBuilder>> = HashMap::new();
        primitive_group.insert(ID(UNIT_SQUARE_MESH_ID), Arc::new(PrimitiveMesh::UnitSquare));
        primitive_group.insert(ID(UNIT_CUBE_MESH_ID), Arc::new(PrimitiveMesh::UnitCube));
        primitive_group.insert(ID(SCREEN_QUAD_MESH_ID), Arc::new(PrimitiveMesh::ScreenQuad));
        groups.insert(ID(PRIMITIVE_MESH_GROUP_ID), primitive_group);

        MeshRegistry {
            groups,
            device: Arc::clone(&device),
        }
    }
}
