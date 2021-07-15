use std::{
    borrow::BorrowMut,
    sync::{Arc, Mutex},
};

use crate::{render::uniform::GenericUniform, resources::camera::Camera2D};

use cgmath::{Matrix2, SquareMatrix};

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
    #[resource] camera_uniforms: &Arc<Mutex<GenericUniform<Camera2DUniforms>>>,
) {
    let camera = camera.lock().unwrap();
    let mut uniforms = camera_uniforms.lock().unwrap();

    uniforms.source.view = [camera.pos.x, camera.pos.y, camera.size.x, camera.size.y];
}

pub fn flatten(mat: Matrix2<f32>) -> [f32; 4] {
    [mat.x[0], mat.y[0], mat.x[1], mat.y[1]]
}
