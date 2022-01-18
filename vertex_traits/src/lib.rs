pub trait VertexLayout {
    fn layout_builder() -> VertexLayoutBuilder;
    fn layout() -> wgpu::VertexBufferLayout<'static>;
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
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: self.attributes.as_slice(),
        }
    }
}

pub trait VertexAttribute {
    fn attribute_size() -> u64;
    fn vertex_format() -> wgpu::VertexFormat;
}

impl VertexAttribute for u32 {
    fn attribute_size() -> u64 {
        std::mem::size_of::<u32>() as wgpu::BufferAddress
    }

    fn vertex_format() -> wgpu::VertexFormat {
        wgpu::VertexFormat::Uint32
    }
}

impl VertexAttribute for [f32; 2] {
    fn attribute_size() -> u64 {
        std::mem::size_of::<[f32; 2]>() as wgpu::BufferAddress
    }

    fn vertex_format() -> wgpu::VertexFormat {
        wgpu::VertexFormat::Float32x2
    }
}

impl VertexAttribute for [f32; 3] {
    fn attribute_size() -> u64 {
        std::mem::size_of::<[f32; 3]>() as wgpu::BufferAddress
    }

    fn vertex_format() -> wgpu::VertexFormat {
        wgpu::VertexFormat::Float32x3
    }
}

impl VertexAttribute for [f32; 4] {
    fn attribute_size() -> u64 {
        std::mem::size_of::<[f32; 4]>() as wgpu::BufferAddress
    }

    fn vertex_format() -> wgpu::VertexFormat {
        wgpu::VertexFormat::Float32x4
    }
}
