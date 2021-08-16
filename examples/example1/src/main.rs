use ember::{
    components::Position3D, renderer::systems::render_3d::forward_basic::Render3D,
    sources::primitives::PrimitiveMesh,
};

// Ember example

fn main() {
    std::env::set_var("RUST_LOG", "ember=debug");
    let (mut engine, event_loop) = ember::engine().default().unwrap();

    let cube_mesh = engine.mesh(PrimitiveMesh::UnitCube);
    engine.world().push((
        Render3D::default("test_cube"),
        Position3D {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        },
        cube_mesh,
    ));

    engine.start(event_loop);
}
