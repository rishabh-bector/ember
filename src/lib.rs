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
    window::WindowBuilder,
};
use winit_input_helper::WinitInputHelper;

mod components;
mod constants;
mod render;
mod resources;
mod systems;

use crate::{
    components::{Position2D, Velocity2D},
    constants::{
        BASE_2D_COMMON_TEXTURE_ID, CAMERA_2D_BIND_GROUP_ID, DEFAULT_SCREEN_HEIGHT,
        DEFAULT_SCREEN_WIDTH,
    },
    render::{buffer::*, graph::GraphBuilder, node::*, uniform::*, *},
    resources::{camera::Camera2D, store::TextureStoreBuilder, ui::UI},
    systems::{base_2d::*, camera_2d::*, lighting_2d::*, physics_2d::*, render_2d::*},
};

pub struct Engine {}

pub fn engine() -> EngineBuilder {
    EngineBuilder {}
}

pub struct EngineBuilder {}

impl EngineBuilder {
    pub fn default(self) -> Result<Engine> {
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

        let event_loop = EventLoop::new();
        let mut input = WinitInputHelper::new();
        let size = LogicalSize::new(DEFAULT_SCREEN_WIDTH as f64, DEFAULT_SCREEN_HEIGHT as f64);
        let window = Rc::new({
            WindowBuilder::new()
                .with_title("Hello World")
                .with_inner_size(size)
                .with_min_inner_size(size)
                .build(&event_loop)?
        });

        let mut resources = Resources::default();

        let gpu = Arc::new(Mutex::new(futures::executor::block_on(
            GpuStateBuilder::winit(Rc::clone(&window)).build(&mut resources),
        )?));
        let gpu_mut = gpu.lock().unwrap();

        // Build TextureStore
        let mut texture_store_builder = TextureStoreBuilder::new();
        texture_store_builder.load_id(
            Uuid::from_str(BASE_2D_COMMON_TEXTURE_ID).unwrap(),
            &base_dir
                .join("src/static/test.png")
                .into_os_string()
                .into_string()
                .unwrap(),
        );
        let (texture_store, texture_bind_group_layout) =
            texture_store_builder.build(&gpu_mut.device, &gpu_mut.queue)?;
        texture_store_builder.build_to_resources(&mut resources);

        // Build UI
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
            .with_id(Uuid::from_str(CAMERA_2D_BIND_GROUP_ID).unwrap());

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
            .with_id(Uuid::from_str(CAMERA_2D_BIND_GROUP_ID).unwrap());

        let base_2d_pipeline_node = NodeBuilder::new(
            "base_2d_node".to_owned(),
            0,
            ShaderSource::WGSL(include_str!("render/shaders/base2D.wgsl").to_owned()),
        )
        .texture_group(resources::store::TextureGroup::Base2D)
        .uniform_group(base_2d_uniforms)
        .uniform_group(camera_2d_uniforms)
        .uniform_group(lighting_2d_uniforms)
        .vertex_buffer_layout(VertexBuffer::layout_2d());

        let common_vertex_buffers: [VertexBuffer; 1] = [VertexBuffer::new_2d(
            &[
                Vertex2D {
                    position: [-1.0, -1.0],
                    uvs: [0.0, 1.0],
                },
                Vertex2D {
                    position: [-1.0, 1.0],
                    uvs: [0.0, 0.0],
                },
                Vertex2D {
                    position: [1.0, 1.0],
                    uvs: [1.0, 0.0],
                },
                Vertex2D {
                    position: [1.0, -1.0],
                    uvs: [1.0, 1.0],
                },
            ],
            &gpu_mut.device,
        )];
        let common_index_buffers: [IndexBuffer; 1] =
            [IndexBuffer::new(&[0, 2, 1, 3, 2, 0], &gpu_mut.device)];

        // let s: legion::systems::System<_, _, _> = physics_2d_system();

        let mut schedule = Schedule::builder();
        schedule
            // Main engine systems
            .add_system(physics_2d_system())
            .add_system(camera_2d_system())
            .add_system(lighting_2d_system())
            .flush()
            // Uniform loading systems
            .add_system(base_2d_uniform_system())
            .add_system(camera_2d_uniform_system())
            .add_system(lighting_2d_uniform_system())
            .flush();

        let graph_schedule = SubSchedule::new();

        let render_graph = GraphBuilder::new()
            .with_source_node(base_2d_pipeline_node)
            .build(
                Arc::clone(&gpu),
                &mut resources,
                &texture_bind_group_layout,
                Arc::clone(&texture_store),
            );

        let schedule = schedule.build();

        let camera = Arc::new(Mutex::new(Camera2D::default(
            DEFAULT_SCREEN_WIDTH as f32,
            DEFAULT_SCREEN_HEIGHT as f32,
        )));

        resources.insert(Arc::clone(&gpu));
        resources.insert(Arc::clone(&camera));

        let mut world = World::default();

        Ok(Engine {})
    }
}

pub enum Step {
    System(Box<dyn Schedulable>),
    Flush,
}

pub struct SubSchedule {
    pub steps: Vec<Step>,
}

impl SubSchedule {
    pub fn new() -> Self {
        Self { steps: vec![] }
    }

    pub fn add_system<F: Fn() -> S + 'static, S: ParallelRunnable + 'static>(
        &mut self,
        system: NodeSystem<F, S>,
    ) {
        self.steps.push(Step::System(Box::new(system)));
    }

    pub fn flush(&mut self) {
        self.steps.push(Step::Flush);
    }
}

pub trait Schedulable {
    fn schedule(&self, schedule: &mut ScheduleBuilder);
}

impl Schedulable for SubSchedule {
    fn schedule(&self, schedule: &mut ScheduleBuilder) {
        for step in &self.steps {
            match step {
                Step::Flush => {
                    schedule.flush();
                }
                Step::System(builder) => builder.schedule(schedule),
            }
        }
    }
}

pub struct NodeSystem<F, S>
where
    F: Fn() -> S,
    S: ParallelRunnable + 'static,
{
    builder: F,
    _marker: PhantomData<S>,
}

impl<F, S> NodeSystem<F, S>
where
    F: Fn() -> S,
    S: ParallelRunnable + 'static,
{
    pub fn new(system_builder: F) -> Self {
        Self {
            builder: system_builder,
            _marker: PhantomData,
        }
    }
}

impl<F, S> Schedulable for NodeSystem<F, S>
where
    F: Fn() -> S,
    S: ParallelRunnable + 'static,
{
    fn schedule(&self, schedule: &mut ScheduleBuilder) {
        schedule.add_system((self.builder)());
    }
}
