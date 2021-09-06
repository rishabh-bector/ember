use legion::Resources;

pub mod camera;
pub mod metrics;
pub mod primitives;
pub mod registry;
pub mod schedule;
pub mod ui;

pub trait ResourceBuilder {
    fn build_to_resource(&self, resources: &mut Resources);
}

pub struct WindowSize {
    pub width: f32,
    pub height: f32,
}
