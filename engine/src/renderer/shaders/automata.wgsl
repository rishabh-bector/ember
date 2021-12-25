// --------------------------------------------------
// Common
// -------------------------------------------------

[[block]]
struct QuadUniforms {
    dimensions: vec2<f32>;
    time: f32;
    delta: f32;
};

[[group(1), binding(0)]]
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
    out.screen_pos = vec2<f32>((in.position.x / 2.0) + 0.5, (1.0 - ((in.position.y / 2.0) + 0.5)));

    return out;
}

// --------------------------------------------------
// Fragment shader
// -------------------------------------------------

let SCREEN_WIDTH: u32 = 3840u;
let SCREEN_HEIGHT: u32 = 2160u;
let SCREEN_SIZE: vec2<f32> = vec2<f32>(3840.0, 2160.0);

let CELL_SIZE: f32 = 2.0;

[[group(0), binding(0)]]
var node_input_tex: texture_2d<f32>;
[[group(0), binding(1)]]
var node_input_smp: sampler;

fn random(pos: vec2<f32>) -> f32 {
    return fract(sin(dot(pos.xy, vec2<f32>(12.9898,78.233)))*43758.5453123);
}

fn sample(uvs: vec2<f32>) -> bool {
    if (textureSample(node_input_tex, node_input_smp, uvs).r > 0.5) {
        return true;
    } else {
        return false;
    }
}

fn sample_cell(pos: vec2<f32>) -> bool {
    return sample(pos / SCREEN_SIZE * CELL_SIZE);
}

fn n_square_avg(cell_pos: vec2<f32>, rad: f32) -> f32 {
    var total: f32 = 0.0;
    var n: f32 = 0.0;

    for (var x: f32 = cell_pos.x - rad; x < cell_pos.x + rad; x = x + 1.0) {
        for (var y: f32 = cell_pos.y - rad; y < cell_pos.y + rad; y = y + 1.0) {
            let samp: bool = sample_cell(vec2<f32>(x, y));
            if (samp) {
                total = total + 1.0;
            }
            n = n + 1.0;
        }
    }

    return total / n;
}

fn n_square_cnt(cell_pos: vec2<f32>, rad: f32) -> u32 {
    var total: u32 = 0u;

    for (var x: f32 = cell_pos.x - rad; x <= cell_pos.x + rad; x = x + 1.0) {
        for (var y: f32 = cell_pos.y - rad; y <= cell_pos.y + rad; y = y + 1.0) {
            if (x == cell_pos.x && y == cell_pos.y) {
                continue;
            }
            let samp: bool = sample_cell(vec2<f32>(x, y));
            if (samp) {
                total = total + 1u;
            }
        }
    }

    return total;
}

[[stage(fragment)]]
fn main(in: VertexOutput) -> [[location(0)]] vec4<f32> {
    let frag_uvs: vec2<f32> = in.screen_pos;
    let previous: vec4<f32> = textureSample(node_input_tex, node_input_smp, frag_uvs);
    let state: bool = sample(frag_uvs);

    let screen_pos: vec2<f32> = frag_uvs * vec2<f32>(f32(SCREEN_WIDTH), f32(SCREEN_HEIGHT));
    let cell_pos: vec2<f32> = round(screen_pos / vec2<f32>(CELL_SIZE, CELL_SIZE));
    let cell_width: vec2<f32> = vec2<f32>(1.0 / f32(SCREEN_WIDTH), 1.0 / f32(SCREEN_HEIGHT));

    // initial condition
    if (quad.time < 10.0) {
        let r: f32 = random(cell_pos / 123.456);
        if (r > 0.6) {
            return vec4<f32>(1.0, 1.0, 1.0, 1.0);
        } else {
            return vec4<f32>(0.0, 0.0, 0.0, 0.0);
        }
    }

    // 7x7 neighborhood = 48 neighbors max
    let n0: u32 = n_square_cnt(cell_pos, 3.0);

    var out: vec4<f32> = previous;

    if (n0 >= 0u && n0 <= 9u) { out = vec4<f32>(0.0, 0.0, 0.0, 0.0); }
    if (n0 >= 16u && n0 <= 17u) { out = vec4<f32>(1.0, 1.0, 1.0, 1.0); }
    if (n0 >= 21u && n0 <= 48u && n0 != 30u && n0 != 31u) { out = vec4<f32>(0.0, 0.0, 0.0, 0.0); }
    // if (n0 >= 0u && n0 <= 13u) { out = vec4<f32>(0.0, 0.0, 0.0, 0.0); }
    // if (n0 >= 14u && n0 <= 16u) { out = vec4<f32>(1.0, 1.0, 1.0, 1.0); }
    // if (n0 >= 23u && n0 <= 48u) { out = vec4<f32>(0.0, 0.0, 0.0, 0.0); }

    return out;

    // main
    // if (state) {
    //     if (n0 >= 0 && n0 < 5) {
    //         out = vec4<f32>(1.0, 1.0, 1.0, 1.0); 
    //     } else {
    //         return vec4<f32>(0.0, 0.0, 0.0, 1.0);
    //     }
    // } else {
    //     if (n0 == 8u || n0 == 9u) {
    //         return vec4<f32>(1.0, 1.0, 1.0, 1.0); 
    //     } else {
    //         return vec4<f32>(0.0, 0.0, 0.0, 1.0);
    //     }
    // }

    // if (state) {
    //     if (n0 == 6u || n0 == 7u || n0 == 8u) {
    //         return vec4<f32>(1.0, 1.0, 1.0, 1.0); 
    //     } else {
    //         return vec4<f32>(0.0, 0.0, 0.0, 1.0);
    //     }
    // } else {
    //     if (n0 == 6u || n0 == 7u) {
    //         return vec4<f32>(1.0, 1.0, 1.0, 1.0); 
    //     } else {
    //         return vec4<f32>(0.0, 0.0, 0.0, 1.0);
    //     }
    // }
}