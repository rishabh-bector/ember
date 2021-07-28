use anyhow::Result;
use std::{
    sync::{Arc, Mutex, MutexGuard},
    time::Instant,
};

use crate::render::texture::Texture;

pub struct UI {
    pub platform: imgui_winit_support::WinitPlatform,
    pub render_target: Arc<Mutex<Option<Texture>>>,
    pub context: Mutex<imgui::Context>,
    pub renderer: Mutex<imgui_wgpu::Renderer>,
    pub state: Mutex<UIState>,
}

pub struct UIState {
    pub last_frame: Instant,
    pub last_cursor: Option<imgui::MouseCursor>,
    pub about_open: bool,
}

impl UI {
    pub fn new(
        render_target: Arc<Mutex<Option<Texture>>>,
        window: &winit::window::Window,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
    ) -> Self {
        // Create Dear ImGui context
        let mut imgui = imgui::Context::create();
        imgui.set_ini_filename(None);

        // Initialize winit support
        let mut platform = imgui_winit_support::WinitPlatform::init(&mut imgui);
        platform.attach_window(
            imgui.io_mut(),
            window,
            imgui_winit_support::HiDpiMode::Default,
        );

        // Configure fonts
        let hidpi_factor = window.scale_factor();
        let font_size = (13.0 * hidpi_factor) as f32;
        imgui.io_mut().font_global_scale = (1.0 / hidpi_factor) as f32;
        imgui
            .fonts()
            .add_font(&[imgui::FontSource::DefaultFontData {
                config: Some(imgui::FontConfig {
                    oversample_h: 1,
                    pixel_snap_h: true,
                    size_pixels: font_size,
                    ..Default::default()
                }),
            }]);

        let config = imgui_wgpu::RendererConfig {
            texture_format: wgpu::TextureFormat::Rgba8UnormSrgb,
            ..Default::default()
        };
        let renderer = imgui_wgpu::Renderer::new(&mut imgui, device, queue, config);

        Self {
            context: Mutex::new(imgui),
            renderer: Mutex::new(renderer),
            state: Mutex::new(UIState {
                last_frame: Instant::now(),
                last_cursor: None,
                about_open: true,
            }),
            platform,
            render_target,
        }
    }

    pub fn prepare(
        &mut self,
        // window: &winit::window::Window,
    ) -> Result<(), winit::error::ExternalError> {
        let mut state = self.state.lock().unwrap();
        let mut imgui = self.context.lock().unwrap();

        let last_frame = state.last_frame;
        let now = Instant::now();

        imgui.io_mut().update_delta_time(now - last_frame);
        state.last_frame = now;

        // self.platform.prepare_frame(imgui.io_mut(), window)
        Ok(())
    }
}
