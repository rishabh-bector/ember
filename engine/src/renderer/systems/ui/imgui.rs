// use std::{sync::Arc, time::Instant};

// use crate::sources::{metrics::SystemReporter, ui::imgui::UI};

// #[system]
// pub fn render(
//     #[state] reporter: &mut SystemReporter,
//     #[resource] ui: &Arc<UI>,
//     #[resource] device: &Arc<wgpu::Device>,
//     #[resource] queue: &Arc<wgpu::Queue>,
//     #[resource] window: &Arc<winit::window::Window>,
// ) {
//     let start_time = Instant::now();
//     debug!("running system render_ui");

//     let mut context = ui.context.lock().unwrap();
//     let mut renderer = ui.renderer.lock().unwrap();
//     let mut state = ui.state.lock().unwrap();
//     let render_target = ui.render_target.lock().unwrap();

//     let now = Instant::now();
//     context.io_mut().update_delta_time(now - state.last_frame);
//     state.last_frame = now;
//     ui.platform
//         .lock()
//         .unwrap()
//         .prepare_frame(context.io_mut(), window.as_ref())
//         .unwrap();

//     let frame = context.frame();
//     let mouse_cursor = frame.mouse_cursor();
//     if state.last_cursor != mouse_cursor {
//         state.last_cursor = mouse_cursor;
//         ui.platform
//             .lock()
//             .unwrap()
//             .prepare_render(&frame, window.as_ref());
//     }

//     // Draw UI //
//     debug!("building ui");
//     for (_id, window) in &ui.imgui_windows {
//         window.build(&frame);
//     }

//     // frame.show_demo_window(&mut false);

//     // Render to texture
//     debug!("rendering ui");
//     let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
//         label: Some("ImGui Encoder"),
//     });
//     let mut pass_handle = render_target
//         .create_render_pass("render_ui", &mut encoder, false)
//         .unwrap();
//     renderer
//         .render(frame.render(), queue, device, &mut pass_handle)
//         .unwrap();

//     debug!("done recording; submitting render pass");
//     drop(pass_handle);
//     queue.submit(std::iter::once(encoder.finish()));

//     debug!("ui pass submitted");
//     reporter.update(start_time.elapsed().as_secs_f64());
// }
