use cgmath::{Angle, Deg, EuclideanSpace, Matrix2, Matrix3, Matrix4, SquareMatrix};
use std::sync::{Arc, Mutex, RwLock};
use winit_input_helper::WinitInputHelper;

use iced_winit::winit;

use crate::{
    constants::{CAMERA_3D_BIND_GROUP_ID, ID},
    renderer::uniform::{
        generic::{GenericUniform, GenericUniformBuilder},
        group::{UniformGroup, UniformGroupBuilder, UniformGroupType},
        Uniform,
    },
    sources::{camera::Camera3D, ui::iced::IcedWinitHelper},
};

pub struct Camera3DUniformGroup {}

impl UniformGroupType<Self> for Camera3DUniformGroup {
    fn builder() -> UniformGroupBuilder<Self> {
        UniformGroup::<Camera3DUniformGroup>::builder()
            .with_uniform(GenericUniformBuilder::from_source(Camera3DUniforms {
                view_pos: Default::default(),
                view_proj: Default::default(),
                inv_view_proj: Default::default(),
                clip: Default::default(),
            }))
            .with_id(ID(CAMERA_3D_BIND_GROUP_ID))
    }
}

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Camera3DUniforms {
    pub view_pos: [f32; 4],
    pub view_proj: [[f32; 4]; 4],
    pub inv_view_proj: [[f32; 4]; 4],
    pub clip: [f32; 2],
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

    info!("{}", input.mouse_held(1));

    if (camera.right_click_move && input.mouse_held(1)) || (!camera.right_click_move) {
        // Mouse movement

        let (dx, dy) = if camera.first {
            camera.first = false;
            input.mouse().unwrap_or_default()
        } else {
            input.mouse_diff()
        };

        camera.yaw += dx * camera.sensitivity;
        camera.pitch -= dy * camera.sensitivity;
        if camera.pitch > 89.0 {
            camera.pitch = 89.0;
        } else if camera.pitch < -89.9 {
            camera.pitch = -89.9;
        }

        camera.dir.x = Angle::cos(Deg(camera.yaw)) * Angle::cos(Deg(camera.pitch));
        camera.dir.y = Angle::sin(Deg(camera.pitch));
        camera.dir.z = Angle::sin(Deg(camera.yaw)) * Angle::cos(Deg(camera.pitch));

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
    }

    // Camera matrices
    let view_proj = camera.build_view_proj();
    let inv_view_proj = view_proj.invert().unwrap();

    camera_uniforms.mut_ref().view_pos = [camera.pos.x, camera.pos.y, camera.pos.z, 0.0];
    camera_uniforms.mut_ref().view_proj = matrix2array_4d(view_proj);
    camera_uniforms.mut_ref().inv_view_proj = matrix2array_4d(inv_view_proj);
    camera_uniforms.mut_ref().clip = [0.01, 10000.0];
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

pub fn matrix2array_2d(mat: Matrix2<f32>) -> [f32; 4] {
    [mat.x[0], mat.y[0], mat.x[1], mat.y[1]]
}

pub fn matrix2array_3d(mat: Matrix3<f32>) -> [[f32; 3]; 3] {
    [
        [mat.x.x, mat.x.y, mat.x.z],
        [mat.y.x, mat.y.y, mat.y.z],
        [mat.z.x, mat.z.y, mat.z.z],
    ]
}

pub fn matrix2array_4d(mat: Matrix4<f32>) -> [[f32; 4]; 4] {
    [
        [mat.x.x, mat.x.y, mat.x.z, mat.x.w],
        [mat.y.x, mat.y.y, mat.y.z, mat.y.w],
        [mat.z.x, mat.z.y, mat.z.z, mat.z.w],
        [mat.w.x, mat.w.y, mat.w.z, mat.w.w],
    ]
}
