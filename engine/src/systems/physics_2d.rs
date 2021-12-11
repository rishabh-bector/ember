

use crate::{
    components::{Position2D, Velocity2D},
};

#[system(for_each)]
pub fn physics_2d(pos: &mut Position2D, vel: &mut Velocity2D) {
    // Todo: replace hardcoding w/ some global config resource
    if vel.bounce {
        if pos.x <= -(1440 as f32) || pos.x >= (1440 as f32) {
            vel.vx *= -1.0;
        }
        if pos.y <= -(900 as f32) || pos.y >= (900 as f32) {
            vel.vy *= -1.0;
        }
    }

    pos.x += vel.vx;
    pos.y += vel.vy;
}
