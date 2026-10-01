//! A small, valid standard scene, built from the schemas the compiler declares.
//!
//! The scene has one light, one mesh, and four instances of that mesh — one per stage, and two
//! in the lighting stage — so the pipeline it compiles to is the whole three stage pipeline, the
//! geometry is written once however many stages draw it, and the two lighting instances are drawn
//! together by one instanced draw.

use definition::{Schema, SchemaKind, Value};
use graphics::layout::components_schema;

use crate::standard::Stage;
use crate::standard::pass_schemas::{
    depth_pass_schema, emission_pass_schema, light_pass_schema, post_pass_schema,
    shadow_pass_schema,
};
use crate::standard::scene_schema::{default_custom_schema, default_instance_schema};

/// The point schema the example's one mesh declares.
///
/// A position and a normal, rather than the position alone the default carries. A normal
/// is what lets a fragment stage work out how steeply a surface faces the light, which is
/// how the bias a shadow comparison needs is scaled — a surface facing the light head on
/// needs almost none and a surface edge on needs a great deal, and one constant cannot be
/// right for both.
fn example_point_schema() -> Schema {
    Schema::new(SchemaKind::structure([
        ("normal".to_owned(), components_schema(3)),
        ("position".to_owned(), components_schema(3)),
    ]))
}

/// Builds the example scene: one light, one mesh, one shader per stage, and one instance
/// per stage.
pub fn example_standard_scene() -> Value {
    Value::structure([
        ("name".to_owned(), Value::String("studio".to_owned())),
        (
            "lights".to_owned(),
            Value::List(vec![Value::variant(
                "Point",
                [
                    ("label".to_owned(), Value::String("key".to_owned())),
                    (
                        "position".to_owned(),
                        Value::List([4.0, 6.0, 2.0].into_iter().map(Value::Float).collect()),
                    ),
                    (
                        "color".to_owned(),
                        Value::List([1.0, 0.95, 0.9].into_iter().map(Value::Float).collect()),
                    ),
                    ("intensity".to_owned(), Value::Float(1.0)),
                    ("shadow_extent".to_owned(), Value::Float(2.5)),
                ],
            )]),
        ),
        (
            "shaders".to_owned(),
            Value::List(vec![
                shader("shadowing", Stage::Shadowing, CASTER_SOURCE),
                shader("depthing", Stage::Depthing, OCCLUDER_SOURCE),
                shader("lighting", Stage::Lighting, QUAD_SOURCE),
            ]),
        ),
        (
            "meshes".to_owned(),
            Value::List(vec![mesh("triangle", triangle())]),
        ),
        (
            "mesh_materials".to_owned(),
            Value::List(vec![
                material("caster", "shadowing"),
                material("occluder", "depthing"),
                material("quad", "lighting"),
            ]),
        ),
        (
            "instances".to_owned(),
            Value::dictionary([
                (
                    "shadowing".to_owned(),
                    Value::List(vec![instance(
                        "caster",
                        "triangle",
                        "caster",
                        [0.0, 0.0, 0.0],
                    )]),
                ),
                (
                    "depthing".to_owned(),
                    Value::List(vec![instance(
                        "occluder",
                        "triangle",
                        "occluder",
                        [1.0, 0.0, 0.0],
                    )]),
                ),
                (
                    "lighting".to_owned(),
                    Value::List(vec![
                        instance("quad", "triangle", "quad", [0.0, 0.0, 0.0]),
                        // A second instance of the same mesh drawn by the same material in the
                        // same pass, so that the two share one pass and one instance buffer with
                        // a row apiece rather than being drawn one at a time.
                        instance("twin", "triangle", "quad", [2.0, 0.0, 0.0]),
                    ]),
                ),
            ]),
        ),
    ])
}

/// Builds one mesh material of the example, which an instance names by label.
fn material(label: &str, shader: &str) -> Value {
    Value::structure([
        ("label".to_owned(), Value::String(label.to_owned())),
        ("shader".to_owned(), Value::String(shader.to_owned())),
        (
            "custom_data".to_owned(),
            Value::structure([(
                "tint".to_owned(),
                Value::List([0.9, 0.9, 0.95].into_iter().map(Value::Float).collect()),
            )]),
        ),
    ])
}

/// Builds one mesh of the example, which is geometry and nothing else.
fn mesh(label: &str, points: Vec<Value>) -> Value {
    Value::structure([
        ("label".to_owned(), Value::String(label.to_owned())),
        ("topology".to_owned(), Value::variant("Triangles", [])),
        ("point_schema".to_owned(), example_point_schema().to_value()),
        ("points".to_owned(), Value::List(points)),
    ])
}

/// Builds one shader of the example, which a material names by label.
fn shader(label: &str, stage: Stage, source: &str) -> Value {
    let pass_schema = match stage {
        Stage::Shadowing => shadow_pass_schema(),
        Stage::Depthing => depth_pass_schema(),
        Stage::Lighting => light_pass_schema(),
        Stage::Emission => emission_pass_schema(),
        Stage::Post => post_pass_schema(),
    };
    Value::variant(
        stage.tag(),
        [
            ("label".to_owned(), Value::String(label.to_owned())),
            ("source".to_owned(), Value::String(source.to_owned())),
            ("pass_schema".to_owned(), pass_schema.to_value()),
            (
                "instance_schema".to_owned(),
                default_instance_schema().to_value(),
            ),
            ("point_schema".to_owned(), example_point_schema().to_value()),
            (
                "custom_schema".to_owned(),
                default_custom_schema().to_value(),
            ),
        ],
    )
}

/// Builds one instance of the example, which places a mesh and names the material drawing it.
fn instance(label: &str, mesh: &str, material: &str, translation: [f64; 3]) -> Value {
    Value::structure([
        ("label".to_owned(), Value::String(label.to_owned())),
        ("mesh".to_owned(), Value::String(mesh.to_owned())),
        (
            "transform".to_owned(),
            Value::structure([
                (
                    "translation".to_owned(),
                    Value::List(translation.into_iter().map(Value::Float).collect()),
                ),
                (
                    "rotation".to_owned(),
                    Value::List([0.0f64; 3].into_iter().map(Value::Float).collect()),
                ),
                (
                    "scale".to_owned(),
                    Value::List([1.0f64; 3].into_iter().map(Value::Float).collect()),
                ),
            ]),
        ),
        ("material".to_owned(), Value::String(material.to_owned())),
    ])
}

/// Builds a triangle of points, which is the one mesh every instance of the example draws.
///
/// The triangle lies flat on the ground plane and faces the light side of the room, so
/// every point of it carries the same normal.
fn triangle() -> Vec<Value> {
    [(-1.0, 0.0), (1.0, 0.0), (0.0, 1.0)]
        .into_iter()
        .map(|(x, y)| {
            Value::structure([
                (
                    "normal".to_owned(),
                    Value::List([0.0f64, 1.0, 0.0].into_iter().map(Value::Float).collect()),
                ),
                (
                    "position".to_owned(),
                    Value::List([x, y, 0.0].into_iter().map(Value::Float).collect()),
                ),
            ])
        })
        .collect()
}

/// The shadowing stage: place the point, then project it from the light.
///
/// A shader carries its own source, so the example renders with nothing on disk. The
/// uniform is declared in the order its schema sorts the fields into.
const CASTER_SOURCE: &str = r#"
struct Projection {
    position: vec3<f32>,
    view_projection: mat4x4<f32>,
};

struct Uniforms {
    projection: Projection,
    time: f32,
};

@group(0) @binding(0) var<uniform> uniforms: Uniforms;

@vertex
fn vertex_main(
    @location(0) normal: vec3<f32>,
    @location(1) position: vec3<f32>,
    @location(2) m0: vec4<f32>,
    @location(3) m1: vec4<f32>,
    @location(4) m2: vec4<f32>,
    @location(5) m3: vec4<f32>,
) -> @builtin(position) vec4<f32> {
    let model = mat4x4<f32>(m0, m1, m2, m3);
    return uniforms.projection.view_projection * model * vec4<f32>(position, 1.0);
}

@fragment
fn fragment_main() -> @location(0) vec4<f32> {
    return vec4<f32>(1.0, 1.0, 1.0, 1.0);
}
"#;

/// The depthing stage: the same projection, and no fragment stage, because the pass draws
/// into a depth buffer alone.
const OCCLUDER_SOURCE: &str = r#"
struct Projection {
    position: vec3<f32>,
    view_projection: mat4x4<f32>,
};

struct Uniforms {
    projection: Projection,
    time: f32,
};

@group(0) @binding(0) var<uniform> uniforms: Uniforms;

@vertex
fn vertex_main(
    @location(0) normal: vec3<f32>,
    @location(1) position: vec3<f32>,
    @location(2) m0: vec4<f32>,
    @location(3) m1: vec4<f32>,
    @location(4) m2: vec4<f32>,
    @location(5) m3: vec4<f32>,
) -> @builtin(position) vec4<f32> {
    let model = mat4x4<f32>(m0, m1, m2, m3);
    return uniforms.projection.view_projection * model * vec4<f32>(position, 1.0);
}
"#;

/// The lighting stage: shade by what the earlier passes wrote, so a frame where this draws
/// is a frame where the pool and the depth buffer were both read.
///
/// The shadow is a real one: the world position is carried into the **light's own** clip
/// space through the projection in that light's own record, and the surface is in shadow when
/// it is further from the light than that light's page recorded. Reading the page at the
/// fragment's own screen coordinate is not a shadow, because the two spaces have nothing to
/// do with each other.
///
/// The loop is over one cluster's lights rather than over the scene's, which is what makes
/// this pass's count a function of the scene's geometry rather than of how many lamps it has.
const QUAD_SOURCE: &str = r#"
struct Projection {
    position: vec3<f32>,
    view_projection: mat4x4<f32>,
};

struct Uniforms {
    cluster_grid: vec4<f32>,
    environment: vec4<f32>,
    frame_size: vec2<f32>,
    projection: Projection,
    time: f32,
};

/// One light, as the clustered loop reads it. The offsets need no padding written here: a
/// three component vector is sixteen byte aligned and twelve bytes long, so the `f32` after
/// each is the padding that aligns the next one.
struct LightRecord {
    color: vec3<f32>,
    page: f32,
    position: vec3<f32>,
    radius: f32,
    shadow_projection: mat4x4<f32>,
};

@group(0) @binding(0) var<uniform> uniforms: Uniforms;
@group(0) @binding(1) var<storage, read> cluster_light_counts: array<f32>;
@group(0) @binding(2) var<storage, read> cluster_light_indices: array<f32>;
@group(0) @binding(4) var environment_map: texture_2d<f32>;
@group(0) @binding(5) var<storage, read> light_records: array<LightRecord>;
@group(0) @binding(6) var shadow_pool: texture_depth_2d_array;

struct Vertex {
    @builtin(position) position: vec4<f32>,
    @location(0) world_position: vec3<f32>,
    @location(1) normal: vec3<f32>,
};

@vertex
fn vertex_main(
    @location(0) normal: vec3<f32>,
    @location(1) position: vec3<f32>,
    @location(2) m0: vec4<f32>,
    @location(3) m1: vec4<f32>,
    @location(4) m2: vec4<f32>,
    @location(5) m3: vec4<f32>,
) -> Vertex {
    let model = mat4x4<f32>(m0, m1, m2, m3);
    let world = model * vec4<f32>(position, 1.0);
    var out: Vertex;
    out.position = uniforms.projection.view_projection * world;
    out.world_position = world.xyz;
    // A transform that is a rotation and a translation carries a normal through unchanged
    // by the model matrix, renormalised. A transform that scales unevenly does not, and
    // would need its inverse transpose instead — which a vertex stage cannot compute,
    // because WGSL has no matrix inverse.
    out.normal = normalize((model * vec4<f32>(normal, 0.0)).xyz);
    return out;
}

/// Returns which cluster of the grid a fragment is in.
///
/// The grid is divided evenly across the frame and evenly through the view's depth, and the
/// depth is the number a fragment already carries, so this is three multiplies and no
/// arithmetic. Rows count from the bottom, because a framebuffer row runs down the screen
/// and a grid row runs up the view.
fn cluster_of(fragment: vec4<f32>) -> u32 {
    let across = clamp(fragment.x / uniforms.frame_size.x, 0.0, 0.99999);
    let up = clamp(1.0 - fragment.y / uniforms.frame_size.y, 0.0, 0.99999);
    let column = u32(across * uniforms.cluster_grid.x);
    let row = u32(up * uniforms.cluster_grid.y);
    let slice = u32(clamp(fragment.z, 0.0, 0.99999) * uniforms.cluster_grid.z);
    return slice * u32(uniforms.cluster_grid.x * uniforms.cluster_grid.y)
        + row * u32(uniforms.cluster_grid.x)
        + column;
}

/// Returns how lit a surface is by one light: unshadowed where the light was given no page,
/// and otherwise by comparing the surface's depth in that light's own space against what the
/// light's page recorded there.
///
/// A light with no page is lit unshadowed because that is the pool working: a page is worth
/// rendering once, a light the scene's budget did not buy one is a light the scene chose to
/// draw approximately, and the alternative would be a neighbour's shadow.
///
/// The taps are taken by `textureLoad` rather than through a sampler because a binding
/// carries a texture and nothing else: there is no sampler to filter with, and a depth
/// compare has to be done by hand in any case.
fn shadowed(light: LightRecord, world_position: vec3<f32>, normal: vec3<f32>) -> f32 {
    let page = i32(light.page);
    if page < 0 {
        return 1.0;
    }
    let from_light = light.shadow_projection * vec4<f32>(world_position, 1.0);
    // A point behind the light has no shadow, and dividing by a negative w would fold it
    // back into the page from behind.
    if from_light.w <= 0.0 {
        return 1.0;
    }
    let ndc = from_light.xyz / from_light.w;
    // Clip space y runs up and texture space runs down.
    let uv = ndc.xy * vec2<f32>(0.5, -0.5) + vec2<f32>(0.5, 0.5);
    if any(uv < vec2<f32>(0.0)) || any(uv > vec2<f32>(1.0)) {
        return 1.0;
    }
    let size = vec2<f32>(textureDimensions(shadow_pool));
    // A surface is its own depth and the recorded value is the depth of whatever was
    // nearest the light, so the comparison is biased. The bias is scaled by how obliquely
    // the light strikes the surface, because the depth a surface's own rasterised faces
    // disagree with its true depth by more the further they lean away from the light, and
    // one constant is either too small for the leaning surfaces or detaches the shadow
    // from the flat ones.
    let slope = 1.0 - abs(dot(normal, normalize(light.position - world_position)));
    let z = ndc.z - (0.002 + 0.04 * slope);
    var lit = 0.0;
    for (var row = -1; row <= 1; row = row + 1) {
        for (var column = -1; column <= 1; column = column + 1) {
            // The offset is whole texels added after the conversion, so the kernel really is
            // three by three rather than nine reads of the same texel. The last argument is
            // the mip level, which a depth texture's `textureLoad` takes; a pool is rendered
            // into and so has one level.
            let coordinate = vec2<i32>(uv * size) + vec2<i32>(column, row);
            let recorded = textureLoad(shadow_pool, coordinate, page, 0);
            lit = lit + select(0.0, 1.0, z <= recorded);
        }
    }
    return lit / 9.0;
}

/// Returns the texel of the environment map a direction falls in, at one level.
///
/// This is the same octahedral fold the gallery's lighting template uses, and it is written
/// out again here because this example has to compile without the gallery: `scene` is the
/// crate below it and a dependency that way is a cycle. The gallery template is the full
/// version and the one to read; what is repeated here is the two halves that are easy to get
/// wrong — the fold is a seam, so the read is a `textureLoad` at a computed texel rather than
/// a filtered one, and the texel is clamped into the image because a direction on the square's
/// edge computes a coordinate of exactly its size.
fn octahedral(direction: vec3<f32>, level: i32) -> vec2<i32> {
    var planar = direction.xy / max(
        abs(direction.x) + abs(direction.y) + abs(direction.z),
        1e-8,
    );
    if direction.z < 0.0 {
        let folded = vec2<f32>(1.0) - abs(planar.yx);
        planar = vec2<f32>(
            select(-folded.x, folded.x, planar.x >= 0.0),
            select(-folded.y, folded.y, planar.y >= 0.0),
        );
    }
    let uv = vec2<f32>(planar.x * 0.5 + 0.5, 0.5 - planar.y * 0.5);
    let extent = vec2<i32>(textureDimensions(environment_map, level));
    return clamp(
        vec2<i32>(uv * vec2<f32>(extent)),
        vec2<i32>(0),
        extent - vec2<i32>(1),
    );
}

/// Returns what a surface facing this normal receives from the environment.
///
/// One ambient term, fed by the scene's environment when it names one and by the two colour
/// hemisphere when it does not, and chosen on the uniform's flag rather than on the colour that
/// was sampled: a scene naming no environment binds a **black** map, so a shader that decided
/// on the colour could not tell no environment from a black one.
fn environment_radiance(normal: vec3<f32>, roughness: f32) -> vec3<f32> {
    if uniforms.environment.w < 0.5 {
        return mix(
            vec3<f32>(0.014, 0.017, 0.026),
            vec3<f32>(0.055, 0.072, 0.110),
            normal.y * 0.5 + 0.5,
        );
    }
    // The level is the roughness scaled by the chain the map actually has and clamped into it,
    // so a roughness of one lands on the last level rather than one past the end of it.
    let levels = i32(textureNumLevels(environment_map));
    let level = clamp(i32(roughness * f32(levels - 1)), 0, levels - 1);
    return textureLoad(environment_map, octahedral(normal, level), level).rgb
        * uniforms.environment.rgb;
}

@fragment
fn fragment_main(in: Vertex) -> @location(0) vec4<f32> {
    let normal = normalize(in.normal);
    let cluster = cluster_of(in.position);
    let count = u32(cluster_light_counts[cluster]);
    let row = cluster * u32(uniforms.cluster_grid.w);
    var color = vec3<f32>(0.0);
    for (var slot = 0u; slot < count; slot = slot + 1u) {
        let light = light_records[u32(cluster_light_indices[row + slot])];
        let to_light = light.position - in.world_position;
        let lambert = max(dot(normal, normalize(to_light)), 0.0);
        color = color + light.color * lambert * shadowed(light, in.world_position, normal);
    }
    // One ambient term, added once whatever the cluster holds, and the material's own tint
    // still reaching it: a term that stopped multiplying by the tint would be a material that
    // had stopped responding to its own colour.
    color = color + environment_radiance(normal, 0.5);
    return vec4<f32>(color.r, color.g, color.b, 1.0);
}
"#;
