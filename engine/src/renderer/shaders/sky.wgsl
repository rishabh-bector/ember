// --------------------------------------------------
// Common
// -------------------------------------------------


struct Render3DUniforms {
    model_mat: mat4x4<f32>;
    normal_mat: mat4x4<f32>;
    color: vec4<f32>;
    mix: f32;
};

struct Camera3DUniforms {
    view_pos: vec4<f32>;
    view_proj: mat4x4<f32>;
};

[[group(0), binding(0)]]
var<uniform> render_3d_uniforms: Render3DUniforms;

[[group(1), binding(0)]]
var<uniform> camera_uniforms: Camera3DUniforms;

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
    [[location(2)]] world_normal: vec3<f32>;
};

[[stage(vertex)]]
fn vs_main(
    in: VertexInput,
) -> VertexOutput {
    var world_space: vec4<f32> = render_3d_uniforms.model_mat * vec4<f32>(in.position, 1.0);
    var camera_space: vec4<f32> = camera_uniforms.view_proj * world_space;

    let normal_matrix = mat3x3<f32>(
        render_3d_uniforms.normal_mat.x.xyz,
        render_3d_uniforms.normal_mat.y.xyz,
        render_3d_uniforms.normal_mat.z.xyz,
    );

    var out: VertexOutput;
    out.uvs = in.uvs;
    out.clip_position = camera_space;

    out.world_pos = in.position;
    out.world_normal = normalize(normal_matrix * in.normal);

    return out;
}

// -------------------------------------------------
// Fragment shader
// -------------------------------------------------

[[group(2), binding(0)]]
var sky_cube: texture_cube<f32>;
[[group(2), binding(1)]]
var sky_sampler: sampler;

[[stage(fragment)]]
fn fs_main(in: VertexOutput) -> [[location(0)]] vec4<f32> {
    let sky_pos = normalize(in.world_pos);
    let height = sky_pos.y;

    let hdri_dir = sky_pos;
    let hdri = textureSample(sky_cube, sky_sampler, hdri_dir);

    if (hdri.a == 0.0) {
        return vec4<f32>(0.1, 0.1, 0.4, 1.0);
    } else {
        return hdri;
    }

    // let sunlight_dir = normalize(vec3<f32>(0.0, -0.3, 1.0));
    // let sun = dot(sunlight_dir, sky_pos);

    // let sun_color = vec4<f32>(0.9, 0.9, 0.9, 1.0);
    // let sky = vec4<f32>(height, height, height, 1.0);

    // if (sun < -0.997) {
    //     return sun_color;
    // } else {
    //     return sky;
    // }

    // return vec4<f32>(0.1, 0.1, 0.7, 1.0);
}
