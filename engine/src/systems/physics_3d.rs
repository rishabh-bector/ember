use legion::{world::SubWorld, IntoQuery};
use std::sync::{Arc, RwLock};

use crate::components::{DeltaTransform3D, FrameMetrics, Transform3D};

#[system]
#[read_component(DeltaTransform3D)]
#[write_component(Transform3D)]
pub fn physics_3d(world: &mut SubWorld, #[resource] frame_metrics: &Arc<RwLock<FrameMetrics>>) {
    let delta = frame_metrics.read().unwrap().delta().as_secs_f32();

    <(&mut Transform3D, &DeltaTransform3D)>::query().par_for_each_mut(
        world,
        |(transform, d_transform)| {
            transform.position[0] += d_transform.position[0] * delta;
            transform.position[1] += d_transform.position[1] * delta;
            transform.position[2] += d_transform.position[2] * delta;

            transform.rotation[0] += d_transform.rotation[0] * delta;
            transform.rotation[1] += d_transform.rotation[1] * delta;
            transform.rotation[2] += d_transform.rotation[2] * delta;

            transform.scale[0] += d_transform.scale[0] * delta;
            transform.scale[1] += d_transform.scale[1] * delta;
            transform.scale[2] += d_transform.scale[2] * delta;
        },
    );
}
