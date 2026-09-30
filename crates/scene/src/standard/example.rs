//! A small, valid standard scene, built from the schemas the compiler declares.
//!
//! The scene has one light, one mesh, and four instances of that mesh — one per stage, and two
//! in the lighting stage — so the pipeline it compiles to is the whole three stage pipeline, the
//! geometry is written once however many stages draw it, and the two lighting instances are drawn
//! together by one instanced draw.

use definition::Value;

use crate::standard::Stage;
use crate::standard::pass_schemas::{
    depthing_pass_schema, lighting_pass_schema, shadowing_pass_schema,
};
use crate::standard::scene_schema::{
    default_custom_schema, default_instance_schema, default_point_schema,
};

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
                ("shadow_extent".to_owned(), Value::Float(8.0)),
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
        ("point_schema".to_owned(), default_point_schema().to_value()),
        ("points".to_owned(), Value::List(points)),
    ])
}

/// Builds one shader of the example, which a material names by label.
fn shader(label: &str, stage: Stage, source: &str) -> Value {
    let pass_schema = match stage {
        Stage::Shadowing => shadowing_pass_schema(),
        Stage::Depthing => depthing_pass_schema(),
        Stage::Lighting => lighting_pass_schema(),
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
            ("point_schema".to_owned(), default_point_schema().to_value()),
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
fn triangle() -> Vec<Value> {
    [(-1.0, 0.0), (1.0, 0.0), (0.0, 1.0)]
        .into_iter()
        .map(|(x, y)| {
            Value::structure([(
                "position".to_owned(),
                Value::List([x, y, 0.0].into_iter().map(Value::Float).collect()),
            )])
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
};

@group(0) @binding(0) var<uniform> uniforms: Uniforms;

@vertex
fn vertex_main(
    @location(0) position: vec3<f32>,
    @location(1) m0: vec4<f32>,
    @location(2) m1: vec4<f32>,
    @location(3) m2: vec4<f32>,
    @location(4) m3: vec4<f32>,
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
};

@group(0) @binding(0) var<uniform> uniforms: Uniforms;

@vertex
fn vertex_main(
    @location(0) position: vec3<f32>,
    @location(1) m0: vec4<f32>,
    @location(2) m1: vec4<f32>,
    @location(3) m2: vec4<f32>,
    @location(4) m3: vec4<f32>,
) -> @builtin(position) vec4<f32> {
    let model = mat4x4<f32>(m0, m1, m2, m3);
    return uniforms.projection.view_projection * model * vec4<f32>(position, 1.0);
}
"#;

/// The lighting stage: shade by what the earlier passes wrote, so a frame where this draws
/// is a frame where the shadow map and the depth buffer were both read.
const QUAD_SOURCE: &str = r#"
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

@vertex
fn vertex_main(
    @location(0) position: vec3<f32>,
    @location(1) m0: vec4<f32>,
    @location(2) m1: vec4<f32>,
    @location(3) m2: vec4<f32>,
    @location(4) m3: vec4<f32>,
) -> @builtin(position) vec4<f32> {
    let model = mat4x4<f32>(m0, m1, m2, m3);
    return uniforms.projection.view_projection * model * vec4<f32>(position, 1.0);
}

@fragment
fn fragment_main(@builtin(position) fragment: vec4<f32>) -> @location(0) vec4<f32> {
    let coordinate = vec2<i32>(fragment.xy);
    let shadow = textureLoad(shadow_map, coordinate, 0);
    let depth = textureLoad(depth_buffer, coordinate, 0);
    let lit = select(0.15, 1.0, shadow >= 0.999);
    let drawn = select(0.0, 1.0, depth < 1.0);
    let tint = uniforms.light_color * lit * drawn;
    return vec4<f32>(tint.r, tint.g, tint.b, 1.0);
}
"#;
