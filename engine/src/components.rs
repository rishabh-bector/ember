use uuid::Uuid;

use crate::{
    constants::ID,
    renderer::{
        buffer::instance::InstanceMutator, systems::render_2d::forward_instance::Render2DInstance,
    },
};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Position2D {
    pub x: f32,
    pub y: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Transform2D {
    pub position: [f32; 2],
    pub scale: [f32; 2],
    pub angle: f32,
}

impl Transform2D {
    pub fn new(x: f32, y: f32, w: f32, h: f32) -> Self {
        Self {
            position: [x, y],
            scale: [w, h],
            angle: 0.0,
        }
    }
}

impl Default for Transform2D {
    fn default() -> Self {
        Self {
            position: [0.0, 0.0],
            scale: [1.0, 1.0],
            angle: 0.0,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Velocity2D {
    pub vx: f32,
    pub vy: f32,
    pub bounce: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Sprite {
    pub width: usize,
    pub height: usize,
    pub pixels: Vec<u8>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Position3D {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Velocity3D {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Scale3D {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

pub struct Rotor3D {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

// --------------------------------------------------
// Instance Mutators
// --------------------------------------------------

impl InstanceMutator<Render2DInstance> for Transform2D {
    fn mutate(&mut self, instance: &mut Render2DInstance) {
        instance.model[0] = self.position[0];
        instance.model[1] = self.position[1];
        instance.model[2] = self.scale[0];
        instance.model[3] = self.scale[1];
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Motion2D {
    pub transform: Transform2D,
    pub velocity: Velocity2D,
}

impl Motion2D {
    pub fn new(x: f32, y: f32, w: f32, h: f32, vx: f32, vy: f32, bounce: bool) -> Self {
        Self {
            transform: Transform2D::new(x, y, w, h),
            velocity: Velocity2D { vx, vy, bounce },
        }
    }
}

impl InstanceMutator<Render2DInstance> for Motion2D {
    fn mutate(&mut self, instance: &mut Render2DInstance) {
        let pos = self.transform.position;
        if self.velocity.bounce {
            if pos[0] <= -(1440 as f32) || pos[0] >= (1440 as f32) {
                self.velocity.vx *= -1.0;
            }
            if pos[1] <= -(900 as f32) || pos[1] >= (900 as f32) {
                self.velocity.vy *= -1.0;
            }
        }

        self.transform.position[0] += self.velocity.vx;
        self.transform.position[1] += self.velocity.vy;
        self.transform.mutate(instance);
    }
}
