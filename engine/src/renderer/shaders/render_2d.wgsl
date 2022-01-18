// Vertex shader


struct Render2DUniforms {
    // [x, y, width, height]
    model: vec4<f32>;

    // color
    color: vec4<f32>;
   
    // mix color and texture 
    mix: f32;
};


struct Camera2DUniforms {
    // [x, y, width, height]
    view: vec4<f32>;
};


struct Light2DUniforms {
    // [x, y, linear, quadratic]
    light_0: vec4<f32>;
    light_1: vec4<f32>;
    light_2: vec4<f32>;
    light_3: vec4<f32>;
    light_4: vec4<f32>;
};

[[group(1), binding(0)]]
var<uniform> render_2d_uniforms: Render2DUniforms;

[[group(2), binding(0)]]
var<uniform> camera_uniforms: Camera2DUniforms;

[[group(3), binding(0)]]
var<uniform> light_uniforms: Light2DUniforms;

struct VertexInput {
    [[location(0)]] position: vec2<f32>;
    [[location(1)]] uvs: vec2<f32>;
};

struct VertexOutput {
    [[builtin(position)]] clip_position: vec4<f32>;
    [[location(0)]] uvs: vec2<f32>;
    [[location(1)]] world_pos: vec2<f32>;
};

fn multiply_vec4_as_mat2(in: vec2<f32>, mat2: vec4<f32>) -> vec2<f32> {
    return vec2<f32>(
        mat2.r*in.x+mat2.b*in.x, 
        mat2.g*in.y+mat2.a*in.y, 
    );
} 

fn snap2grid(in: vec2<f32>, grid_size: i32) -> vec2<f32> {
    return vec2<f32>(f32(i32(in.x/f32(grid_size))*grid_size), f32(i32(in.y/f32(grid_size))*grid_size));
}

[[stage(vertex)]]
fn vs_main(
    in: VertexInput,
) -> VertexOutput {
    var world_space: vec2<f32> = in.position * render_2d_uniforms.model.zw + render_2d_uniforms.model.xy;
    // var snapped: vec2<f32> = vec2<f32>(round(world_space.x), round(world_space.y));
    var camera_space: vec2<f32> = snap2grid(world_space + camera_uniforms.view.xy, i32(1)) / camera_uniforms.view.zw;

    var out: VertexOutput;
    out.uvs = in.uvs;
    out.clip_position = vec4<f32>(camera_space, 0.0, 1.0);
    out.world_pos = world_space;

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
fn fs_main(in: VertexOutput) -> [[location(0)]] vec4<f32> {
    var world_pos: vec2<f32> = in.world_pos;
    
    var sample_texture: vec4<f32> = textureSample(texture0, sampler0, in.uvs);
    var sample_final: vec4<f32> = (render_2d_uniforms.color * render_2d_uniforms.mix) + ((1.0 - render_2d_uniforms.mix) * sample_texture);

    var lighting_0: f32 = point_light_2d(world_pos.xy, light_uniforms.light_0);
    var lighting_1: f32 = point_light_2d(world_pos.xy, light_uniforms.light_1);
    var lighting_2: f32 = point_light_2d(world_pos.xy, light_uniforms.light_2);
    var lighting_3: f32 = point_light_2d(world_pos.xy, light_uniforms.light_3);
    var lighting_4: f32 = point_light_2d(world_pos.xy, light_uniforms.light_4);
    var lighting: f32 = lighting_0 + lighting_1 + lighting_2 + lighting_3 + lighting_4;

    return vec4<f32>(sample_final.rgb * lighting, 1.0);
}