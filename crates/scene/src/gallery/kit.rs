//! The parts a gallery scene is assembled from.
//!
//! A standard scene document has five fields, and each one is written here once. A scene
//! that builds itself says `light("key", [4.0, 6.0, 2.0], ...)`, which reads as the scene
//! it is, rather than repeating the shape of a light at every call site.
//!
//! # Invariants
//!
//! A builder here produces the field names the scene schema declares and the emitter
//! reads. Nothing in this module knows what a stage does, only which tag names it.

use definition::{Schema, Value};

use crate::standard::{
    CUSTOM_DATA, CUSTOM_SCHEMA, INSTANCE_SCHEMA, LABEL, MATERIAL, MESH, PASS_SCHEMA, POINT_SCHEMA,
    POINTS, SHADER, SOURCE, Stage, TOPOLOGY, TRANSFORM,
};

/// Builds a three component list of floats.
pub fn triple(components: [f64; 3]) -> Value {
    Value::List(components.into_iter().map(Value::Float).collect())
}

/// Builds one light of a scene.
pub fn light(
    label: &str,
    position: [f64; 3],
    color: [f64; 3],
    intensity: f64,
    shadow_extent: f64,
) -> Value {
    Value::structure([
        ("label".to_owned(), Value::String(label.to_owned())),
        ("position".to_owned(), triple(position)),
        ("color".to_owned(), triple(color)),
        ("intensity".to_owned(), Value::Float(intensity)),
        ("shadow_extent".to_owned(), Value::Float(shadow_extent)),
    ])
}

/// Builds one mesh of a scene: its label, how its points are read, and the points.
pub fn mesh(label: &str, topology: &str, point_schema: Schema, points: Vec<Value>) -> Value {
    Value::structure([
        (LABEL.to_owned(), Value::String(label.to_owned())),
        (TOPOLOGY.to_owned(), Value::variant(topology, [])),
        (POINT_SCHEMA.to_owned(), point_schema.to_value()),
        (POINTS.to_owned(), Value::List(points)),
    ])
}

/// Builds one shader of a scene, in one of the three mesh stages.
pub fn shader(
    stage: Stage,
    label: &str,
    source: &str,
    pass_schema: Schema,
    point_schema: Schema,
    instance_schema: Schema,
    custom_schema: Schema,
) -> Value {
    Value::variant(
        stage.tag(),
        [
            (LABEL.to_owned(), Value::String(label.to_owned())),
            (SOURCE.to_owned(), Value::String(source.to_owned())),
            (PASS_SCHEMA.to_owned(), pass_schema.to_value()),
            (INSTANCE_SCHEMA.to_owned(), instance_schema.to_value()),
            (POINT_SCHEMA.to_owned(), point_schema.to_value()),
            (CUSTOM_SCHEMA.to_owned(), custom_schema.to_value()),
        ],
    )
}

/// Builds the material an instance is drawn with.
pub fn material(shader: &str, custom_data: Value) -> Value {
    Value::structure([
        (SHADER.to_owned(), Value::String(shader.to_owned())),
        (CUSTOM_DATA.to_owned(), custom_data),
    ])
}

/// Builds one instance: a mesh, where it is, how it is turned, and what draws it.
pub fn instance(
    label: &str,
    mesh: &str,
    translation: [f64; 3],
    rotation: [f64; 3],
    scale: [f64; 3],
    material: Value,
) -> Value {
    Value::structure([
        (LABEL.to_owned(), Value::String(label.to_owned())),
        (MESH.to_owned(), Value::String(mesh.to_owned())),
        (
            TRANSFORM.to_owned(),
            Value::structure([
                ("translation".to_owned(), triple(translation)),
                ("rotation".to_owned(), triple(rotation)),
                ("scale".to_owned(), triple(scale)),
            ]),
        ),
        (MATERIAL.to_owned(), material),
    ])
}
