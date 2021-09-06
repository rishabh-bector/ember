// --------------------------------------------------
// Common
// -------------------------------------------------

[[block]]
struct QuadUniforms {
    dimensions: vec2<f32>;
    time: f32;
    nut: f32;
};

[[group(0), binding(0)]]
var<uniform> quad_uniforms: QuadUniforms;

// --------------------------------------------------
// Vertex shader
// --------------------------------------------------

struct VertexInput {
    [[location(0)]] position: vec2<f32>;
    [[location(1)]] uvs: vec2<f32>;
};

struct VertexOutput {
    [[builtin(position)]] position: vec4<f32>;
    [[location(0)]] frag_pos: vec2<f32>;
    [[location(1)]] uvs: vec2<f32>;
};

[[stage(vertex)]]
fn main(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    out.position = vec4<f32>(in.position, 0.0, 0.0);
    out.frag_pos = in.position;
    out.uvs = in.uvs;

    return out;
}

// --------------------------------------------------
// Fragment shader
// -------------------------------------------------

[[stage(fragment)]]
fn main(in: VertexOutput) -> [[location(0)]] vec4<f32> {    
    return vec4<f32>(in.uvs.y, 0.0, 1.0, 1.0);
}