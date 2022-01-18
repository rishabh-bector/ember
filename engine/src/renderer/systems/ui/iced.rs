use futures::task::SpawnExt;
use iced_wgpu::{wgpu, Backend, Renderer, Settings, Viewport};
use iced_winit::conversion;
use std::{
    sync::{Arc, Mutex},
    time::Instant,
};
use wgpu::util::StagingBelt;

use crate::{
    renderer::graph::NodeState,
    sources::{
        metrics::SystemReporter,
        ui::iced::{IcedUI, IcedWinitHelper},
    },
};

#[system]
pub fn render(
    #[state] reporter: &mut SystemReporter,
    #[resource] ui: &Arc<Mutex<IcedUI>>,
    #[resource] helper: &Arc<Mutex<IcedWinitHelper>>,
    #[resource] staging_belt: &mut StagingBelt,
    #[resource] device: &Arc<wgpu::Device>,
    #[resource] queue: &Arc<wgpu::Queue>,
) {
    let start_time = Instant::now();
    debug!("running system render_ui_iced");

    let mut ui = ui.lock().unwrap();
    let mut renderer = ui.renderer.lock().unwrap();
    let helper = helper.lock().unwrap();

    // Render to texture
    debug!("rendering ui");

    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("IcedUI Encoder"),
    });

    let target = ui.target.lock().unwrap();
    let view = target.get_view();

    renderer.with_primitives(|backend, primitive| {
        backend.present(
            &device,
            staging_belt,
            &mut encoder,
            view,
            primitive,
            &helper.viewport,
            &["ligma"],
        );
    });

    staging_belt.finish();
    queue.submit(std::iter::once(encoder.finish()));

    ui.local_pool
        .spawner()
        .spawn(staging_belt.recall())
        .expect("Recall staging buffers");

    drop(renderer);
    drop(view);
    drop(target);
    ui.local_pool.run_until_stalled();

    // debug!("done recording; submitting render pass");
    // drop(pass_handle);
    // queue.submit(std::iter::once(encoder.finish()));

    debug!("ui pass submitted");
    reporter.update(start_time.elapsed().as_secs_f64());
}
