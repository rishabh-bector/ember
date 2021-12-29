use anyhow::*;
use std::{num::NonZeroU32, sync::Arc};
use wgpu::TextureViewDimension;

pub struct Texture {
    pub texture: wgpu::Texture,
    pub view: wgpu::TextureView,
    pub sampler: wgpu::Sampler,
    pub bind_group: Option<Arc<wgpu::BindGroup>>,
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
            bind_group: Some(Arc::new(bind_group)),
        })
    }

    pub fn depth_buffer(
        name: &str,
        device: &wgpu::Device,
        size: (u32, u32),
        format: wgpu::TextureFormat,
    ) -> Self {
        debug!("building depth buffer: {}", name);
        let size = wgpu::Extent3d {
            width: size.0,
            height: size.1,
            depth_or_array_layers: 1,
        };
        let desc = wgpu::TextureDescriptor {
            label: Some(name),
            size,
            format,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            usage: wgpu::TextureUsage::RENDER_ATTACHMENT | wgpu::TextureUsage::SAMPLED,
        };
        let texture = device.create_texture(&desc);

        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Nearest,
            compare: Some(wgpu::CompareFunction::LessEqual),
            lod_min_clamp: -100.0,
            lod_max_clamp: 100.0,
            ..Default::default()
        });

        Self {
            texture,
            view,
            sampler,
            bind_group: None,
        }
    }

    fn blank_cubemap(
        dimensions: (u32, u32),
        device: &wgpu::Device,
        format: wgpu::TextureFormat,
        group_layout: &wgpu::BindGroupLayout,
        label: Option<&str>,
    ) -> Result<Texture> {
        let size = wgpu::Extent3d {
            width: dimensions.0,
            height: dimensions.1,
            depth_or_array_layers: 6,
        };

        let texture = device.create_texture(&wgpu::TextureDescriptor {
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            usage: wgpu::TextureUsage::SAMPLED | wgpu::TextureUsage::COPY_DST,
            label,
            size,
            format,
        });

        let view = texture.create_view(&wgpu::TextureViewDescriptor {
            dimension: Some(TextureViewDimension::Cube),
            base_array_layer: 0,
            array_layer_count: Some(NonZeroU32::new(6).unwrap()),
            ..Default::default()
        });

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
            label: Some("cube_texture_bind_group"),
        });

        Ok(Self {
            texture,
            view,
            sampler,
            bind_group: Some(Arc::new(bind_group)),
        })
    }

    pub fn load_cubemap(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        preferred_format: wgpu::TextureFormat,
        faces: &[image::RgbaImage], // [right, left, top, bottom, back, front]
        group_layout: &wgpu::BindGroupLayout,
        label: Option<&str>,
    ) -> Result<Self> {
        let dimensions = faces[0].dimensions();
        let size = wgpu::Extent3d {
            width: dimensions.0,
            height: dimensions.1,
            depth_or_array_layers: 6,
        };

        let texture =
            Self::blank_cubemap(dimensions, device, preferred_format, group_layout, label)?;

        let slice0: &[u8] = &faces[0];
        let slice1: &[u8] = &faces[1];
        let slice2: &[u8] = &faces[2];
        let slice3: &[u8] = &faces[3];
        let slice4: &[u8] = &faces[4];
        let slice5: &[u8] = &faces[5];

        let combo: &[u8] = &[slice0, slice1, slice2, slice3, slice4, slice5].concat();

        queue.write_texture(
            wgpu::ImageCopyTexture {
                texture: &texture.texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
            },
            &combo,
            wgpu::ImageDataLayout {
                offset: 0 as u64,
                bytes_per_row: std::num::NonZeroU32::new(4 * dimensions.0),
                rows_per_image: std::num::NonZeroU32::new(dimensions.1),
            },
            size,
        );

        // for (i, image) in faces.iter().enumerate() {
        //     debug!("buffering cubemap texture {}", i);
        //     queue.write_texture(
        //         wgpu::ImageCopyTexture {
        //             texture: &texture.texture,
        //             mip_level: 0,
        //             origin: wgpu::Origin3d::ZERO,
        //         },
        //         image,
        //         wgpu::ImageDataLayout {
        //             offset: 0 as u64,
        //             bytes_per_row: std::num::NonZeroU32::new(4 * dimensions.0),
        //             rows_per_image: std::num::NonZeroU32::new(dimensions.1),
        //         },
        //         size,
        //     );
        // }

        Ok(texture)
    }
}
