use ember::{
    components::{Position2D, Velocity2D},
    renderer::systems::render_2d::{forward_instance::Render2DInstance, Render2D},
    systems::lighting_2d::Light2D,
};
use rand::Rng;

// Ember example

fn main() {
    std::env::set_var("RUST_LOG", "info");
    let (mut engine, event_loop) = ember::engine().default().unwrap();

    let mut particle_group = Render2DInstance::default_group();

    // engine.world().push((
    //     Render2D::solid_rect("background", 1440.0, 900.0, [0.02, 0.02, 0.05, 1.0]),
    //     Position2D {
    //         x: 0.0, //rng.gen_range(100..500) as f32,
    //         y: 0.0, //rng.gen_range(100..500) as f32,
    //     },
    // ));

    let mut rng = rand::thread_rng();
    for i in 0..5000 {
        engine.world().push((
            particle_group.insert(Render2DInstance::new(
                rng.gen_range(100.0..500.0),
                rng.gen_range(100.0..500.0),
                3.0,
                3.0,
                [1.0, 1.0, 0.0, 1.0],
            )),
            // Render2D::solid_rect(&format!("light_{}", i), 10.0, 10.0, [1.0, 1.0, 1.0, 1.0]),
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

    engine.with_instance_group(particle_group).start(event_loop);
}
