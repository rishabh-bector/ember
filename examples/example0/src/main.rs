use ember::{
    components::Motion2D,
    constants::{ID, PRIMITIVE_MESH_GROUP_ID, UNIT_SQUARE_MESH_ID},
    renderer::systems::render_2d::forward_instance::Render2DInstance,
};
use rand::Rng;
use std::sync::{Arc, Mutex};

// Ember example: 2D instance group

fn main() {
    std::env::set_var("RUST_LOG", "debug");
    let (mut engine, event_loop) = ember::engine_builder().default_2d().unwrap();

    let mut instance_group = Render2DInstance::new_default_group();
    let instance_mesh = engine.clone_mesh(&ID(UNIT_SQUARE_MESH_ID), &ID(PRIMITIVE_MESH_GROUP_ID));

    let mut rng = rand::thread_rng();
    for _i in 0..5000 {
        instance_group.push(
            Render2DInstance::new([1.0, 1.0, 1.0, 1.0]),
            vec![Arc::new(Mutex::new(Motion2D::new(
                rng.gen_range(-5.0..5.0),
                rng.gen_range(-5.0..5.0),
                1.0,
                1.0,
                rng.gen_range(-5.0..5.0),
                rng.gen_range(-5.0..5.0),
                true,
            )))],
        );
    }

    engine.world().push((instance_group, instance_mesh));
    engine.start(event_loop);
}
