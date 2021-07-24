use anyhow::Result;
use std::time::Instant;

pub struct UI {
    imgui: imgui::Context,
    platform: imgui_winit_support::WinitPlatform,
    renderer: imgui_wgpu::Renderer,
    last_frame: Instant,
    last_cursor: Option<imgui::MouseCursor>,
    about_open: bool,
}

impl UI {
    pub fn new(window: &winit::window::Window, device: &wgpu::Device, queue: &wgpu::Queue) -> Self {
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
            imgui,
            platform,
            renderer,
            last_frame: Instant::now(),
            last_cursor: None,
            about_open: true,
        }
    }

    pub fn prepare(
        &mut self,
        window: &winit::window::Window,
    ) -> Result<(), winit::error::ExternalError> {
        let now = Instant::now();
        self.imgui.io_mut().update_delta_time(now - self.last_frame);
        self.last_frame = now;
        self.platform.prepare_frame(self.imgui.io_mut(), window)
    }

    pub fn render(
        &mut self,
        window: &winit::window::Window,
        encoder: &mut wgpu::CommandEncoder,
        render_target: &wgpu::TextureView,
    ) -> Result<()> {
        // Start a new Dear ImGui frame and update the cursor
        let ui = self.imgui.frame();

        let mouse_cursor = ui.mouse_cursor();
        if self.last_cursor != mouse_cursor {
            self.last_cursor = mouse_cursor;
            self.platform.prepare_render(&ui, window);
        }

        // Draw windows and GUI elements here
        let mut about_open = false;
        ui.main_menu_bar(|| {
            ui.menu(imgui::im_str!("Help"), true, || {
                about_open = imgui::MenuItem::new(imgui::im_str!("About...")).build(&ui);
            });
        });
        if about_open {
            self.about_open = true;
        }

        if self.about_open {
            ui.show_about_window(&mut self.about_open);
        }

        Ok(())
    }
}