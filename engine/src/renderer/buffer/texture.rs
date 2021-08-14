use anyhow::*;
use std::sync::Arc;

pub struct Texture {
    pub texture: wgpu::Texture,
    pub view: wgpu::TextureView,
    pub sampler: wgpu::Sampler,
    pub bind_group: Arc<wgpu::BindGroup>,
}

impl Texture {
    pub fn _load_bytes(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        preferred_format: wgpu::TextureFormat,
        bytes: &[u8],
        group_layout: &wgpu::BindGroupLayout,
        label: &str,
    ) -> Result<Self> {
        let img = image::load_from_memory(bytes)?;
        Self::load_image(
            device,
            queue,
            preferred_format,
            &img.into_rgba8(),
            group_layout,
            Some(label),
        )
    }

    pub fn load_image(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        preferred_format: wgpu::TextureFormat,
        rgba: &image::RgbaImage,
        group_layout: &wgpu::BindGroupLayout,
        label: Option<&str>,
    ) -> Result<Self> {
        let dimensions = rgba.dimensions();
        let size = wgpu::Extent3d {
            width: dimensions.0,
            height: dimensions.1,
            depth_or_array_layers: 1,
        };

        let texture = Self::blank(
            dimensions,
            device,
            preferred_format,
            group_layout,
            label,
            false,
        )?;

        queue.write_texture(
            wgpu::ImageCopyTexture {
                texture: &texture.texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
            },
            rgba,
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: std::num::NonZeroU32::new(4 * dimensions.0),
                rows_per_image: std::num::NonZeroU32::new(dimensions.1),
            },
            size,
        );
        Ok(texture)
    }

    pub fn blank(
        dimensions: (u32, u32),
        device: &wgpu::Device,
        format: wgpu::TextureFormat,
        group_layout: &wgpu::BindGroupLayout,
        label: Option<&str>,
        is_render_target: bool,
    ) -> Result<Texture> {
        let size = wgpu::Extent3d {
            width: dimensions.0,
            height: dimensions.1,
            depth_or_array_layers: 1,
        };

        let texture = device.create_texture(&wgpu::TextureDescriptor {
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            usage: match is_render_target {
                false => wgpu::TextureUsage::SAMPLED | wgpu::TextureUsage::COPY_DST,
                true => {
                    wgpu::TextureUsage::SAMPLED
                        | wgpu::TextureUsage::COPY_DST
                        | wgpu::TextureUsage::RENDER_ATTACHMENT
                }
            },
            label,
            size,
            format,
        });

        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
            ],
            label: Some("texture_bind_group"),
        });

        Ok(Self {
            texture,
            view,
            sampler,
            bind_group: Arc::new(bind_group),
        })
    }
}
