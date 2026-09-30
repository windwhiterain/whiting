//! A small, valid standard scene, built from the schemas the compiler declares.
//!
//! The scene has one light, one mesh, and three instances of that mesh — one per stage — so
//! the pipeline it compiles to is the whole three stage pipeline, and the geometry is
//! written once however many stages draw it.

use definition::Value;

use crate::standard::Stage;
use crate::standard::pass_schemas::{
    depthing_pass_schema, lighting_pass_schema, shadowing_pass_schema,
};
use crate::standard::scene_schema::{
    default_custom_schema, default_instance_schema, default_point_schema,
};

/// Builds the example scene: one light, one mesh, and one instance per stage.
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
            Value::List(vec![mesh("triangle", triangle())]),
        ),
        (
            "instances".to_owned(),
            Value::List(vec![
                instance("caster", "triangle", Stage::Shadowing, [0.0, 0.0, 0.0]),
                instance("occluder", "triangle", Stage::Depthing, [1.0, 0.0, 0.0]),
                instance("quad", "triangle", Stage::Lighting, [0.0, 0.0, 0.0]),
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

/// Builds one instance of the example, which places a mesh and says what draws it.
fn instance(label: &str, mesh: &str, stage: Stage, translation: [f64; 3]) -> Value {
    let pass_schema = match stage {
        Stage::Shadowing => shadowing_pass_schema(),
        Stage::Depthing => depthing_pass_schema(),
        Stage::Lighting => lighting_pass_schema(),
    };
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
