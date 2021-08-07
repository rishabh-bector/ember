pub trait VertexAttribute {
    fn attribute_size() -> u64;
    fn vertex_format() -> wgpu::VertexFormat;
}

impl VertexAttribute for [f32; 2] {
    fn attribute_size() -> u64 {
        std::mem::size_of::<[f32; 2]>() as wgpu::BufferAddress
    }

    fn vertex_format() -> wgpu::VertexFormat {
        wgpu::VertexFormat::Float32x2
    }
}

pub trait VertexLayout {
    fn layout_builder() -> VertexLayoutBuilder;
}
pub struct VertexLayoutBuilder {
    pub attributes: Vec<wgpu::VertexAttribute>,
}

impl VertexLayoutBuilder {
    pub fn new(attributes: Vec<wgpu::VertexAttribute>) -> Self {
        Self { attributes }
    }

    pub fn build<'a>(&'a self, array_stride: u64) -> wgpu::VertexBufferLayout<'a> {
        wgpu::VertexBufferLayout {
            array_stride,
            step_mode: wgpu::InputStepMode::Vertex,
            attributes: self.attributes.as_slice(),
        }
    }
}
