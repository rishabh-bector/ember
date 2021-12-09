// --------------------------------------------------
// Common
// -------------------------------------------------

[[block]]
struct QuadUniforms {
    dimensions: vec2<f32>;
    time: f32;
    delta: f32;
};

[[group(0), binding(0)]]
var<uniform> quad: QuadUniforms;

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
    out.screen_pos = in.position;

    return out;
}

// --------------------------------------------------
// Fragment shader
// -------------------------------------------------

// --- Main ---

// [[group(2), binding(0)]]
// var texture0: texture_2d<f32>;
// [[group(2), binding(1)]]
// var sampler0: sampler;

[[stage(fragment)]]
fn main(in: VertexOutput) -> [[location(0)]] vec4<f32> {
    in.screen_pos

    // gamma correction
    // pixel_color = pow(pixel_color, vec3<f32>(0.4545));
    return vec4<f32>(total, 1.0);
}