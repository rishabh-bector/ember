pub const DEFAULT_SCREEN_WIDTH: usize = 1440;
pub const DEFAULT_SCREEN_HEIGHT: usize = 900;

pub const DEFAULT_MAX_DYNAMIC_ENTITIES_PER_PASS: u32 = 64;
pub const DEFAULT_DYNAMIC_BUFFER_MIN_BINDING_SIZE: u64 = 128;

pub const BASE_2D_COMMON_TEXTURE_ID: &str = "8a22d465-7935-41e5-9e90-686ef5632c54";
pub const BASE_2D_COMMON_VERTEX_BUFFER: usize = 0;
pub const BASE_2D_COMMON_INDEX_BUFFER: usize = 0;

pub const CAMERA_2D_BIND_GROUP_ID: &str = "2fc8e285-38ca-45e2-a910-00f49a7455d1";

#[rustfmt::skip]
pub const OPENGL_TO_WGPU_MATRIX: cgmath::Matrix4<f32> = cgmath::Matrix4::new(
    1.0, 0.0, 0.0, 0.0,
    0.0, 1.0, 0.0, 0.0,
    0.0, 0.0, 0.5, 0.0,
    0.0, 0.0, 0.5, 1.0,
);
