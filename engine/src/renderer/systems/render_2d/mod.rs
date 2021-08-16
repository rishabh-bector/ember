use std::str::FromStr;
use uuid::Uuid;

use crate::constants::{ID, RENDER_2D_COMMON_TEXTURE_ID, UNIT_SQUARE_MESH_ID};

pub mod forward_dynamic;
pub mod forward_instance;

#[derive(Clone, Debug, PartialEq)]
pub struct Render2D {
    pub name: String,

    pub color: [f32; 4],
    pub texture: Uuid,
    pub mix: f32,

    // Todo: make these into a Size2D component
    pub width: f32,
    pub height: f32,

    pub mesh: Uuid,
}

impl Render2D {
    pub fn _test(name: &str, width: f32, height: f32) -> Self {
        Render2D::solid_rect(name, width, height, [1.0, 1.0, 1.0, 1.0])
    }

    pub fn solid_rect(name: &str, width: f32, height: f32, color: [f32; 4]) -> Self {
        Render2D {
            name: name.to_owned(),
            color,
            mix: 1.0,
            width,
            height,
            texture: Uuid::from_str(RENDER_2D_COMMON_TEXTURE_ID).unwrap(),
            mesh: ID(UNIT_SQUARE_MESH_ID),
        }
    }

    pub fn texture(name: &str, texture: Uuid, width: f32, height: f32) -> Self {
        Render2D {
            name: name.to_owned(),
            color: [1.0, 1.0, 1.0, 1.0],
            mix: 0.0,
            width,
            height,
            texture,
            mesh: ID(UNIT_SQUARE_MESH_ID),
        }
    }
}

// pub fn _flatten(mat: Matrix2<f32>) -> [f32; 4] {
//     [mat.x[0], mat.y[0], mat.x[1], mat.y[1]]
// }
