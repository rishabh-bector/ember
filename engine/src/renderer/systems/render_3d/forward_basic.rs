pub struct Render3D {}

pub struct Render3DUniforms {
    pub model: [[f32; 4]; 4],
    pub color: [f32; 4],
    pub mix: f32,
}

// Phantom type
pub struct Render3DForwardUniformGroup {}

#[system]
#[read_component(Render3D)]
#[read_component(Position3D)]
pub fn load(
    world: &mut SubWorld,
    #[state] state: &mut NodeState,
    #[resource] device: &Arc<wgpu::Device>,
    #[resource] queue: &Arc<wgpu::Queue>,
    #[resource] uniforms: &Arc<Mutex<GenericUniform<Render3DUniforms>>>,
    #[resource] uniform_group: &Arc<Mutex<UniformGroup<Render3DForwardUniformGroup>>>,
) {
    let start_time = Instant::now();
    debug!("running system forward_render_3d (graph node)");
    let node = Arc::clone(&state.node);

    let render_target = state.render_target.lock().unwrap();
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("Render3D Encoder"),
    });
    let mut pass = render_target
        .create_render_pass(&mut encoder, "forward_render_3d", true)
        .unwrap();

    pass.set_pipeline(&node.pipeline);

    pass.set_bind_group(
        0,
        &node.binder.texture_groups[&ID(RENDER_2D_COMMON_TEXTURE_ID)],
        &[],
    );

    let mut uniforms = uniforms.lock().unwrap();
    let mut uniform_group = uniform_group.lock().unwrap();

    let mut query = <(&Render3D, &Position3D)>::query();
    for (render_3d, pos) in query.iter_mut(world) {}
}
