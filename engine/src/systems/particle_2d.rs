use cgmath::{Angle, InnerSpace};
use legion::{world::SubWorld, IntoQuery, World};
use rand::Rng;
use rayon::iter::{
    IndexedParallelIterator, IntoParallelIterator, IntoParallelRefMutIterator, ParallelBridge,
    ParallelIterator,
};
use std::sync::{Arc, Mutex, RwLock};
use uuid::Uuid;

use crate::{
    components::{FrameMetrics, Motion2D, ParticleMutator2D},
    renderer::{
        buffer::instance::InstanceGroup, systems::render_2d::forward_instance::Render2DInstance,
    },
};

pub struct ParticleSystem2D {
    pub id: Uuid,
    pub num_particles: u32,
    mutators: Vec<Arc<Mutex<ParticleMutator2D>>>,
    emitters: Vec<Arc<Mutex<ParticleEmitter2D>>>,

    pub lifetime: f32,
    pub scale: [[f32; 2]; 2],
    pub speed: [f32; 2],
    pub color: [[f32; 4]; 2],
}

impl Default for ParticleSystem2D {
    fn default() -> Self {
        Self::new(
            5.0,
            [2.0, 2.0],
            [[2.0, 2.0], [2.0, 2.0]],
            [[1.0, 1.0, 1.0, 1.0], [0.3, 0.3, 0.5, 0.5]],
            5000,
        )
    }
}

impl ParticleSystem2D {
    pub fn new(
        lifetime: f32,
        speed: [f32; 2],
        scale: [[f32; 2]; 2],
        color: [[f32; 4]; 2],
        num_particles: u32,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            mutators: vec![],
            emitters: vec![],
            num_particles,
            lifetime,
            speed,
            scale,
            color,
        }
    }

    pub fn from_emitters(
        lifetime: f32,
        speed: [f32; 2],
        scale: [[f32; 2]; 2],
        color: [[f32; 4]; 2],
        num_particles: u32,
        emitters: Vec<ParticleEmitter2D>,
    ) -> Self {
        Self {
            emitters: emitters.into_iter().map(Mutex::new).map(Arc::new).collect(),
            mutators: vec![],
            id: Uuid::new_v4(),
            num_particles,
            lifetime,
            speed,
            scale,
            color,
        }
    }

    pub fn push(&mut self, emitter: ParticleEmitter2D) {
        self.emitters.push(Arc::new(Mutex::new(emitter)));
    }
}

pub fn init_particle_systems(world: &mut World) {
    <(&mut ParticleSystem2D, &mut InstanceGroup<Render2DInstance>)>::query().par_for_each_mut(
        world,
        |(system, group)| {
            for _ in 0..system.num_particles {
                let mutator = Arc::new(Mutex::new(ParticleMutator2D::default()));
                system.mutators.push(Arc::clone(&mutator));
                group.push(Render2DInstance::new([0.0, 0.0, 0.0, 0.0]), vec![mutator]);
            }
        },
    );
}

pub enum EmitterShape {
    Line { end: [f32; 2] },
    Arc { radius: [f32; 2], angle: f32 },
}

impl Shape2D for EmitterShape {
    fn parametric(&self, t: f32, pos: [f32; 2]) -> [[f32; 2]; 2] {
        match &self {
            EmitterShape::Line { end } => {
                let dx = end[0] - pos[0];
                let dy = end[1] - pos[1];
                [[pos[0] + t * dx, pos[1] + t * dy], [-dy, dx]]
            }
            EmitterShape::Arc { radius, angle } => {
                let cos = Angle::cos(cgmath::Deg(t * angle));
                let sin = Angle::sin(cgmath::Deg(t * angle));
                let dir = cgmath::vec2::<f32>(cos, sin).normalize();
                [
                    [pos[0] + cos * radius[0], pos[1] + sin * radius[1]],
                    [dir.x, dir.y],
                ]
            }
        }
    }
}

pub trait Shape2D {
    fn parametric(&self, t: f32, pos: [f32; 2]) -> [[f32; 2]; 2];
}

pub enum EmitterMode {
    Random,
    Direction { next: u32, reverse: bool },
}

impl EmitterMode {
    pub fn emit(&mut self, shape: &EmitterShape, pos: [f32; 2], zones: u32) -> [[f32; 2]; 2] {
        match self {
            EmitterMode::Random => {
                if zones > 0 {
                    shape.parametric(
                        ((rand::thread_rng().gen::<f32>() * (zones as f32)) as u32) as f32
                            / (zones as f32),
                        pos,
                    )
                } else {
                    shape.parametric(rand::thread_rng().gen(), pos)
                }
            }
            EmitterMode::Direction { next, reverse } => {
                let out = shape.parametric((*next / zones) as f32, pos);
                if *reverse {
                    if *next == 0 {
                        *next = zones;
                    } else {
                        *next = (*next as i32 - 1) as u32;
                    }
                } else {
                    if *next >= zones {
                        *next = 0;
                    } else {
                        *next = *next + 1;
                    }
                }

                out
            }
        }
    }
}

pub struct ParticleEmitter2D {
    pub position: [f32; 2],
    pub shape: EmitterShape,
    pub mode: EmitterMode,
    pub zones: u32,
    pub batches: u32,
    pub launch_freq: f32,
}

impl ParticleEmitter2D {
    pub fn emit(&mut self, delta: f32) -> Vec<[[f32; 2]; 2]> {
        vec![
            self.mode.emit(&self.shape, self.position, self.zones),
            self.mode.emit(&self.shape, self.position, self.zones),
            self.mode.emit(&self.shape, self.position, self.zones),
            self.mode.emit(&self.shape, self.position, self.zones),
            self.mode.emit(&self.shape, self.position, self.zones),
        ]
    }
}

impl Default for ParticleEmitter2D {
    fn default() -> Self {
        Self {
            position: [0.0, 0.0],
            shape: EmitterShape::Arc {
                radius: [50.0, 50.0],
                angle: 360.0,
            },
            zones: 0,
            batches: 0,
            mode: EmitterMode::Random,
            launch_freq: 10.0,
        }
    }
}

#[system]
#[write_component(ParticleSystem2D)]
#[write_component(InstanceGroup<Render2DInstance>)]
pub fn particle_2d_emission(
    world: &mut SubWorld,
    #[resource] frame_metrics: &Arc<RwLock<FrameMetrics>>,
) {
    let delta = frame_metrics.read().unwrap().delta().as_secs_f32();
    <(&mut ParticleSystem2D, &mut InstanceGroup<Render2DInstance>)>::query().par_for_each_mut(
        world,
        |(system, group)| {
            let emitted: Arc<Mutex<Vec<[[f32; 2]; 2]>>> = Arc::new(Mutex::new(
                system
                    .emitters
                    .iter()
                    .map(|emitter| emitter.lock().unwrap().emit(delta))
                    .flatten()
                    .collect(),
            ));
            let launch_speed = system.speed[0];
            let launch_scale = system.scale[0];
            let launch_color = system.color[0];
            let launch_lifetime = system.lifetime;
            group
                .instances
                .par_iter_mut()
                .enumerate()
                .for_each(|(i, instance)| {
                    let mut mutator = system.mutators[i].lock().unwrap();
                    if mutator.lifetime > 0.0 {
                        // Interpolations
                    } else {
                        if mutator.lifetime < 0.0 {
                            mutator.reset();
                        }
                        if mutator.lifetime == 0.0 {
                            let mut emitted = emitted.lock().unwrap();
                            let range = emitted.len().saturating_sub(1)..;
                            let next = emitted.drain(range).next_back();
                            drop(emitted);
                            if let Some(pos_dir) = next {
                                mutator.launch(
                                    pos_dir[0],
                                    pos_dir[1],
                                    launch_scale,
                                    launch_speed,
                                    launch_lifetime,
                                );
                                instance.color = launch_color;
                            }
                        }
                    }
                });

            // group
            //     .instances
            //     .iter_mut()
            //     .zip(Arc::clone(&group.components).write().unwrap().iter())
            //     .par_bridge()
            //     .into_par_iter()
            //     .for_each(|(instance, components)| {});
        },
    );
}

// Emission algorithm (par_iter through particle systems):
// Needs to:
//      - Recycle old particles
//      - Launch new particles
// Individual particles should be updated via an Instance Mutator which wraps Motion2D.
