use std::sync::{Arc, Mutex};

use legion::{world::SubWorld, IntoQuery};

use crate::{component::Position2D, render::uniform::GenericUniform};

pub struct Lighting2DUniformGroup {}

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Lighting2DUniforms {
    pub light_0: [f32; 4],
    pub light_1: [f32; 4],
    pub light_2: [f32; 4],
    pub light_3: [f32; 4],
    pub light_4: [f32; 4],
}

#[derive(Clone, Debug, PartialEq)]
pub struct Light2D {
    pub linear: f32,
    pub quadratic: f32,
}

#[system]
#[read_component(Light2D)]
#[read_component(Position2D)]
pub fn lighting_2d(
    world: &mut SubWorld,
    #[resource] lighting_2d_uniforms: &Arc<Mutex<GenericUniform<Lighting2DUniforms>>>,
) {
    let mut forms = lighting_2d_uniforms.lock().unwrap();
    let mut query = <(&Light2D, &Position2D)>::query();
    let mut i = 0;
    for (light, pos) in query.iter_mut(world) {
        let flat = [pos.x, pos.y, light.linear, light.quadratic];
        match i {
            0 => forms.source.light_0 = flat,
            1 => forms.source.light_1 = flat,
            2 => forms.source.light_2 = flat,
            3 => forms.source.light_3 = flat,
            4 => forms.source.light_4 = flat,
            _ => {}
        }
        i += 1;
        if i == 5 {
            break;
        }
    }
}
