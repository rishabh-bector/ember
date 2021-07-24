use anyhow::{anyhow, Result};
use image::io::Reader as ImageReader;
use legion::Resources;
use std::{
    collections::HashMap,
    rc::Rc,
    sync::{Arc, Mutex},
};
use uuid::Uuid;

use crate::render::texture::Texture;

pub struct TextureStore {
    pub textures: HashMap<TextureGroup, HashMap<Uuid, Texture>>,
}

#[derive(Default)]
pub struct TextureStoreBuilder {
    pub to_load: HashMap<TextureGroup, Vec<String>>,
    pub load_group: TextureGroup,
    pub bind_group_layout: Rc<Option<wgpu::BindGroupLayout>>,
    pub texture_store: Option<Arc<Mutex<TextureStore>>>,
}

impl TextureStoreBuilder {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn begin_group(mut self, group: TextureGroup) -> Self {
        self.load_group = group;
        self
    }

    pub fn load(mut self, path: &str) -> Self {
        self.to_load
            .get_mut(&self.load_group)
            .unwrap()
            .push(path.to_owned());
        self
    }

    pub fn build(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
    ) -> Result<(Arc<Mutex<TextureStore>>, wgpu::BindGroupLayout)> {
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
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
                .map(|path| {
                    let rgba = ImageReader::open(&path)
                        .map_err(|err| anyhow!("error loading texture {}: - {}", path, err))?
                        .decode()?
                        .into_rgba8();
                    Ok((
                        Uuid::new_v4(),
                        Texture::load_image(device, queue, &rgba, &bind_group_layout, None)?,
                    ))
                })
                .collect::<Result<HashMap<Uuid, Texture>>>()?;
            textures.insert(*group, group_textures);
        }

        self.texture_store = Some(Arc::new(Mutex::new(TextureStore { textures })));
        Ok((
            Arc::clone(self.texture_store.as_ref().unwrap()),
            bind_group_layout,
        ))
    }

    pub fn build_to_resources(&self, resources: &mut Resources) {
        resources
            .insert::<Arc<Mutex<TextureStore>>>(Arc::clone(self.texture_store.as_ref().unwrap()));
    }
}

impl TextureStore {
    // pub fn bind_group(&self,  name: &str) -> Option<&wgpu::BindGroup> {
    //     self.textures.get(name).map(|t| &t.bind_group)
    // }

    pub fn build_bind_map(&self, groups: &[TextureGroup]) -> BindMap {
        let mut bind_map = BindMap::new();
        for group in groups {
            bind_map.add_texture_group(&self.textures[&group])
        }
        bind_map
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub enum TextureGroup {
    Base2D,
    _Base3D,
}

impl Default for TextureGroup {
    fn default() -> Self {
        Self::Base2D
    }
}

#[derive(Clone)]
pub struct BindMap {
    bind_groups: HashMap<Uuid, Arc<wgpu::BindGroup>>,
}

impl BindMap {
    pub fn new() -> Self {
        Self {
            bind_groups: HashMap::new(),
        }
    }

    pub fn add_texture_group(&mut self, textures: &HashMap<Uuid, Texture>) {
        self.bind_groups.extend(
            textures
                .iter()
                .map(|(id, tex)| (*id, Arc::clone(&tex.bind_group))),
        );
    }

    pub fn add_uniform_group(&mut self, id: Uuid, group: Arc<wgpu::BindGroup>) {
        self.bind_groups.insert(id, group);
    }
}
