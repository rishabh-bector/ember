extern crate pretty_env_logger;
#[macro_use]
extern crate log;
#[macro_use]
extern crate legion;

use anyhow::Result;
use legion::{
    systems::{Builder as ScheduleBuilder, ParallelRunnable, Runnable},
    Resources, Schedule, World,
};
use rand::Rng;
use render::graph::RenderGraph;
use resource::store::TextureStore;
use std::{
    env,
    marker::PhantomData,
    path::PathBuf,
    rc::Rc,
    str::FromStr,
    sync::{Arc, Mutex},
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
    components::{Position2D, Velocity2D},
    constants::{
        BASE_2D_BIND_GROUP_ID, BASE_2D_COMMON_TEXTURE_ID, CAMERA_2D_BIND_GROUP_ID,
        DEFAULT_SCREEN_HEIGHT, DEFAULT_SCREEN_WIDTH, LIGHTING_2D_BIND_GROUP_ID,
    },
    render::{buffer::*, graph::GraphBuilder, node::*, uniform::*, *},
    resource::{
        camera::Camera2D,
        schedule::{Schedulable, SubSchedule},
        store::TextureStoreBuilder,
        ui::UI,
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

// Engine properties:
//  - event loop
//  - Dschedule
//  - Dworld
//  - resources
//  - Drender graph

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

pub struct Engine {
    pub gpu: Arc<Mutex<GpuState>>,
    pub graph: Arc<RenderGraph>,
    pub store: Arc<Mutex<TextureStore>>,
    pub window: Rc<Window>,
    pub input: WinitInputHelper,
    pub legion: LegionState,
    pub ui: UI,
}

impl Engine {
    pub fn start(mut self, event_loop: EventLoop<()>) {
        event_loop.run(move |event, _, control_flow| {
            if let Event::RedrawRequested(_) = event {
                self.legion.execute();
            }

            if self.input.update(&event) {
                if self.input.key_pressed(VirtualKeyCode::Escape) || self.input.quit() {
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

        // Build winit window
        let event_loop = EventLoop::new();
        let size = LogicalSize::new(DEFAULT_SCREEN_WIDTH as f64, DEFAULT_SCREEN_HEIGHT as f64);
        let window = Rc::new({
            WindowBuilder::new()
                .with_title("Hello World")
                .with_inner_size(size)
                .with_min_inner_size(size)
                .build(&event_loop)?
        });

        let mut resources = Resources::default();

        // Build gpu context, attach to window
        let gpu = Arc::new(Mutex::new(futures::executor::block_on(
            GpuStateBuilder::winit(Rc::clone(&window)).build(&mut resources),
        )?));
        let gpu_mut = gpu.lock().unwrap();

        let mut texture_store_builder = TextureStoreBuilder::new();
        texture_store_builder.load_id(
            Uuid::from_str(BASE_2D_COMMON_TEXTURE_ID).unwrap(),
            &base_dir
                .join("src/resource/static/test.png")
                .into_os_string()
                .into_string()
                .unwrap(),
        );
        let (texture_store, texture_bind_group_layout) =
            texture_store_builder.build(&gpu_mut.device, &gpu_mut.queue)?;
        texture_store_builder.build_to_resources(&mut resources);

        let ui = UI::new(window.as_ref(), &gpu_mut.device, &gpu_mut.queue);

        let base_2d_uniforms = UniformGroup::<Base2DUniformGroup>::builder()
            .with_uniform(
                GenericUniformBuilder::from_source(Base2DUniforms {
                    model: [0.0, 0.0, 1.0, 1.0],
                    color: [1.0, 1.0, 1.0, 1.0],
                    mix: 1.0,
                    _padding: [0.0; 32],
                    __padding: [0.0; 23],
                })
                .enable_dynamic_buffering()
                .with_dynamic_entity_limit(96),
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

        let base_2d_pipeline_node = NodeBuilder::new(
            "base_2d_node".to_owned(),
            0,
            ShaderSource::WGSL(include_str!("render/shaders/base2D.wgsl").to_owned()),
        )
        .with_vertex_layout(VertexBuffer::layout_2d())
        .with_texture_group(resource::store::TextureGroup::Base2D)
        .with_uniform_group(base_2d_uniforms)
        .with_uniform_group(camera_2d_uniforms)
        .with_uniform_group(lighting_2d_uniforms)
        .with_system(base_2d_uniform_system);

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

        let mut graph_schedule = SubSchedule::new();
        let render_graph = GraphBuilder::new()
            .with_master_node(base_2d_pipeline_node)
            .build(
                Arc::clone(&gpu),
                &mut resources,
                &mut graph_schedule,
                &texture_bind_group_layout,
                Arc::clone(&texture_store),
            )?;

        graph_schedule.schedule(&mut schedule);
        let schedule = schedule.build();

        let camera = Arc::new(Mutex::new(Camera2D::default(
            DEFAULT_SCREEN_WIDTH as f32,
            DEFAULT_SCREEN_HEIGHT as f32,
        )));

        drop(gpu_mut);
        resources.insert(Arc::clone(&gpu));
        resources.insert(Arc::clone(&camera));

        Ok((
            Engine {
                window,
                input: WinitInputHelper::new(),
                legion: LegionState {
                    world: World::default(),
                    schedule,
                    resources,
                },
                graph: render_graph,
                store: texture_store,
                gpu,
                ui,
            },
            event_loop,
        ))
    }
}
