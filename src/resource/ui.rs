use anyhow::Result;
use imgui::im_str;
use legion::Resources;
use std::{
    collections::HashMap,
    sync::{Arc, Mutex, MutexGuard},
    time::{Duration, Instant},
};
use uuid::Uuid;

use crate::render::{graph::RenderTarget, texture::Texture};

pub struct UI {
    pub platform: Mutex<imgui_winit_support::WinitPlatform>,
    pub render_target: Arc<Mutex<RenderTarget>>,
    pub context: Mutex<imgui::Context>,
    pub renderer: Mutex<imgui_wgpu::Renderer>,
    pub state: Mutex<UIState>,
    pub imgui_windows: HashMap<Uuid, Arc<dyn ImguiWindow>>,
}

pub struct UIBuilder {
    pub imgui_windows: HashMap<Uuid, Arc<dyn ImguiWindow>>,
}

impl UIBuilder {
    pub fn new() -> Self {
        UIBuilder {
            imgui_windows: HashMap::new(),
        }
    }

    pub fn with_imgui_window(mut self, window: Arc<dyn ImguiWindow>, id: Uuid) -> Self {
        self.imgui_windows.insert(id, window);
        self
    }

    pub fn build_to_resources(
        self,
        resources: &mut Resources,
        render_target: Arc<Mutex<RenderTarget>>,
        window: &winit::window::Window,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
    ) {
        debug!("building ui");

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
            texture_format: wgpu::TextureFormat::Bgra8UnormSrgb,
            ..Default::default()
        };
        let renderer = imgui_wgpu::Renderer::new(&mut imgui, device, queue, config);

        resources.insert(Arc::new(UI {
            context: Mutex::new(imgui),
            renderer: Mutex::new(renderer),
            state: Mutex::new(UIState {
                last_frame: Instant::now(),
                last_cursor: None,
            }),
            platform: Mutex::new(platform),
            imgui_windows: self.imgui_windows,
            render_target,
        }));
    }
}

pub struct UIState {
    pub last_frame: Instant,
    pub last_cursor: Option<imgui::MouseCursor>,
}

impl UI {
    pub fn prepare(&mut self) -> Result<(), winit::error::ExternalError> {
        let mut state = self.state.lock().unwrap();
        let mut imgui = self.context.lock().unwrap();

        let last_frame = state.last_frame;
        let now = Instant::now();

        imgui.io_mut().update_delta_time(now - last_frame);
        state.last_frame = now;
        Ok(())
    }
}

pub trait ImguiWindow {
    fn build(&self, frame: &imgui::Ui);
    fn impl_imgui(self: Arc<Self>) -> Arc<dyn ImguiWindow>;
}
