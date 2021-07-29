use std::sync::{Arc, Mutex};

use crate::resource::ui::UI;
use crate::system::render_2d::create_render_pass;

#[system]
pub fn render_ui(
    #[resource] ui: &Arc<UI>,
    #[resource] device: &Arc<wgpu::Device>, // read only
    #[resource] queue: &Arc<wgpu::Queue>,   // read only
) {
    let mut context = ui.context.lock().unwrap();
    let mut renderer = ui.renderer.lock().unwrap();
    let mut state = ui.state.lock().unwrap();

    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("Render2D Encoder"),
    });

    let mut pass_handle = ui.render_target.create_render_pass(&mut encoder).unwrap();
    let frame = context.frame();

    let mouse_cursor = frame.mouse_cursor();
    if state.last_cursor != mouse_cursor {
        state.last_cursor = mouse_cursor;
        // self.platform.prepare_render(&ui, window);
    }

    // Draw UI //

    let mut about_open = false;
    frame.main_menu_bar(|| {
        frame.menu(imgui::im_str!("Help"), true, || {
            about_open = imgui::MenuItem::new(imgui::im_str!("About...")).build(&frame);
        });
    });
    if about_open {
        state.about_open = true;
    }

    if state.about_open {
        frame.show_about_window(&mut state.about_open);
    }

    // Render to texture
    renderer
        .render(frame.render(), queue, device, &mut pass_handle)
        .unwrap();
}
