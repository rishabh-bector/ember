use ember::renderer::graph::node::ShaderSource;

// Ember example: pure shader

fn main() {
    std::env::set_var("RUST_LOG", "debug");
    let (engine, event_loop) = ember::engine_builder()
        .default_quad(ShaderSource::WGSL(include_str!("./shader.wgsl").to_owned()))
        .unwrap();

    engine.start(event_loop);
}
