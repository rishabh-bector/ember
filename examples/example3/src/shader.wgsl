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
    let px: vec2<f32> = (2.0 * (screen_pos + vec2<f32>(1.0, 0.0)) - screen_res) / screen_res.y;
    let py: vec2<f32> = (2.0 * (screen_pos + vec2<f32>(0.0, 1.0)) - screen_res) / screen_res.y;

    let dx = inv_view_proj * normalize(vec4<f32>(px, focal_length, 0.0));
    let dy = inv_view_proj * normalize(vec4<f32>(py, focal_length, 0.0));

    return RayDelta(dx.xyz, dy.xyz);
}

fn smooth_sdf(d1: f32, d2: f32, k: f32) -> f32 {
    let res = exp(-k * d1) + exp(-k * d2);
    return -log(max(0.0001, res)) / k;
}

// --- Materials ---

// http://iquilezles.org/www/articles/checkerfiltering/checkerfiltering.htm
fn grid(frag_pos: vec3<f32>, ray_src: vec3<f32>, ray_dir: vec3<f32>, ray_dx: vec3<f32>, ray_dy: vec3<f32>, cell_size: f32) -> f32 {
    let dp_dx: vec2<f32> = ray_src.y * (ray_dir / ray_dir.y - ray_dx / ray_dx.y).xz;
    let dp_dy: vec2<f32> = ray_src.y * (ray_dir / ray_dir.y - ray_dy / ray_dy.y).xz;
    let pos: vec2<f32> = frag_pos.xz;
    
    let scale: f32 = 0.5;   

    let w: vec2<f32> = abs(dp_dx*cell_size) + abs(dp_dy*cell_size) + 0.001;
    let i: vec2<f32> = 2.0*(
        abs(fract((pos*cell_size - scale*w) * scale) - scale) - 
        abs(fract((pos*cell_size + scale*w) * scale) - scale)
    ) / w;

    return scale - scale * i.x * i.y;
}

// --- Signed Distance Functions ---
//
// many of these are adapted from https://iquilezles.org
//
// all functions return the distance to the target (eye space)
// the target vector should point from the eye to the shape origin.

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

fn capsule(target: vec3<f32>, a: vec3<f32>, b: vec3<f32>, radius: f32) -> f32 {
	let pa = target - a;
    let ba = b - a;
    let h = clamp(dot(pa, ba) / dot(ba, ba), 0.0, 1.0);
	return length(pa - ba * h) - radius;
}

// -1.0 if not hit
// t, color if hit
fn mandelbulb(target: vec3<f32>, scale: f32) -> vec4<f32> {
    var w: vec3<f32> = target * scale;
    var m: f32 = dot(w, w);

    var trap: vec4<f32> = vec4<f32>(abs(w), m);
	var dz: f32 = 1.0;
    
	for(var i: i32 = 0; i < 5; i = i + 1) {
        dz = 8.0 * pow(m, 3.5) * dz + 1.0;
        // dz = 8.0*pow(sqrt(m),7.0)*dz + 1.0;
        // z = z^8+z
        let r = length(w);
        let b = 8.0 * acos(w.y / r);
        let a = 8.0 * atan2(w.x, w.z);
        w = (target * scale) + pow(r, 8.0) * vec3<f32>(sin(b) * sin(a), cos(b), sin(b) * cos(a));
        trap = min(trap, vec4<f32>(abs(w), m));

        m = dot(w, w);
		if(m > 256.0) {
            break;
        }
    }

    let color = vec3<f32>(m, trap.yz);
    // distance estimation (through the Hubbard-Douady potential)
    let dist =  0.25 * log(m) * sqrt(m) / dz;

    return vec4<f32>(dist / scale, color);
}

// --- Lighting ---

fn sun_light(input_color: vec3<f32>, normal: vec3<f32>, ray_dir: vec3<f32>) -> vec3<f32> {
    let light_dir = normalize(vec3<f32>(-0.5, 0.4, -0.6));
    let hal = normalize(light_dir - ray_dir);
    let dif = clamp(dot(normal, light_dir), 0.0, 1.0);
    //if( dif>0.0001 )
    //dif *= calcSoftshadow( pos, lig, 0.02, 2.5 );
    var spe: f32 = pow(clamp(dot(normal, hal), 0.0, 1.0), 4.0);
    spe = spe * dif;
    spe = spe * 0.04 + 0.96 * pow(clamp(1.0 - dot(hal, light_dir), 0.0, 1.0), 5.0);

    var output_color: vec3<f32> = vec3<f32>(0.0);
    output_color = input_color + input_color * 2.0 * dif * vec3<f32>(1.30, 1.00, 0.70);
    // output_color = output_color + 1.00 * spe * vec3<f32>(1.30, 1.00, 0.70) * 1.0;

    return output_color;
}

fn sky_light(input_color: vec3<f32>, shine: f32, light_color: vec3<f32>, normal: vec3<f32>, ray_dir: vec3<f32>, reflected_ray: vec3<f32>, occlusion: f32) -> vec3<f32> {
    var diffuse: f32 = sqrt(clamp(0.5 + 0.5 * normal.y, 0.0, 1.0));
    diffuse = diffuse * occlusion;

    var specular: f32 = smoothStep(-0.3, 0.3, reflected_ray.y);
    specular = specular * diffuse;
    specular = specular * (0.04 + 0.96 * pow(clamp(1.0 + dot(normal, ray_dir), 0.0, 1.0), 4.0));

    //if( spe>0.001 )

    //specular *= calcSoftshadow( pos, ref, 0.02, 2.5 );
    var output_color: vec3<f32> = vec3<f32>(0.0);
    output_color = input_color * 0.6 * diffuse * light_color; //vec3<f32>(0.40, 0.60, 1.15);
    output_color = output_color + 2.00 * specular * light_color * shine; // vec3<f32>(0.40, 0.60, 1.30) * shine;
    
    return output_color;
}

// --- Scene ---

fn scene(target: vec3<f32>) -> f32 {
    var closest_t: f32 = 1.0e10;

    // closest_t = min(
    //     closest_t, 
    //     smooth_sdf(
    //         smooth_sdf(
    //             sphere(vec3<f32>(0.0, 10.0, 0.0) - target, 10.0), 
    //             sphere(vec3<f32>(10.0, 10.0, 10.0) - target, 10.0), 
    //             1.0,
    //         ), 
    //         capsule(
    //             vec3<f32>(5.0, 18.0, 5.0) - target, 
    //             vec3<f32>(0.0, 0.0, 0.0), 
    //             vec3<f32>(40.0, -5.0, -40.0), 
    //             6.0,
    //         ),
    //         4.0,
    //     ),
    // );

    let r = quad.delta * 0.05;
    let rot_mat = mat3x3<f32>(
        vec3<f32>(cos(r), 0.0, -sin(r)),
        vec3<f32>(0.0, 1.0, 0.0),
        vec3<f32>(sin(r), 0.0, cos(r)),
    );

    closest_t = min(closest_t, mandelbulb(rot_mat * (vec3<f32>(0.0, 12.0, 0.0) - target), 0.1).x);
    
    // closest_t_2 = min(closest_t, sphere(vec3<f32>(10.0, 10.0, 4.0) - target, 3.0));
    // closest_t = min(closest_t, sphere(vec3<f32>(-25.0, 10.0, -21.0) - target, 6.0));
    // closest_t = min(closest_t, sphere(vec3<f32>(0.0, 10.0, 0.0) - target, 10.0));
    // closest_t = min(closest_t, sphere(vec3<f32>(0.0, 10.0, 0.0) - target, 10.0));

    return closest_t;
}

// http://iquilezles.org/www/articles/normalsSDF/normalsSDF.htm
fn scene_normal(frag_pos: vec3<f32>) -> vec3<f32> {
    let e = vec2<f32>(1.0, -1.0) * 0.5773 * 0.0005;
    return normalize(vec3<f32>(
        e.xyy * scene(frag_pos + e.xyy) + 
        e.yyx * scene(frag_pos + e.yyx) + 
        e.yxy * scene(frag_pos + e.yxy) + 
        e.xxx * scene(frag_pos + e.xxx)
    ));
}

fn scene_occlusion(frag_pos: vec3<f32>, normal: vec3<f32>) -> f32 {
    var occlusion: f32 = 0.0;
    var scale: f32 = 0.04;
    for(var i: i32 = 0; i < 5; i = i + 1) {
        let height = (1.5 + 1.2 * f32(i) / 4.0);
        let closest_t = scene(frag_pos + height * normal);

        occlusion = occlusion + (height - closest_t) * scale;
        scale = scale * 0.95;

        if (occlusion > 0.35) {
            break;
        }
    }
    return 0.06+clamp(1.0 - 3.0 * occlusion, 0.0, 1.0) * (0.5 + 0.5 * normal.y);
}

// --- Ray Tracer ---

// returns [closest_t, id] where:
//      - if id < 0.0, the ray hit nothing
//      - if id == 0.0, this is the ground plane
//      - otherwise, this is a shape
fn cast_ray(ray_src: vec3<f32>, ray_dir: vec3<f32>) -> vec2<f32> {
    var current_t: f32 = 0.0;
    var output: vec2<f32> = vec2<f32>(-1.0, -1.0);

    for(var i: i32 = 0; i < 128; i = i + 1) {
        let dist = scene(ray_src + current_t*ray_dir);
        if (abs(dist) < 0.001 * current_t) {
            output = vec2<f32>(current_t, 1.0);
            break;
        }
        current_t = current_t + dist;
    }

    let plane = infinite_plane(ray_src, ray_dir);
    if (plane > 0.0 && output.g < 0.0) {
        return vec2<f32>(plane, 0.0);
    } elseif (plane < 0.0 && output.g > 0.0) {
        return output;
    } elseif (plane > 0.0 && output.g > 0.0) {
        if (plane < output.r) { return vec2<f32>(plane, 0.0); }
        else { return output; }
    } else {
        return vec2<f32>(-1.0, -1.0);
    }
}

fn render(ray_src: vec3<f32>, ray_dir: vec3<f32>, ray_del: RayDelta) -> vec3<f32> {
    let hit = cast_ray(ray_src, ray_dir);
    let frag_pos: vec3<f32> = ray_src + hit.r * ray_dir;

    let dist = length(frag_pos - ray_src);
    var fog_amount: f32 = 1.0 - sqrt(exp(-0.01 * dist));
    if (fog_amount > 0.99) {
        fog_amount = 1.0; 
    }

    let fog_color = vec3<f32>(0.5, 0.5, 0.8);

    var albedo: vec3<f32>;
    if (hit.y == 0.0) {
        let grid_ray_src = ray_src * vec3<f32>(1.0, 0.0, 1.0);
        let grid_mat = grid(frag_pos, grid_ray_src, ray_dir, ray_del.dx, ray_del.dy, 0.2);
        albedo = vec3<f32>(0.05) + grid_mat * vec3<f32>(0.05) * vec3<f32>(grid_mat, grid_mat, grid_mat);

    // Object
    } elseif (hit.y > 0.0) {
        let obj_color: vec3<f32> = vec3<f32>(0.4, 0.2, 0.4);
        albedo = obj_color;

    // Sky
    } else {
        let sky_col = sky_dome(fog_color, 0.8, 0.5, ray_dir);
        albedo = sky_col;
    }

    var normal: vec3<f32>;
    if (hit.y <= 0.0) {
        normal = vec3<f32>(0.0, 1.0, 0.0);
    } else {
        normal = scene_normal(frag_pos);
    }

    var frag_color: vec3<f32>;
    if (hit.y < 0.0) {
        frag_color = albedo;
    } else {
        let reflected_ray = normalize(reflect(ray_dir, normal));
        // let reflected_hit = cast_ray(frag_pos, reflected_ray);
        let occlusion = scene_occlusion(frag_pos, normal);

        // let ray_del = ray_delta(in.screen_pos, quad.dimensions, camera.inv_view_proj, camera.clip.y - camera.clip.x);
        // let grid_ray_src = ray_src * vec3<f32>(1.0, 0.0, 1.0);
        // let grid_mat = grid(frag_pos, grid_ray_src, ray_dir, ray_del.dx, ray_del.dy, 0.2);
        // obj_color = vec3<f32>(grid_mat, grid_mat, grid_mat);
        var shine: f32 = 1.0;
        if (hit.y == 0.0) {
            shine = 0.2;
        }

        frag_color = sky_light(albedo, shine, vec3<f32>(0.5, 0.5, 0.7), normal, ray_dir, reflected_ray, occlusion);
        frag_color = sun_light(frag_color, normal, ray_dir);

        // if (reflected_hit.g == 0.0) {
        //     let frag_pos_2: vec3<f32> = frag_pos + reflected_hit.r * reflected_ray;
        //     let ray_del_2: vec3<f32> = vec3<f32>(0.0); // bruh paper
        //     let grid_ray_src = frag_pos_2 * vec3<f32>(1.0, 0.0, 1.0);
        //     let grid_mat = grid(frag_pos_2, grid_ray_src, reflected_ray, ray_del.dx, ray_del.dy, 0.2);
        //     let grid_col = vec3<f32>(0.05) + grid_mat * vec3<f32>(0.05) * vec3<f32>(grid_mat, grid_mat, grid_mat);
        //     frag_color = frag_color * grid_col;
        // }
    }

    var pixel_color: vec3<f32> = clamp(mix(frag_color, fog_color, vec3<f32>(fog_amount)), vec3<f32>(0.0), vec3<f32>(1.0));
    return pixel_color;
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
    let ray_del = ray_delta(in.screen_pos*quad.dimensions, quad.dimensions, camera.inv_view_proj, camera.clip.y - camera.clip.x);

    var total: vec3<f32> = vec3<f32>(0.0);
    let multisampling = 1;
    let spread: f32 = 0.01;

    for(var mx: i32 = 0; mx < multisampling; mx = mx + 1) {
        for(var my: i32 = 0; my < multisampling; my = my + 1) {
            let offset = vec2<f32>(f32(mx), f32(my)) / f32(multisampling) - 0.5;
            let m_ray_src = ray_src + vec3<f32>(offset.x, 0.0, offset.y) * spread;
            total = total + render(m_ray_src, ray_dir, ray_del);
        }
    }

    total = total / f32(multisampling * multisampling);


    
    // gamma correction
    // pixel_color = pow(pixel_color, vec3<f32>(0.4545));
    return vec4<f32>(total, 1.0);
}