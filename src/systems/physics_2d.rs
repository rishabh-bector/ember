use crate::component::{Position2D, Velocity2D};

#[system(for_each)]
pub fn physics_2d(pos: &mut Position2D, vel: &mut Velocity2D) {
    pos.x += vel.dx;
    pos.y += vel.dy;
}
