//! The schema of the conventional lit scene a `StandardScene` compiler accepts.
//!
//! # What a standard scene is
//!
//! A list of lights and a list of meshes, nothing else. The pipeline those two imply —
//! a shadow map per light, a depth buffer, then the shaded image — is the compiler's
//! business, not a field an author fills in. An author says what is in the scene and
//! the compiler works out what has to run to draw it.
//!
//! # What a shader declares
//!
//! Every material names the shader that draws it, and that shader names the stage it
//! belongs to and the schemas of the data it reads. Those schemas are the interesting
//! part: a shader that wants a texcoord per point and a mesh that does not carry one
//! is a compile error rather than a black triangle.

use definition::{Schema, SchemaKind};

/// Field holding the name of the scene.
pub const NAME: &str = "name";

/// Field holding the lights of the scene.
pub const LIGHTS: &str = "lights";

/// Field holding the meshes of the scene.
pub const MESHES: &str = "meshes";

/// Field holding the label of a mesh.
pub const LABEL: &str = "label";

/// Field holding how a mesh's points are read.
pub const TOPOLOGY: &str = "topology";

/// Field holding the schema a mesh's points are verified against.
pub const POINT_SCHEMA: &str = "point_schema";

/// Field holding the points of a mesh.
pub const POINTS: &str = "points";

/// Field holding the per instance transform data of a mesh.
pub const INSTANCE_DATA: &str = "instance_data";

/// Field holding the material of a mesh.
pub const MATERIAL: &str = "material";

/// Field holding the shader a material is drawn with.
pub const SHADER: &str = "shader";

/// Field holding the data a material's own shader reads.
pub const CUSTOM_DATA: &str = "custom_data";

/// Field naming the source a shader is loaded from.
pub const PATH: &str = "path";

/// Field holding the schema of the pass a shader is bound to.
pub const PASS_SCHEMA: &str = "pass_schema";

/// Field holding the schema of the instance data a shader reads.
pub const INSTANCE_SCHEMA: &str = "instance_schema";

/// Field holding the schema of a point a shader reads.
pub const POINT_SCHEMA_FIELD: &str = "point_schema";

/// Field holding the schema of the custom data a shader reads.
pub const CUSTOM_SCHEMA: &str = "custom_schema";

/// Field naming the stage a screen shader belongs to.
pub const STAGE: &str = "stage";

/// Builds the schema of a standard scene document.
pub fn standard_scene_schema() -> Schema {
    Schema::new(SchemaKind::structure([
        (NAME.to_owned(), Schema::new(SchemaKind::String)),
        (
            LIGHTS.to_owned(),
            Schema::new(SchemaKind::list(light_schema()))
                .with_documentation("One shadowing pass is emitted per light."),
        ),
        (
            MESHES.to_owned(),
            Schema::new(SchemaKind::list(mesh_schema()))
                .with_documentation("A mesh is drawn by the stage its material belongs to."),
        ),
    ]))
    .with_documentation("The conventional lit scene: lights and meshes.")
}

/// Builds the schema of one light.
pub fn light_schema() -> Schema {
    Schema::new(SchemaKind::structure([
        (
            "label".to_owned(),
            Schema::new(SchemaKind::String)
                .with_documentation("Name used in pass labels and in errors."),
        ),
        (
            "position".to_owned(),
            float_triple("Position in world space."),
        ),
        ("color".to_owned(), float_triple("Colour in zero to one.")),
        (
            "intensity".to_owned(),
            Schema::new(SchemaKind::Float).with_documentation("Brightness multiplier."),
        ),
        (
            "shadow_extent".to_owned(),
            Schema::new(SchemaKind::Float)
                .with_documentation("Half the width of the volume the light shadows."),
        ),
    ]))
    .with_documentation("A light, which contributes one shadowing pass.")
}

/// Builds the schema of one mesh.
pub fn mesh_schema() -> Schema {
    Schema::new(SchemaKind::structure([
        (LABEL.to_owned(), Schema::new(SchemaKind::String)),
        (
            TOPOLOGY.to_owned(),
            Schema::new(SchemaKind::enumeration([
                (
                    "Triangles".to_owned(),
                    Schema::new(SchemaKind::structure([])),
                ),
                (
                    "Segments".to_owned(),
                    Schema::new(SchemaKind::structure([])),
                ),
            ]))
            .with_documentation("How the points are read as primitives."),
        ),
        (
            POINT_SCHEMA.to_owned(),
            Schema::document_schema()
                .with_documentation("The shape of a point, authored per mesh."),
        ),
        (
            POINTS.to_owned(),
            Schema::new(SchemaKind::list(Schema::new(SchemaKind::Any)))
                .with_documentation("Verified against the mesh's own point schema."),
        ),
        (
            INSTANCE_DATA.to_owned(),
            Schema::new(SchemaKind::Any)
                .with_documentation("Verified against the material shader's instance schema."),
        ),
        (MATERIAL.to_owned(), material_schema()),
    ]))
    .with_documentation("A mesh, drawn by the stage its material belongs to.")
}

/// Builds the schema of one material.
pub fn material_schema() -> Schema {
    Schema::new(SchemaKind::structure([
        (SHADER.to_owned(), shader_schema()),
        (
            CUSTOM_DATA.to_owned(),
            Schema::new(SchemaKind::Any)
                .with_documentation("Verified against the shader's custom schema."),
        ),
    ]))
    .with_documentation("What a mesh looks like.")
}

/// Builds the schema of a shader: the three mesh stages and the screen stage.
///
/// # Why the mesh stages are variants of one enum
///
/// The vocabulary nests a `MeshShader` around the three stage shaders, and a document
/// here says the same thing with the stage in the tag. A variant payload in this language
/// is a struct of named fields, so a nested enum would have to be a field holding a
/// single value under a synthetic name; the tag already carries it. The three stages stay
/// the variants a reader sees, and `Stage` in the emitter is what a tag means.
pub fn shader_schema() -> Schema {
    let mesh = || {
        Schema::new(SchemaKind::structure([
            (
                PATH.to_owned(),
                Schema::new(SchemaKind::String)
                    .with_documentation("Source, resolved against the shader root."),
            ),
            (
                PASS_SCHEMA.to_owned(),
                Schema::document_schema()
                    .with_documentation("Verified against the pass this stage emits."),
            ),
            (
                INSTANCE_SCHEMA.to_owned(),
                Schema::document_schema().with_documentation(
                    "Verified against the mesh's instance data, and the layout it is packed with.",
                ),
            ),
            (
                POINT_SCHEMA_FIELD.to_owned(),
                Schema::document_schema().with_documentation(
                    "Verified against every point of the mesh, alongside the mesh's own schema.",
                ),
            ),
            (
                CUSTOM_SCHEMA.to_owned(),
                Schema::document_schema().with_documentation(
                    "The custom schema: what this material's own data must be.",
                ),
            ),
        ]))
    };
    let mut variants = vec![
        ("ShadowingShader".to_owned(), mesh()),
        ("DepthShader".to_owned(), mesh()),
        ("LightingShader".to_owned(), mesh()),
    ];
    variants.push((
        "ScreenShader".to_owned(),
        Schema::new(SchemaKind::structure([
            (
                STAGE.to_owned(),
                Schema::new(SchemaKind::String)
                    .with_documentation("Name of the stage this shader belongs to."),
            ),
            (PATH.to_owned(), Schema::new(SchemaKind::String)),
            (PASS_SCHEMA.to_owned(), Schema::document_schema()),
            (INSTANCE_SCHEMA.to_owned(), Schema::document_schema()),
            (CUSTOM_SCHEMA.to_owned(), Schema::document_schema()),
        ]))
        .with_documentation("A shader over the screen, which reads no points."),
    ));
    Schema::new(SchemaKind::enumeration(variants))
        .with_documentation("The shader a material is drawn with.")
}

/// Builds a schema of exactly three floats.
fn float_triple(documentation: &str) -> Schema {
    Schema::new(SchemaKind::list(Schema::new(SchemaKind::Float))).with_documentation(documentation)
}

/// Builds the point schema a mesh gets when the author does not write one.
///
/// A mesh that needs texcoords writes its own [`POINT_SCHEMA`]; this is the one every
/// mesh can rely on being valid against.
pub fn default_point_schema() -> Schema {
    Schema::new(SchemaKind::structure([(
        "position".to_owned(),
        float_triple("Position in object space."),
    )]))
    .with_documentation("A position and nothing else.")
}

/// Builds the instance schema a standard material gets when it does not write one.
///
/// The per instance data is packed according to this schema, which is why it has to be a
/// struct: the field order is the layout.
pub fn default_instance_schema() -> Schema {
    Schema::new(SchemaKind::structure([(
        "model".to_owned(),
        Schema::new(SchemaKind::list(Schema::new(SchemaKind::Float)))
            .with_documentation("Model matrix, column major."),
    )]))
    .with_documentation("One model matrix per instance.")
}

/// Builds the custom schema a standard material gets when it does not write one.
pub fn default_custom_schema() -> Schema {
    Schema::new(SchemaKind::structure([(
        "tint".to_owned(),
        float_triple("Colour multiplier, in zero to one."),
    )]))
    .with_documentation("One tint per material.")
}
