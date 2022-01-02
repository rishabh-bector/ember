// --------------------------------------------------
// Common
// -------------------------------------------------

[[block]]
struct RenderPBRUniforms {
    model_mat: mat4x4<f32>;
    normal_mat: mat4x4<f32>;
    color: vec4<f32>;
    mix: f32;
};

[[block]]
struct Camera3DUniforms {
    view_pos: vec4<f32>;
    view_proj: mat4x4<f32>;
};

// [[block]]
// struct Light3DUniforms {
//     // [x, y, linear, quadratic]
//     light_0: vec4<f32>;
//     light_1: vec4<f32>;
//     light_2: vec4<f32>;
//     light_3: vec4<f32>;
//     light_4: vec4<f32>;
// };

[[group(1), binding(0)]]
var<uniform> render_pbr_uniforms: RenderPBRUniforms;

[[group(2), binding(0)]]
var<uniform> camera_uniforms: Camera3DUniforms;

// [[group(3), binding(0)]]
// var<uniform> light_uniforms: LightUniforms;

// --------------------------------------------------
// Vertex shader
// --------------------------------------------------

struct VertexInput {
    [[location(0)]] position: vec3<f32>;
    [[location(1)]] uvs: vec2<f32>;
    [[location(2)]] normal: vec3<f32>;
};

struct VertexOutput {
    [[builtin(position)]] clip_position: vec4<f32>;
    [[location(0)]] uvs: vec2<f32>;
    [[location(1)]] world_pos: vec3<f32>;
    [[location(2)]] world_normal: vec3<f32>;
};

[[stage(vertex)]]
fn main(
    in: VertexInput,
) -> VertexOutput {
    var world_space: vec4<f32> = render_pbr_uniforms.model_mat * vec4<f32>(in.position, 1.0);
    var camera_space: vec4<f32> = camera_uniforms.view_proj * world_space;

    let normal_matrix = mat3x3<f32>(
        render_pbr_uniforms.normal_mat.x.xyz,
        render_pbr_uniforms.normal_mat.y.xyz,
        render_pbr_uniforms.normal_mat.z.xyz,
    );

    var out: VertexOutput;
    out.uvs = in.uvs;
    out.clip_position = camera_space;

    out.world_pos = world_space.xyz;
    out.world_normal = normalize(normal_matrix * in.normal);

    return out;
}

// -------------------------------------------------
// Fragment shader
// -------------------------------------------------

[[group(0), binding(0)]]
var texture0: texture_2d<f32>;
[[group(0), binding(1)]]
var sampler0: sampler;

[[group(3), binding(0)]]
var sky_cube: texture_cube<f32>;
[[group(3), binding(1)]]
var sky_sampler: sampler;

[[group(4), binding(0)]]
var sky_cube_blur: texture_cube<f32>;
[[group(4), binding(1)]]
var sky_sampler_blur: sampler;

// ----- HIGH-PERFORMANCE IRRADIANCE (IBL) -----
// Implementation based on http://graphics.stanford.edu/papers/envmap/envmap.pdf

struct SHCoefficients {
    l00: vec3<f32>;
    l1m1: vec3<f32>;
    l10: vec3<f32>;
    l11: vec3<f32>;
    l2m2: vec3<f32>;
    l2m1: vec3<f32>;
    l20: vec3<f32>;
    l21: vec3<f32>;
    l22: vec3<f32>;
};

let SHC: SHCoefficients = SHCoefficients(
    vec3<f32>(0.4167677, 0.41648358, 0.38331264),
	vec3<f32>(-0.0043605487, -0.0026134395, -0.0006568894),
	vec3<f32>(-0.01213964, -0.008434562, 0.023041306),
	vec3<f32>(0.46987548, 0.4635618, 0.42295167),
	vec3<f32>(0.015393221, 0.015422308, 0.010281778),
	vec3<f32>(-0.011692239, -0.014198665, -0.019392435),
	vec3<f32>(0.27746662, 0.27147454, 0.24605234),
	vec3<f32>(-0.00097278244, 0.010546771, 0.045822047),
	vec3<f32>(0.3920225, 0.36590222, 0.32920602)
);

fn sh_irradiance(nrm: vec3<f32>) -> vec3<f32> {
    let c = SHC;
    let c1 = 0.429043;
	let c2 = 0.511664;
	let c3 = 0.743125;
	let c4 = 0.886227;
	let c5 = 0.247708;

    return c1 * c.l22 * (nrm.x * nrm.x - nrm.y * nrm.y) +
		c3 * c.l20 * nrm.z * nrm.z +
		c4 * c.l00 -
		c5 * c.l20 +
		2.0 * c1 * c.l2m2 * nrm.x * nrm.y +
		2.0 * c1 * c.l21  * nrm.x * nrm.z +
		2.0 * c1 * c.l2m1 * nrm.y * nrm.z +
		2.0 * c2 * c.l11  * nrm.x +
		2.0 * c2 * c.l1m1 * nrm.y +
		2.0 * c2 * c.l10  * nrm.z;
}
// ----- HIGH PERFORMANCE BRDF
// Implementation based on https://www.unrealengine.com/en-US/blog/physically-based-shading-on-mobile
// 
// The bidirectional reflectance distribution function describes 
// how light reflects off an opaque surface with a given roughness.
fn env_brdf_approx(specular: vec3<f32>, roughness: f32, ndotv: f32) -> vec3<f32> {
	let c0 = vec4<f32>(-1.0, -0.0275, -0.572, 0.022);
	let c1 = vec4<f32>(1.0, 0.0425, 1.04, -0.04);
	let r = roughness * c0 + c1;
	let a004 = min(r.x * r.x, exp2(-9.28 * ndotv)) * r.x + r.y;
	let AB = vec2<f32>(-1.04, 1.04) * a004 + r.zw;
	return specular * AB.x + AB.y;
}

// Lighting

fn visibility_term(roughness: f32, ndotv: f32, ndotl: f32) -> f32 {
    let r2 = roughness * roughness;
	let gv = ndotl * sqrt(ndotv * (ndotv - ndotv * r2) + r2);
	let gl = ndotv * sqrt(ndotl * (ndotl - ndotl * r2) + r2);
	return 0.5 / max(gv + gl, 0.00001);
}

let MATH_PI: f32 = 3.14159;

fn distribution_term(roughness: f32, ndoth: f32) -> f32 {
	let r2 = roughness * roughness;
	let d = (ndoth * r2 - ndoth) * ndoth + 1.0;
	return r2 / (d * d * MATH_PI);
}

fn fresnel_term(specular_color: vec3<f32>, vdoth: f32) -> vec3<f32> {
	let fresnel = specular_color + (1.0 - specular_color) * pow((1.0 - vdoth), 5.0);
	return fresnel;
}

fn ambient_occlusion() -> f32 {
    return 1.0;
}

fn clamp0(in: vec3<f32>) -> vec3<f32> {
    return clamp(in, vec3<f32>(0.0, 0.0, 0.0), vec3<f32>(1.0, 1.0, 1.0));
}

fn clampf(in: f32) -> f32 {
    return clamp(in, 0.0, 1.0);
}

fn remap(in: vec3<f32>) -> vec3<f32> {
    return pow(2.0 * in, vec3<f32>(2.2, 2.2, 2.2));
}

[[stage(fragment)]]
fn main(in: VertexOutput) -> [[location(0)]] vec4<f32> {    
    var sample_texture: vec4<f32> = textureSample(texture0, sampler0, in.uvs);
    var sample_final: vec4<f32> = (render_pbr_uniforms.color * (1.0 - render_pbr_uniforms.mix)) + (render_pbr_uniforms.mix * sample_texture);

    let light_color = vec3<f32>(0.65, 0.65, 0.6);
    let light_dir = vec3<f32>(0.0, 0.3, 1.0);

    let base_color = pow(sample_final.rgb, vec3<f32>(2.0, 2.0, 2.0));
    let metal = false;

    var diffuse_color: vec3<f32> = vec3<f32>(0.0, 0.0, 0.0);
    if (!metal) {
        diffuse_color = base_color;
    }

    var specular_color: vec3<f32> = base_color;
    if (!metal) {
        specular_color = vec3<f32>(0.02, 0.02, 0.02);
    }

    let roughness = 0.3;

    let roughnessE = roughness * roughness;
	let roughnessL = max(0.01, roughnessE);

    let normal = normalize(in.world_normal);
    let view_dir = normalize(in.world_pos - camera_uniforms.view_pos.xyz);
    let half_vec = normalize(view_dir + light_dir);
    
    let vdoth = clampf(dot(view_dir, half_vec));
    let ndoth = clampf(dot(normal, half_vec));
    let ndotv = clampf(dot(normal, view_dir));
    let ndotl = clampf(dot(normal, light_dir));

    let env_specular_color = env_brdf_approx(specular_color, roughnessE, ndotv);
    let refl = normalize(reflect(view_dir, normal));

    let env_sample_clear = textureSample(sky_cube, sky_sampler, refl).xyz;
    let env_sample_blur = textureSample(sky_cube_blur, sky_sampler_blur, refl).xyz;
    let env_refl_irrad = remap(sh_irradiance(refl));
    var env: vec3<f32> = mix(env_sample_clear, env_sample_blur, vec3<f32>(clampf(roughnessE * 4.0)));
    env = mix(env, env_refl_irrad, vec3<f32>(clampf((roughnessE - 0.25) / 0.75)));

    let irradiance = remap(sh_irradiance(normal));
    let ao: f32 = ambient_occlusion();

    // DIRECTIONAL LIGHT

    let light_fresnel: vec3<f32> = fresnel_term(specular_color, vdoth);
    let light_distribution: f32 = distribution_term(roughnessL, ndoth);
    let light_visibility: f32 = visibility_term(roughnessL, ndotv, ndotl);
    let specular_light = light_color * light_fresnel * (light_distribution * light_visibility * ndotl * 0.01);

    //
    // FINAL COLOR = DIFFUSE (light scattered from environment) + SPECULAR (light reflected at definite angle)
    // 
    // METALS have NO DIFFUSE component because all light is REFLECTED at a DEFINITE ANGLE.
    // DIELECTRICS have both DIFFUSE and SPECULAR components.
    //
    // BOTH DIFFUSE and SPECULAR components depend on the ENVIRONMENT (aka IRRADIANCE) and ROUGHNESS.
    // THE SPECULAR component uses a BRDF.
    //

    // [(material color)(environment irradiance via normal irradiance) + (material color)(light color)(light angle)][ambient occlusion]
    let diffuse: vec3<f32> = (diffuse_color * irradiance + diffuse_color * light_color * clampf(dot(normal, light_dir))) * ao;

    // [(material color via BRDF)(environment irradiance via blur lerping and reflected irradiance) + (light color via fresnel lighting)][ambient occlusion]
    let specular: vec3<f32> = (env_specular_color * env + specular_light) * clampf(pow(ndotv + ao, roughnessE) - 1.0 + ao);

    let color = diffuse + specular;
    let gamma_corrected = pow(color * 0.4, vec3<f32>(1.0 / 2.2));

    return vec4<f32>(color, 1.0);
}