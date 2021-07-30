use std::str::FromStr;

use uuid::Uuid;

pub const DEFAULT_SCREEN_WIDTH: usize = 1440;
pub const DEFAULT_SCREEN_HEIGHT: usize = 900;

pub const DEFAULT_TEXTURE_BUFFER_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Bgra8UnormSrgb;
pub const DEFAULT_MAX_DYNAMIC_ENTITIES_PER_PASS: u32 = 128;
pub const DEFAULT_DYNAMIC_BUFFER_MIN_BINDING_SIZE: u64 = 128;

// Engine render nodes

pub const BASE_2D_RENDER_NODE_ID: &str = "0660ca73-c74c-40b0-afee-8cd9128aa190";

// Engine bind groups

pub const BASE_2D_BIND_GROUP_ID: &str = "2fc8e285-38ca-45e2-a910-00f49a7455d1";
pub const CAMERA_2D_BIND_GROUP_ID: &str = "50cdf623-c003-4c7c-ae56-646339c4f026";
pub const LIGHTING_2D_BIND_GROUP_ID: &str = "eb964ee1-abc3-435f-ab03-0dceb692661e";

// 2D render graph base

pub const BASE_2D_COMMON_TEXTURE_ID: &str = "8a22d465-7935-41e5-9e90-686ef5632c54";
pub const BASE_2D_COMMON_VERTEX_BUFFER: usize = 0;
pub const BASE_2D_COMMON_INDEX_BUFFER: usize = 0;

// Common shapes

pub const UNIT_SQUARE_VRT_BUFFER_ID: &str = "6fd0eeb3-9847-4a26-9eec-370e9839cbd3";
pub const UNIT_SQUARE_IND_BUFFER_ID: &str = "61c66b5d-8569-4cc7-9d44-a0ab7c4cdf24";

#[rustfmt::skip]
pub const OPENGL_TO_WGPU_MATRIX: cgmath::Matrix4<f32> = cgmath::Matrix4::new(
    1.0, 0.0, 0.0, 0.0,
    0.0, 1.0, 0.0, 0.0,
    0.0, 0.0, 0.5, 0.0,
    0.0, 0.0, 0.5, 1.0,
);

pub fn ID(const_id: &str) -> Uuid {
    Uuid::from_str(const_id).expect("failed to parse Uuid from constant str")
}
