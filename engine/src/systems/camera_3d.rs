use cgmath::Matrix2;
use std::sync::{Arc, Mutex};

use crate::{
    renderer::uniform::{generic::GenericUniform, group::UniformGroup, Uniform},
    sources::camera::{Camera2D, Camera3D},
};

pub struct Camera3DUniformGroup {}

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Camera3DUniforms {
    pub view_proj: [[f32; 4]; 4],
}

#[system]
pub fn camera_3d(
    #[resource] camera: &Arc<Mutex<Camera3D>>,
    #[resource] camera_uniform: &Arc<Mutex<GenericUniform<Camera3DUniforms>>>,
) {
    let camera = camera.lock().unwrap();
    let mut camera_uniforms = camera_uniform.lock().unwrap();

    let mat = camera.build_view_proj();
    camera_uniforms.mut_ref().view_proj = [
        [mat.x.x, mat.x.y, mat.x.z, mat.x.w],
        [mat.y.x, mat.y.y, mat.y.z, mat.y.w],
        [mat.z.x, mat.z.y, mat.z.z, mat.z.w],
        [mat.w.x, mat.w.y, mat.w.z, mat.w.w],
    ];
}

// TODO: Make this a macro?
#[system]
pub fn camera_3d_uniform(
    #[resource] queue: &Arc<wgpu::Queue>,
    #[resource] camera_uniform: &Arc<Mutex<GenericUniform<Camera3DUniforms>>>,
    #[resource] camera_uniform_group: &Arc<Mutex<UniformGroup<Camera3DUniformGroup>>>,
) {
    camera_uniform.lock().unwrap().write_buffer(
        &queue,
        camera_uniform_group.lock().unwrap().default_buffer(0),
    );
}

pub fn _flatten(mat: Matrix2<f32>) -> [f32; 4] {
    [mat.x[0], mat.y[0], mat.x[1], mat.y[1]]
}
