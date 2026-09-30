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
    depthing_pass_schema, lighting_pass_schema, post_pass_schema, shadowing_pass_schema,
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
            Value::List(vec![Value::structure([
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
            ])]),
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
            "instances".to_owned(),
            Value::List(vec![
                instance("caster", "triangle", "shadowing", [0.0, 0.0, 0.0]),
                instance("occluder", "triangle", "depthing", [1.0, 0.0, 0.0]),
                instance("quad", "triangle", "lighting", [0.0, 0.0, 0.0]),
                // A second instance of the same mesh drawn by the same shader in the same stage,
                // so that the two share one pass and one instance buffer with a row apiece rather
                // than being drawn one at a time.
                instance("twin", "triangle", "lighting", [2.0, 0.0, 0.0]),
            ]),
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
        Stage::Shadowing => shadowing_pass_schema(),
        Stage::Depthing => depthing_pass_schema(),
        Stage::Lighting => lighting_pass_schema(),
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

/// Builds one instance of the example, which places a mesh and names what draws it.
fn instance(label: &str, mesh: &str, shader: &str, translation: [f64; 3]) -> Value {
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
        (
            "material".to_owned(),
            Value::structure([
                ("shader".to_owned(), Value::String(shader.to_owned())),
                (
                    "custom_data".to_owned(),
                    Value::structure([(
                        "tint".to_owned(),
                        Value::List([0.9, 0.9, 0.95].into_iter().map(Value::Float).collect()),
                    )]),
                ),
            ]),
        ),
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
/// is a frame where the shadow map and the depth buffer were both read.
///
/// The shadow is a real one: the world position is carried into the light's own clip
/// space, and the surface is in shadow when it is further from the light than the map
/// recorded. Reading the shadow map at the fragment's own screen coordinate is not a
/// shadow, because the two spaces have nothing to do with each other.
const QUAD_SOURCE: &str = r#"
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

@group(0) @binding(0) var<uniform> uniforms: Uniforms;
@group(0) @binding(1) var depth_buffer: texture_depth_2d;
@group(0) @binding(2) var shadow_map: texture_depth_2d;

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

/// Returns how lit a surface is, by comparing its depth in the light's own space against
/// what the shadow map recorded there.
///
/// A nine tap kernel is what turns a hard staircase into an edge. The taps are taken by
/// `textureLoad` rather than through a sampler because a binding carries a texture and
/// nothing else: there is no sampler to filter with, and a depth compare has to be done
/// by hand in any case.
fn shadow(world_position: vec3<f32>, normal: vec3<f32>) -> f32 {
    let to_light = uniforms.light_position - world_position;
    let from_light = uniforms.light_projection * vec4<f32>(world_position, 1.0);
    // A point behind the light has no shadow, and dividing by a negative w would fold it
    // back into the map from behind.
    if from_light.w <= 0.0 {
        return 1.0;
    }
    let ndc = from_light.xyz / from_light.w;
    // Clip space y runs up and texture space runs down.
    let uv = ndc.xy * vec2<f32>(0.5, -0.5) + vec2<f32>(0.5, 0.5);
    if any(uv < vec2<f32>(0.0)) || any(uv > vec2<f32>(1.0)) {
        return 1.0;
    }
    let size = vec2<f32>(textureDimensions(shadow_map));
    // A surface is its own depth and the recorded value is the depth of whatever was
    // nearest the light, so the comparison is biased. The bias is scaled by how obliquely
    // the light strikes the surface, because the depth a surface's own rasterised faces
    // disagree with its true depth by more the further they lean away from the light, and
    // one constant is either too small for the leaning surfaces or detaches the shadow
    // from the flat ones.
    let slope = 1.0 - abs(dot(normal, normalize(to_light)));
    let z = ndc.z - (0.002 + 0.04 * slope);
    var lit = 0.0;
    for (var row = -1; row <= 1; row = row + 1) {
        for (var column = -1; column <= 1; column = column + 1) {
            let offset = vec2<f32>(f32(column), f32(row)) * uniforms.light_shadow_texel;
            let coordinate = vec2<i32>(uv * size + offset);
            let recorded = textureLoad(shadow_map, coordinate, 0);
            lit = lit + select(0.0, 1.0, z <= recorded);
        }
    }
    return lit / 9.0;
}

@fragment
fn fragment_main(in: Vertex) -> @location(0) vec4<f32> {
    let lit = shadow(in.world_position, normalize(in.normal));
    let color = uniforms.light_color * lit;
    return vec4<f32>(color.r, color.g, color.b, 1.0);
}
"#;
