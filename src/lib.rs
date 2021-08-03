extern crate pretty_env_logger;
#[macro_use]
extern crate log;
#[macro_use]
extern crate legion;

use anyhow::Result;
use legion::{Resources, Schedule, World};
use render::graph::RenderGraph;
use resource::{metrics::EngineReporter, store::TextureStore};
use std::{
    env,
    path::PathBuf,
    str::FromStr,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};
use uuid::Uuid;
use winit::{
    dpi::LogicalSize,
    event::{Event, VirtualKeyCode},
    event_loop::{ControlFlow, EventLoop},
    window::{Window, WindowBuilder},
};
use winit_input_helper::WinitInputHelper;

use crate::{
    constants::{
        BASE_2D_BIND_GROUP_ID, BASE_2D_COMMON_TEXTURE_ID, CAMERA_2D_BIND_GROUP_ID,
        DEFAULT_SCREEN_HEIGHT, DEFAULT_SCREEN_WIDTH, DEFAULT_TEXTURE_BUFFER_FORMAT,
        FORWARD_2D_NODE_ID, ID, LIGHTING_2D_BIND_GROUP_ID, METRICS_UI_IMGUI_ID,
        RENDER_UI_SYSTEM_ID,
    },
    render::{buffer::*, graph::GraphBuilder, node::*, uniform::*, *},
    resource::{
        camera::Camera2D,
        metrics::EngineMetrics,
        schedule::{Schedulable, SubSchedule},
        store::TextureStoreBuilder,
        ui::{ImguiWindow, UIBuilder, UI},
    },
    system::{base_2d::*, camera_2d::*, lighting_2d::*, physics_2d::*, render_2d::*},
};

pub fn engine() -> EngineBuilder {
    EngineBuilder {}
}

pub mod components;
pub mod constants;
pub mod render;
pub mod resource;
pub mod system;

pub struct Engine {
    gpu: Arc<Mutex<GpuState>>,
    graph: Arc<RenderGraph>,
    store: Arc<Mutex<TextureStore>>,
    window: Arc<Window>,
    input: WinitInputHelper,
    legion: LegionState,
    metrics: Arc<EngineMetrics>,
    reporter: EngineReporter,
}

impl Engine {
    pub fn world(&mut self) -> &mut World {
        &mut self.legion.world
    }

    pub fn start(mut self, event_loop: EventLoop<()>) {
        info!("starting engine");

        let metrics_last_updated = Arc::new(Mutex::new(Instant::now()));
        event_loop.run(move |event, _, control_flow| {
            if let Event::RedrawRequested(_) = event {
                debug!("executing all systems");
                self.legion.execute();
                self.reporter.update();

                if metrics_last_updated.lock().unwrap().elapsed() >= Duration::from_secs(1) {
                    self.metrics.calculate();
                    *metrics_last_updated.lock().unwrap() = Instant::now();
                }
            }

            let ui = self.legion.resources.get_mut::<Arc<UI>>().unwrap();
            let mut context = ui.context.lock().unwrap();
            ui.platform
                .lock()
                .unwrap()
                .handle_event(context.io_mut(), &self.window, &event);

            if self.input.update(&event) {
                if self.input.key_pressed(VirtualKeyCode::Escape) || self.input.quit() {
                    debug!("received exit signal; shutting down");
                    *control_flow = ControlFlow::Exit;
                    return;
                }

                if let Some(physical_size) = self.input.resolution() {
                    &self.gpu.lock().unwrap().resize(physical_size);
                }

                // Request a redraw
                self.window.request_redraw();
            }
        });
    }
}

pub struct EngineBuilder {}

impl EngineBuilder {
    pub fn default(self) -> Result<(Engine, EventLoop<()>)> {
        pretty_env_logger::init();
        info!("building engine");

        let base_dir = option_env!("CARGO_MANIFEST_DIR").map_or_else(
            || {
                let exe_path = env::current_exe().expect("Failed to get exe path");
                exe_path
                    .parent()
                    .expect("Failed to get exe dir")
                    .to_path_buf()
            },
            |crate_dir| PathBuf::from(crate_dir),
        );

        info!("creating window");
        let event_loop = EventLoop::new();
        let size = LogicalSize::new(DEFAULT_SCREEN_WIDTH as f64, DEFAULT_SCREEN_HEIGHT as f64);
        let window = Arc::new({
            WindowBuilder::new()
                .with_title("Hello World")
                .with_inner_size(size)
                .with_min_inner_size(size)
                .build(&event_loop)?
        });

        let mut resources = Resources::default();

        info!("building gpu state");
        let gpu = Arc::new(Mutex::new(futures::executor::block_on(
            GpuStateBuilder::winit(Arc::clone(&window)).build(&mut resources),
        )?));
        let gpu_mut = gpu.lock().unwrap();

        info!("loading textures");
        let mut texture_store_builder = TextureStoreBuilder::new();
        texture_store_builder.load_id(
            Uuid::from_str(BASE_2D_COMMON_TEXTURE_ID).unwrap(),
            &base_dir
                .join("src/resource/static/test.png")
                .into_os_string()
                .into_string()
                .unwrap(),
        );
        let device_preferred_format = gpu_mut
            .adapter
            .get_swap_chain_preferred_format(&gpu_mut.surface)
            .unwrap_or(DEFAULT_TEXTURE_BUFFER_FORMAT);
        debug!("texture store format: {:?}", device_preferred_format);
        let (texture_store, texture_bind_group_layout) = texture_store_builder.build(
            &gpu_mut.device,
            &gpu_mut.queue,
            &device_preferred_format,
        )?;
        texture_store_builder.build_to_resources(&mut resources);

        info!("building uniforms");
        let base_2d_uniforms = UniformGroup::<Base2DUniformGroup>::builder()
            .with_uniform(
                GenericUniformBuilder::from_source(Base2DUniforms {
                    model: [0.0, 0.0, 1.0, 1.0],
                    color: [1.0, 1.0, 1.0, 1.0],
                    mix: 1.0,
                    _padding: [0.0; 32],
                    __padding: [0.0; 23],
                })
                .enable_dynamic_buffering(),
            )
            .with_id(Uuid::from_str(BASE_2D_BIND_GROUP_ID).unwrap());

        let camera_2d_uniforms = UniformGroup::<Camera2DUniformGroup>::builder()
            .with_uniform(GenericUniformBuilder::from_source(Camera2DUniforms {
                view: [1.0, 1.0, 1.0, 1.0],
                _padding: [0.0; 32],
                __padding: [0.0; 28],
            }))
            .with_id(Uuid::from_str(CAMERA_2D_BIND_GROUP_ID).unwrap());

        let lighting_2d_uniforms = UniformGroup::<Lighting2DUniformGroup>::builder()
            .with_uniform(GenericUniformBuilder::from_source(Lighting2DUniforms {
                light_0: Default::default(),
                light_1: Default::default(),
                light_2: Default::default(),
                light_3: Default::default(),
                light_4: Default::default(),
            }))
            .with_id(Uuid::from_str(LIGHTING_2D_BIND_GROUP_ID).unwrap());

        info!("building render graph nodes");
        let base_2d_pipeline_node = NodeBuilder::new(
            "base_2d_node".to_owned(),
            0,
            ShaderSource::WGSL(include_str!("render/shaders/base2D.wgsl").to_owned()),
        )
        .with_id(ID(FORWARD_2D_NODE_ID))
        .with_vertex_layout(VertexBuffer::layout_2d())
        .with_texture_group(resource::store::TextureGroup::Base2D)
        .with_uniform_group(base_2d_uniforms)
        .with_uniform_group(camera_2d_uniforms)
        .with_uniform_group(lighting_2d_uniforms)
        .with_system(forward_render_2d_system);

        info!("scheduling systems");
        let mut schedule = Schedule::builder();
        schedule
            // Main engine systems
            .add_system(physics_2d_system())
            .add_system(camera_2d_system())
            .add_system(lighting_2d_system())
            // Uniform loading systems
            .flush()
            .add_system(base_2d_uniform_system())
            .add_system(camera_2d_uniform_system())
            .add_system(lighting_2d_uniform_system());

        let mut metrics_ui = EngineMetrics::new();

        info!("building render graph");
        let mut graph_schedule = SubSchedule::new();
        let (render_graph, metrics) = GraphBuilder::new()
            .with_master_node(base_2d_pipeline_node)
            .with_ui_master()
            .build(
                Arc::clone(&gpu_mut.device),
                Arc::clone(&gpu_mut.queue),
                &mut resources,
                &mut graph_schedule,
                device_preferred_format,
                &texture_bind_group_layout,
                Arc::clone(&texture_store),
                &window,
                metrics_ui,
            )?;

        info!("scheduling render graph");
        graph_schedule.schedule(&mut schedule);
        let schedule = schedule.build();
        resources.insert(Arc::clone(&render_graph));

        let camera = Arc::new(Mutex::new(Camera2D::default(
            DEFAULT_SCREEN_WIDTH as f32,
            DEFAULT_SCREEN_HEIGHT as f32,
        )));

        drop(gpu_mut);
        resources.insert(Arc::clone(&gpu));
        resources.insert(Arc::clone(&window));
        resources.insert(Arc::clone(&camera));

        info!("ready to start!");

        Ok((
            Engine {
                reporter: EngineReporter::new(Arc::clone(&metrics.fps)),
                input: WinitInputHelper::new(),
                legion: LegionState {
                    world: World::default(),
                    schedule,
                    resources,
                },
                graph: render_graph,
                store: texture_store,
                window,
                metrics,
                gpu,
            },
            event_loop,
        ))
    }
}

pub struct LegionState {
    pub schedule: Schedule,
    pub world: World,
    pub resources: Resources,
}

impl LegionState {
    pub fn execute(&mut self) {
        self.schedule.execute(&mut self.world, &mut self.resources);
    }
}
