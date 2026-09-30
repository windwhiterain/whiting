//! A small, valid standard scene, built from the schemas the compiler declares.
//!
//! The scene has one light and one mesh per stage, so the pipeline it compiles to is the
//! whole three stage pipeline: a shadowing pass, a depthing pass and a lighting pass.

use definition::Value;

use crate::standard::Stage;
use crate::standard::pass_schemas::{
    depthing_pass_schema, lighting_pass_schema, shadowing_pass_schema,
};
use crate::standard::scene_schema::{
    default_custom_schema, default_instance_schema, default_point_schema,
};

/// Builds the example scene: one light and one mesh per stage.
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
            "meshes".to_owned(),
            Value::List(vec![
                mesh("caster", Stage::Shadowing, triangle(0.0)),
                mesh("occluder", Stage::Depthing, triangle(1.0)),
                mesh("quad", Stage::Lighting, triangle(0.0)),
            ]),
        ),
    ])
}

/// Builds one mesh of the example, drawn by the stage its material belongs to.
fn mesh(label: &str, stage: Stage, points: Vec<Value>) -> Value {
    let pass_schema = match stage {
        Stage::Shadowing => shadowing_pass_schema(),
        Stage::Depthing => depthing_pass_schema(),
        Stage::Lighting => lighting_pass_schema(),
    };
    Value::structure([
        ("label".to_owned(), Value::String(label.to_owned())),
        ("topology".to_owned(), Value::variant("Triangles", [])),
        ("point_schema".to_owned(), default_point_schema().to_value()),
        ("points".to_owned(), Value::List(points)),
        (
            "instance_data".to_owned(),
            Value::structure([("model".to_owned(), identity())]),
        ),
        (
            "material".to_owned(),
            Value::structure([
                (
                    "shader".to_owned(),
                    Value::variant(
                        stage.tag(),
                        [
                            (
                                "path".to_owned(),
                                Value::String(format!("shaders/{label}.wgsl")),
                            ),
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
                    ),
                ),
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

/// Builds a triangle of points, offset along the x axis.
fn triangle(offset: f64) -> Vec<Value> {
    [(-1.0, 0.0), (1.0, 0.0), (0.0, 1.0)]
        .into_iter()
        .map(|(x, y)| {
            Value::structure([(
                "position".to_owned(),
                Value::List([x + offset, y, 0.0].into_iter().map(Value::Float).collect()),
            )])
        })
        .collect()
}

/// Returns an identity model matrix, column major.
fn identity() -> Value {
    let mut components = vec![0.0f64; 16];
    for index in [0usize, 5, 10, 15] {
        components[index] = 1.0;
    }
    Value::List(components.into_iter().map(Value::Float).collect())
}
