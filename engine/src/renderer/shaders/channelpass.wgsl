// --------------------------------------------------
// Common
// -------------------------------------------------

[[block]]
struct QuadUniforms {
    dimensions: vec2<f32>;
    time: f32;
    delta: f32;
};

[[block]]
struct Camera3DUniforms {
    view_pos: vec4<f32>;
    view_proj: mat4x4<f32>;
};

[[group(1), binding(0)]]
var<uniform> quad: QuadUniforms;

[[group(2), binding(0)]]
var<uniform> camera: Camera3DUniforms;

// --------------------------------------------------
// Vertex shader
// --------------------------------------------------

struct VertexInput {
    [[location(0)]] position: vec2<f32>;
    [[location(1)]] uvs: vec2<f32>;
};

struct VertexOutput {
    [[builtin(position)]] position: vec4<f32>;
    [[location(0)]] screen_pos: vec2<f32>;
};

[[stage(vertex)]]
fn main(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;

    out.position = vec4<f32>(in.position, 0.0, 1.0);
    out.screen_pos = vec2<f32>((in.position.x / 2.0) + 0.5, (1.0 - ((in.position.y / 2.0) + 0.5)));

    return out;
}

// --------------------------------------------------
// Fragment shader
// -------------------------------------------------

// --- Main ---

[[group(0), binding(0)]]
var node_input_tex: texture_2d<f32>;
[[group(0), binding(1)]]
var node_input_smp: sampler;

[[stage(fragment)]]
fn main(in: VertexOutput) -> [[location(0)]] vec4<f32> {

    // gamma correction
    // pixel_color = pow(pixel_color, vec3<f32>(0.4545));
    var m: f32 = 0.1;
    // return vec4<f32>(in.screen_pos, 0.0, 1.0);
    return mix(
        textureSample(node_input_tex, node_input_smp, in.screen_pos),
        vec4<f32>(0.0, 1.0, 0.0, 1.0),
        vec4<f32>(m, m, m, m)
    );
}