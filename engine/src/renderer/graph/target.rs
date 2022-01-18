use anyhow::{anyhow, Result};
use std::{
    borrow::BorrowMut,
    sync::{Arc, RwLockReadGuard},
};
use wgpu::{BindGroupLayout, Device};

use crate::{
    renderer::buffer::texture::Texture,
    sources::registry::{TextureRegistry, TextureType},
};

pub enum RenderTarget {
    Empty,
    Texture {
        color_buffer: Arc<Texture>,
        depth_buffer: Option<Arc<DepthBuffer>>,
    },
    Master {
        screen_buffer: Option<Arc<wgpu::SurfaceTexture>>,
        screen_view: Option<Arc<wgpu::TextureView>>,
        depth_buffer: Option<Arc<DepthBuffer>>,
    },
}

pub struct DepthBuffer(pub Texture);

impl DepthBuffer {
    pub fn new(name: &str, size: (u32, u32), device: Arc<Device>) -> Self {
        DepthBuffer(Texture::depth_buffer(
            &format!("{}_depth_target", name),
            &device,
            size,
            wgpu::TextureFormat::Depth32Float,
        ))
    }
}

impl RenderTarget {
    pub fn empty_master(depth_buffer: Option<Arc<DepthBuffer>>) -> Self {
        RenderTarget::Master {
            screen_buffer: None,
            screen_view: None,
            depth_buffer,
        }
    }

    pub fn new(
        name: &str,
        size: (u32, u32),
        depth: Option<Arc<DepthBuffer>>,
        tex_reg: &RwLockReadGuard<TextureRegistry>,
        device: Arc<Device>,
    ) -> Self {
        RenderTarget::Texture {
            color_buffer: Arc::new(
                Texture::blank(
                    size,
                    &device,
                    tex_reg.format,
                    &tex_reg.bind_group_layout(TextureType::Image),
                    Some(&format!("{}_render_target", name)),
                    true,
                )
                .unwrap(),
            ),
            depth_buffer: match depth {
                Some(buf) => Some(Arc::clone(&buf)),
                None => None,
            },
        }
    }

    pub fn create_render_pass<'a>(
        &'a self,
        name: &'a str,
        encoder: &'a mut wgpu::CommandEncoder,
        clear: bool,
    ) -> Result<wgpu::RenderPass<'a>> {
        match self {
            RenderTarget::Empty => Err(anyhow!("cannot render to an empty target")),
            RenderTarget::Texture {
                color_buffer,
                depth_buffer,
            } => Ok(create_render_pass(
                name,
                &color_buffer.view,
                depth_buffer.as_ref().map(|tex| &tex.0.view),
                encoder,
                clear,
            )),
            RenderTarget::Master {
                screen_buffer,
                screen_view,
                depth_buffer,
            } => match screen_buffer {
                Some(buf) => Ok(create_render_pass(
                    name,
                    screen_view.as_ref().unwrap(),
                    depth_buffer.as_ref().map(|tex| &tex.0.view),
                    encoder,
                    clear,
                )),
                None => Err(anyhow!("no screen buffer")),
            },
        }
    }

    pub fn borrow_if_master(&self) -> Option<Arc<wgpu::SurfaceTexture>> {
        match self {
            RenderTarget::Empty => None,
            RenderTarget::Texture { .. } => None,
            RenderTarget::Master {
                screen_buffer,
                screen_view: _,
                depth_buffer: _,
            } => Some(Arc::clone(screen_buffer.as_ref().unwrap())),
        }
    }

    pub fn get_view(&self) -> &wgpu::TextureView {
        match self {
            RenderTarget::Empty => todo!(),
            RenderTarget::Texture {
                color_buffer,
                depth_buffer: _,
            } => &color_buffer.view,
            RenderTarget::Master {
                screen_view,
                screen_buffer: _,
                depth_buffer: _,
            } => screen_view.as_ref().unwrap(),
        }
    }

    pub fn get_bind_group(&self) -> Option<Arc<wgpu::BindGroup>> {
        match self {
            RenderTarget::Empty => None,
            RenderTarget::Texture {
                color_buffer,
                depth_buffer: _,
            } => Some(Arc::clone(color_buffer.bind_group.as_ref().unwrap())),
            // Master node cannot be used as input
            RenderTarget::Master { .. } => None,
        }
    }

    pub fn get_depth_buffer(&self) -> Option<Arc<DepthBuffer>> {
        match self {
            RenderTarget::Empty => None,
            RenderTarget::Texture { .. } => None,
            RenderTarget::Master {
                screen_buffer: _,
                screen_view: _,
                depth_buffer,
            } => depth_buffer.as_ref().map(Arc::clone),
        }
    }

    pub fn set_depth_buffer(&mut self, buffer: Arc<DepthBuffer>) {
        match self {
            RenderTarget::Empty => (),
            RenderTarget::Texture {
                color_buffer: _,
                depth_buffer,
            } => *depth_buffer = Some(buffer),
            RenderTarget::Master {
                screen_buffer: _,
                screen_view: _,
                depth_buffer,
            } => *depth_buffer = Some(buffer),
        }
    }

    pub fn set_swap_chain(&mut self, buffer: Arc<wgpu::SurfaceTexture>) {
        if let RenderTarget::Master {
            screen_buffer,
            screen_view,
            depth_buffer: _,
        } = self
        {
            *screen_view = Some(Arc::new(
                buffer
                    .texture
                    .create_view(&wgpu::TextureViewDescriptor::default()),
            ));
            *screen_buffer = Some(buffer);
        }
    }

    // Release lock on swap chain so that buffer can
    // be drawn to window
    pub fn release_swap_chain(&mut self) {
        if let RenderTarget::Master {
            screen_buffer,
            screen_view: _,
            depth_buffer: _,
        } = self
        {
            let screen_buffer = screen_buffer.borrow_mut();
            let released = std::mem::replace(screen_buffer, None).unwrap();
            Arc::try_unwrap(released).unwrap().present();
        }
    }

    pub fn arc(&self) -> Self {
        match self {
            RenderTarget::Empty => RenderTarget::Empty,
            RenderTarget::Texture {
                color_buffer,
                depth_buffer,
            } => RenderTarget::Texture {
                color_buffer: Arc::clone(&color_buffer),
                depth_buffer: depth_buffer.as_ref().map(Arc::clone),
            },
            RenderTarget::Master {
                screen_buffer,
                screen_view,
                depth_buffer,
            } => RenderTarget::Master {
                screen_buffer: Some(Arc::clone(screen_buffer.as_ref().unwrap())),
                screen_view: Some(Arc::clone(screen_view.as_ref().unwrap())),
                depth_buffer: depth_buffer.as_ref().map(Arc::clone),
            },
        }
    }
}

impl Clone for RenderTarget {
    fn clone(&self) -> Self {
        self.arc()
    }
}

pub fn create_render_pass<'a>(
    name: &'a str,
    color_target: &'a wgpu::TextureView,
    depth_target: Option<&'a wgpu::TextureView>,
    encoder: &'a mut wgpu::CommandEncoder,
    clear: bool,
) -> wgpu::RenderPass<'a> {
    debug!(
        "creating render pass: {}, depth_buffer: {}, clear: {}",
        name,
        depth_target.is_some(),
        clear,
    );

    let ops = match clear {
        true => wgpu::Operations {
            load: wgpu::LoadOp::Clear(wgpu::Color {
                r: 0.0,
                g: 0.0,
                b: 0.0,
                a: 0.0,
            }),
            store: true,
        },
        false => wgpu::Operations {
            load: wgpu::LoadOp::Load,
            store: true,
        },
    };

    encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: Some(name),
        color_attachments: &[wgpu::RenderPassColorAttachment {
            view: color_target,
            resolve_target: None,
            ops,
        }],
        depth_stencil_attachment: depth_target.map(|view| wgpu::RenderPassDepthStencilAttachment {
            view: &view,
            depth_ops: Some(wgpu::Operations {
                load: wgpu::LoadOp::Clear(1.0),
                store: true,
            }),
            stencil_ops: None,
        }),
    })
}
