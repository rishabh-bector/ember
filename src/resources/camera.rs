use crate::render::uniform::UniformBuffer;
use cgmath::SquareMatrix;

pub struct Camera3D {
    pub eye: cgmath::Point3<f32>,
    pub target: cgmath::Point3<f32>,
    pub up: cgmath::Vector3<f32>,
    pub aspect: f32,
    pub fov: f32,
    pub z_near: f32,
    pub z_far: f32,
}

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Camera3DUniforms {
    pub view_proj: [[f32; 4]; 4],
}

impl Camera3D {
    pub fn default(screen_width: f32, screen_height: f32) -> Self {
        Self {
            eye: (0.0, 1.0, 2.0).into(),
            target: (0.0, 0.0, 0.0).into(),
            up: cgmath::Vector3::unit_y(),
            aspect: screen_width / screen_height,
            fov: 45.0,
            z_near: 0.1,
            z_far: 100.0,
        }
    }

    pub fn build_matrices(&self) -> cgmath::Matrix4<f32> {
        let view = cgmath::Matrix4::look_at_rh(self.eye, self.target, self.up);
        let proj = cgmath::perspective(cgmath::Deg(self.fov), self.aspect, self.z_near, self.z_far);
        return OPENGL_TO_WGPU_MATRIX * proj * view;
    }
}

impl From<&Camera3D> for Camera3DUniforms {
    fn from(camera_3d: &Camera3D) -> Self {
        Self {
            view_proj: camera_3d.build_matrices().into(),
        }
    }
}

#[rustfmt::skip]
pub const OPENGL_TO_WGPU_MATRIX: cgmath::Matrix4<f32> = cgmath::Matrix4::new(
    1.0, 0.0, 0.0, 0.0,
    0.0, 1.0, 0.0, 0.0,
    0.0, 0.0, 0.5, 0.0,
    0.0, 0.0, 0.5, 1.0,
);

pub struct Camera2D {
    pub pos: cgmath::Point2<f32>,
    pub size: cgmath::Point2<f32>,
    pub zoom: f32,
}

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Camera2DUniforms {
    pub view: [f32; 4],
}

impl Camera2D {
    pub fn default(screen_width: f32, screen_height: f32) -> Self {
        Self {
            pos: (screen_width / 2.0, screen_height / 2.0).into(),
            size: (screen_width, screen_height).into(),
            zoom: 1.0,
        }
    }
}
