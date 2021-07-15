use std::{
    borrow::BorrowMut,
    sync::{Arc, Mutex},
};

use crate::{
    render::uniform::UniformBuffer,
    resources::camera::{Camera2D, Camera2DUniforms},
};

use cgmath::{Matrix2, SquareMatrix};

#[system]
pub fn camera_2d(
    #[resource] camera: &Arc<Mutex<Camera2D>>,
    #[resource] camera_uniforms: &Arc<Mutex<UniformBuffer<Camera2DUniforms>>>,
) {
    let camera = camera.lock().unwrap();
    let mut uniforms = camera_uniforms.lock().unwrap();

    let view_matrix = Matrix2::<f32>::from_angle(cgmath::Rad(0.0))
        * Matrix2::from_cols((camera.size.x, 0.0).into(), (0.0, camera.size.y).into());

    //uniforms.source.view = Matrix2::<f32>::from_value(1.0).into(); //Matrix2::identity().into();
}

fn rotate_2x2(mut mat: Matrix2<f32>) -> Matrix2<f32> {
    mat.y = (mat.y.y, mat.y.x).into();
    mat
}
