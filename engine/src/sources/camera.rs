use cgmath::EuclideanSpace;

use crate::constants::OPENGL_TO_WGPU_MATRIX;

pub struct Camera3D {
    pub speed: f32,
    pub sensitivity: f32,
    pub scroll_sensitivity: f32,

    // State
    pub pos: cgmath::Point3<f32>,
    pub dir: cgmath::Point3<f32>,

    pub pitch: f32,
    pub yaw: f32,

    pub up: cgmath::Vector3<f32>,
    pub aspect: f32,
    pub fov: f32,

    pub z_near: f32,
    pub z_far: f32,

    pub first: bool,
}

impl Camera3D {
    pub fn default(screen_width: f32, screen_height: f32) -> Self {
        Self {
            speed: 0.3,
            sensitivity: 0.0075,
            scroll_sensitivity: 0.5,
            pos: (0.0, 1.0, 2.0).into(),
            dir: (0.0, 0.0, -1.0).into(),
            pitch: 0.0,
            yaw: -90.0,
            up: cgmath::Vector3::unit_y(),
            aspect: screen_width / screen_height,
            fov: 45.0,
            z_near: 0.1,
            z_far: 100.0,
            first: true,
        }
    }

    pub fn build_view_proj(&self) -> cgmath::Matrix4<f32> {
        let view = cgmath::Matrix4::look_at_rh(self.pos, self.pos + self.dir.to_vec(), self.up);
        let proj = cgmath::perspective(cgmath::Deg(self.fov), self.aspect, self.z_near, self.z_far);
        return OPENGL_TO_WGPU_MATRIX * proj * view;
    }
}

pub struct Camera2D {
    pub pos: cgmath::Point2<f32>,
    pub size: cgmath::Point2<f32>,
    pub zoom: f32,
}

impl Camera2D {
    pub fn default(screen_width: f32, screen_height: f32) -> Self {
        Self {
            pos: (0.0, 0.0).into(),
            size: (screen_width, screen_height).into(),
            zoom: 1.0,
        }
    }
}
