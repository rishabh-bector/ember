extern crate pretty_env_logger;
#[macro_use]
extern crate log;
#[macro_use]
extern crate legion;

use anyhow::Result;
use legion::{Resources, Schedule, World};
use rand::Rng;
use std::{
    env,
    path::PathBuf,
    sync::{Arc, Mutex},
};
use winit::{
    dpi::LogicalSize,
    event::{Event, VirtualKeyCode},
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};
use winit_input_helper::WinitInputHelper;

mod component;
mod render;
mod resources;
mod systems;

use crate::{
    component::{Position2D, Velocity2D},
    render::{buffer::*, pipeline::*, uniform::*, *},
    resources::{camera::Camera2D, store::TextureStoreBuilder},
    systems::{camera_2d::*, lighting_2d::*, physics_2d::*, render_2d::*},
};

// NEXT TODO: FIGURE OUT HOW TO GENERALIZE WHAT I CURRENTLY HAVE HARDCODED, WHICH IS:
//      - Multiple uniform groups in one render pass, via a dynamic offset system.
//        This is currently hardcoded to the two entities which work.
//
// items:
//  - problem: destination buffer size of 512, in GenericUniformBuilder::BuildBuffer
//    fix: each uniform needs a size, pass in on builder creation via dynamic()
//         when will the needed size be known? size depends on number of entities in scene... set changeable max 64

const SCREEN_WIDTH: usize = 1440;
const SCREEN_HEIGHT: usize = 900;

fn main() -> Result<()> {
    std::env::set_var("RUST_LOG", "ember=info");
    pretty_env_logger::init();
    info!("Starting engine...");

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
    let size = LogicalSize::new(SCREEN_WIDTH as f64, SCREEN_HEIGHT as f64);
    let window = {
        WindowBuilder::new()
            .with_title("Hello World")
            .with_inner_size(size)
            .with_min_inner_size(size)
            .build(&event_loop)?
    };

    let base_2d_uniforms = UniformGroup::<Base2DUniformGroup>::builder().uniform(
        GenericUniformBuilder::source(Base2DUniforms {
            model: [0.0, 0.0, 1.0, 1.0],
            color: [1.0, 1.0, 1.0, 1.0],
            mix: 1.0,
            _padding: [0.0; 32],
            __padding: [0.0; 23],
        }),
    );

    let camera_2d_uniforms = UniformGroup::<Camera2DUniformGroup>::builder().uniform(
        GenericUniformBuilder::source(Camera2DUniforms {
            //view: [-(SCREEN_WIDTH as f32), -(SCREEN_HEIGHT as f32), 1.0/(SCREEN_WIDTH as f32), 1.0/(SCREEN_HEIGHT as f32)],
            // view: [(SCREEN_WIDTH as f32)/2.0, (SCREEN_HEIGHT as f32)/2.0, 1.0/(SCREEN_WIDTH as f32), 1.0/(SCREEN_HEIGHT as f32)],
            view: [1.0, 1.0, 1.0, 1.0],
            _padding: [0.0; 32],
            __padding: [0.0; 28],
        }),
    );

    let lighting_2d_uniforms = UniformGroup::<Lighting2DUniformGroup>::builder().uniform(
        GenericUniformBuilder::source(Lighting2DUniforms {
            light_0: Default::default(),
            light_1: Default::default(),
            light_2: Default::default(),
            light_3: Default::default(),
            light_4: Default::default(),
        }),
    );

    let base_2d_pipeline = PipelineBuilder::new(ShaderSource::WGSL(
        include_str!("render/shaders/base2D.wgsl").to_owned(),
    ))
    .texture_group()
    .uniform_group(base_2d_uniforms)
    .uniform_group(camera_2d_uniforms)
    .uniform_group(lighting_2d_uniforms)
    .vertex_buffer_layout(VertexBuffer::layout_2d());

    let mut resources = Resources::default();

    let gpu_state = Arc::new(Mutex::new(futures::executor::block_on(
        GpuStateBuilder::winit(&window)
            .pipeline(base_2d_pipeline)
            .build(
                &mut TextureStoreBuilder::new().load(
                    "test",
                    &base_dir
                        .join("src/static/test.png")
                        .into_os_string()
                        .into_string()
                        .unwrap(),
                ),
                &mut resources,
            ),
    )?));

    let camera = Arc::new(Mutex::new(Camera2D::default(
        SCREEN_WIDTH as f32,
        SCREEN_HEIGHT as f32,
    )));

    resources.insert(Arc::clone(&gpu_state));
    resources.insert(Arc::clone(&camera));

    let mut world = World::default();

    world.push((
        Base2D::solid_rect(
            "background",
            SCREEN_WIDTH as f32,
            SCREEN_HEIGHT as f32,
            [0.02, 0.02, 0.05, 1.0],
        ),
        Position2D {
            x: 0.0, //rng.gen_range(100..500) as f32,
            y: 0.0, //rng.gen_range(100..500) as f32,
        },
    ));

    let mut rng = rand::thread_rng();
    for i in 0..5 {
        world.push((
            Base2D::solid_rect(&format!("light_{}", i), 10.0, 10.0, [1.0, 1.0, 1.0, 1.0]),
            Position2D {
                x: rng.gen_range(100.0..500.0),
                y: rng.gen_range(100.0..500.0),
            },
            Velocity2D {
                dx: rng.gen_range(-15.0..15.0),
                dy: rng.gen_range(-15.0..15.0),
                bounce: true,
            },
            Light2D {
                linear: 0.007,
                quadratic: 0.0002,
            },
        ));
    }

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
        &gpu_state.lock().unwrap().device,
    )];
    let common_index_buffers: [IndexBuffer; 1] = [IndexBuffer::new(
        &[0, 2, 1, 3, 2, 0],
        &gpu_state.lock().unwrap().device,
    )];

    let mut schedule = Schedule::builder()
        .add_system(physics_2d_system())
        .add_system(lighting_2d_system())
        .flush()
        .add_system(camera_2d_system())
        .flush()
        .add_thread_local(render_2d_system(Render2DSystem {
            common_vertex_buffers,
            common_index_buffers,
        }))
        .build();

    event_loop.run(move |event, _, control_flow| {
        if let Event::RedrawRequested(_) = event {
            schedule.execute(&mut world, &mut resources);
        }

        if input.update(&event) {
            if input.key_pressed(VirtualKeyCode::Escape) || input.quit() {
                *control_flow = ControlFlow::Exit;
                return;
            }

            if let Some(physical_size) = input.resolution() {
                &gpu_state.lock().unwrap().resize(physical_size);
            }

            // Request a redraw
            window.request_redraw();
        }
    });
}