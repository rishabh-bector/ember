// Vertex shader

[[block]]
struct Render3DUniforms {
    // [x, y, width, height]
    model: mat4x4<f32>;

    // color
    color: vec4<f32>;
   
    // mix color and texture 
    mix: f32;
};

[[block]]
struct Camera3DUniforms {
    view_proj: mat4x4<f32>;
};

// [[block]]
// struct Light3DUniforms {
//     // [x, y, linear, quadratic]
//     light_0: vec4<f32>;
//     light_1: vec4<f32>;
//     light_2: vec4<f32>;
//     light_3: vec4<f32>;
//     light_4: vec4<f32>;
// };

[[group(1), binding(0)]]
var<uniform> render_3d_uniforms: Render3DUniforms;

[[group(2), binding(0)]]
var<uniform> camera_uniforms: Camera3DUniforms;

// [[group(3), binding(0)]]
// var<uniform> light_uniforms: LightUniforms;

struct VertexInput {
    [[location(0)]] position: vec3<f32>;
    [[location(1)]] uvs: vec2<f32>;
};

struct VertexOutput {
    [[builtin(position)]] clip_position: vec4<f32>;
    [[location(0)]] uvs: vec2<f32>;
    [[location(1)]] world_pos: vec3<f32>;
};

[[stage(vertex)]]
fn main(
    in: VertexInput,
) -> VertexOutput {
    //var world_space: vec2<f32> = in.position * render_3d_uniforms.model.zw + render_3d_uniforms.model.xy;
    // var snapped: vec2<f32> = vec2<f32>(round(world_space.x), round(world_space.y));
    //var camera_space: vec2<f32> = snap2grid(world_space + camera_uniforms.view.xy, i32(1)) / camera_uniforms.view.zw;

    var world_space: vec4<f32> = render_3d_uniforms.model * vec4<f32>(in.position, 1.0);
    var camera_space: vec4<f32> = camera_uniforms.view_proj * world_space;

    var out: VertexOutput;
    out.uvs = in.uvs;
    out.clip_position = camera_space;
    out.world_pos = world_space.xyz;

    return out;
}

// Fragment shader

[[group(0), binding(0)]]
var texture0: texture_2d<f32>;
[[group(0), binding(1)]]
var sampler0: sampler;

fn point_light_2d(pos: vec2<f32>, light: vec4<f32>) -> f32 {
    let d: f32 = length(light.xy - pos);
    let attenuation: f32 = 1.0 / (1.0 + light.z * d + light.w * (d * d));
    return attenuation;
}

[[stage(fragment)]]
fn main(in: VertexOutput) -> [[location(0)]] vec4<f32> {    
    var sample_texture: vec4<f32> = textureSample(texture0, sampler0, in.uvs);
    var sample_final: vec4<f32> = (render_3d_uniforms.color * render_3d_uniforms.mix) + ((1.0 - render_3d_uniforms.mix) * sample_texture);

    // var lighting_0: f32 = point_light_2d(world_pos.xy, light_uniforms.light_0);
    // var lighting_1: f32 = point_light_2d(world_pos.xy, light_uniforms.light_1);
    // var lighting_2: f32 = point_light_2d(world_pos.xy, light_uniforms.light_2);
    // var lighting_3: f32 = point_light_2d(world_pos.xy, light_uniforms.light_3);
    // var lighting_4: f32 = point_light_2d(world_pos.xy, light_uniforms.light_4);
    // var lighting: f32 = lighting_0 + lighting_1 + lighting_2 + lighting_3 + lighting_4;
    var lighting: f32 = 1.0;
    
    return vec4<f32>(sample_final.rgb * lighting, 1.0);
}