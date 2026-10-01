//! The shaders a gallery scene declares, and the parts of them that never change.
//!
//! A shader carries its own source, so a scene names a shader from here and the document
//! it produces is still self contained. That is what lets a scene be sent to a device that
//! cannot see the file it was written in.
//!
//! # Writing a shader against this pipeline
//!
//! Three layouts have to be matched, and only one of them is checked. The device parses a
//! shader back with `naga` and compares its uniform struct against the interface the
//! shader declared, so a uniform that disagrees is a render error naming both offsets
//! rather than a frame of the wrong numbers. The other two are not checked, so they are
//! stated here once rather than left to each shader:
//!
//! - **The vertex attributes.** A point schema sorted by name lays out locations from
//!   zero, and the instance row continues from where the point's ended. A row of
//!   `emissive`, `metallic`, `model`, `roughness`, `tint` sorts into that order, so the
//!   model matrix is read as four consecutive locations.
//! - **The bindings.** The uniform is binding zero, and every bound resource follows it in
//!   name order. A texture a pass renders *into* is an attachment and is not bound, so
//!   the lighting stage binds the depth buffer at one and the shadow map at two, and the
//!   post stage binds the shaded image at one and the depth at two.

use definition::{Schema, SchemaKind, Value};

use crate::standard::{
    Stage, depth_pass_schema, light_pass_schema, post_pass_schema, shadow_pass_schema,
};

use super::kit;

/// The uniform the shadowing and depth stages read.
///
/// Only the projection and the time, because that is all a pass of those stages supplies.
/// The device parses this shader back with `naga` and compares its uniform struct against
/// the interface the shader declared, so declaring a field the pass does not supply is a
/// render error rather than a frame of the wrong numbers.
const PLACEMENT_UNIFORM: &str = r#"
struct Projection {
    position: vec3<f32>,
    view_projection: mat4x4<f32>,
};

struct Uniforms {
    projection: Projection,
    time: f32,
};
"#;

/// The uniform the lighting stage reads.
///
/// The fields are in the order the interface schema sorts them into, which is by name:
/// `light_color`, `light_position`, `light_projection`, `light_shadow_texel`, `projection`,
/// `time`. The order is not cosmetic — a three component vector is padded to sixteen
/// bytes, so the field written second starts at sixteen either way, and a struct written
/// in any other order puts the wrong thing there.
///
/// `light_projection` is the light's own view projection rather than the camera's: a
/// surface is in shadow when it is further from the light than the shadow map recorded,
/// and deciding that means carrying the surface's world position into the light's clip
/// space. The example that shipped before this had only the camera's, and sampled the
/// shadow map at the fragment's own screen coordinate, which is a different lookup and not
/// a shadow.
const LIGHTING_UNIFORM: &str = r#"
struct Projection {
    position: vec3<f32>,
    view_projection: mat4x4<f32>,
};

struct Uniforms {
    light_color: vec3<f32>,
    light_position: vec3<f32>,
    light_projection: mat4x4<f32>,
    light_shadow_texel: vec2<f32>,
    projection: Projection,
    time: f32,
};
"#;

/// The shadowing stage: the point is placed, and the fragment writes depth through the
/// target itself, so the stage declares no fragment entry point.
///
/// `vertex_inputs` is the attribute list [`crate::gallery::attributes`] writes out from the
/// two schemas, rather than a transcription of it.
pub fn shadowing(vertex_inputs: &str) -> String {
    format!(
        r#"{PLACEMENT_UNIFORM}
@group(0) @binding(0) var<uniform> uniforms: Uniforms;

@vertex
fn vertex_main(
{vertex_inputs}) -> @builtin(position) vec4<f32> {{
    let model = mat4x4<f32>(model0, model1, model2, model3);
    return uniforms.projection.view_projection * model * vec4<f32>(position, 1.0);
}}

@fragment
fn fragment_main() -> @location(0) vec4<f32> {{
    return vec4<f32>(1.0, 1.0, 1.0, 1.0);
}}
"#
    )
}

/// The depth stage: the same placement, and no fragment stage at all.
///
/// `vertex_inputs` is the attribute list [`crate::gallery::attributes`] writes out, and it is
/// the same list the shadowing stage was given: a stage that places a point differently
/// places it differently, and nothing downstream can tell by how much.
pub fn depthing(vertex_inputs: &str) -> String {
    format!(
        r#"{PLACEMENT_UNIFORM}
@group(0) @binding(0) var<uniform> uniforms: Uniforms;

@vertex
fn vertex_main(
{vertex_inputs}) -> @builtin(position) vec4<f32> {{
    let model = mat4x4<f32>(model0, model1, model2, model3);
    return uniforms.projection.view_projection * model * vec4<f32>(position, 1.0);
}}
"#
    )
}

/// The lighting stage, which shades by what the earlier passes wrote.
///
/// `helpers` is WGSL a scene needs beyond the shading itself — a noise, a curve — written
/// as functions. `WGSL` has no nested functions, so a scene's helpers cannot go in
/// `surface`, which is written into the body of one; they are declared beside it here.
///
/// `surface` is the shading of a surface, written against the varyings below. It is a
/// separate argument because what a surface looks like is the scene's business and how it
/// is lit is not: the shadow term, the falloff and the Fresnel are the same for every
/// surface, and a scene that rewrote them would get one of them subtly wrong.
///
/// `vertex_inputs` is the attribute list [`crate::gallery::attributes`] writes out from the
/// two schemas the stages and the mesh all declare, so a scene that changes its point schema
/// gets the attributes for it without writing them out.
pub fn lighting(vertex_inputs: &str, surface: &str, helpers: &str) -> String {
    format!(
        r#"{LIGHTING_UNIFORM}
@group(0) @binding(0) var<uniform> uniforms: Uniforms;
@group(0) @binding(1) var depth_buffer: texture_depth_2d;
@group(0) @binding(2) var shadow_map: texture_depth_2d;

/// Returns how lit a surface is by comparing its depth in the light's own space against
/// what the shadow map recorded there.
///
/// A nine tap kernel is what turns a hard staircase into an edge. The taps are taken by
/// `textureLoad` rather than through a sampler because a binding carries a texture and
/// nothing else: there is no sampler to filter with, and a depth compare has to be done
/// by hand in any case.
fn shadow(world_position: vec3<f32>, normal: vec3<f32>) -> f32 {{
    // A light here is parallel, so the point a surface is looked up at is displaced from
    // the surface itself by a constant in world space. Displacing it along the normal
    // rather than along the light is what keeps the shadow attached: a surface is pushed
    // away from the map by the same amount everywhere, so a slope and a flat both move by
    // one shadow-map texel and neither detaches.
    let lookup = world_position + normal * 1.6;
    let from_light = uniforms.light_projection * vec4<f32>(lookup, 1.0);
    // A point behind the light has no shadow, and dividing by a negative w would fold it
    // back into the map from behind.
    if from_light.w <= 0.0 {{
        return 1.0;
    }}
    let ndc = from_light.xyz / from_light.w;
    // Clip space y runs up and texture space runs down.
    let uv = ndc.xy * vec2<f32>(0.5, -0.5) + vec2<f32>(0.5, 0.5);
    if any(uv < vec2<f32>(0.0)) || any(uv > vec2<f32>(1.0)) {{
        return 1.0;
    }}
    let size = vec2<f32>(textureDimensions(shadow_map));
    // A surface is its own depth and the recorded value is the depth of whatever was
    // nearest the light, so the comparison is biased. The bias scales with how obliquely
    // the light strikes the surface, because a surface's own rasterised faces disagree
    // with its true depth by more the further they lean away from the light, and one
    // constant is either too small for the leaning surfaces or detaches the shadow from
    // the flat ones. The normal offset above has already taken the large part of that
    // error, so what is left here is small.
    let to_light = normalize(uniforms.light_position - world_position);
    let slope = 1.0 - abs(dot(normal, to_light));
    let z = ndc.z - (0.0006 + 0.003 * slope);
    var lit = 0.0;
    for (var row = -1; row <= 1; row = row + 1) {{
        for (var column = -1; column <= 1; column = column + 1) {{
            let offset = vec2<f32>(f32(column), f32(row)) * uniforms.light_shadow_texel;
            let coordinate = vec2<i32>(uv * size + offset);
            lit = lit + select(0.0, 1.0, z <= textureLoad(shadow_map, coordinate, 0));
        }}
    }}
    return lit / 9.0;
}}

{helpers}
struct Vertex {{
    @builtin(position) position: vec4<f32>,
    @location(0) world_position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) texture: vec2<f32>,
    @location(3) emissive: vec3<f32>,
    @location(4) tint: vec3<f32>,
    @location(5) material: vec3<f32>,
}};

@vertex
fn vertex_main(
{vertex_inputs}) -> Vertex {{
    let model = mat4x4<f32>(model0, model1, model2, model3);
    // A normal is carried through the inverse transpose of the model, which the emitter
    // fills in beside the model itself. A transform that scales unevenly tilts a normal
    // by exactly the amount it is uneven by, so an instance twenty times taller than it
    // is wide is lit wrongly by the model alone — and `WGSL` has no matrix inverse for a
    // shader to recover it with.
    let normals = mat4x4<f32>(normal_matrix0, normal_matrix1, normal_matrix2, normal_matrix3);
    let world = model * vec4<f32>(position, 1.0);
    var out: Vertex;
    out.position = uniforms.projection.view_projection * world;
    out.world_position = world.xyz;
    out.normal = normalize((normals * vec4<f32>(normal, 0.0)).xyz);
    out.texture = texture;
    out.emissive = emissive;
    out.tint = tint;
    out.material = vec3<f32>(metallic, roughness, reflect);
    return out;
}}

@fragment
fn fragment_main(in: Vertex) -> @location(0) vec4<f32> {{
    let normal = normalize(in.normal);
    let to_light = uniforms.light_position - in.world_position;
    let light_range = max(length(to_light), 0.0001);
    let direction = to_light / light_range;
    let lit = shadow(in.world_position, normal);

    // A point light falls off with the square of its distance. The constants are the
    // gentlest that still read as a falloff over a scene a couple of hundred units
    // across: a steeper one puts the light's own volume inside the scene, and every
    // surface near the light then reads as white no matter what its colour is.
    let attenuation = 1.0 / (1.0 + 0.0035 * light_range + 0.00002 * light_range * light_range);
    let radiance = uniforms.light_color * attenuation * lit;

    let metallic = in.material.x;
    let roughness = clamp(in.material.y, 0.04, 1.0);
    let diffuse_color = in.tint * (1.0 - metallic);

    let lambert = max(dot(normal, direction), 0.0);
    let to_view = normalize(uniforms.projection.position - in.world_position);
    let half_vector = normalize(direction + to_view);
    let normal_dot_view = max(dot(normal, to_view), 0.0);

    // A metal takes its colour from what it reflects and has no diffuse term; a dielectric
    // reflects about four percent at normal incidence, rising to one at grazing angles,
    // which is the Fresnel term below.
    let f0 = mix(vec3<f32>(0.04), in.tint, metallic);
    let fresnel = f0 + (vec3<f32>(1.0) - f0) * pow(1.0 - normal_dot_view, 5.0);
    // A rough surface spreads its highlight, so the exponent of the lobe follows the
    // roughness rather than being a constant.
    let exponent = mix(512.0, 4.0, roughness);
    let specular = pow(max(dot(normal, half_vector), 0.0), exponent)
        * mix(0.02, 1.0, (1.0 - roughness) * (1.0 - roughness));

    // A hemisphere of ambient light, brighter from above, which stands in for the
    // environment a surface would otherwise have nothing to reflect.
    let sky = mix(
        vec3<f32>(0.014, 0.017, 0.026),
        vec3<f32>(0.055, 0.072, 0.110),
        normal.y * 0.5 + 0.5,
    );
    let ambient = sky * mix(vec3<f32>(1.0), in.tint, metallic);

    var color = diffuse_color * lambert * radiance;
    color = color + fresnel * specular * radiance * 3.0;
    color = color + ambient * diffuse_color;
    color = color + in.emissive;
{surface}
    return vec4<f32>(color, 1.0);
}}
"#
    )
}

/// The post stage, which draws over the whole frame.
///
/// The three points arrive already in clip space, so the vertex stage passes them
/// through: a post stage is not drawn through the camera's projection, it *is* the frame.
/// The fullscreen mesh is a position and nothing else, and the pass binds no instance row,
/// so the only attribute is at location zero.
///
/// `helpers` is WGSL a scene needs beyond the grade itself — a sky, a fog, a noise —
/// declared at module level beside `ray_through`, because `WGSL` has no nested functions
/// and `grade` is written into the body of one. It is the same arrangement the lighting
/// stage has, and for the same reason.
pub fn post(grade: &str, helpers: &str) -> String {
    format!(
        r#"
struct Camera {{
    forward: vec3<f32>,
    half_extent: vec2<f32>,
    position: vec3<f32>,
    right: vec3<f32>,
    up: vec3<f32>,
}};

struct Projection {{
    position: vec3<f32>,
    view_projection: mat4x4<f32>,
}};

struct Uniforms {{
    camera: Camera,
    depth_range: vec2<f32>,
    projection: Projection,
    time: f32,
}};

@group(0) @binding(0) var<uniform> uniforms: Uniforms;
@group(0) @binding(1) var color_target: texture_2d<f32>;
@group(0) @binding(2) var depth_buffer: texture_depth_2d;

struct Vertex {{
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
}};

@vertex
fn vertex_main(@location(0) position: vec3<f32>) -> Vertex {{
    var out: Vertex;
    out.position = vec4<f32>(position.xy, 0.0, 1.0);
    // Clip space runs from minus one to one with y up; texture space runs from zero to
    // one with y down, so the two differ by a scale and a flip.
    out.uv = position.xy * vec2<f32>(0.5, -0.5) + vec2<f32>(0.5, 0.5);
    return out;
}}

/// Returns the ray through a pixel of the frame.
///
/// # Invariants
///
/// A ray through a pixel is recovered from the camera's three axes and the half extent of
/// the frame, not from the projection matrix. Recovering it from the matrix would mean
/// inverting it, and `WGSL` has no matrix inverse — which is why the emitter hands a post
/// stage the basis at all.
///
/// The offset runs from minus one to one with **y up**, which is the way the basis is
/// defined. Texture space runs with y down, so the vertical component of the offset is
/// flipped; a sky built on the unflipped one is a sky with its zenith at the bottom of the
/// frame, which reads as a vignette rather than as a sky.
fn ray_through(uv: vec2<f32>) -> vec3<f32> {{
    let offset = vec2<f32>(uv.x * 2.0 - 1.0, 1.0 - uv.y * 2.0);
    return normalize(
        uniforms.camera.forward
        + uniforms.camera.right * offset.x * uniforms.camera.half_extent.x
        + uniforms.camera.up * offset.y * uniforms.camera.half_extent.y,
    );
}}

/// Returns how far from the camera a surface at this depth is, measured along the view.
///
/// # Invariants
///
/// A depth buffer holds a fraction of the range between the near and far planes, and that
/// fraction is linear in a divide rather than in the distance: `near * far` over
/// `far - depth * (far - near)`. The two planes are handed over rather than left to be
/// assumed, because a stage without them cannot divide at all — and `WGSL` cannot recover
/// them, for the same reason it cannot invert the matrix the basis stands in for.
///
/// The result is along the view axis rather than along the ray through the pixel, so a
/// caller walking through the air along a ray divides it by `dot(ray, forward)`.
fn distance_to(depth: f32) -> f32 {{
    let range = uniforms.depth_range;
    return range.x * range.y / (range.y - depth * (range.y - range.x));
}}

{helpers}
@fragment
fn fragment_main(in: Vertex) -> @location(0) vec4<f32> {{
    let size = vec2<f32>(textureDimensions(color_target));
    let pixel = vec2<i32>(in.uv * size);
    // The shaded image is read as colour only: its alpha carries whatever the shading
    // happened to leave there, and a grade applied to it is a grade of nothing.
    var color = textureLoad(color_target, pixel, 0).rgb;
    // A depth of one is where nothing was drawn, because the depth buffer is cleared to
    // the far plane. This is what lets a post stage be the sky: it draws over the whole
    // frame, and only the pixels the scene never reached are the ones left to it.
    let depth = textureLoad(depth_buffer, pixel, 0);
{grade}
    return vec4<f32>(color, 1.0);
}}
"#
    )
}

/// Builds the three mesh shaders every gallery scene declares, and the point schema they
/// all read.
///
/// # Invariants
///
/// The attributes are derived from the point schema this passes, so the three stages and
/// the mesh a scene draws all read one layout, and a scene that declares a different point
/// schema gets the attributes for it without writing them out.
pub fn mesh_shaders(surface: &str, helpers: &str) -> Vec<Value> {
    let point = kit::shaded_point_schema();
    let inputs = crate::gallery::attributes::gallery_vertex_inputs(&point);
    vec![
        kit::shader(
            Stage::Shadowing,
            "shadowing",
            &shadowing(&inputs),
            shadow_pass_schema(),
            point.clone(),
            kit::gallery_instance_schema(),
            kit::gallery_custom_schema(),
        ),
        kit::shader(
            Stage::Depthing,
            "depthing",
            &depthing(&inputs),
            depth_pass_schema(),
            point.clone(),
            kit::gallery_instance_schema(),
            kit::gallery_custom_schema(),
        ),
        kit::shader(
            Stage::Lighting,
            "lighting",
            &lighting(&inputs, surface, helpers),
            light_pass_schema(),
            point,
            kit::gallery_instance_schema(),
            kit::gallery_custom_schema(),
        ),
    ]
}

/// Builds the post shader a scene declares, which grades the frame the scene produced.
///
/// `helpers` is handed to [`post`] unchanged: the WGSL the grade's arithmetic is written
/// against, declared at module level.
pub fn post_shader(grade: &str, helpers: &str) -> Value {
    // A screen shader's variant carries a `stage` naming the stage it belongs to, which
    // the three mesh variants do not: they are named by their own tag, and a fourth one
    // that was not would leave a document with no way to say which stage a shader reads.
    Value::variant(
        Stage::Post.tag(),
        [
            (
                crate::standard::LABEL.to_owned(),
                Value::String("post".to_owned()),
            ),
            (
                crate::standard::SOURCE.to_owned(),
                Value::String(post(grade, helpers)),
            ),
            (
                crate::standard::PASS_SCHEMA.to_owned(),
                post_pass_schema().to_value(),
            ),
            // A post stage draws over the frame rather than over a mesh, so it declares no
            // point schema and no instance row. The emitter refuses a screen shader that
            // carries either, because a shader that declares an interface nothing supplies
            // is a shader whose layout nothing can check.
            (
                crate::standard::INSTANCE_SCHEMA.to_owned(),
                Schema::new(SchemaKind::Unit).to_value(),
            ),
            (
                crate::standard::CUSTOM_SCHEMA.to_owned(),
                Schema::new(SchemaKind::Unit).to_value(),
            ),
            ("stage".to_owned(), Value::String("post".to_owned())),
        ],
    )
}
