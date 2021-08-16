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

use crate::renderer::buffer::texture::Texture;

pub struct Registry {
    pub textures: RwLock<TextureRegistry>,
}

impl Registry {
    pub fn build(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        texture_format: wgpu::TextureFormat,
        texture_builder: TextureRegistryBuilder,
    ) -> Result<Registry> {
        Ok(Registry {
            textures: RwLock::new(texture_builder.build(device, queue, texture_format)?),
        })
    }
}

pub struct TextureRegistry {
    pub textures: HashMap<TextureGroup, HashMap<Uuid, Texture>>,
    pub bind_layout: wgpu::BindGroupLayout,
    pub format: wgpu::TextureFormat,
}

impl TextureRegistry {
    pub fn texture_group(&self, group: &TextureGroup) -> HashMap<Uuid, Arc<BindGroup>> {
        self.textures[group]
            .iter()
            .map(|(id, tex)| (*id, Arc::clone(tex.bind_group.as_ref().unwrap())))
            .collect()
    }
}

pub struct TextureRegistryBuilder {
    pub to_load: HashMap<TextureGroup, Vec<(Uuid, String)>>,
    pub bind_group_layout: Rc<Option<wgpu::BindGroupLayout>>,
}

impl TextureRegistryBuilder {
    pub fn new() -> Self {
        let mut to_load: HashMap<TextureGroup, Vec<(Uuid, String)>> = HashMap::new();
        Self {
            to_load,
            bind_group_layout: Rc::new(None),
        }
    }

    pub fn load(&mut self, path: &str, group: TextureGroup) -> Uuid {
        let id = Uuid::new_v4();
        match self.to_load.get_mut(&group) {
            Some(paths) => paths.push((id, path.to_owned())),
            None => {
                self.to_load.insert(group, vec![(id, path.to_owned())]);
            }
        }
        id
    }

    pub fn load_id(&mut self, id: Uuid, path: &str, group: TextureGroup) {
        match self.to_load.get_mut(&group) {
            Some(paths) => paths.push((id, path.to_owned())),
            None => {
                self.to_load.insert(group, vec![(id, path.to_owned())]);
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

        let mut textures: HashMap<TextureGroup, HashMap<Uuid, Texture>> = HashMap::new();
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

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub enum TextureGroup {
    Render2D,
    Render3D,
}
