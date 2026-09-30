//! The schema of what each stage of the standard pipeline supplies to its shader.
//!
//! A pass schema is an interface: the `uniform` and the `binders` a pass of that stage
//! carries, described as one struct. That is exactly the shape a shader's `pass_schema`
//! is checked against, so a material that wants the standard shadowing pass writes
//! [`shadowing_pass_schema`] and the compiler is on the hook to supply exactly that.
//!
//! # Invariants
//!
//! The binding point names here are the contract between the compiler and the shaders
//! that read it. Renaming one is a change to this file and to every shader that declared
//! it, which is why they are constants rather than literals spread across the crate.

use definition::{Schema, SchemaKind};
use graphics::binders::{buffer_binding_schema, texture_binding_schema};
use graphics::pass::{BINDERS, UNIFORM};

/// Binding point holding the depth map a shadowing pass renders into.
pub const SHADOW_MAP: &str = "shadow_map";

/// Binding point holding the depth buffer a depthing pass renders into.
pub const DEPTH_BUFFER: &str = "depth_buffer";

/// Binding point holding the image a lighting pass renders into.
pub const COLOR_TARGET: &str = "color_target";

/// Binding point holding the vertex data of every mesh a pass draws.
pub const VERTICES: &str = "vertices";

/// Binding point holding the per instance data of every mesh a pass draws.
pub const INSTANCE_DATA: &str = "instance_data";

/// Field holding the projection a pass draws through.
pub const PROJECTION: &str = "projection";

/// Field holding the position of a light, in a lighting pass.
pub const LIGHT_POSITION: &str = "light_position";

/// Field holding the colour of a light, in a lighting pass.
pub const LIGHT_COLOR: &str = "light_color";

/// Width in pixels of every shadow map the standard pipeline renders.
pub const SHADOW_MAP_WIDTH: i64 = 1024;

/// Height in pixels of every shadow map the standard pipeline renders.
pub const SHADOW_MAP_HEIGHT: i64 = 1024;

/// Format every shadow map is rendered in.
pub const SHADOW_MAP_FORMAT: &str = "depth32float";

/// Width in pixels of the depth buffer the standard pipeline renders.
pub const DEPTH_WIDTH: i64 = 1024;

/// Height in pixels of the depth buffer the standard pipeline renders.
pub const DEPTH_HEIGHT: i64 = 1024;

/// Format the depth buffer is rendered in.
pub const DEPTH_FORMAT: &str = "depth32float";

/// Width in pixels of the image the standard pipeline shades into.
pub const COLOR_WIDTH: i64 = 1024;

/// Height in pixels of the image the standard pipeline shades into.
pub const COLOR_HEIGHT: i64 = 1024;

/// Format the shaded image is rendered in.
pub const COLOR_FORMAT: &str = "rgba8unorm";

/// Builds the schema of a projection, which every stage draws through.
///
/// A projection is data on the pass rather than arithmetic in the renderer, because a
/// reload may change the scene the projection covers and the pass has to say so. The
/// matrix is sixteen floats, column major, which is the order a shader indexes in.
pub fn projection_schema() -> Schema {
    Schema::new(SchemaKind::structure([
        (
            "view_projection".to_owned(),
            matrix().with_documentation("Projection times view, column major."),
        ),
        (
            "position".to_owned(),
            vector().with_documentation("Where the projection was made from."),
        ),
    ]))
    .with_documentation("What a pass draws through.")
}

/// Builds the interface of a shadowing pass.
pub fn shadowing_pass_schema() -> Schema {
    interface(
        "A shadowing pass, which renders one light's depth map.",
        vec![
            (
                VERTICES,
                buffer_binding_schema()
                    .with_documentation("Vertex data of the shadow casting meshes."),
            ),
            (
                INSTANCE_DATA,
                buffer_binding_schema().with_documentation("Per instance data of those meshes."),
            ),
            (
                SHADOW_MAP,
                texture_binding_schema()
                    .with_documentation("The depth map this pass renders into."),
            ),
        ],
        vec![(
            PROJECTION,
            projection_schema().with_documentation("Looking down the light's direction."),
        )],
    )
}

/// Builds the interface of a depthing pass.
pub fn depthing_pass_schema() -> Schema {
    interface(
        "A depthing pass, which renders the depth of every mesh in the scene.",
        vec![
            (
                VERTICES,
                buffer_binding_schema().with_documentation("Vertex data of the meshes."),
            ),
            (
                INSTANCE_DATA,
                buffer_binding_schema().with_documentation("Per instance data of those meshes."),
            ),
            (
                DEPTH_BUFFER,
                texture_binding_schema()
                    .with_documentation("The depth buffer this pass renders into."),
            ),
        ],
        vec![(
            PROJECTION,
            projection_schema().with_documentation("Looking at the scene from the camera."),
        )],
    )
}

/// Builds the interface of a lighting pass.
pub fn lighting_pass_schema() -> Schema {
    interface(
        "A lighting pass, which shades every mesh in the scene.",
        vec![
            (
                VERTICES,
                buffer_binding_schema().with_documentation("Vertex data of the meshes."),
            ),
            (
                INSTANCE_DATA,
                buffer_binding_schema().with_documentation("Per instance data of those meshes."),
            ),
            (
                COLOR_TARGET,
                texture_binding_schema().with_documentation("The image this pass renders into."),
            ),
            (
                DEPTH_BUFFER,
                texture_binding_schema()
                    .with_documentation("The depth the earlier passes produced."),
            ),
            (
                SHADOW_MAP,
                texture_binding_schema().with_documentation("The shadow map of every light."),
            ),
        ],
        vec![
            (
                PROJECTION,
                projection_schema().with_documentation("Looking at the scene from the camera."),
            ),
            (
                LIGHT_POSITION,
                vector().with_documentation("Position of the light being applied."),
            ),
            (
                LIGHT_COLOR,
                vector().with_documentation("Colour of the light, scaled by its intensity."),
            ),
        ],
    )
}

/// Builds a shader interface of a `uniform` and a `binders` block.
fn interface(
    documentation: &str,
    binders: Vec<(&str, Schema)>,
    uniform: Vec<(&str, Schema)>,
) -> Schema {
    Schema::new(SchemaKind::structure([
        (
            UNIFORM.to_owned(),
            Schema::new(SchemaKind::structure(
                uniform
                    .into_iter()
                    .map(|(field, schema)| (field.to_owned(), schema)),
            ))
            .with_documentation("Uniform data, uploaded verbatim."),
        ),
        (
            BINDERS.to_owned(),
            Schema::new(SchemaKind::structure(
                binders
                    .into_iter()
                    .map(|(point, schema)| (point.to_owned(), schema)),
            ))
            .with_documentation("What the pass binds, keyed by binding point."),
        ),
    ]))
    .with_documentation(documentation)
}

/// Builds the schema of a four by four matrix, written as sixteen floats.
fn matrix() -> Schema {
    graphics::layout::components_schema(16).with_documentation("Four by four matrix, column major.")
}

/// Builds the schema of a three component vector.
fn vector() -> Schema {
    graphics::layout::components_schema(3).with_documentation("Three components.")
}
