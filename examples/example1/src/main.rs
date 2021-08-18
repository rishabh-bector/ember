use ember::{
    components::Position3D, renderer::systems::render_3d::forward_basic::Render3D, MeshGroup,
};
use uuid::Uuid;

// Ember example

// Make this a macro:
//
// #[texture_group]
// pub enum Textures {
//     #[texture(Dog, "./dog.png")]
//     #[texture(Cat, "./cat.png")]
//     #[texture(Mouse, "./mouse.png")]
// }
//
// And a similar thing for meshes:
//
// #[mesh_group]
// pub enum Meshes {
//     #[mesh(Dog, "./dog.obj")]
//     #[mesh(Cat, "./cat.obj")]
//     #[mesh(Mouse, "./mouse.obj")]
// }
//
// This would generate something like this,
// using my uuid macro:
//
// pub struct Textures;
// impl Textures {
//     pub const Dog: Uuid = uuid_v4!();
// }

fn main() {
    std::env::set_var("RUST_LOG", "ember=debug");
    let engine_builder = ember::builder();

    let airplane_mesh_group_id = Uuid::new_v4();
    let airplane_mesh_id = Uuid::new_v4();
    let airplane_mesh_group = MeshGroup {
        id: airplane_mesh_group_id,
        meshes: vec![(
            airplane_mesh_id,
            "./engine/src/sources/static/airplane.obj".to_owned(),
        )],
    };

    let (mut engine, event_loop) = engine_builder
        .with_mesh_group(airplane_mesh_group)
        .default()
        .unwrap();

    let airplane_mesh = engine.clone_mesh(&airplane_mesh_id, &airplane_mesh_group_id);
    engine.world().push((
        Render3D::default("test_cube"),
        Position3D {
            x: 0.0,
            y: 100.0,
            z: 2000.0,
        },
        airplane_mesh,
    ));

    engine.start(event_loop);
}
