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
    [[location(0)]] screen_pos: vec2<f32>;
    [[location(1)]] ray_origin: vec3<f32>;
    [[location(2)]] ray: vec3<f32>;
};

[[stage(vertex)]]
fn main(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;

    out.position = vec4<f32>(in.position, 0.0, 1.0);

    out.screen_pos = in.position;

    out.ray_origin = (camera.inv_view_proj * vec4<f32>(in.position, -1.0, 1.0) * camera.clip.x).xyz;

    out.ray = vec4<f32>(
        camera.inv_view_proj * 
        vec4<f32>(
            in.position * (camera.clip.y - camera.clip.x), 
            camera.clip.x + camera.clip.y, 
            camera.clip.y - camera.clip.x,
        )
    ).xyz;

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

// --- Ray Utilities ---

fn sky_dome(color: vec3<f32>, darken: f32, blend: f32, ray_dir: vec3<f32>) -> vec3<f32> {
    return color - (max(ray_dir.y, 0.0)) * blend;
}

struct RayDelta {
    dx: vec3<f32>;
    dy: vec3<f32>;
};

fn ray_delta(screen_pos: vec2<f32>, screen_res: vec2<f32>, inv_view_proj: mat4x4<f32>, focal_length: f32) -> RayDelta {
    let px: vec2<f32> = (2.0 * screen_pos - screen_res) / screen_res.y;
    let py: vec2<f32> = (2.0 * screen_pos - screen_res) / screen_res.y;

    let dx = inv_view_proj * normalize(vec4<f32>(px, focal_length, 0.0));
    let dy = inv_view_proj * normalize(vec4<f32>(py, focal_length, 0.0));

    return RayDelta(dx.xyz, dy.xyz);
}

// --- Materials ---

// http://iquilezles.org/www/articles/checkerfiltering/checkerfiltering.htm
fn grid(frag_pos: vec3<f32>, ray_dir: vec3<f32>, ray_dx: vec3<f32>, ray_dy: vec3<f32>, cell_size: f32) -> f32 {
    let dp_dx: vec3<f32> = frag_pos.y * (ray_dir / ray_dir.y - ray_dx / ray_dx.y);
    let dp_dy: vec3<f32> = frag_pos.y * (ray_dir / ray_dir.y - ray_dy / ray_dy.y);

    let scale: f32 = 0.5;

    let w = abs(dp_dx.xz*cell_size) + abs(dp_dy.xz*cell_size) + 0.001;
    let i = 2.0*(abs(fract((frag_pos.xz*cell_size - scale*w) * scale) - scale) - abs(fract((frag_pos.xz*cell_size + scale*w) * scale) - scale)) / w;

    return scale - scale * i.x * i.y;
}

// --- Signed Distance Functions ---
//
// all functions return the distance to the target (eye space)

fn infinite_plane(target: vec3<f32>, ray_dir: vec3<f32>) -> f32 {
    let tp1 = (0.0 - target.y) / ray_dir.y;
    if (tp1 > 0.0) {
        return max(tp1, 2.0);
    } else {
        return -1.0;
    }
}

fn sphere(target: vec3<f32>, radius: f32) -> f32 {
    return length(target) - radius;
}

// --- Ray Tracer ---

fn scene(ray_src: vec3<f32>, ray_dir: vec3<f32>) -> f32 {
    var closest_t: f32 = 1.0e10;

    closest_t = min(closest_t, sphere(ray_src, 50.0));

    return closest_t;
}

// returns [closest_t, id] where:
//      - if id < 0.0, the ray hit nothing
//      - if id == 0.0, this is the ground plane
//      - otherwise, this is a shape
fn cast_ray(ray_src: vec3<f32>, ray_dir: vec3<f32>) -> vec2<f32> {
    var output: vec2<f32> = vec2<f32>(-1.0, -1.0);

    let plane = infinite_plane(ray_src, ray_dir);
    output = vec2<f32>(plane, 0.0);
    if (plane != -1.0) {
        output = vec2<f32>(plane, 0.0);
    }

    var current_t: f32 = 0.0;

    for(var i: i32 = 0; i < 100; i = i + 1) {
        let dist = scene(ray_src + current_t*ray_dir, ray_dir);
        if (abs(dist) < 0.0001 * current_t) {
            output = vec2<f32>(current_t, 1.0);
            break;
        }
        current_t = current_t + dist;
    }

    if (plane > 0.0 && plane < output.r) {
        return vec2<f32>(plane, 0.0);
    } else {
        return output;
    }
}

// --- Main ---

// [[group(2), binding(0)]]
// var texture0: texture_2d<f32>;
// [[group(2), binding(1)]]
// var sampler0: sampler;

[[stage(fragment)]]
fn main(in: VertexOutput) -> [[location(0)]] vec4<f32> {
    let ray_src = in.ray_origin;
    let ray_dir = normalize(in.ray);

    let hit = cast_ray(ray_src, ray_dir);
    let frag_pos: vec3<f32> = ray_src + hit.r * ray_dir;

    let fog_amount: f32 = 1.0 - exp(-0.00001 * hit.r * hit.r);
    let fog_color = vec3<f32>(0.7, 0.7, 0.9);

    var frag_color: vec3<f32>;

    if (hit.y < 0.0) {
        let sky_col = sky_dome(vec3<f32>(0.6, 0.6, 0.9), 0.8, 0.5, ray_dir);
        frag_color = sky_col;

    } elseif (hit.y == 0.0) {
        let ray_del = ray_delta(in.screen_pos, quad.dimensions, camera.inv_view_proj, camera.clip.y - camera.clip.x);
        let grid_mat = grid(frag_pos, ray_dir, ray_del.dx, ray_del.dy, 0.2);
        frag_color = vec3<f32>(grid_mat, grid_mat, grid_mat);

    } else {
        frag_color = vec3<f32>(0.9, 0.1, 0.1);
    }

    let pixel_color: vec3<f32> = mix(frag_color, fog_color, vec3<f32>(fog_amount));
    return vec4<f32>(pixel_color, 1.0);
}