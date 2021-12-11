#![windows_subsystem = "windows"]
#![allow(dead_code)]

extern crate pretty_env_logger;
extern crate vertex_traits;

#[macro_use]
extern crate derive_more;
#[macro_use]
extern crate log;
#[macro_use]
extern crate legion;
#[macro_use]
extern crate vertex_layout_derive;

use anyhow::Result;
use legion::{Resources, Schedule, World};
use std::{
    env,
    path::PathBuf,
    str::FromStr,
    sync::{Arc, Mutex, RwLock},
    time::{Duration, Instant},
};
use uuid::Uuid;
use winit::{
    dpi::LogicalSize,
    event::{Event, VirtualKeyCode},
    event_loop::{ControlFlow, EventLoop},
    window::{Fullscreen, Window, WindowBuilder},
};
use winit_input_helper::WinitInputHelper;

use crate::{
    components::FrameMetrics,
    constants::*,
    renderer::{
        buffer::{instance::*, *},
        graph::{
            node::{NodeBuilder, ShaderSource},
            GraphBuilder, RenderGraph,
        },
        mesh::Mesh,
        systems::{
            quad::QuadUniformGroup, render_2d::forward_dynamic::Render2DForwardDynamicGroup,
            render_3d::forward_basic::Render3DForwardUniformGroup, *,
        },
        uniform::group::{GroupStateBuilder, UniformGroupBuilder, UniformGroupType},
        GpuState, GpuStateBuilder,
    },
    sources::{
        camera::{Camera2D, Camera3D},
        metrics::{EngineMetrics, EngineReporter},
        registry::{MeshRegistryBuilder, Registry, TextureRegistryBuilder},
        schedule::{Schedulable, SubSchedule},
        ui::UI,
        WindowSize,
    },
    systems::{
        camera_2d::*, camera_3d::*, lighting_2d::*, particle_2d::*, physics_2d::*, physics_3d::*,
    },
};

pub fn engine_builder() -> EngineBuilder {
    pretty_env_logger::init();
    EngineBuilder {
        window_size: (DEFAULT_SCREEN_WIDTH, DEFAULT_SCREEN_HEIGHT),
        texture_registry_builder: TextureRegistryBuilder::new(),
        mesh_registry_builder: MeshRegistryBuilder::new(),
    }
}

pub mod components;
pub mod constants;
pub mod renderer;
pub mod sources;
pub mod systems;

#[allow(dead_code)]
pub struct Engine {
    gpu: Arc<Mutex<GpuState>>,
    graph: Arc<RenderGraph>,
    window: Arc<Window>,
    input: Arc<RwLock<WinitInputHelper>>,
    registry: Registry,
    legion: LegionState,
    reporter: EngineReporter,
    engine_metrics: Arc<EngineMetrics>,
    frame_metrics: Arc<RwLock<FrameMetrics>>,
    mode: EngineMode,
}

enum EngineMode {
    Forward2D,
    Forward3D,
    Quad,
}

impl Engine {
    pub fn world(&mut self) -> &mut World {
        &mut self.legion.world
    }

    pub fn clone_mesh(&self, mesh_id: &Uuid, group_id: &Uuid) -> Mesh {
        self.registry
            .meshes
            .read()
            .unwrap()
            .clone_mesh(mesh_id, group_id)
    }

    pub fn start(mut self, event_loop: EventLoop<()>) {
        info!("starting engine");

        self.init();

        // top-level event loop; hijacks thread
        let metrics_last_updated = Arc::new(Mutex::new(Instant::now()));
        event_loop.run(move |event, _, control_flow| {
            if let Event::RedrawRequested(_) = event {
                debug!("executing all systems");
                self.frame_metrics.write().unwrap().begin_frame();
                self.legion.execute();
                self.reporter.update();
                self.frame_metrics.write().unwrap().end_frame();

                if metrics_last_updated.lock().unwrap().elapsed() >= Duration::from_secs(1) {
                    self.engine_metrics.calculate();
                    *metrics_last_updated.lock().unwrap() = Instant::now();
                }
            }

            let ui = self.legion.resources.get_mut::<Arc<UI>>().unwrap();
            let mut context = ui.context.lock().unwrap();
            ui.platform
                .lock()
                .unwrap()
                .handle_event(context.io_mut(), &self.window, &event);

            if self.input.write().unwrap().update(&event) {
                let input = self.input.read().unwrap();
                if input.key_pressed(VirtualKeyCode::Escape) || input.quit() {
                    debug!("shutting down");
                    *control_flow = ControlFlow::Exit;
                    return;
                }

                if let Some(physical_size) = input.resolution() {
                    self.gpu.lock().unwrap().resize(physical_size);
                }
                self.window.request_redraw();
            }
        });
    }

    fn init(&mut self) {
        match &self.mode {
            EngineMode::Forward3D | EngineMode::Quad => {
                self.window.set_cursor_visible(false);
                let _ = self.window.set_cursor_grab(true);
            }
            _ => {}
        }

        init_particle_systems(self.world());
    }
}

pub struct EngineBuilder {
    // Engine config
    window_size: (u32, u32),

    // Static assets
    texture_registry_builder: TextureRegistryBuilder,
    mesh_registry_builder: MeshRegistryBuilder,
}

impl EngineBuilder {
    pub fn with_texture_group(mut self, group: TextureGroup) -> Self {
        for tex in group.textures {
            self.texture_registry_builder
                .load_id(tex.0, &tex.1, &group.id);
        }
        self
    }

    pub fn with_mesh_group(mut self, group: MeshGroup) -> Self {
        for mesh in group.meshes {
            self.mesh_registry_builder
                .load_id(mesh.0, &mesh.1, &group.id);
        }
        self
    }

    // Todo: distil this into several functions
    pub fn default_2d(self) -> Result<(Engine, EventLoop<()>)> {
        info!("building engine: default_2d");

        let (gpu, window, event_loop, registry, mut resources) = build_engine_common(
            self.window_size,
            self.texture_registry_builder,
            self.mesh_registry_builder,
        )?;
        let gpu_mut = gpu.lock().unwrap();

        info!("building uniforms");
        let render_2d_dynamic_group_builder =
            Arc::new(Mutex::new(Render2DForwardDynamicGroup::builder()));
        let camera_2d_group_builder = Arc::new(Mutex::new(Camera2DUniformGroup::builder()));
        let lighting_2d_group_builder = Arc::new(Mutex::new(Lighting2DUniformGroup::builder()));

        info!("building render graph nodes");
        let _node_2d_forward_dynamic = build_node_2d_forward_dynamic(
            Arc::clone(&render_2d_dynamic_group_builder),
            Arc::clone(&camera_2d_group_builder),
            Arc::clone(&lighting_2d_group_builder),
        );
        let node_2d_forward_instance = build_node_2d_forward_instance(
            Arc::clone(&camera_2d_group_builder),
            Arc::clone(&lighting_2d_group_builder),
        );

        // Todo: replace this with something better
        resources.insert(InstanceBuffer::<
            render_2d::forward_instance::Render2DInstance,
        >::new(
            &gpu_mut.device,
            Arc::clone(&gpu_mut.queue),
            DEFAULT_MAX_INSTANCES_PER_BUFFER,
        ));

        info!("scheduling systems");
        let mut schedule = Schedule::builder();
        schedule
            // Main engine systems
            .add_system(physics_2d_system())
            .add_system(camera_2d_system())
            .add_system(lighting_2d_system())
            .add_system(particle_2d_emission_system())
            // .add_system(render_2d::forward_instance::attractor_system())
            // Uniform loading systems
            .flush()
            .add_system(render_2d::forward_instance::load_system())
            .add_system(camera_2d_uniform_system())
            .add_system(lighting_2d_uniform_system());

        info!("building render graph");
        let metrics_ui = EngineMetrics::new();
        let mut graph_schedule = SubSchedule::new();
        let (render_graph, engine_metrics) = GraphBuilder::new()
            .with_master_node(node_2d_forward_instance)
            .with_ui_master()
            .build(
                Arc::clone(&gpu_mut.device),
                Arc::clone(&gpu_mut.queue),
                &mut resources,
                &mut graph_schedule,
                &registry,
                &window,
                metrics_ui,
            )?;

        info!("scheduling render graph");
        graph_schedule.schedule(&mut schedule);
        let schedule = schedule.build();

        // resource
        let camera_2d = Arc::new(Mutex::new(Camera2D::default(
            self.window_size.0 as f32,
            self.window_size.1 as f32,
        )));

        // resource
        let input_helper = Arc::new(RwLock::new(WinitInputHelper::new()));

        // resource
        let frame_metrics = Arc::new(RwLock::new(FrameMetrics::new()));

        drop(gpu_mut);
        resources.insert(Arc::clone(&gpu));
        resources.insert(Arc::clone(&window));
        resources.insert(Arc::clone(&registry.textures));
        resources.insert(Arc::clone(&registry.meshes));
        resources.insert(Arc::clone(&input_helper));
        resources.insert(Arc::clone(&frame_metrics));
        resources.insert(Arc::clone(&render_graph));
        resources.insert(Arc::clone(&camera_2d));

        info!("ready to start!");
        Ok((
            Engine {
                mode: EngineMode::Forward2D,
                reporter: EngineReporter::new(Arc::clone(&engine_metrics.fps)),
                input: input_helper,
                legion: LegionState {
                    world: World::default(),
                    schedule,
                    resources,
                },
                graph: render_graph,
                registry,
                window,
                engine_metrics,
                frame_metrics,
                gpu,
            },
            event_loop,
        ))
    }

    pub fn default_3d(self) -> Result<(Engine, EventLoop<()>)> {
        info!("building engine: default_3d");

        let (gpu, window, event_loop, registry, mut resources) = build_engine_common(
            self.window_size,
            self.texture_registry_builder,
            self.mesh_registry_builder,
        )?;
        let gpu_mut = gpu.lock().unwrap();

        info!("building uniforms");
        let render_3d_group_builder = Arc::new(Mutex::new(Render3DForwardUniformGroup::builder()));
        let camera_3d_group_builder = Arc::new(Mutex::new(Camera3DUniformGroup::builder()));

        info!("building render graph nodes");
        let node_3d_forward_basic = build_node_3d_forward_basic(
            Arc::clone(&render_3d_group_builder),
            Arc::clone(&camera_3d_group_builder),
        );

        info!("scheduling systems");
        let mut schedule = Schedule::builder();
        schedule
            // Main engine systems
            .add_system(camera_3d_system())
            .add_system(physics_3d_system())
            // Uniform loading systems
            .flush()
            .add_system(render_3d::forward_basic::load_system())
            .add_system(camera_3d_uniform_system());

        let metrics_ui = EngineMetrics::new();

        info!("building render graph");
        let mut graph_schedule = SubSchedule::new();
        let (render_graph, engine_metrics) = GraphBuilder::new()
            .with_master_node(node_3d_forward_basic)
            .build(
                Arc::clone(&gpu_mut.device),
                Arc::clone(&gpu_mut.queue),
                &mut resources,
                &mut graph_schedule,
                &registry,
                &window,
                metrics_ui,
            )?;

        info!("scheduling render graph");
        graph_schedule.schedule(&mut schedule);
        let schedule = schedule.build();

        // resource
        let camera_3d = Arc::new(Mutex::new(Camera3D::default(
            self.window_size.0 as f32,
            self.window_size.1 as f32,
        )));

        // resource
        let input_helper = Arc::new(RwLock::new(WinitInputHelper::new()));

        // resource
        let frame_metrics = Arc::new(RwLock::new(FrameMetrics::new()));

        drop(gpu_mut);
        resources.insert(Arc::clone(&gpu));
        resources.insert(Arc::clone(&window));
        resources.insert(Arc::clone(&registry.textures));
        resources.insert(Arc::clone(&registry.meshes));
        resources.insert(Arc::clone(&input_helper));
        resources.insert(Arc::clone(&frame_metrics));
        resources.insert(Arc::clone(&render_graph));
        resources.insert(Arc::clone(&render_3d_group_builder));
        resources.insert(Arc::clone(&camera_3d));

        info!("ready to start!");
        Ok((
            Engine {
                mode: EngineMode::Forward3D,
                reporter: EngineReporter::new(Arc::clone(&engine_metrics.fps)),
                input: input_helper,
                legion: LegionState {
                    world: World::default(),
                    schedule,
                    resources,
                },
                graph: render_graph,
                registry,
                window,
                engine_metrics,
                frame_metrics,
                gpu,
            },
            event_loop,
        ))
    }

    pub fn default_quad(self, shader_source: ShaderSource) -> Result<(Engine, EventLoop<()>)> {
        info!("building engine: default_shader");

        let (gpu, window, event_loop, registry, mut resources) = build_engine_common(
            self.window_size,
            self.texture_registry_builder,
            self.mesh_registry_builder,
        )?;
        let gpu_mut = gpu.lock().unwrap();

        info!("building uniforms");
        let quad_group_builder = Arc::new(Mutex::new(QuadUniformGroup::builder()));
        let camera_3d_group_builder = Arc::new(Mutex::new(Camera3DUniformGroup::builder()));

        info!("building render graph nodes");
        let node_quad = build_node_quad(
            Arc::clone(&quad_group_builder),
            Arc::clone(&camera_3d_group_builder),
            shader_source,
        );

        info!("scheduling systems");
        let mut schedule = Schedule::builder();
        schedule
            // Main engine systems
            .add_system(camera_3d_system())
            // Uniform loading systems
            .flush()
            .add_system(camera_3d_uniform_system())
            .add_system(quad::load_system());

        info!("building render graph");
        let metrics_ui = EngineMetrics::new();
        let mut graph_schedule = SubSchedule::new();
        let (render_graph, engine_metrics) =
            GraphBuilder::new().with_master_node(node_quad).build(
                Arc::clone(&gpu_mut.device),
                Arc::clone(&gpu_mut.queue),
                &mut resources,
                &mut graph_schedule,
                &registry,
                &window,
                metrics_ui,
            )?;

        info!("scheduling render graph");
        graph_schedule.schedule(&mut schedule);
        let schedule = schedule.build();

        // resource
        let input_helper = Arc::new(RwLock::new(WinitInputHelper::new()));

        // resource
        let frame_metrics = Arc::new(RwLock::new(FrameMetrics::new()));

        // resource
        let quad = {
            let quad_group_builder = resources
                .get::<Arc<Mutex<GroupStateBuilder<QuadUniformGroup>>>>()
                .unwrap();

            let builder_mut = quad_group_builder.lock().unwrap();

            quad::Quad {
                mesh: registry
                    .meshes
                    .read()
                    .unwrap()
                    .clone_mesh(&ID(SCREEN_QUAD_MESH_ID), &ID(PRIMITIVE_MESH_GROUP_ID)),
                uniforms: Default::default(),
                uniform_group: builder_mut.single_state(&gpu_mut.device, &gpu_mut.queue)?,
            }
        };

        // resource
        let camera_3d = Arc::new(Mutex::new(Camera3D::default(
            self.window_size.0 as f32,
            self.window_size.1 as f32,
        )));

        drop(gpu_mut);
        resources.insert(quad);
        resources.insert(Arc::clone(&gpu));
        resources.insert(Arc::clone(&window));
        resources.insert(Arc::clone(&registry.textures));
        resources.insert(Arc::clone(&registry.meshes));
        resources.insert(Arc::clone(&input_helper));
        resources.insert(Arc::clone(&frame_metrics));
        resources.insert(Arc::clone(&render_graph));
        resources.insert(Arc::clone(&camera_3d));

        info!("ready to start!");
        Ok((
            Engine {
                mode: EngineMode::Quad,
                reporter: EngineReporter::new(Arc::clone(&engine_metrics.fps)),
                input: input_helper,
                legion: LegionState {
                    world: World::default(),
                    schedule,
                    resources,
                },
                graph: render_graph,
                registry,
                window,
                engine_metrics,
                frame_metrics,
                gpu,
            },
            event_loop,
        ))
    }

    // RENDER GRAPH TEST MODE
    pub fn test_channel_node(self) -> Result<(Engine, EventLoop<()>)> {
        warn!("RUNNING EXPERIMENTAL ENGINE MODE: test_channel_node");
        info!("building engine: test_channel_node");

        let (gpu, window, event_loop, registry, mut resources) = build_engine_common(
            self.window_size,
            self.texture_registry_builder,
            self.mesh_registry_builder,
        )?;
        let gpu_mut = gpu.lock().unwrap();

        info!("building uniforms");
        let quad_group_builder = Arc::new(Mutex::new(QuadUniformGroup::builder()));
        let camera_3d_group_builder = Arc::new(Mutex::new(Camera3DUniformGroup::builder()));
        let render_3d_group_builder = Arc::new(Mutex::new(Render3DForwardUniformGroup::builder()));

        info!("building render graph nodes");
        let node_channel = build_node_channel(
            Arc::clone(&quad_group_builder),
            Arc::clone(&camera_3d_group_builder),
        );
        let node_3d_forward_basic = build_node_3d_forward_basic(
            Arc::clone(&render_3d_group_builder),
            Arc::clone(&camera_3d_group_builder),
        );

        info!("scheduling systems");
        let mut schedule = Schedule::builder();
        schedule
            // Main engine systems
            .add_system(camera_3d_system())
            // Uniform loading systems
            .flush()
            .add_system(camera_3d_uniform_system())
            .add_system(render_3d::forward_basic::load_system())
            .add_system(quad::load_system());

        info!("building render graph");
        let metrics_ui = EngineMetrics::new();
        let mut graph_schedule = SubSchedule::new();
        let (render_graph, engine_metrics) = GraphBuilder::new()
            .with_channel(
                node_3d_forward_basic.dest_id.clone(),
                node_channel.dest_id.clone(),
            )
            .with_source_node(node_3d_forward_basic)
            .with_master_node(node_channel)
            .build(
                Arc::clone(&gpu_mut.device),
                Arc::clone(&gpu_mut.queue),
                &mut resources,
                &mut graph_schedule,
                &registry,
                &window,
                metrics_ui,
            )?;

        info!("scheduling render graph");
        graph_schedule.schedule(&mut schedule);
        let schedule = schedule.build();

        // resource
        let input_helper = Arc::new(RwLock::new(WinitInputHelper::new()));

        // resource
        let frame_metrics = Arc::new(RwLock::new(FrameMetrics::new()));

        // resource
        let quad = {
            let quad_group_builder = resources
                .get::<Arc<Mutex<GroupStateBuilder<QuadUniformGroup>>>>()
                .unwrap();

            let builder_mut = quad_group_builder.lock().unwrap();

            quad::Quad {
                mesh: registry
                    .meshes
                    .read()
                    .unwrap()
                    .clone_mesh(&ID(SCREEN_QUAD_MESH_ID), &ID(PRIMITIVE_MESH_GROUP_ID)),
                uniforms: Default::default(),
                uniform_group: builder_mut.single_state(&gpu_mut.device, &gpu_mut.queue)?,
            }
        };

        // resource
        let camera_3d = Arc::new(Mutex::new(Camera3D::default(
            self.window_size.0 as f32,
            self.window_size.1 as f32,
        )));

        drop(gpu_mut);
        resources.insert(quad);
        resources.insert(Arc::clone(&gpu));
        resources.insert(Arc::clone(&window));
        resources.insert(Arc::clone(&registry.textures));
        resources.insert(Arc::clone(&registry.meshes));
        resources.insert(Arc::clone(&input_helper));
        resources.insert(Arc::clone(&frame_metrics));
        resources.insert(Arc::clone(&render_graph));
        resources.insert(Arc::clone(&render_3d_group_builder)); // what the shit is this?
        resources.insert(Arc::clone(&camera_3d));

        info!("ready to start!");
        Ok((
            Engine {
                mode: EngineMode::Forward3D,
                reporter: EngineReporter::new(Arc::clone(&engine_metrics.fps)),
                input: input_helper,
                legion: LegionState {
                    world: World::default(),
                    schedule,
                    resources,
                },
                graph: render_graph,
                registry,
                window,
                engine_metrics,
                frame_metrics,
                gpu,
            },
            event_loop,
        ))
    }
}

fn build_engine_common(
    window_size: (u32, u32),
    tex_reg_builder: TextureRegistryBuilder,
    mesh_reg_builder: MeshRegistryBuilder,
) -> Result<(
    Arc<Mutex<GpuState>>,
    Arc<Window>,
    EventLoop<()>,
    Registry,
    Resources,
)> {
    let mut resources = Resources::default();
    resources.insert(RwLock::new(FrameMetrics::new()));

    info!("building gpu");
    let (gpu, window, event_loop) = build_gpu(&mut resources, window_size)?;

    info!("building registry");
    let registry = build_registry(Arc::clone(&gpu), tex_reg_builder, mesh_reg_builder)?;

    let window_size = WindowSize {
        width: window_size.0 as f32,
        height: window_size.1 as f32,
    };
    resources.insert(Arc::new(window_size));

    Ok((gpu, window, event_loop, registry, resources))
}

// Dimension-agnostic init logic
fn build_gpu(
    resources: &mut Resources,
    window_size: (u32, u32),
) -> Result<(Arc<Mutex<GpuState>>, Arc<Window>, EventLoop<()>)> {
    let event_loop = EventLoop::new();
    let window = build_window(window_size, &event_loop)?;

    let gpu = Arc::new(Mutex::new(futures::executor::block_on(
        GpuStateBuilder::winit(Arc::clone(&window)).build(resources),
    )?));
    Ok((gpu, window, event_loop))
}

fn get_crate_directory() -> PathBuf {
    option_env!("CARGO_MANIFEST_DIR").map_or_else(
        || {
            let exe_path = env::current_exe().expect("Failed to get exe path");
            exe_path
                .parent()
                .expect("Failed to get exe dir")
                .to_path_buf()
        },
        |crate_dir| PathBuf::from(crate_dir),
    )
}

fn build_window(size: (u32, u32), event_loop: &EventLoop<()>) -> Result<Arc<Window>> {
    let size = LogicalSize::new(size.0 as f64, size.1 as f64);

    // Set initial size
    let ss_u32 = (size.width as u32, size.height as u32);
    *renderer::SCREEN_SIZE.write().unwrap() = ss_u32;
    info!("INITIAL SCREEN_SIZE: {}, {}", ss_u32.0, ss_u32.1);

    Ok(Arc::new({
        WindowBuilder::new()
            .with_title("Ember Engine")
            .with_inner_size(size)
            .with_min_inner_size(size)
            .with_max_inner_size(size)
            .with_resizable(false)
            // .with_fullscreen(None)
            .with_fullscreen(Some(Fullscreen::Borderless(None)))
            .build(event_loop)?
    }))
}

fn build_registry(
    gpu: Arc<Mutex<GpuState>>,
    mut tex_reg_builder: TextureRegistryBuilder,
    mesh_reg_builder: MeshRegistryBuilder,
) -> Result<Registry> {
    let mut gpu_mut = gpu.lock().unwrap();
    let base_dir = get_crate_directory();

    load_engine_textures(&mut tex_reg_builder, &base_dir);
    let texture_format = gpu_mut.device_preferred_format();
    Registry::build(
        Arc::clone(&gpu_mut.device),
        &gpu_mut.queue,
        texture_format,
        tex_reg_builder,
        mesh_reg_builder,
    )
}

fn load_engine_textures(builder: &mut TextureRegistryBuilder, base_dir: &PathBuf) {
    builder.load_id(
        Uuid::from_str(RENDER_2D_COMMON_TEXTURE_ID).unwrap(),
        &base_dir
            .join("src/sources/static/test.png")
            .into_os_string()
            .into_string()
            .unwrap(),
        &ID(RENDER_2D_TEXTURE_GROUP),
    );
    builder.load_id(
        Uuid::from_str(RENDER_3D_COMMON_TEXTURE_ID).unwrap(),
        &base_dir
            .join("src/sources/static/arrow.jpg")
            .into_os_string()
            .into_string()
            .unwrap(),
        &ID(RENDER_3D_TEXTURE_GROUP),
    );
}

fn build_node_2d_forward_dynamic(
    render_2d_dynamic_group_builder: Arc<Mutex<UniformGroupBuilder<Render2DForwardDynamicGroup>>>,
    camera_2d_group_builder: Arc<Mutex<UniformGroupBuilder<Camera2DUniformGroup>>>,
    lighting_2d_group_builder: Arc<Mutex<UniformGroupBuilder<Lighting2DUniformGroup>>>,
) -> NodeBuilder {
    NodeBuilder::new(
        "render_2d_node".to_owned(),
        0,
        ShaderSource::WGSL(include_str!("renderer/shaders/render_2d.wgsl").to_owned()),
    )
    .with_id(ID(FORWARD_2D_NODE_ID))
    .with_vertex_layout(VERTEX2D_BUFFER_LAYOUT)
    .with_texture_group(ID(RENDER_2D_TEXTURE_GROUP))
    .with_shared_uniform_group(Arc::clone(&render_2d_dynamic_group_builder))
    .with_shared_uniform_group(Arc::clone(&camera_2d_group_builder))
    .with_shared_uniform_group(Arc::clone(&lighting_2d_group_builder))
    .with_system(render_2d::forward_dynamic::render_system)
}

fn build_node_2d_forward_instance(
    camera_2d_group_builder: Arc<Mutex<UniformGroupBuilder<Camera2DUniformGroup>>>,
    lighting_2d_group_builder: Arc<Mutex<UniformGroupBuilder<Lighting2DUniformGroup>>>,
) -> NodeBuilder {
    NodeBuilder::new(
        "render_2d_instance_node".to_owned(),
        0,
        ShaderSource::WGSL(include_str!("renderer/shaders/render_2d_instance.wgsl").to_owned()),
    )
    .with_id(ID(INSTANCE_2D_NODE_ID))
    .with_vertex_layout(VERTEX2D_BUFFER_LAYOUT)
    .with_vertex_layout(render_2d::forward_instance::RENDER2DINSTANCE_BUFFER_LAYOUT)
    .with_texture_group(ID(RENDER_2D_TEXTURE_GROUP))
    .with_shared_uniform_group(Arc::clone(&camera_2d_group_builder))
    .with_shared_uniform_group(Arc::clone(&lighting_2d_group_builder))
    .with_system(render_2d::forward_instance::render_system)
}

fn build_node_3d_forward_basic(
    render_3d_group_builder: Arc<Mutex<UniformGroupBuilder<Render3DForwardUniformGroup>>>,
    camera_3d_group_builder: Arc<Mutex<UniformGroupBuilder<Camera3DUniformGroup>>>,
    //lighting_3d_group_builder: Arc<Mutex<UniformGroupBuilder<Lighting3DUniformGroup>>>,
) -> NodeBuilder {
    NodeBuilder::new(
        "render_3d_basic_node".to_owned(),
        0,
        ShaderSource::WGSL(include_str!("renderer/shaders/render_3d.wgsl").to_owned()),
    )
    .with_id(ID(FORWARD_3D_NODE_ID))
    .with_vertex_layout(VERTEX3D_BUFFER_LAYOUT)
    .with_texture_group(ID(RENDER_3D_TEXTURE_GROUP))
    .with_shared_uniform_group(Arc::clone(&render_3d_group_builder))
    .with_shared_uniform_group(Arc::clone(&camera_3d_group_builder))
    // .with_depth_buffer()
    .with_system(render_3d::forward_basic::render_system)
}

// similar to channel, but meant for raytracing
fn build_node_quad(
    quad_group_builder: Arc<Mutex<UniformGroupBuilder<QuadUniformGroup>>>,
    camera_3d_group_builder: Arc<Mutex<UniformGroupBuilder<Camera3DUniformGroup>>>,
    shader_source: ShaderSource,
) -> NodeBuilder {
    NodeBuilder::new("render_quad_node".to_owned(), 0, shader_source)
        .with_id(ID(QUAD_NODE_ID))
        .with_vertex_layout(VERTEX2D_BUFFER_LAYOUT)
        .with_shared_uniform_group(Arc::clone(&quad_group_builder))
        .with_shared_uniform_group(Arc::clone(&camera_3d_group_builder))
        .with_system(quad::render_system)
}

// similar to node, but meant for post-processing
fn build_node_channel(
    quad_group_builder: Arc<Mutex<UniformGroupBuilder<QuadUniformGroup>>>,
    camera_3d_group_builder: Arc<Mutex<UniformGroupBuilder<Camera3DUniformGroup>>>,
) -> NodeBuilder {
    NodeBuilder::new(
        "render_channel_node".to_owned(),
        1,
        ShaderSource::WGSL(include_str!("renderer/shaders/channelpass.wgsl").to_owned()),
    )
    .with_id(ID(CHANNEL_NODE_ID))
    .with_vertex_layout(VERTEX2D_BUFFER_LAYOUT)
    .with_node_input()
    .with_shared_uniform_group(Arc::clone(&quad_group_builder))
    .with_shared_uniform_group(Arc::clone(&camera_3d_group_builder))
    .with_system(channel::render_system)
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

pub struct TextureGroup {
    pub id: Uuid,
    pub textures: Vec<(Uuid, String)>,
}

pub struct MeshGroup {
    pub id: Uuid,
    pub meshes: Vec<(Uuid, String)>,
}
