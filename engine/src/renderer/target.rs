pub enum RenderTarget {
    Empty,
    Texture(Arc<Texture>),
    Master(Arc<wgpu::SwapChainTexture>),
}

impl RenderTarget {
    pub fn begin_render_pass(&self) -> Option<&wgpu::TextureView> {
        None
    }

    pub fn create_render_pass<'a>(
        &'a self,
        encoder: &'a mut wgpu::CommandEncoder,
        label: &'a str,
    ) -> Option<wgpu::RenderPass<'a>> {
        match self {
            RenderTarget::Empty => None,
            RenderTarget::Texture(tex) => Some(create_render_pass(&tex.view, encoder, label)),
            RenderTarget::Master(opt) => Some(create_render_pass(&opt.view, encoder, label)),
        }
    }

    pub fn bind_group(&self) -> Option<Arc<wgpu::BindGroup>> {
        match self {
            RenderTarget::Empty => None,
            RenderTarget::Texture(tex) => Some(Arc::clone(&tex.bind_group)),
            RenderTarget::Master(_) => None, // Master node cannot be used as input
        }
    }

    pub fn borrow_if_master(&self) -> Option<Arc<wgpu::SwapChainTexture>> {
        match self {
            RenderTarget::Empty => None,
            RenderTarget::Texture(_) => None,
            RenderTarget::Master(opt) => Some(Arc::clone(opt)),
        }
    }

    pub fn arc(&self) -> Self {
        match self {
            RenderTarget::Empty => RenderTarget::Empty,
            RenderTarget::Texture(tex) => RenderTarget::Texture(Arc::clone(&tex)),
            RenderTarget::Master(opt) => RenderTarget::Master(Arc::clone(&opt)),
        }
    }
}

impl Clone for RenderTarget {
    fn clone(&self) -> Self {
        self.arc()
    }
}
