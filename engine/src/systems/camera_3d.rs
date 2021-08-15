use cgmath::{Angle, EuclideanSpace, Matrix2, Rad};
use std::sync::{Arc, Mutex, RwLock};
use winit_input_helper::WinitInputHelper;

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
    #[resource] input: &Arc<RwLock<WinitInputHelper>>,
) {
    let mut camera = camera.lock().unwrap();
    let mut camera_uniforms = camera_uniform.lock().unwrap();
    let input = input.read().unwrap();

    // Mouse movement
    let (dx, dy) = input.mouse_diff();
    camera.yaw += dx * camera.sensitivity;
    camera.pitch -= dy * camera.sensitivity;
    if camera.pitch > 89.0 {
        camera.pitch = 89.0;
    } else if camera.pitch < -89.0 {
        camera.pitch = -89.0;
    }
    camera.dir.x = Angle::cos(Rad(camera.yaw)) * Angle::cos(Rad(camera.pitch));
    camera.dir.y = Angle::sin(Rad(camera.pitch));
    camera.dir.z = Angle::sin(Rad(camera.yaw)) * Angle::cos(Rad(camera.pitch));

    // WASD movement
    if input.key_held(winit::event::VirtualKeyCode::W) {
        let delta = (camera.dir * camera.speed).to_vec();
        camera.pos += delta;
    } else if input.key_held(winit::event::VirtualKeyCode::S) {
        let delta = -(camera.dir * camera.speed).to_vec();
        camera.pos += delta;
    }
    if input.key_held(winit::event::VirtualKeyCode::D) {
        let delta = camera.dir.to_vec().cross(camera.up) * camera.speed;
        camera.pos += delta;
    } else if input.key_held(winit::event::VirtualKeyCode::A) {
        let delta = -(camera.dir.to_vec().cross(camera.up) * camera.speed);
        camera.pos += delta;
    }

    // Scroll altitude
    camera.pos.y += input.scroll_diff() * camera.scroll_sensitivity;

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
