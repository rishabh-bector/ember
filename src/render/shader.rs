pub enum ShaderSource {
    WGSL(String),
    SPIRV(String),
}

pub struct ShaderBuilder {
    pub source: ShaderSource,
    pub groups: Vec<&'static str>,
}

pub struct Shader {
    pub groups: Vec<&'static str>,
    pub module: wgpu::ShaderModule,
}

impl ShaderBuilder {
    pub fn source(source: ShaderSource) -> Self {
        Self {
            source,
            groups: vec![],
        }
    }

    pub fn group<T>(mut self) -> Self {
        self.groups.push(super::type_key::<T>());
        self
    }

    pub fn build(&mut self, device: &wgpu::Device) -> Shader {
        Shader {
            groups: self.groups.clone(),
            module: device.create_shader_module(&wgpu::ShaderModuleDescriptor {
                label: Some("Shader"),
                flags: wgpu::ShaderFlags::all(),
                source: match &self.source {
                    ShaderSource::WGSL(src) => wgpu::ShaderSource::Wgsl(src.clone().into()),
                    _ => panic!("ShaderSource: only wgsl shaders are supported currently"),
                },
            }),
        }
    }
}
