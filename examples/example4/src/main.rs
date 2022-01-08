use ember::{
    components::{DeltaTransform3D, Transform3D},
    constants::{ID, PRIMITIVE_MESH_GROUP_ID, UNIT_CUBE_MESH_ID},
    renderer::systems::render_3d::forward_pbr::RenderPBR,
    MeshGroup,
};
use uuid::Uuid;

// TEST EXAMPLE: render graph and channel nodes

fn main() {
    std::env::set_var("RUST_LOG", "ember=info");

    let sphere_mesh_group_id = Uuid::new_v4();
    let sphere_mesh_id = Uuid::new_v4();
    let sphere_mesh_group = MeshGroup {
        id: sphere_mesh_group_id,
        meshes: vec![(
            sphere_mesh_id,
            "./engine/src/sources/static/obj/sphere.obj".to_owned(),
        )],
    };

    let (mut engine, event_loop) = ember::engine_builder()
        .with_mesh_group(sphere_mesh_group)
        .test_channel_node()
        .unwrap();

    let sphere_mesh = engine.clone_mesh(&ID(UNIT_CUBE_MESH_ID), &ID(PRIMITIVE_MESH_GROUP_ID));
    // let sphere_mesh = engine.clone_mesh(&sphere_mesh_id, &sphere_mesh_group_id);
    // engine.world().push((
    //     RenderPBR::colored("test_sphere", [0.3, 0.1, 0.1, 1.0]),
    //     Transform3D {
    //         scale: [10.0, 10.0, 10.0],
    //         ..Default::default()
    //     },
    //     DeltaTransform3D {
    //         rotation: [10.0, 6.0, 0.0],
    //         ..Default::default()
    //     },
    //     sphere_mesh,
    // ));

    engine.start(event_loop);
}
