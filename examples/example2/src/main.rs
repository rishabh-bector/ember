use ember::{
    constants::{ID, PRIMITIVE_MESH_GROUP_ID, UNIT_SQUARE_MESH_ID},
    renderer::systems::render_2d::forward_instance::Render2DInstance,
    systems::particle_2d::{EmitterShape, ParticleEmitter2D, ParticleSystem2D},
};

// Ember example: 2D instance group

fn main() {
    std::env::set_var("RUST_LOG", "ember=info");
    let (mut engine, event_loop) = ember::engine_builder().default_2d().unwrap();

    let particle_group = Render2DInstance::new_default_group();
    let particle_mesh = engine.clone_mesh(&ID(UNIT_SQUARE_MESH_ID), &ID(PRIMITIVE_MESH_GROUP_ID));

    let mut particle_system = ParticleSystem2D::default();
    particle_system.push(ParticleEmitter2D::default());

    // let mut p2 = ParticleEmitter2D::default();
    // p2.position = [500.0, 500.0];
    // p2.shape = EmitterShape::Line {
    //     end: [600.0, 500.0],
    //     reverse: true,
    // };
    // particle_system.push(p2);

    engine
        .world()
        .push((particle_system, particle_mesh, particle_group));
    engine.start(event_loop);
}
