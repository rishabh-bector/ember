// --------------------------------------------------
// Common
// -------------------------------------------------

[[block]]
struct Render3DUniforms {
    model_mat: mat4x4<f32>;
    normal_mat: mat4x4<f32>;
    color: vec4<f32>;
    mix: f32;
};

[[block]]
struct Camera3DUniforms {
    view_pos: vec4<f32>;
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
    // [[location(1)]] world_pos: vec3<f32>;
    // [[location(2)]] world_normal: vec3<f32>;
};

[[stage(vertex)]]
fn main(
    in: VertexInput,
) -> VertexOutput {
    var world_space: vec4<f32> = render_3d_uniforms.model_mat * vec4<f32>(in.position, 1.0);
    var camera_space: vec4<f32> = camera_uniforms.view_proj * world_space;

    // let normal_matrix = mat3x3<f32>(
    //     render_3d_uniforms.normal_mat.x.xyz,
    //     render_3d_uniforms.normal_mat.y.xyz,
    //     render_3d_uniforms.normal_mat.z.xyz,
    // );

    var out: VertexOutput;
    out.uvs = in.uvs;
    out.clip_position = camera_space;

    // out.world_pos = world_space.xyz;
    // out.world_normal = normalize(normal_matrix * in.normal);

    return out;
}

// --------------------------------------------------
// Fragment shader
// -------------------------------------------------

[[group(0), binding(0)]]
var texture0: texture_2d<f32>;
[[group(0), binding(1)]]
var sampler0: sampler;

// fn diffuse(light_dir: vec3<f32>, fragment_normal: vec3<f32>) -> f32 {
//     return max(dot(normalize(fragment_normal), normalize(light_dir)), 0.0);
// }

// fn specular(shine: f32, light_dir: vec3<f32>, view_pos: vec3<f32>, frag_pos: vec3<f32>, frag_normal: vec3<f32>) -> f32 {
//     var view_dir: vec3<f32> = normalize(view_pos - frag_pos);
//     let half_dir = normalize(light_dir + view_dir);
//     return pow(max(dot(frag_normal, half_dir), 0.0), shine);
// }

// fn directed_diffuse(light_dir: vec3<f32>, light_color: vec3<f32>, frag_normal: vec3<f32>) -> vec3<f32> {
//     return light_color * diffuse(-light_dir, frag_normal);
// }

// fn directed_diffuse_specular(light_dir: vec3<f32>, light_color: vec3<f32>, frag_normal: vec3<f32>, frag_pos: vec3<f32>, view_pos: vec3<f32>) -> vec3<f32> {
//     return light_color * diffuse(-light_dir, frag_normal) + light_color * specular(8.0, -light_dir, view_pos, frag_pos, frag_normal);
// }

[[stage(fragment)]]
fn main(in: VertexOutput) -> [[location(0)]] vec4<f32> {    
    // var sample_texture: vec4<f32> = textureSample(texture0, sampler0, in.uvs);
    // var sample_final: vec4<f32> = (render_3d_uniforms.color * (1.0 - render_3d_uniforms.mix)) + (render_3d_uniforms.mix * sample_texture);

    // let ambient_light = vec3<f32>(0.05, 0.05, 0.05);
    // var light_0: vec3<f32> = directed_diffuse_specular(vec3<f32>(0.0, 0.0, 1.0), vec3<f32>(0.5, 0.5, 0.5), in.world_normal, in.world_pos, camera_uniforms.view_pos.xyz);
    // let fragment_light = ambient_light + light_0;
    
    return vec4<f32>(1.0, 0.0, 1.0, 1.0); // vec4<f32>(sample_final.rgb * fragment_light, 1.0);
}