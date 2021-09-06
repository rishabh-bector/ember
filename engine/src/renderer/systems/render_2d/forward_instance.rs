use legion::{world::SubWorld, IntoQuery};
use rayon::iter::{IntoParallelRefMutIterator, ParallelIterator};
use std::{
    sync::{Arc, RwLock},
    time::Instant,
};

use crate::{
    components::{FrameMetrics, Position2D},
    constants::{
        CAMERA_2D_BIND_GROUP_ID, ID, LIGHTING_2D_BIND_GROUP_ID, RENDER_2D_COMMON_TEXTURE_ID,
    },
    renderer::{
        buffer::instance::{Instance, InstanceBuffer, InstanceGroup, InstanceGroupBinder},
        graph::NodeState,
        mesh::Mesh,
    },
    sources::registry::MeshRegistry,
};

#[instance((4, 44usize))]
#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Render2DInstance {
    pub model: [f32; 4],
    pub color: [f32; 4],
    pub mix: f32,
    pub group_id: u32,
    pub id: u32,
}

impl Render2DInstance {
    pub fn new(color: [f32; 4]) -> Self {
        Self {
            color,
            model: [0.0, 0.0, 1.0, 1.0],
            mix: 1.0,
            group_id: 0,
            id: 0,
        }
    }

    pub fn new_default_group() -> InstanceGroup<Render2DInstance> {
        InstanceGroup::new(0, ID(RENDER_2D_COMMON_TEXTURE_ID))
    }

    pub fn update_position(&mut self, pos: &Position2D) {
        self.model[0] = pos.x;
        self.model[1] = pos.y;
    }
}

impl Default for Render2DInstance {
    fn default() -> Self {
        Self::new([1.0, 1.0, 1.0, 1.0])
    }
}

impl Instance for Render2DInstance {
    fn id(&self) -> (u32, u32) {
        (self.group_id, self.id)
    }

    fn set_id(&mut self, group_id: u32, inst_id: u32) {
        self.group_id = group_id;
        self.id = inst_id;
    }

    fn size() -> usize {
        36
    }
}

pub struct Attractor2D {
    pub force: f32,
}

pub struct Render2DUniformGroup {}

#[system]
#[write_component(InstanceGroup<Render2DInstance>)]
#[write_component(Mesh)]
pub fn load(world: &mut SubWorld, #[resource] frame_metrics: &Arc<RwLock<FrameMetrics>>) {
    debug!("running system render_2d_instance_loader");
    let delta = frame_metrics.read().unwrap().delta().as_secs_f32();
    <(&mut InstanceGroup<Render2DInstance>, &Mesh)>::query().par_for_each_mut(
        world,
        |(group, _)| {
            let components = Arc::clone(&group.components);
            let mutators = components.read().unwrap();
            group.instances.par_iter_mut().for_each(|instance| {
                for component in &mutators[instance.id as usize] {
                    component.lock().unwrap().mutate(instance, delta);
                }
            })
        },
    );
}

#[system]
#[read_component(InstanceGroup<Render2DInstance>)]
#[read_component(Mesh)]
pub fn render(
    world: &SubWorld,
    #[state] state: &mut NodeState,
    #[resource] mesh_registry: &Arc<RwLock<MeshRegistry>>,
    #[resource] instance_buffer: &InstanceBuffer<Render2DInstance>,
    #[resource] device: &Arc<wgpu::Device>,
    #[resource] queue: &Arc<wgpu::Queue>,
) {
    let start_time = Instant::now();
    debug!("running system render_2d_forward_instance (graph node)");
    let node = Arc::clone(&state.node);
    let mesh_registry = mesh_registry.read().unwrap();

    let render_target = state.render_target.lock().unwrap();
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("render_2d_forward_instance_encoder"),
    });
    let mut pass = render_target
        .create_render_pass("render_2d_forward_instance_pass", &mut encoder, true)
        .unwrap();
    pass.set_pipeline(&node.pipeline);

    // Global bindings
    pass.set_bind_group(
        1,
        &node.binder.uniform_groups[&ID(CAMERA_2D_BIND_GROUP_ID)],
        &[],
    );
    pass.set_bind_group(
        2,
        &node.binder.uniform_groups[&ID(LIGHTING_2D_BIND_GROUP_ID)],
        &[],
    );

    for (group, mesh) in <(&InstanceGroup<Render2DInstance>, &Mesh)>::query().iter(world) {
        debug!(
            "rendering instance group => type: render_2d, name: {}, size: {}",
            "",
            group.num_instances()
        );

        // One instance buffer is managed per group type
        // (in this case: InstanceBuffer<Render2DInstance>)
        instance_buffer.load_group(group.buffer_bytes());

        // Every instance in a group shares the same texture and mesh
        pass.set_bind_group(0, &node.binder.texture_groups[&group.texture()], &[]);
        pass.set_vertex_buffer(0, mesh.vertex_buffer.buffer.0.slice(..));
        pass.set_index_buffer(
            mesh.index_buffer.buffer.0.slice(..),
            wgpu::IndexFormat::Uint32,
        );

        // Load and draw all instances in the group
        pass.set_vertex_buffer(1, instance_buffer.state.buffer.slice(..));
        pass.draw_indexed(
            0..mesh.index_buffer.buffer.1,
            0,
            0..group.num_instances() as _,
        );
    }

    debug!("done recording; submitting render pass");
    drop(pass);
    drop(mesh_registry);
    queue.submit(std::iter::once(encoder.finish()));

    debug!("render_2d_forward_instance pass submitted");
    state.reporter.update(start_time.elapsed().as_secs_f64());
}
