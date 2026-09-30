//! The WGSL a gallery scene draws with.
//!
//! A shader carries its own source, so a scene that names one from this library still
//! produces a self contained document. That is what lets a scene be sent to a device that
//! cannot see the file it was written in.
//!
//! # Invariants
//!
//! - The uniform struct of a shader is written in the order its interface schema sorts
//!   its fields into, because that is the order the packer writes them at and the order
//!   the layout check compares them against. Getting it wrong is a render error that
//!   names the field and both offsets, rather than a wrong image.
//! - The instance row is read at the locations its merged schema lays out to: the
//!   per vertex attributes of the point schema first, then the per instance ones. Unlike
//!   the uniform, this layout is not checked, so it is written out here in one place.
//! - A binding point a pass renders into is an attachment and is not bound. Every other
//!   texture is numbered from one in name order, with the uniform at zero.

/// The shadowing stage: the point is placed, and the fragment writes depth through the
/// target itself, so the stage declares no fragment entry point.
pub const SHADOWING: &str = r#"
struct Projection {
    position: vec3<f32>,
    view_projection: mat4x4<f32>,
};

struct Uniforms {
    light_color: vec3<f32>,
    light_position: vec3<f32>,
    projection: Projection,
};

@group(0) @binding(0) var<uniform> uniforms: Uniforms;

@vertex
fn vertex_main(
    @location(0) normal: vec3<f32>,
    @location(1) position: vec3<f32>,
    @location(2) texture: vec2<f32>,
    @location(3) emissive: vec3<f32>,
    @location(4) metallic: f32,
    @location(5) m0: vec4<f32>,
    @location(6) m1: vec4<f32>,
    @location(7) m2: vec4<f32>,
    @location(8) m3: vec4<f32>,
    @location(9) roughness: f32,
    @location(10) tint: vec3<f32>,
) -> @builtin(position) vec4<f32> {
    let model = mat4x4<f32>(m0, m1, m2, m3);
    return uniforms.projection.view_projection * model * vec4<f32>(position, 1.0);
}
"#;

/// The depth stage: the same placement, and no fragment stage at all.
pub const DEPTHING: &str = r#"
struct Projection {
    position: vec3<f32>,
    view_projection: mat4x4<f32>,
};

struct Uniforms {
    light_color: vec3<f32>,
    light_position: vec3<f32>,
    projection: Projection,
};

@group(0) @binding(0) var<uniform> uniforms: Uniforms;

@vertex
fn vertex_main(
    @location(0) normal: vec3<f32>,
    @location(1) position: vec3<f32>,
    @location(2) texture: vec2<f32>,
    @location(3) emissive: vec3<f32>,
    @location(4) metallic: f32,
    @location(5) m0: vec4<f32>,
    @location(6) m1: vec4<f32>,
    @location(7) m2: vec4<f32>,
    @location(8) m3: vec4<f32>,
    @location(9) roughness: f32,
    @location(10) tint: vec3<f32>,
) -> @builtin(position) vec4<f32> {
    let model = mat4x4<f32>(m0, m1, m2, m3);
    return uniforms.projection.view_projection * model * vec4<f32>(position, 1.0);
}
"#;

/// The lighting stage: shades by the normal, the light and the material.
///
/// The instance row carries the model matrix and the material, so two instances of one
/// mesh drawn by one pass differ only in the row they are drawn from.
pub const LIGHTING: &str = r#"
struct Projection {
    position: vec3<f32>,
    view_projection: mat4x4<f32>,
};

struct Uniforms {
    light_color: vec3<f32>,
    light_position: vec3<f32>,
    projection: Projection,
};

@group(0) @binding(0) var<uniform> uniforms: Uniforms;
@group(0) @binding(1) var depth_buffer: texture_depth_2d;
@group(0) @binding(2) var shadow_map: texture_depth_2d;

struct Material {
    albedo: vec3<f32>,
    emissive: vec3<f32>,
    metallic: f32,
    roughness: f32,
};

struct Vertex {
    @builtin(position) position: vec4<f32>,
    @location(0) normal: vec3<f32>,
    @location(1) world_position: vec3<f32>,
    @location(2) texture: vec2<f32>,
    @location(3) emissive: vec3<f32>,
    @location(4) tint: vec3<f32>,
    @location(5) material: vec2<f32>,
};

@vertex
fn vertex_main(
    @location(0) normal: vec3<f32>,
    @location(1) position: vec3<f32>,
    @location(2) texture: vec2<f32>,
    @location(3) emissive: vec3<f32>,
    @location(4) metallic: f32,
    @location(5) m0: vec4<f32>,
    @location(6) m1: vec4<f32>,
    @location(7) m2: vec4<f32>,
    @location(8) m3: vec4<f32>,
    @location(9) roughness: f32,
    @location(10) tint: vec3<f32>,
) -> Vertex {
    let model = mat4x4<f32>(m0, m1, m2, m3);
    let world = model * vec4<f32>(position, 1.0);
    // A non uniform scale tilts a normal, so the normal is carried by the inverse
    // transpose of the model rather than by the model itself.
    let normal_matrix = transpose(inverse(model));
    var out: Vertex;
    out.position = uniforms.projection.view_projection * world;
    out.normal = (normal_matrix * vec4<f32>(normal, 0.0)).xyz;
    out.world_position = world.xyz;
    out.texture = texture;
    out.emissive = emissive;
    out.tint = tint;
    out.material = vec2<f32>(metallic, roughness);
    return out;
}

@fragment
fn fragment_main(in: Vertex) -> @location(0) vec4<f32> {
    let normal = normalize(in.normal);
    let to_light = uniforms.light_position - in.world_position;
    let distance = max(length(to_light), 0.0001);
    let direction = to_light / distance;

    // A point light falls off with the square of its distance. The falloff is damped
    // near the source so a surface the light is inside of does not blow out.
    let attenuation = 1.0 / (1.0 + 0.08 * distance + 0.02 * distance * distance);
    let lit = uniforms.light_color * attenuation;

    let metallic = in.material.x;
    let roughness = clamp(in.material.y, 0.03, 1.0);
    let albedo = in.tint;
    let diffuse_color = albedo * (1.0 - metallic);

    let lambert = max(dot(normal, direction), 0.0);
    let to_view = normalize(uniforms.projection.position - in.world_position);
    let half_vector = normalize(direction + to_view);
    let normal_dot_view = max(dot(normal, to_view), 0.0);

    // A metal takes its colour from what it reflects and has no diffuse term; a
    // dielectric reflects about four percent at normal incidence, rising to one at
    // grazing angles, which is the Fresnel term below.
    let f0 = mix(vec3<f32>(0.04), albedo, metallic);
    let fresnel = f0 + (vec3<f32>(1.0) - f0) * pow(1.0 - normal_dot_view, 5.0);

    // A rough surface spreads its highlight, so the exponent of the lobe follows the
    // roughness rather than being a constant.
    let exponent = mix(256.0, 4.0, roughness);
    let specular = pow(max(dot(normal, half_vector), 0.0), exponent)
        * mix(0.04, 1.0, (1.0 - roughness) * (1.0 - roughness));

    // A hemisphere of ambient light, brighter from above, which stands in for the
    // environment a metallic surface would otherwise have nothing to reflect.
    let sky = mix(vec3<f32>(0.03, 0.03, 0.04), vec3<f32>(0.10, 0.12, 0.16), normal.y * 0.5 + 0.5);
    let ambient = sky * mix(vec3<f32>(1.0), albedo, metallic);

    var color = diffuse_color * lambert * lit;
    color = color + fresnel * specular * lit * 4.0;
    color = color + ambient * diffuse_color;
    color = color + in.emissive;
    return vec4<f32>(color, 1.0);
}
"#;

/// The shader sources a gallery scene draws with, by stage.
pub struct ShaderLibrary;

impl ShaderLibrary {
    /// Returns the source of the shadowing stage.
    pub fn shadowing() -> &'static str {
        SHADOWING
    }

    /// Returns the source of the depth stage.
    pub fn depthing() -> &'static str {
        DEPTHING
    }

    /// Returns the source of the lighting stage.
    pub fn lighting() -> &'static str {
        LIGHTING
    }
}
