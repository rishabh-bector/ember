use cgmath::Matrix2;
use std::sync::{Arc, Mutex};

use crate::{
    render::uniform::{generic::GenericUniform, group::UniformGroup, Uniform},
    resource::camera::Camera2D,
};

pub struct Camera2DUniformGroup {}

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Camera2DUniforms {
    pub view: [f32; 4],
    pub _padding: [f32; 32],
    pub __padding: [f32; 28],
}

#[system]
pub fn camera_2d(
    #[resource] camera: &Arc<Mutex<Camera2D>>,
    #[resource] camera_uniform: &Arc<Mutex<GenericUniform<Camera2DUniforms>>>,
) {
    let camera = camera.lock().unwrap();
    let mut camera_uniform = camera_uniform.lock().unwrap();

    camera_uniform.mut_ref().view = [camera.pos.x, camera.pos.y, camera.size.x, camera.size.y];
}

// TODO: Make this a macro?
#[system]
pub fn camera_2d_uniform(
    #[resource] camera_uniform: &Arc<Mutex<GenericUniform<Camera2DUniforms>>>,
    #[resource] camera_uniform_group: &Arc<Mutex<UniformGroup<Camera2DUniformGroup>>>,
) {
    camera_uniform_group
        .lock()
        .unwrap()
        .load_buffer(0, camera_uniform.lock().unwrap().as_bytes());
}

pub fn _flatten(mat: Matrix2<f32>) -> [f32; 4] {
    [mat.x[0], mat.y[0], mat.x[1], mat.y[1]]
}
