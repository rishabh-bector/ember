// --------------------------------------------------
// Common
// -------------------------------------------------

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

// --------------------------------------------------
// Vertex shader
// --------------------------------------------------

struct VertexInput {
    [[location(0)]] position: vec3<f32>;
    [[location(1)]] uvs: vec2<f32>;
    [[location(2)]] normal: vec3<f32>;
};

struct VertexOutput {
    [[builtin(position)]] clip_position: vec4<f32>;
    [[location(0)]] uvs: vec2<f32>;
    [[location(1)]] world_pos: vec3<f32>;
    [[location(2)]] tangent_normal: vec3<f32>;
};

[[stage(vertex)]]
fn main(
    in: VertexInput,
) -> VertexOutput {
    var world_space: vec4<f32> = render_3d_uniforms.model * vec4<f32>(in.position, 1.0);
    var camera_space: vec4<f32> = camera_uniforms.view_proj * world_space;

    var out: VertexOutput;
    out.uvs = in.uvs;
    out.clip_position = camera_space;
    out.world_pos = world_space.xyz;
    out.tangent_normal = in.normal;

    return out;
}

// --------------------------------------------------
// Fragment shader
// -------------------------------------------------

[[group(0), binding(0)]]
var texture0: texture_2d<f32>;
[[group(0), binding(1)]]
var sampler0: sampler;

fn phong(light_dir: vec3<f32>, fragment_normal: vec3<f32>) -> f32 {
    return max(dot(normalize(fragment_normal), normalize(-light_dir)), 0.0);
}

fn directional_light_3d(light_dir: vec3<f32>, light_color: vec3<f32>, fragment_normal: vec3<f32>) -> vec3<f32> {
    return light_color * phong(light_dir, fragment_normal);
}

[[stage(fragment)]]
fn main(in: VertexOutput) -> [[location(0)]] vec4<f32> {    
    var sample_texture: vec4<f32> = textureSample(texture0, sampler0, in.uvs);
    var sample_final: vec4<f32> = (render_3d_uniforms.color * (1.0 - render_3d_uniforms.mix)) + (render_3d_uniforms.mix * sample_texture);

    let ambient_light = vec3<f32>(0.05, 0.05, 0.05);
    var light_0: vec3<f32> = directional_light_3d(vec3<f32>(1.0, -1.0, -1.0), vec3<f32>(0.5, 0.5, 0.5), in.tangent_normal);
    let fragment_light = ambient_light + light_0;
    
    return vec4<f32>(sample_final.rgb * fragment_light, 1.0);
}