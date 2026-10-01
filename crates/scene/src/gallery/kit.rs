//! The parts a gallery scene is assembled from.
//!
//! A standard scene document has seven fields, and each one is written here once. A scene
//! that builds itself says `light("key", [4.0, 6.0, 2.0], ...)`, which reads as the scene
//! it is, rather than repeating the shape of a light at every call site.
//!
//! # Invariants
//!
//! A builder here produces the field names the scene schema declares and the emitter
//! reads. Nothing in this module knows what a stage does, only which tag names it.

use definition::{PackedList, Schema, SchemaKind, Value};
use graphics::layout::components_schema;

/// The tint a material's shader multiplies its albedo by.
pub const TINT: &str = "tint";
/// The colour a surface emits on its own, which no light reaches.
pub const EMISSIVE: &str = "emissive";
/// How metallic a surface is, from zero to one.
pub const METALLIC: &str = "metallic";
/// How much of the sky a surface shows, from zero to one.
pub const REFLECT: &str = "reflect";
/// How rough a surface is, from zero to one.
pub const ROUGHNESS: &str = "roughness";
/// The height of a fog bank, and how thick it is.
pub const FOG: &str = "fog";

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
///
/// # Invariants
///
/// The points are a [`PackedList`], which is the same list a document would write as values and
/// is what a buffer is built from directly. A generator produces the run rather than a tree of
/// maps, so the geometry a scene is built from never exists in the expensive form at all.
pub fn mesh(label: &str, topology: &str, point_schema: Schema, points: PackedList) -> Value {
    Value::structure([
        (
            crate::standard::LABEL.to_owned(),
            Value::String(label.to_owned()),
        ),
        (
            crate::standard::TOPOLOGY.to_owned(),
            Value::variant(topology, []),
        ),
        (
            crate::standard::POINT_SCHEMA.to_owned(),
            point_schema.to_value(),
        ),
        (crate::standard::POINTS.to_owned(), Value::Packed(points)),
    ])
}

/// Builds one shader of a scene, in one of the four stages.
pub fn shader(
    stage: crate::standard::Stage,
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
            (
                crate::standard::LABEL.to_owned(),
                Value::String(label.to_owned()),
            ),
            (
                crate::standard::SOURCE.to_owned(),
                Value::String(source.to_owned()),
            ),
            (
                crate::standard::PASS_SCHEMA.to_owned(),
                pass_schema.to_value(),
            ),
            (
                crate::standard::INSTANCE_SCHEMA.to_owned(),
                instance_schema.to_value(),
            ),
            (
                crate::standard::POINT_SCHEMA.to_owned(),
                point_schema.to_value(),
            ),
            (
                crate::standard::CUSTOM_SCHEMA.to_owned(),
                custom_schema.to_value(),
            ),
        ],
    )
}

/// Builds a mesh material, which an instance names by label.
///
/// A material names one shader, and the shader's stage is the only pass the material can be
/// drawn in: an instance is listed under the passes that draw it, and each of those buckets
/// names the material for that pass.
pub fn mesh_material(label: &str, shader: &str, custom_data: Value) -> Value {
    Value::structure([
        (
            crate::standard::LABEL.to_owned(),
            Value::String(label.to_owned()),
        ),
        (
            crate::standard::SHADER.to_owned(),
            Value::String(shader.to_owned()),
        ),
        (crate::standard::CUSTOM_DATA.to_owned(), custom_data),
    ])
}

/// Builds one instance: a mesh, where it is, how it is turned, and what draws it.
pub fn instance(
    label: &str,
    mesh: &str,
    translation: [f64; 3],
    rotation: [f64; 3],
    scale: [f64; 3],
    material: &str,
) -> Value {
    Value::structure([
        (
            crate::standard::LABEL.to_owned(),
            Value::String(label.to_owned()),
        ),
        (
            crate::standard::MESH.to_owned(),
            Value::String(mesh.to_owned()),
        ),
        (
            crate::standard::TRANSFORM.to_owned(),
            Value::structure([
                ("translation".to_owned(), triple(translation)),
                ("rotation".to_owned(), triple(rotation)),
                ("scale".to_owned(), triple(scale)),
            ]),
        ),
        (
            crate::standard::MATERIAL.to_owned(),
            Value::String(material.to_owned()),
        ),
    ])
}

/// Builds the instances of a scene, bucketed by the pass that draws them.
///
/// # Invariants
///
/// The key is a pass and the value is the instances drawn in it, so an instance listed under
/// several passes is drawn once per pass. A bucket no pass draws is not written at all rather
/// than written empty: an absent key and an empty list mean the same thing here, and the
/// shorter document is the one an agent reads.
pub fn instances(buckets: Vec<(&str, Vec<Value>)>) -> Value {
    Value::dictionary(
        buckets
            .into_iter()
            .map(|(pass, instances)| (pass.to_owned(), Value::List(instances))),
    )
}

/// Builds a camera that is drawn from the eye it writes, rather than fitted to the scene.
///
/// # Invariants
///
/// A camera that names both depth planes keeps its own eye and distance, and one that names
/// neither is fitted until the scene's bounds fit the lens. A scene whose ground reaches much
/// further than its subject wants this one: the extent of the water decides nothing about
/// where the camera stood, and a fitted camera pulls back until all of it is in frame.
pub fn camera_placed(
    eye: [f64; 3],
    target: [f64; 3],
    field_of_view: f64,
    near: f64,
    far: f64,
) -> Value {
    Value::structure([
        ("eye".to_owned(), triple(eye)),
        ("target".to_owned(), triple(target)),
        ("field_of_view".to_owned(), Value::Float(field_of_view)),
        ("projection".to_owned(), Value::variant("Perspective", [])),
        ("near".to_owned(), Value::Float(near)),
        ("far".to_owned(), Value::Float(far)),
    ])
}

/// Builds the camera a scene is drawn through.
pub fn camera(eye: [f64; 3], target: [f64; 3], field_of_view: f64, padding: f64) -> Value {
    Value::structure([
        ("eye".to_owned(), triple(eye)),
        ("target".to_owned(), triple(target)),
        ("field_of_view".to_owned(), Value::Float(field_of_view)),
        ("projection".to_owned(), Value::variant("Perspective", [])),
        ("padding".to_owned(), Value::Float(padding)),
    ])
}

/// Builds the point schema a shaded mesh declares: a normal, a position and a texcoord.
///
/// # Invariants
///
/// The fields are read in the order a schema sorts them, which is also the order the
/// buffer lays them out in, so a shader reads its normal at location zero, its position at
/// one and its texcoord at two. The custom data the same scene carries is packed with the
/// names below for the same reason.
pub fn shaded_point_schema() -> Schema {
    Schema::new(SchemaKind::structure([
        ("normal".to_owned(), components_schema(3)),
        ("position".to_owned(), components_schema(3)),
        ("texture".to_owned(), components_schema(2)),
    ]))
}

/// Builds the point schema a mesh that reads as lines declares: a position alone.
pub fn line_point_schema() -> Schema {
    Schema::new(SchemaKind::structure([(
        "position".to_owned(),
        components_schema(3),
    )]))
}

/// Builds the instance schema a gallery material is packed with.
///
/// A row is this merged with the material's own custom schema, and the two are laid out
/// in the order the merged schema sorts them into: `emissive`, `metallic`, `model`,
/// `normal_matrix`, `roughness`, `tint`. The two matrices are four consecutive locations
/// each, because they are sixteen floats wide and a backend reads four at a location.
///
/// `normal_matrix` is the inverse transpose of `model`, which is what carries a normal
/// through a transform that scales unevenly. A transform that only rotates and translates
/// needs it no more than it needs the model, but a column twenty-eight times taller than
/// it is wide tilts every normal on it, and a lit surface whose normals are wrong is a
/// surface that cannot be lit.
pub fn gallery_instance_schema() -> Schema {
    Schema::new(SchemaKind::structure([
        (
            "model".to_owned(),
            components_schema(16).with_documentation("Model matrix, column major."),
        ),
        (
            "normal_matrix".to_owned(),
            components_schema(16)
                .with_documentation("Inverse transpose of the model matrix, column major."),
        ),
    ]))
}

/// Builds the custom schema a gallery material carries.
///
/// These names are chosen so that merged with [`gallery_instance_schema`] they sort into
/// the order the shaders in [`crate::gallery::shaders`] read them at.
pub fn gallery_custom_schema() -> Schema {
    Schema::new(SchemaKind::structure([
        (
            EMISSIVE.to_owned(),
            components_schema(3).with_documentation("Colour the surface emits on its own."),
        ),
        (
            METALLIC.to_owned(),
            Schema::new(SchemaKind::Float)
                .with_documentation("How metallic the surface is, from zero to one."),
        ),
        (
            REFLECT.to_owned(),
            Schema::new(SchemaKind::Float).with_documentation(
                "How much of the sky the surface shows at a grazing angle. A wet floor \
                 reflects nearly all of it and a stone reflects almost none, and both are \
                 drawn by the same pass, so what tells them apart is the row.",
            ),
        ),
        (
            ROUGHNESS.to_owned(),
            Schema::new(SchemaKind::Float)
                .with_documentation("How rough the surface is, from zero to one."),
        ),
        (
            TINT.to_owned(),
            components_schema(3).with_documentation("Colour multiplier, from zero to one."),
        ),
    ]))
}

/// Builds the custom data one gallery material carries.
pub fn surface(tint: [f64; 3], metallic: f64, roughness: f64, emissive: [f64; 3]) -> Value {
    surface_reflecting(tint, metallic, roughness, emissive, 0.0)
}

/// Builds the custom data of a surface that shows the sky at a grazing angle.
///
/// This is the same row with `reflect` set, and it is the reason the field exists: a wet
/// floor and a stone are drawn by the same pass with the same shader, and the only thing
/// that tells the shading which one it is on is the row it was drawn from.
pub fn surface_reflecting(
    tint: [f64; 3],
    metallic: f64,
    roughness: f64,
    emissive: [f64; 3],
    reflect: f64,
) -> Value {
    Value::structure([
        (TINT.to_owned(), triple(tint)),
        (METALLIC.to_owned(), Value::Float(metallic)),
        (ROUGHNESS.to_owned(), Value::Float(roughness)),
        (EMISSIVE.to_owned(), triple(emissive)),
        (REFLECT.to_owned(), Value::Float(reflect)),
    ])
}
