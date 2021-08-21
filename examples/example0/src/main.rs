use std::sync::Arc;

use ember::{
    components::{Position2D, Transform2D, Velocity2D},
    constants::{ID, PRIMITIVE_MESH_GROUP_ID, UNIT_SQUARE_MESH_ID},
    renderer::systems::render_2d::forward_instance::{Attractor2D, Render2DInstance},
    systems::lighting_2d::Light2D,
};
use rand::Rng;

// Ember example

pub struct ParticleInstanceGroup();

fn main() {
    std::env::set_var("RUST_LOG", "debug");
    let (mut engine, event_loop) = ember::builder().default().unwrap();

    let mut particle_group = Render2DInstance::new_default_group();
    let particle_mesh = engine.clone_mesh(&ID(UNIT_SQUARE_MESH_ID), &ID(PRIMITIVE_MESH_GROUP_ID));

    // engine.world().push((
    //     Render2D::solid_rect("background", 1440.0, 900.0, [0.02, 0.02, 0.05, 1.0]),
    //     Position2D {
    //         x: 0.0, //rng.gen_range(100..500) as f32,
    //         y: 0.0, //rng.gen_range(100..500) as f32,
    //     },
    // ));

    let mut rng = rand::thread_rng();
    for _i in 0..1000 {
        particle_group.push(
            Render2DInstance::new(
                rng.gen_range(-700.0..700.0),
                rng.gen_range(-400.0..400.0),
                1.0,
                1.0,
                [1.0, 1.0, 0.0, 1.0],
            ),
            vec![Arc::new(Transform2D::default())],
        );
    }

    // for _i in 0..1000 {
    //     engine.world().push((
    //         particle_group.push(Render2DInstance::new(
    //             rng.gen_range(-700.0..700.0),
    //             rng.gen_range(-400.0..400.0),
    //             1.0,
    //             1.0,
    //             [1.0, 1.0, 0.0, 1.0],
    //         )),
    //         // Render2D::solid_rect(&format!("light_{}", i), 10.0, 10.0, [1.0, 1.0, 1.0, 1.0]),
    //         Position2D {
    //             x: rng.gen_range(-700.0..700.0),
    //             y: rng.gen_range(-400.0..400.0),
    //         },
    //         Velocity2D {
    //             dx: rng.gen_range(-10.0..10.0),
    //             dy: rng.gen_range(-10.0..10.0),
    //             bounce: true,
    //         },
    //     ));
    // }

    // engine.world().push((
    //     particle_group.push(Render2DInstance::new(
    //         0.0,
    //         0.0,
    //         20.0,
    //         20.0,
    //         [1.0, 1.0, 1.0, 1.0],
    //     )),
    //     Position2D { x: 0.0, y: 0.0 },
    //     Attractor2D { force: 10000.0 },
    //     Light2D {
    //         linear: 0.01,
    //         quadratic: 0.0002,
    //     },
    // ));

    engine.world().push((particle_group, particle_mesh));

    // for i in 0..120 {
    //     let size = rng.gen_range(5.0..25.0);
    //     engine.world().push((
    //         Render2D::solid_rect(&format!("block_{}", i), size, size, [1.0, 1.0, 1.0, 1.0]),
    //         Position2D {
    //             x: rng.gen_range(100.0..500.0),
    //             y: rng.gen_range(100.0..500.0),
    //         },
    //         Velocity2D {
    //             dx: rng.gen_range(-15.0..15.0),
    //             dy: rng.gen_range(-15.0..15.0),
    //             bounce: true,
    //         },
    //     ));
    // }

    engine.start(event_loop);
}
