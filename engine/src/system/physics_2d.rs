use std::sync::{Arc, Mutex};

use crate::{
    components::{Position2D, Velocity2D},
    render::GpuState,
};

#[system(for_each)]
pub fn physics_2d(
    pos: &mut Position2D,
    vel: &mut Velocity2D,
    #[resource] gpu: &Arc<Mutex<GpuState>>,
) {
    let (_width, _height) = gpu.lock().unwrap().screen_size;

    // Todo: replace hardcoding w/ some global config resource
    if vel.bounce {
        if pos.x <= -(1440 as f32) || pos.x >= (1440 as f32) {
            vel.dx *= -1.0;
        }
        if pos.y <= -(900 as f32) || pos.y >= (900 as f32) {
            vel.dy *= -1.0;
        }
    }

    pos.x += vel.dx;
    pos.y += vel.dy;
}
