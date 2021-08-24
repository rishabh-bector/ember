use legion::world::SubWorld;
use std::sync::Arc;
use uuid::Uuid;

use crate::renderer::{
    buffer::instance::InstanceGroup, systems::render_2d::forward_instance::Render2DInstance,
};

pub struct ParticleSystem2D {
    pub id: Uuid,
    emitters: Vec<Arc<ParticleEmitter2D>>,

    pub lifetime: f32,
    pub size: (f32, f32),
    pub speed: (f32, f32),
}

impl ParticleSystem2D {
    pub fn new(lifetime: f32, size: (f32, f32), speed: (f32, f32)) -> Self {
        Self {
            id: Uuid::new_v4(),
            emitters: vec![],
            lifetime,
            size,
            speed,
        }
    }

    pub fn push(&mut self, emitter: Arc<ParticleEmitter2D>) {
        self.emitters.push(emitter)
    }
}

pub enum EmitterShape {
    Line,
    Arc {},
}

pub enum EmitterMode {
    Random,
    Forward,
    Backward,
    Parallel,
}

pub struct ParticleEmitter2D {
    pub system: Uuid,
    pub shape: EmitterShape,
    pub mode: EmitterMode,
    pub launch_freq: f32,
}

#[system]
#[write_component(ParticleSystem2D)]
#[write_component(InstanceGroup<Render2DInstance>)]
pub fn particle_2d_emission(world: &mut SubWorld) {}
