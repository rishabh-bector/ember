use anyhow::{anyhow, Result};
use image::io::Reader as ImageReader;
use legion::Resources;
use std::{
    collections::HashMap,
    rc::Rc,
    sync::{Arc, Mutex},
};
use uuid::Uuid;
use wgpu::BindGroup;

use crate::render::texture::Texture;

pub struct TextureStore {
    pub textures: HashMap<TextureGroup, HashMap<Uuid, Texture>>,
}

pub struct TextureStoreBuilder {
    pub to_load: HashMap<TextureGroup, Vec<(Uuid, String)>>,
    pub load_group: TextureGroup,
    pub bind_group_layout: Rc<Option<wgpu::BindGroupLayout>>,
    pub texture_store: Option<Arc<Mutex<TextureStore>>>,
}

impl TextureStoreBuilder {
    pub fn new() -> Self {
        let mut to_load: HashMap<TextureGroup, Vec<(Uuid, String)>> = HashMap::new();
        to_load.insert(TextureGroup::Base2D, vec![]);
        to_load.insert(TextureGroup::_Base3D, vec![]);
        Self {
            to_load,
            load_group: TextureGroup::Base2D,
            bind_group_layout: Rc::new(None),
            texture_store: None,
        }
    }

    pub fn begin_group(&mut self, group: TextureGroup) {
        self.load_group = group;
    }

    pub fn load(&mut self, path: &str) -> Uuid {
        let id = Uuid::new_v4();
        self.to_load
            .get_mut(&self.load_group)
            .unwrap()
            .push((id, path.to_owned()));
        id
    }

    pub fn load_id(&mut self, id: Uuid, path: &str) {
        self.to_load
            .get_mut(&self.load_group)
            .unwrap()
            .push((id, path.to_owned()));
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
                .map(|(id, path)| {
                    let rgba = ImageReader::open(&path)
                        .map_err(|err| anyhow!("error loading texture {}: - {}", path, err))?
                        .decode()?
                        .into_rgba8();
                    Ok((
                        *id,
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
    pub fn bind_group(&self, group: &TextureGroup) -> HashMap<Uuid, Arc<BindGroup>> {
        self.textures[group]
            .iter()
            .map(|(id, tex)| (*id, Arc::clone(&tex.bind_group)))
            .collect()
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub enum TextureGroup {
    Base2D,
    _Base3D,
}
