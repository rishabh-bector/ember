use std::time::{Duration, Instant};

use crate::renderer::{
    buffer::instance::InstanceMutator, systems::render_2d::forward_instance::Render2DInstance,
};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FrameMetrics {
    delta: Duration,
    start: Instant,
}

impl FrameMetrics {
    pub fn new() -> Self {
        Self {
            delta: Duration::from_secs(0),
            start: Instant::now(),
        }
    }

    pub fn delta(&self) -> Duration {
        self.delta
    }

    pub(crate) fn begin_frame(&mut self) {
        self.start = Instant::now();
    }

    pub(crate) fn end_frame(&mut self) {
        self.delta = self.start.elapsed();
    }
}

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

#[derive(Clone, Copy, Debug, PartialEq, Default)]
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

#[derive(Clone, Copy, Debug, PartialEq, Default)]
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
    fn mutate(&mut self, instance: &mut Render2DInstance, _delta: f32) {
        instance.model[0] = self.position[0];
        instance.model[1] = self.position[1];
        instance.model[2] = self.scale[0];
        instance.model[3] = self.scale[1];
    }
}

#[derive(Clone, Copy, Debug, Default)]
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
    fn mutate(&mut self, instance: &mut Render2DInstance, delta: f32) {
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
        self.transform.mutate(instance, delta);
    }
}

#[derive(Clone, Copy, Default, Debug)]
pub struct ParticleMotion2D {
    pub transform: Transform2D,
    pub velocity: Velocity2D,
    pub speed: [f32; 2],
}

impl ParticleMotion2D {
    pub fn new(x: f32, y: f32, w: f32, h: f32, vx: f32, vy: f32, bounce: bool) -> Self {
        Self {
            transform: Transform2D::new(x, y, w, h),
            velocity: Velocity2D { vx, vy, bounce },
            speed: [0.0, 0.0],
        }
    }
}

impl InstanceMutator<Render2DInstance> for ParticleMotion2D {
    fn mutate(&mut self, instance: &mut Render2DInstance, delta: f32) {
        let pos = self.transform.position;
        if self.velocity.bounce {
            if pos[0] <= -(1440 as f32) || pos[0] >= (1440 as f32) {
                self.velocity.vx *= -1.0;
            }
            if pos[1] <= -(900 as f32) || pos[1] >= (900 as f32) {
                self.velocity.vy *= -1.0;
            }
        }

        self.transform.position[0] += self.velocity.vx * self.speed[0];
        self.transform.position[1] += self.velocity.vy * self.speed[1];
        self.transform.mutate(instance, delta);
    }
}

#[derive(Clone, Copy, Debug)]
pub struct ParticleMutator2D {
    pub motion: ParticleMotion2D,
    pub lifetime: f32,
}

impl ParticleMutator2D {
    pub fn reset(&mut self) {
        self.motion = Default::default();
        self.lifetime = -1.0;
    }

    pub fn launch(&mut self, pos: [f32; 2], dir: [f32; 2], scale: [f32; 2], speed: [f32; 2]) {
        self.motion.transform.position = pos;
        self.motion.transform.scale = scale;
        self.motion.velocity.vx = dir[0] * speed[0];
        self.motion.velocity.vy = dir[1] * speed[1];
        self.lifetime = 0.0;
    }
}

impl InstanceMutator<Render2DInstance> for ParticleMutator2D {
    fn mutate(&mut self, instance: &mut Render2DInstance, delta: f32) {
        if self.lifetime >= 0.0 {
            self.lifetime += delta;
        }
        self.motion.mutate(instance, delta);
    }
}

impl Default for ParticleMutator2D {
    fn default() -> Self {
        Self {
            lifetime: -1.0,
            motion: Default::default(),
        }
    }
}
