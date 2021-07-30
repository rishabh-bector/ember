use std::sync::{Arc, Mutex};
use std::time::Instant;

use imgui::im_str;

use crate::resource::ui::UI;
use crate::system::render_2d::create_render_pass;

#[system]
pub fn render_ui(
    #[resource] ui: &Arc<UI>,
    #[resource] device: &Arc<wgpu::Device>,
    #[resource] queue: &Arc<wgpu::Queue>,
    #[resource] window: &Arc<winit::window::Window>,
) {
    debug!("running system render_ui");
    let mut context = ui.context.lock().unwrap();
    let mut renderer = ui.renderer.lock().unwrap();
    let mut state = ui.state.lock().unwrap();
    let render_target = ui.render_target.lock().unwrap();

    let now = Instant::now();
    context.io_mut().update_delta_time(now - state.last_frame);
    state.last_frame = now;
    ui.platform
        .lock()
        .unwrap()
        .prepare_frame(context.io_mut(), &window)
        .unwrap();

    let frame = context.frame();
    let mouse_cursor = frame.mouse_cursor();
    if state.last_cursor != mouse_cursor {
        state.last_cursor = mouse_cursor;
        ui.platform.lock().unwrap().prepare_render(&frame, &window);
    }

    // Draw UI //
    debug!("building ui");

    imgui::Window::new(im_str!("EMBER UI"))
        .size([300.0, 100.0], imgui::Condition::FirstUseEver)
        .build(&frame, || {
            if imgui::CollapsingHeader::new(im_str!("I'm a collapsing header. Click me!"))
                .build(&frame)
            {
                frame.text(
                    "A collapsing header can be used to toggle rendering of a group of widgets",
                );
            }

            frame.spacing();
            if imgui::CollapsingHeader::new(im_str!("I'm open by default"))
                .default_open(true)
                .build(&frame)
            {
                frame.text("You can still close me with a click!");
            }

            frame.text(im_str!("Hello world!"));
            frame.text(im_str!("こんにちは世界！"));
            frame.text(im_str!("This...is...imgui-rs!"));
            frame.separator();
            let mouse_pos = frame.io().mouse_pos;
            frame.text(format!(
                "Mouse Position: ({:.1},{:.1})",
                mouse_pos[0], mouse_pos[1]
            ));
        });

    frame.show_demo_window(&mut true);

    // Render to texture
    debug!("rendering ui");
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("ImGui Encoder"),
    });
    let mut pass_handle = render_target
        .create_render_pass(&mut encoder, "render_ui")
        .unwrap();
    renderer
        .render(frame.render(), queue, device, &mut pass_handle)
        .unwrap();

    debug!("done recording; submitting render pass");
    drop(pass_handle);
    queue.submit(std::iter::once(encoder.finish()));
    debug!("ui pass submitted");
}
