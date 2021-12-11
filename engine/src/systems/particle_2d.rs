use cgmath::{Angle, InnerSpace};
use legion::{world::SubWorld, IntoQuery, World};
use rand::Rng;
use rayon::iter::{IndexedParallelIterator, IntoParallelRefMutIterator, ParallelIterator};
use std::{
    ops::{Add, Mul, Sub},
    sync::{Arc, Mutex, RwLock},
};
use uuid::Uuid;

use crate::{
    components::{FrameMetrics, ParticleMutator2D},
    renderer::{
        buffer::instance::InstanceGroup, systems::render_2d::forward_instance::Render2DInstance,
    },
};

pub struct ParticleSystem2D {
    mutators: Vec<Arc<Mutex<ParticleMutator2D>>>,
    pub id: Uuid,
    pub num_particles: u32,

    pub emitters: Vec<Arc<Mutex<ParticleEmitter2D>>>,

    pub lifetime: f32,
    pub scale: Interpolator<SmoothF32x2>,
    pub speed: Interpolator<SmoothF32x2>,
    pub color: Interpolator<SmoothF32x4>,
}

impl Default for ParticleSystem2D {
    fn default() -> Self {
        Self::new_empty(
            3.0,
            Interpolator::<SmoothF32x2>::new([3.0, 3.0], [0.0, 0.0]),
            Interpolator::<SmoothF32x2>::new([3.0, 3.0], [-2.0, -2.0]),
            Interpolator::<SmoothF32x4>::new([1.0, 1.0, 0.0, 1.0], [1.0, 0.0, 1.0, 1.0]),
            2000,
        )
    }
}

impl ParticleSystem2D {
    pub fn new(
        lifetime: f32,
        speed: Interpolator<SmoothF32x2>,
        scale: Interpolator<SmoothF32x2>,
        color: Interpolator<SmoothF32x4>,
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

    pub fn new_empty(
        lifetime: f32,
        speed: Interpolator<SmoothF32x2>,
        scale: Interpolator<SmoothF32x2>,
        color: Interpolator<SmoothF32x4>,
        num_particles: u32,
    ) -> Self {
        Self::new(lifetime, speed, scale, color, num_particles, vec![])
    }

    pub fn from_emitters(
        lifetime: f32,
        speed: Interpolator<SmoothF32x2>,
        scale: Interpolator<SmoothF32x2>,
        color: Interpolator<SmoothF32x4>,
        num_particles: u32,
        emitters: Vec<ParticleEmitter2D>,
    ) -> Self {
        Self::new(lifetime, speed, scale, color, num_particles, emitters)
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
    Line { end: [f32; 2], reverse: bool },
    Arc { radius: [f32; 2], angle: f32 },
}

impl Shape2D for EmitterShape {
    fn parametric(&self, t: f32, pos: [f32; 2]) -> [[f32; 2]; 2] {
        match &self {
            EmitterShape::Line { end, reverse } => {
                let dx = end[0] - pos[0];
                let dy = end[1] - pos[1];
                let dir = if *reverse {
                    cgmath::vec2::<f32>(dy, -dx)
                } else {
                    cgmath::vec2::<f32>(-dy, dx)
                }
                .normalize();
                [[pos[0] + t * dx, pos[1] + t * dy], [dir.x, dir.y]]
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
                let out = shape.parametric(*next as f32 / zones as f32, pos);
                if *reverse {
                    if *next == 0 {
                        *next = zones;
                    } else {
                        *next = (*next as i32 - 1) as u32;
                    }
                } else {
                    if *next == zones - 1 {
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
    pub rate: u32,
    pub launch_freq: f32,
}

impl ParticleEmitter2D {
    pub fn emit(&mut self, _delta: f32) -> Vec<[[f32; 2]; 2]> {
        (0..self.rate)
            .into_iter()
            .map(|_| self.mode.emit(&self.shape, self.position, self.zones))
            .collect()
    }
}

impl Default for ParticleEmitter2D {
    fn default() -> Self {
        Self {
            position: [0.0, 0.0],
            shape: EmitterShape::Arc {
                radius: [0.0, 0.0],
                angle: 360.0,
            },
            zones: 0,
            rate: 10,
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

            let launch_speed = system.speed.initial().0;
            let launch_scale = system.scale.initial().0;
            let launch_color = system.color.initial().0;

            // - update active particles
            // - deactivate expired particles
            // - recycle deactivated particles
            group
                .instances
                .par_iter_mut()
                .enumerate()
                .for_each(|(i, instance)| {
                    let mut mutator = system.mutators[i].lock().unwrap();
                    // mutate active particles
                    if mutator.lifetime >= 0.0 && mutator.lifetime <= system.lifetime {
                        let t = mutator.lifetime / system.lifetime;
                        instance.color = system.color.linear(t).0;
                        mutator.motion.transform.scale = system.scale.linear(t).0;
                        mutator.motion.speed = system.speed.linear(t).0;
                    // recycle expired particles
                    } else {
                        if mutator.lifetime > system.lifetime {
                            mutator.reset();
                        }
                        if mutator.lifetime == -1.0 {
                            let mut emitted = emitted.lock().unwrap();
                            let range = emitted.len().saturating_sub(1)..;
                            let next = emitted.drain(range).next_back();
                            drop(emitted);
                            if let Some(pos_dir) = next {
                                mutator.launch(pos_dir[0], pos_dir[1], launch_scale, launch_speed);
                                instance.color = launch_color;
                            }
                        }
                    }
                });
        },
    );
}

pub trait Quantity:
    Clone + Copy + Add<Self, Output = Self> + Sub<Self, Output = Self> + Mul<f32, Output = Self> + Sized
{
}

pub struct Interpolator<T: Quantity> {
    from: T,
    to: T,
    delta: T,
}

impl<T> Interpolator<T>
where
    T: Quantity,
{
    pub fn new<I: Into<T>>(from: I, to: I) -> Self {
        let from: T = from.into();
        let to: T = to.into();
        Self {
            delta: to - from,
            from,
            to,
        }
    }

    pub fn linear(&self, param: f32) -> T {
        self.from + self.delta * param
    }

    pub fn initial(&self) -> T {
        self.from
    }

    pub fn target(&self) -> T {
        self.to
    }
}

#[derive(Clone, Copy, Add, Sub, Mul)]
pub struct SmoothF32(f32);
impl Quantity for SmoothF32 {}

#[derive(Clone, Copy, From)]
pub struct SmoothF32x2([f32; 2]);
impl Add for SmoothF32x2 {
    type Output = Self;
    fn add(self, rhs: Self) -> Self::Output {
        SmoothF32x2([self.0[0] + rhs.0[0], self.0[1] + rhs.0[1]])
    }
}
impl Sub for SmoothF32x2 {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self::Output {
        SmoothF32x2([self.0[0] - rhs.0[0], self.0[1] - rhs.0[1]])
    }
}
impl Mul<f32> for SmoothF32x2 {
    type Output = Self;
    fn mul(self, rhs: f32) -> Self::Output {
        SmoothF32x2([self.0[0] * rhs, self.0[1] * rhs])
    }
}
impl Quantity for SmoothF32x2 {}

#[derive(Clone, Copy, From)]
pub struct SmoothF32x4([f32; 4]);
impl Add for SmoothF32x4 {
    type Output = Self;
    fn add(self, rhs: Self) -> Self::Output {
        SmoothF32x4([
            self.0[0] + rhs.0[0],
            self.0[1] + rhs.0[1],
            self.0[2] + rhs.0[2],
            self.0[3] + rhs.0[3],
        ])
    }
}
impl Sub for SmoothF32x4 {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self::Output {
        SmoothF32x4([
            self.0[0] - rhs.0[0],
            self.0[1] - rhs.0[1],
            self.0[2] - rhs.0[2],
            self.0[3] - rhs.0[3],
        ])
    }
}
impl Mul<f32> for SmoothF32x4 {
    type Output = Self;
    fn mul(self, rhs: f32) -> Self::Output {
        SmoothF32x4([
            self.0[0] * rhs,
            self.0[1] * rhs,
            self.0[2] * rhs,
            self.0[3] * rhs,
        ])
    }
}
impl Quantity for SmoothF32x4 {}
