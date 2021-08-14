use ember::{
    components::{Position2D, Position3D, Velocity2D},
    renderer::systems::{
        render_2d::forward_instance::{Attractor2D, Render2DInstance},
        render_3d::forward_basic::Render3D,
    },
    systems::lighting_2d::Light2D,
};

// Ember example

fn main() {
    std::env::set_var("RUST_LOG", "debug");
    let (mut engine, event_loop) = ember::engine().default().unwrap();

    engine.world().push((
        Render3D::default("test"),
        Position3D {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        },
    ));

    engine.start(event_loop);
}
