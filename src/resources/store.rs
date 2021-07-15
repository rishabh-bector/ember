use anyhow::{anyhow, Result};
use image::io::Reader as ImageReader;
use std::{
    cell::RefCell,
    collections::HashMap,
    rc::Rc,
    sync::{Arc, Mutex},
};

use crate::render::{texture::Texture, GpuState};

pub struct TextureStore {
    pub textures: HashMap<String, Texture>,
}

impl TextureStore {
    pub fn new(gpu: &Arc<Mutex<GpuState>>, to_load: Vec<(&str, &str)>) -> Result<Self> {
        let gpu = gpu.lock().unwrap();
        let mut textures = HashMap::new();
        for (name, path) in to_load {
            let rgba = ImageReader::open(&path)
                .map_err(|err| anyhow!("error loading texture {}: - {}", path, err))?
                .decode()?
                .into_rgba8();
            textures.insert(
                name.to_owned(),
                Texture::load_image(&gpu.device, &gpu.queue, &rgba, None)?,
            );
        }
        Ok(TextureStore { textures })
    }

    pub fn bind_group(&self, name: &str) -> Option<&wgpu::BindGroup> {
        self.textures.get(name).map(|t| &t.bind_group)
    }
}
