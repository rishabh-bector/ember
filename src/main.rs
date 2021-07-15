extern crate pretty_env_logger;
#[macro_use]
extern crate log;
#[macro_use]
extern crate legion;

use anyhow::Result;
use cgmath::SquareMatrix;
use legion::Schedule;
use legion::{Resources, World};
use rand::Rng;
use render::GpuState;
use std::borrow::Borrow;
use std::env;
use std::path::PathBuf;
use std::sync::Mutex;
use std::{cell::RefCell, rc::Rc, sync::Arc};
use winit::dpi::LogicalSize;
use winit::event::{Event, VirtualKeyCode};
use winit::event_loop::{ControlFlow, EventLoop};
use winit::window::WindowBuilder;
use winit_input_helper::WinitInputHelper;

mod component;
mod render;
mod resources;
mod systems;

use component::{Base2D, Position2D, Velocity2D};
use resources::store::TextureStore;
use systems::{camera_2d::*, physics_2d::*, render_2d::*};

use crate::render::{
    buffer::{IndexBuffer, Vertex2D, VertexBuffer},
    uniform::{ShaderStage, UniformBuffer},
};
use crate::resources::camera::{Camera2D, Camera2DUniforms};

const SCREEN_WIDTH: usize = 1920;
const SCREEN_HEIGHT: usize = 1080;

fn main() -> Result<()> {
    pretty_env_logger::init();
    info!("Starting engine...");

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

    let gpu = Arc::new(Mutex::new(futures::executor::block_on(GpuState::new(
        &window,
    ))?));

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

    let texture_store = Arc::new(Mutex::new(TextureStore::new(
        &gpu,
        vec![(
            "test",
            &base_dir
                .join("src/static/test.png")
                .into_os_string()
                .into_string()
                .unwrap(),
        )],
    )?));

    let camera = Arc::new(Mutex::new(Camera2D::default(1920.0, 1080.0)));
    let camera_uniforms = Arc::new(Mutex::new(UniformBuffer::generic(
        Camera2DUniforms {
            view: [1.0, 1.0, 1.0, 1.0],
        },
        ShaderStage::VERTEX_FRAGMENT,
        &gpu.lock().unwrap().device,
    )));

    let mut resources = Resources::default();
    resources.insert(Arc::clone(&gpu));
    resources.insert(Arc::clone(&texture_store));
    resources.insert(Arc::clone(&camera));
    resources.insert(Arc::clone(&camera_uniforms));

    let mut world = World::default();

    let mut rng = rand::thread_rng();
    for _ in 0..1 {
        world.push((
            Base2D::common(),
            Position2D {
                x: rng.gen_range(100..500) as f32,
                y: rng.gen_range(100..500) as f32,
            },
            Velocity2D {
                dx: [-1, 1][rng.gen_range(0..=1)] as f32,
                dy: [-1, 1][rng.gen_range(0..=1)] as f32,
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
        &gpu.lock().unwrap().device,
    )];
    let common_index_buffers: [IndexBuffer; 1] = [IndexBuffer::new(
        &[0, 2, 1, 3, 2, 0],
        &gpu.lock().unwrap().device,
    )];

    let mut schedule = Schedule::builder()
        .add_system(physics_2d_system())
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
                &gpu.lock().unwrap().resize(winit::dpi::PhysicalSize::new(
                    physical_size.0,
                    physical_size.1,
                ));
            }

            // Request a redraw
            window.request_redraw();
        }
    });
}
