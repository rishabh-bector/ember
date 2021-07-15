// Vertex shader

[[block]]
struct Uniforms {
    view: vec4<f32>;
};

[[group(1), binding(0)]]
var<uniform> uniforms: Uniforms;

struct VertexInput {
    [[location(0)]] position: vec2<f32>;
    [[location(1)]] uvs: vec2<f32>;
};

struct VertexOutput {
    [[builtin(position)]] clip_position: vec4<f32>;
    [[location(0)]] uvs: vec2<f32>;
};

[[stage(vertex)]]
fn main(
    in: VertexInput,
) -> VertexOutput {
    var out: VertexOutput;
    out.uvs = in.uvs;
    out.clip_position = vec4<f32>(vec 0.1 * in.position, 0.0, 1.0);
    return out;
}

// Fragment shader

[[group(0), binding(0)]]
var texture0: texture_2d<f32>;
[[group(0), binding(1)]]
var sampler0: sampler;

[[stage(fragment)]]
fn main(in: VertexOutput) -> [[location(0)]] vec4<f32> {
    // if (uniforms.view[0].x == 1.0 && uniforms.view[0].y == 0.0 && uniforms.view[1].x == 0.0 && uniforms.view[1].y == 0.0) {
    //     return vec4<f32>(1.0, 1.0, 0.0, 1.0);
    // }
    if (uniforms.view[1].x == 1.0) {
        return vec4<f32>(1.0, 0.0, 1.0, 1.0);
    }
    return textureSample(texture0, sampler0, in.uvs);
}