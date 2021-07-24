use cgmath::Matrix2;
use legion::world::SubWorld;
use legion::IntoQuery;
use std::sync::{Arc, Mutex};

use crate::{
    component::Position2D,
    render::uniform::{GenericUniform, Uniform, UniformGroup},
};

pub const BASE_2D_COMMON_TEXTURE: &str = "test";
pub const BASE_2D_COMMON_VERTEX_BUFFER: usize = 0;
pub const BASE_2D_COMMON_INDEX_BUFFER: usize = 0;

#[derive(Clone, Debug, PartialEq)]
pub struct Base2D {
    pub name: String,

    pub color: [f32; 4],
    pub texture: String,
    pub mix: f32,

    pub width: f32,
    pub height: f32,
    pub common_vertex_buffer: usize,
    pub common_index_buffer: usize,
}

impl Base2D {
    pub fn _test(name: &str, width: f32, height: f32) -> Self {
        Base2D::solid_rect(name, width, height, [1.0, 1.0, 1.0, 1.0])
    }

    pub fn solid_rect(name: &str, width: f32, height: f32, color: [f32; 4]) -> Self {
        Base2D {
            name: name.to_owned(),
            color,
            mix: 1.0,
            width,
            height,
            texture: BASE_2D_COMMON_TEXTURE.to_string(),
            common_vertex_buffer: BASE_2D_COMMON_VERTEX_BUFFER,
            common_index_buffer: BASE_2D_COMMON_INDEX_BUFFER,
        }
    }
}

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Base2DUniforms {
    pub model: [f32; 4],
    pub color: [f32; 4],
    pub mix: f32,
    pub _padding: [f32; 32],
    pub __padding: [f32; 23],
}

pub struct Base2DUniformGroup {}

// TODO: Make this a macro?
#[system]
#[read_component(Base2D)]
#[read_component(Position2D)]
pub fn base_2d_uniform(
    world: &mut SubWorld,
    #[resource] base_uniforms: &Arc<Mutex<GenericUniform<Base2DUniforms>>>,
    #[resource] base_uniforms_group: &Arc<Mutex<UniformGroup<Base2DUniformGroup>>>,
) {
    let mut base_uniforms = base_uniforms.lock().unwrap();
    let mut base_uniforms_group = base_uniforms_group.lock().unwrap();

    let mut query = <(&Base2D, &Position2D)>::query();

    base_uniforms_group.begin_dynamic_loading(0);
    for (base_2d, pos) in query.iter_mut(world) {
        base_uniforms.mut_ref().model = [pos.x, pos.y, base_2d.width, base_2d.height];
        base_uniforms.mut_ref().color = base_2d.color;
        base_uniforms.mut_ref().mix = base_2d.mix;
        base_uniforms_group.load_dynamic_uniform(0, base_uniforms.as_bytes());
    }
}

pub fn _flatten(mat: Matrix2<f32>) -> [f32; 4] {
    [mat.x[0], mat.y[0], mat.x[1], mat.y[1]]
}
