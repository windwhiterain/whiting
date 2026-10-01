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

/// Field holding the shaders of the scene.
pub const SHADERS: &str = "shaders";

/// Field holding the meshes of the scene.
pub const MESHES: &str = "meshes";

/// Field holding the instances of the scene.
pub const INSTANCES: &str = "instances";

/// Field holding the camera the scene is drawn through.
pub const CAMERA: &str = "camera";

/// Field holding where the camera is.
pub const EYE: &str = "eye";

/// Field holding what the camera looks at.
pub const TARGET: &str = "target";

/// Field holding which way is up, for the camera.
pub const UP: &str = "up";

/// Field holding the angle the camera sees through, in radians.
pub const FIELD_OF_VIEW: &str = "field_of_view";

/// Field holding which projection the camera draws through.
pub const PROJECTION: &str = "projection";

/// Field holding how much wider than its box the camera frames the scene.
pub const PADDING: &str = "padding";

/// Field holding how far from the eye the near plane of the camera is.
pub const NEAR: &str = "near";

/// Field holding how far from the eye the far plane of the camera is.
pub const FAR: &str = "far";

/// Field holding the image the scene is drawn into.
pub const FRAME: &str = "frame";

/// Field holding the time the scene is drawn at, in seconds.
pub const TIME: &str = "time";

/// Field holding the label of a mesh an instance draws.
pub const MESH: &str = "mesh";

/// Field holding where an instance is.
pub const TRANSFORM: &str = "transform";

/// Field holding the label of a mesh.
pub const LABEL: &str = "label";

/// Field holding how a mesh's points are read.
pub const TOPOLOGY: &str = "topology";

/// Field holding the schema a mesh's points are verified against.
pub const POINT_SCHEMA: &str = "point_schema";

/// Field holding the points of a mesh.
pub const POINTS: &str = "points";

/// Field holding the material an instance is drawn with.
pub const MATERIAL: &str = "material";

/// Field holding the shader a material is drawn with.
pub const SHADER: &str = "shader";

/// Field holding the data a material's own shader reads.
pub const CUSTOM_DATA: &str = "custom_data";

/// Field holding the text a shader is written in.
pub const SOURCE: &str = "source";

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
            SHADERS.to_owned(),
            Schema::new(SchemaKind::list(shader_schema())).with_documentation(
                "Shaders, listed once per shader. A material names one by label, so two \
                 objects drawn the same way carry one copy of the source.",
            ),
        ),
        (
            MESHES.to_owned(),
            Schema::new(SchemaKind::list(mesh_schema().reference())).with_documentation(
                "Geometry, listed once per shape. A mesh is drawn only where an instance \
                 names it. A mesh may be written here or named from another definition, \
                 which is what lets several scenes share one mesh instead of each holding \
                 its own copy of the points.",
            ),
        ),
        (
            INSTANCES.to_owned(),
            Schema::new(SchemaKind::list(instance_schema())).with_documentation(
                "One instance is drawn by the stage its material's shader belongs to, so a \
                 mesh drawn in three stages is three instances of one mesh.",
            ),
        ),
        (
            CAMERA.to_owned(),
            camera_schema().optional().with_documentation(
                "Where the scene is drawn from. A scene without a camera is drawn by the \
                     stated convention: parallel, dead ahead, framing everything that is drawn.",
            ),
        ),
        (
            TIME.to_owned(),
            Schema::new(SchemaKind::Float)
                .optional()
                .with_documentation(
                    "Seconds since the scene started, which every pass carries to its shader. \
                     A scene that does not write one is a still, and a scene that does is \
                     reproducible: the same documents at the same time give the same frame.",
                ),
        ),
        (
            FRAME.to_owned(),
            frame_schema().optional().with_documentation(
                "How large the image is. A scene that names no frame is drawn into the \
                 square the pipeline has always drawn into.",
            ),
        ),
    ]))
    .with_documentation("The conventional lit scene: lights, shaders, geometry, and what draws it.")
}

/// Builds the schema of the image a scene is drawn into.
///
/// # Invariants
///
/// Both fields are optional, and a field that is absent falls back to the square the
/// pipeline has always drawn into, so a scene that names no frame draws the frame it
/// always drew. The frame is a property of the scene rather than a constant of the
/// backend because its aspect is part of the composition: a wide valley drawn into a
/// square is cropped at the sides, and the same valley drawn wide is not.
pub fn frame_schema() -> Schema {
    Schema::new(SchemaKind::structure([
        (
            "width".to_owned(),
            Schema::new(SchemaKind::Int)
                .optional()
                .with_documentation("How wide the shaded image is, in pixels."),
        ),
        (
            "height".to_owned(),
            Schema::new(SchemaKind::Int)
                .optional()
                .with_documentation("How tall the shaded image is, in pixels."),
        ),
    ]))
    .with_documentation("The image a scene is drawn into.")
}

/// Builds the schema of a camera.
///
/// # Invariants
///
/// Every field but `eye` is optional, and a field that is absent falls back to the
/// convention rather than to a default the author has to know. A camera that names only an
/// eye is a camera that looks at the middle of what is drawn through a fifty degree lens.
pub fn camera_schema() -> Schema {
    Schema::new(SchemaKind::structure([
        (
            EYE.to_owned(),
            float_triple("Where the camera is, in world space."),
        ),
        (
            TARGET.to_owned(),
            float_triple("What the camera looks at. Defaults to the middle of what is drawn.")
                .optional(),
        ),
        (
            UP.to_owned(),
            float_triple("Which way is up. Defaults to the world y axis.").optional(),
        ),
        (
            FIELD_OF_VIEW.to_owned(),
            Schema::new(SchemaKind::Float)
                .optional()
                .with_documentation("The angle seen through, vertically, in radians."),
        ),
        (
            PROJECTION.to_owned(),
            Schema::new(SchemaKind::enumeration([
                (
                    "Perspective".to_owned(),
                    Schema::new(SchemaKind::structure([])),
                ),
                (
                    "Orthographic".to_owned(),
                    Schema::new(SchemaKind::structure([])),
                ),
            ]))
            .optional()
            .with_documentation(
                "Which projection the camera draws through. A parallel projection measures \
                 every distance the same, which is what a diagram is made of.",
            ),
        ),
        (
            PADDING.to_owned(),
            Schema::new(SchemaKind::Float)
                .optional()
                .with_documentation(
                    "How much wider than the scene the camera frames it. A value above one leaves \
                 a margin; below one crops.",
                ),
        ),
        (
            NEAR.to_owned(),
            Schema::new(SchemaKind::Float)
                .optional()
                .with_documentation("How far from the eye the near plane is."),
        ),
        (
            FAR.to_owned(),
            Schema::new(SchemaKind::Float)
                .optional()
                .with_documentation("How far from the eye the far plane is."),
        ),
    ]))
    .with_documentation("Where the scene is drawn from.")
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

/// Schema identifier a mesh definition is registered under when a scene names it.
///
/// # Why the framework names it
///
/// A scene may name a mesh that lives in another definition, and the definition that holds it is
/// read with [`mesh_schema`]. Which identifier it is registered under is the client's to choose,
/// but a scene that names a mesh has to name the *same* one, so the framework states the
/// conventional one rather than leaving two clients to invent two.
pub const MESH_SCHEMA: &str = "whiting.scene.mesh";

/// Builds the schema of one mesh: geometry, and nothing about where it is drawn.
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
    ]))
    .with_documentation("Geometry an instance draws, listed once and referenced by label.")
}

/// Builds the schema of one instance: a mesh, where it is, and what draws it.
pub fn instance_schema() -> Schema {
    Schema::new(SchemaKind::structure([
        (
            LABEL.to_owned(),
            Schema::new(SchemaKind::String)
                .with_documentation("Name used in pass labels and in errors."),
        ),
        (
            MESH.to_owned(),
            Schema::new(SchemaKind::String).with_documentation(
                "The label of a mesh in this scene, which is how one mesh is drawn by \
                 several stages without its points being listed again.",
            ),
        ),
        (TRANSFORM.to_owned(), transform_schema()),
        (MATERIAL.to_owned(), material_schema()),
    ]))
    .with_documentation("A mesh placed in the scene and drawn by one stage.")
}

/// Builds the schema of a transform: where an instance is, and how it is turned and sized.
pub fn transform_schema() -> Schema {
    Schema::new(SchemaKind::structure([
        (
            "translation".to_owned(),
            float_triple("Offset from the origin, in world space."),
        ),
        (
            "rotation".to_owned(),
            float_triple("Rotation in radians about x, then y, then z."),
        ),
        ("scale".to_owned(), float_triple("Scale per axis.")),
    ]))
    .with_documentation("Composed into the model matrix the instance row carries.")
}

/// Builds the schema of one material.
pub fn material_schema() -> Schema {
    Schema::new(SchemaKind::structure([
        (
            SHADER.to_owned(),
            Schema::new(SchemaKind::String).with_documentation(
                "The label of a shader this scene declares, which is where the stage an \
                 instance is drawn in comes from.",
            ),
        ),
        (
            CUSTOM_DATA.to_owned(),
            Schema::new(SchemaKind::Any)
                .with_documentation("Verified against the shader's custom schema."),
        ),
    ]))
    .with_documentation("How an instance is drawn.")
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
    let label = (
        LABEL.to_owned(),
        Schema::new(SchemaKind::String).with_documentation(
            "The name a material refers to this shader by, used in pass labels and in errors.",
        ),
    );
    let source = (
        SOURCE.to_owned(),
        Schema::new(SchemaKind::String).with_documentation(
            "The shader source. A shader carries its own text so that a compiled scene \
             renders wherever it is sent.",
        ),
    );

    let mesh = || {
        Schema::new(SchemaKind::structure([
            label.clone(),
            source.clone(),
            (
                PASS_SCHEMA.to_owned(),
                Schema::document_schema()
                    .with_documentation("Verified against the pass this stage emits."),
            ),
            (
                INSTANCE_SCHEMA.to_owned(),
                Schema::document_schema().with_documentation(
                    "Verified against the instance row, and the layout it is packed with.",
                ),
            ),
            (
                POINT_SCHEMA_FIELD.to_owned(),
                Schema::document_schema().with_documentation(
                    "Verified against every point of the mesh an instance draws, alongside \
                     the mesh's own schema.",
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
            label,
            source,
            (PASS_SCHEMA.to_owned(), Schema::document_schema()),
            (INSTANCE_SCHEMA.to_owned(), Schema::document_schema()),
            (CUSTOM_SCHEMA.to_owned(), Schema::document_schema()),
        ]))
        .with_documentation("A shader over the screen, which reads no points."),
    ));
    Schema::new(SchemaKind::enumeration(variants))
        .with_documentation("A shader a material names, declared once per scene.")
}

/// Builds a schema of exactly three floats.
///
/// The width is declared rather than left to the value, because a point schema is also the
/// layout a vertex buffer is read with, and a list that does not declare its width has no
/// layout at all.
fn float_triple(documentation: &str) -> Schema {
    graphics::layout::components_schema(3).with_documentation(documentation)
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
/// struct and why the matrix declares its width: the field order is the layout, and a
/// matrix is read as four attributes of four components each.
pub fn default_instance_schema() -> Schema {
    Schema::new(SchemaKind::structure([(
        "model".to_owned(),
        graphics::layout::components_schema(16).with_documentation("Model matrix, column major."),
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
