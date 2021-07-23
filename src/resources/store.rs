use anyhow::{anyhow, Result};
use image::io::Reader as ImageReader;
use legion::Resources;
use std::{
    borrow::Borrow,
    cell::RefCell,
    collections::HashMap,
    rc::Rc,
    sync::{Arc, Mutex},
};

use crate::render::{texture::Texture, GpuState};

pub struct TextureStore {
    pub textures: HashMap<String, Texture>,
}

#[derive(Default)]
pub struct TextureStoreBuilder {
    pub to_load: Vec<(String, String)>,
    pub bind_group_layout: Rc<Option<wgpu::BindGroupLayout>>,
    pub texture_store: Option<Arc<Mutex<TextureStore>>>,
}

impl TextureStoreBuilder {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn load(mut self, name: &str, path: &str) -> Self {
        self.to_load.push((name.to_owned(), path.to_owned()));
        self
    }

    pub fn build(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
    ) -> Result<wgpu::BindGroupLayout> {
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

        let mut textures = HashMap::new();
        for (name, path) in &self.to_load {
            let rgba = ImageReader::open(&path)
                .map_err(|err| anyhow!("error loading texture {}: - {}", path, err))?
                .decode()?
                .into_rgba8();
            textures.insert(
                name.to_owned(),
                Texture::load_image(device, queue, &rgba, &bind_group_layout, None)?,
            );
        }

        self.texture_store = Some(Arc::new(Mutex::new(TextureStore { textures })));

        Ok(bind_group_layout)
    }

    pub fn build_to_resource(&self, resources: &mut Resources) {
        resources
            .insert::<Arc<Mutex<TextureStore>>>(Arc::clone(self.texture_store.as_ref().unwrap()));
    }
}

impl TextureStore {
    pub fn bind_group(&self, name: &str) -> Option<&wgpu::BindGroup> {
        self.textures.get(name).map(|t| &t.bind_group)
    }
}
