pub const BASE_2D_COMMON_TEXTURE: &str = "test";
pub const BASE_2D_COMMON_VERTEX_BUFFER: usize = 0;
pub const BASE_2D_COMMON_INDEX_BUFFER: usize = 0;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Position2D {
    pub x: f32,
    pub y: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Velocity2D {
    pub dx: f32,
    pub dy: f32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Sprite {
    pub width: usize,
    pub height: usize,
    pub pixels: Vec<u8>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Base2D {
    pub texture: String,
    pub common_vertex_buffer: usize,
    pub common_index_buffer: usize,
}

impl Base2D {
    pub fn common() -> Self {
        Base2D {
            texture: BASE_2D_COMMON_TEXTURE.to_string(),
            common_vertex_buffer: BASE_2D_COMMON_VERTEX_BUFFER,
            common_index_buffer: BASE_2D_COMMON_INDEX_BUFFER,
        }
    }
}
