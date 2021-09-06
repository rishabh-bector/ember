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
struct CameraUniforms {
    origin: vec4<f32>;
    view_proj: mat4x4<f32>;
    inv_view_proj: mat4x4<f32>;
    clip: vec2<f32>;
};

[[group(0), binding(0)]]
var<uniform> quad: QuadUniforms;

[[group(1), binding(0)]]
var<uniform> camera: CameraUniforms;

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
    [[location(1)]] ray_origin: vec3<f32>;
    [[location(2)]] ray: vec3<f32>;
};

[[stage(vertex)]]
fn main(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    out.position = vec4<f32>(in.position, 0.0, 1.0);
    out.frag_pos = in.position;

    out.ray_origin = (camera.inv_view_proj * vec4<f32>(in.position, -1.0, 1.0) * camera.clip.x).xyz;
    out.ray = (camera.inv_view_proj * vec4<f32>(in.position * (camera.clip.y - camera.clip.x), camera.clip.x + camera.clip.y, camera.clip.y - camera.clip.x)).xyz;

    return out;
}

// --------------------------------------------------
// Fragment shader
// -------------------------------------------------

// --- Ray Camera ---

struct RayCamera {
    eye: vec3<f32>;
    target: vec3<f32>;
    roll: f32;
    focal_length: f32;
};

fn ray(camera: RayCamera) {
    
}

// --- Signed Distance Functions ---



// --- Main ---

// [[group(2), binding(0)]]
// var texture0: texture_2d<f32>;
// [[group(2), binding(1)]]
// var sampler0: sampler;

[[stage(fragment)]]
fn main(in: VertexOutput) -> [[location(0)]] vec4<f32> {
    let ray_pos = in.ray_origin;
    let ray_dir = normalize(in.ray);
    
    return vec4<f32>(camera.origin.x, in.frag_pos.y, 1.0, 1.0);
}