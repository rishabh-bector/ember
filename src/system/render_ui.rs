use imgui::im_str;
use std::sync::Arc;
use std::time::Instant;

use crate::{
    constants::{ID, RENDER_UI_SYSTEM_ID},
    resource::{metrics::EngineMetrics, ui::UI},
};

#[system]
pub fn render_ui(
    #[resource] ui: &Arc<UI>,
    #[resource] device: &Arc<wgpu::Device>,
    #[resource] queue: &Arc<wgpu::Queue>,
    #[resource] window: &Arc<winit::window::Window>,
    #[resource] metrics: &Arc<EngineMetrics>,
) {
    let start_time = Instant::now();
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
    for (_id, window) in &ui.imgui_windows {
        window.build(&frame);
    }

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
    metrics.submit_system_run_time(&ID(RENDER_UI_SYSTEM_ID), start_time.elapsed().as_secs_f64());
}
