// extern crate pretty_env_logger;
// #[macro_use]
// extern crate log;
// #[macro_use]
// extern crate legion;

fn main() {
    std::env::set_var("RUST_LOG", "trace");

    let (engine, event_loop) = ember::engine().default().unwrap();

    engine.start(event_loop);
}
