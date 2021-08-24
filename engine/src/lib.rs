extern crate pretty_env_logger;
#[macro_use]
extern crate log;
#[macro_use]
extern crate legion;
#[macro_use]
extern crate vertex_layout_derive;
extern crate vertex_traits;

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
    window::{Window, WindowBuilder},
};
use winit_input_helper::WinitInputHelper;

use crate::{
    constants::*,
    renderer::{
        buffer::{instance::*, *},
        graph::{
            node::{NodeBuilder, ShaderSource},
            GraphBuilder, RenderGraph,
        },
        mesh::Mesh,
        systems::{
            render_2d::{
                forward_dynamic::Render2DForwardDynamicGroup,
                forward_instance::Render2DUniformGroup,
            },
            render_3d::forward_basic::Render3DForwardUniformGroup,
            *,
        },
        uniform::{
            generic::GenericUniformBuilder,
            group::{UniformGroup, UniformGroupBuilder, UniformGroupType},
        },
        GpuState, GpuStateBuilder,
    },
    sources::{
        camera::{Camera2D, Camera3D},
        metrics::{EngineMetrics, EngineReporter},
        registry::{MeshRegistryBuilder, Registry, TextureRegistryBuilder},
        schedule::{Schedulable, SubSchedule},
        ui::UI,
    },
    systems::{camera_2d::*, camera_3d::*, lighting_2d::*, physics_2d::*},
};

pub fn engine_builder() -> EngineBuilder {
    EngineBuilder {
        window_size: (1440, 900),
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
    metrics: Arc<EngineMetrics>,
    registry: Registry,
    legion: LegionState,
    reporter: EngineReporter,
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
        self.window.set_cursor_visible(false);
        let _ = self.window.set_cursor_grab(true);
        let metrics_last_updated = Arc::new(Mutex::new(Instant::now()));

        // top-level event loop; hijacks thread
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

            if self.input.write().unwrap().update(&event) {
                let input = self.input.read().unwrap();
                if input.key_pressed(VirtualKeyCode::Escape) || input.quit() {
                    debug!("shutting down");
                    *control_flow = ControlFlow::Exit;
                    return;
                }

                if let Some(physical_size) = input.resolution() {
                    let _ = &self.gpu.lock().unwrap().resize(physical_size);
                }
                self.window.request_redraw();
            }
        });
    }
}

pub struct EngineBuilder {
    // Engine config
    window_size: (usize, usize),

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
    pub fn default_2d(mut self) -> Result<(Engine, EventLoop<()>)> {
        info!("building engine: default 2d");

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
            // .add_system(render_2d::forward_instance::attractor_system())
            // Uniform loading systems
            .flush()
            .add_system(render_2d::forward_instance::load_system())
            .add_system(camera_2d_uniform_system())
            .add_system(lighting_2d_uniform_system());

        info!("building render graph");
        let metrics_ui = EngineMetrics::new();
        let mut graph_schedule = SubSchedule::new();
        let (render_graph, metrics) = GraphBuilder::new()
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
            DEFAULT_SCREEN_WIDTH as f32,
            DEFAULT_SCREEN_HEIGHT as f32,
        )));

        // resource
        let input_helper = Arc::new(RwLock::new(WinitInputHelper::new()));

        drop(gpu_mut);
        resources.insert(Arc::clone(&gpu));
        resources.insert(Arc::clone(&window));
        resources.insert(Arc::clone(&registry.textures));
        resources.insert(Arc::clone(&registry.meshes));
        resources.insert(Arc::clone(&input_helper));
        resources.insert(Arc::clone(&render_graph));
        resources.insert(Arc::clone(&camera_2d));

        info!("ready to start!");
        Ok((
            Engine {
                reporter: EngineReporter::new(Arc::clone(&metrics.fps)),
                input: input_helper,
                legion: LegionState {
                    world: World::default(),
                    schedule,
                    resources,
                },
                graph: render_graph,
                registry,
                window,
                metrics,
                gpu,
            },
            event_loop,
        ))
    }

    pub fn default_3d(mut self) -> Result<(Engine, EventLoop<()>)> {
        info!("building engine: default 3d");

        let (gpu, window, event_loop, registry, mut resources) = build_engine_common(
            self.window_size,
            self.texture_registry_builder,
            self.mesh_registry_builder,
        )?;
        let gpu_mut = gpu.lock().unwrap();

        info!("building uniforms");
        let render_2d_dynamic_group_builder =
            Arc::new(Mutex::new(Render2DForwardDynamicGroup::builder()));
        let render_3d_group_builder = Arc::new(Mutex::new(Render3DForwardUniformGroup::builder()));
        let camera_2d_group_builder = Arc::new(Mutex::new(Camera2DUniformGroup::builder()));
        let camera_3d_group_builder = Arc::new(Mutex::new(Camera3DUniformGroup::builder()));
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
        let node_3d_forward_basic = build_node_3d_forward_basic(
            Arc::clone(&render_3d_group_builder),
            Arc::clone(&camera_3d_group_builder),
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
            .add_system(camera_3d_system())
            .add_system(lighting_2d_system())
            // .add_system(render_2d::forward_instance::attractor_system())
            // Uniform loading systems
            .flush()
            .add_system(render_2d::forward_instance::load_system())
            .add_system(render_3d::forward_basic::load_system())
            .add_system(camera_2d_uniform_system())
            .add_system(camera_3d_uniform_system())
            .add_system(lighting_2d_uniform_system());

        let metrics_ui = EngineMetrics::new();

        info!("building render graph");
        let mut graph_schedule = SubSchedule::new();
        let (render_graph, metrics) = GraphBuilder::new()
            .with_node(node_3d_forward_basic)
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
            DEFAULT_SCREEN_WIDTH as f32,
            DEFAULT_SCREEN_HEIGHT as f32,
        )));

        // resource
        let camera_3d = Arc::new(Mutex::new(Camera3D::default(
            DEFAULT_SCREEN_WIDTH as f32,
            DEFAULT_SCREEN_HEIGHT as f32,
        )));

        // resource
        let input_helper = Arc::new(RwLock::new(WinitInputHelper::new()));

        drop(gpu_mut);
        resources.insert(Arc::clone(&gpu));
        resources.insert(Arc::clone(&window));
        resources.insert(Arc::clone(&registry.textures));
        resources.insert(Arc::clone(&registry.meshes));
        resources.insert(Arc::clone(&input_helper));
        resources.insert(Arc::clone(&render_graph));
        resources.insert(Arc::clone(&render_3d_group_builder));
        resources.insert(Arc::clone(&camera_2d));
        resources.insert(Arc::clone(&camera_3d));

        info!("ready to start!");
        Ok((
            Engine {
                reporter: EngineReporter::new(Arc::clone(&metrics.fps)),
                input: input_helper,
                legion: LegionState {
                    world: World::default(),
                    schedule,
                    resources,
                },
                graph: render_graph,
                registry,
                window,
                metrics,
                gpu,
            },
            event_loop,
        ))
    }
}

fn build_engine_common(
    window_size: (usize, usize),
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

    info!("building gpu");
    let (gpu, window, event_loop) = build_gpu(&mut resources, window_size)?;

    info!("building registry");
    let registry = build_registry(Arc::clone(&gpu), tex_reg_builder, mesh_reg_builder)?;

    Ok((gpu, window, event_loop, registry, resources))
}

// Dimension-agnostic init logic
fn build_gpu(
    resources: &mut Resources,
    window_size: (usize, usize),
) -> Result<(Arc<Mutex<GpuState>>, Arc<Window>, EventLoop<()>)> {
    pretty_env_logger::init();
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

fn build_window(size: (usize, usize), event_loop: &EventLoop<()>) -> Result<Arc<Window>> {
    let size = LogicalSize::new(size.0 as f64, size.1 as f64);
    Ok(Arc::new({
        WindowBuilder::new()
            .with_title("Hello World")
            .with_inner_size(size)
            .with_min_inner_size(size)
            .with_max_inner_size(size)
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
    .with_depth_buffer()
    .with_system(render_3d::forward_basic::render_system)
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
