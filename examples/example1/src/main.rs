use ember::{
    components::Position3D,
    constants::{ID, UNIT_CUBE_MESH_ID},
    renderer::systems::render_3d::forward_basic::Render3D,
};

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
    let (mut engine, event_loop) = ember::engine().default().unwrap();

    let cube_mesh = engine.clone_mesh(ID(UNIT_CUBE_MESH_ID));
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
