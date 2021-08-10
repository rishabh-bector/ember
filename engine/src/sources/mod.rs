use legion::Resources;

pub mod camera;
pub mod group;
pub mod metrics;
pub mod schedule;
pub mod store;
pub mod ui;

pub trait ResourceBuilder {
    fn build_to_resource(&self, resources: &mut Resources);
}
