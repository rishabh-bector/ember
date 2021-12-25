// TEST EXAMPLE: render graph and channel nodes

fn main() {
    std::env::set_var("RUST_LOG", "debug");
    let (engine, event_loop) = ember::engine_builder().test_automata_node().unwrap();
    engine.start(event_loop);
}
