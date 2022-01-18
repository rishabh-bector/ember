// --------------------------------------------------
// Common
// -------------------------------------------------


struct QuadUniforms {
    dimensions: vec2<f32>;
    time: f32;
    delta: f32;
};


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
fn vs_main(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;

    out.position = vec4<f32>(in.position, 0.0, 1.0);
    out.screen_pos = vec2<f32>((in.position.x / 2.0) + 0.5, (1.0 - ((in.position.y / 2.0) + 0.5)));

    return out;
}

// --------------------------------------------------
// Fragment shader
// -------------------------------------------------

[[group(0), binding(0)]]
var node_input_tex: texture_2d<f32>;
[[group(0), binding(1)]]
var node_input_smp: sampler;

[[stage(fragment)]]
fn fs_main(in: VertexOutput) -> [[location(0)]] vec4<f32> {
   var out: vec4<f32> = textureSample(node_input_tex, node_input_smp, in.screen_pos);
   // let gamma: f32 = 2.2;
   //let out2: vec3<f32> = pow(out.rgb, vec3<f32>(1.0/gamma, 1.0/gamma, 1.0/gamma));
   //return vec4<f32>(out2, 1.0);
   return out;
}