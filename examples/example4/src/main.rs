use ember::{
    components::{DeltaTransform3D, Transform3D},
    constants::{ID, PRIMITIVE_MESH_GROUP_ID, UNIT_CUBE_MESH_ID},
    renderer::systems::render_3d::forward_basic::Render3D,
};

// TEST EXAMPLE: render graph and channel nodes

fn main() {
    std::env::set_var("RUST_LOG", "debug");
    let (mut engine, event_loop) = ember::engine_builder().test_channel_node().unwrap();

    let cube_mesh = engine.clone_mesh(&ID(UNIT_CUBE_MESH_ID), &ID(PRIMITIVE_MESH_GROUP_ID));
    engine.world().push((
        Render3D::default("test_cube"),
        cube_mesh,
        Transform3D {
            position: [0.0, 0.0, 0.0],
            rotation: [-90.0, 0.0, -90.0],
            ..Default::default() //scale: [0.1, 0.1, 0.1],
        },
        DeltaTransform3D {
            rotation: [0.0, 0.0, -20.0],
            ..Default::default()
        },
    ));

    engine.start(event_loop);
}
