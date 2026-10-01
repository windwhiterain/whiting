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

/// Binding point holding the image a post stage renders into.
///
/// A pass that reads the colour target cannot also write it, so a post stage writes
/// somewhere else. That image is the one the frame is read back from, which is what makes
/// the post stage the end of the pipeline rather than a pass something else follows.
pub const PRESENT_TARGET: &str = "present_target";

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

/// Field holding the light's own view projection, in a lighting pass.
///
/// A lighting pass needs both projections and they are not the same one: the scene is drawn
/// through `projection`, and a surface is in shadow when it is further from the light than
/// the shadow map recorded. Deciding that needs the world position of the surface carried
/// into the light's own clip space, which is what this matrix is for. Without it a shader
/// can only read the shadow map at a screen coordinate, which is a different lookup and not
/// a shadow.
pub const LIGHT_PROJECTION: &str = "light_projection";

/// Field holding the size of one texel of the shadow map, in a lighting pass.
///
/// The shadow map is sampled with a filter kernel, and a kernel of a fixed width in texels
/// is a fixed width on the surface only because the map's resolution is known. Carrying it
/// keeps a scene that changes the shadow map's size from changing the width of its penumbra.
pub const LIGHT_SHADOW_TEXEL: &str = "light_shadow_texel";

/// Field holding the time a pass is drawn at, in seconds.
///
/// Time is data on the pass rather than a clock inside the renderer, so a render stays a
/// pure function of the documents and this one number: a frame taken at `t = 2.5` is
/// reproduced exactly by rendering the same documents at `t = 2.5` again.
pub const TIME: &str = "time";

/// Field holding the camera a post pass reconstructs its rays from.
pub const CAMERA: &str = "camera";

/// Field holding how far the near and far planes are, as two numbers rather than one.
///
/// It is on the pass for the same reason the camera's basis is: turning a depth back into a
/// distance divides by a quantity the stage is not otherwise given and cannot recover, and
/// `WGSL` has no matrix inverse to recover it with.
pub const DEPTH_RANGE: &str = "depth_range";

/// Field holding the direction the camera looks along.
pub const FORWARD: &str = "forward";

/// Field holding the axis the camera's right runs along.
pub const RIGHT: &str = "right";

/// Field holding the axis the camera's up runs along.
pub const UP: &str = "up";

/// Field holding where the camera is.
pub const POSITION: &str = "position";

/// Field holding how far off centre the edge of the frame is.
pub const HALF_EXTENT: &str = "half_extent";

/// Builds the schema of the camera a post pass draws through.
///
/// # Invariants
///
/// The basis is carried beside the projection rather than derived from it, because a ray
/// through a pixel is recovered from a matrix by an inverse that a shader cannot compute:
/// `WGSL` has no matrix inverse. A post pass that has these three axes and the half
/// extent of the frame reconstructs the ray the camera would have cast, and with it a sky.
///
/// The fields sort as `forward`, `half_extent`, `position`, `right`, `up`.
pub fn camera_schema() -> Schema {
    Schema::new(SchemaKind::structure([
        (
            FORWARD.to_owned(),
            vector().with_documentation("The direction the camera looks along."),
        ),
        (
            HALF_EXTENT.to_owned(),
            vector2().with_documentation(
                "How far off centre the edge of the frame is, horizontally and then \
                 vertically. A ray through a pixel is forward plus so much of right and up \
                 as that pixel is off centre.",
            ),
        ),
        (
            POSITION.to_owned(),
            vector().with_documentation("Where the camera is, in world space."),
        ),
        (
            RIGHT.to_owned(),
            vector().with_documentation("The axis the camera's right runs along."),
        ),
        (
            UP.to_owned(),
            vector().with_documentation("The axis the camera's up runs along."),
        ),
    ]))
    .with_documentation("The camera a post pass draws through, and the rays it casts.")
}

/// Width in pixels of every shadow map the standard pipeline renders.
///
/// Two thousand over the few hundred units a scene's shadow volume spans is a fifth of a
/// unit a texel, which is what a step two units wide needs for its shadow to outlive the
/// blur the comparison kernel spreads it by. At one thousand the texel was the step.
pub const SHADOW_MAP_WIDTH: i64 = 2048;

/// Height in pixels of every shadow map the standard pipeline renders.
pub const SHADOW_MAP_HEIGHT: i64 = 2048;

/// Format every shadow map is rendered in.
pub const SHADOW_MAP_FORMAT: &str = "depth32float";

/// Format the depth buffer is rendered in.
///
/// The buffer's size is not a constant: the shaded image, this and the present target are
/// all the size of the scene's `frame`, because a post stage reads the first two at the
/// coordinates of the pixel it is writing to the third. See
/// [`crate::standard::frame`].
pub const DEPTH_FORMAT: &str = "depth32float";

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
        vec![
            (
                PROJECTION,
                projection_schema().with_documentation("Looking down the light's direction."),
            ),
            (
                TIME,
                time().with_documentation("Seconds since the scene started."),
            ),
        ],
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
        vec![
            (
                PROJECTION,
                projection_schema().with_documentation("Looking at the scene from the camera."),
            ),
            (
                TIME,
                time().with_documentation("Seconds since the scene started."),
            ),
        ],
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
            (
                LIGHT_PROJECTION,
                matrix().with_documentation(
                    "The light's own view projection, which carries a world position into the \
                     shadow map's clip space.",
                ),
            ),
            (
                LIGHT_SHADOW_TEXEL,
                vector2().with_documentation("The size of one texel of the shadow map."),
            ),
            (
                TIME,
                time().with_documentation("Seconds since the scene started."),
            ),
        ],
    )
}

/// Builds the interface of a post pass, which draws over the whole frame.
///
/// # Invariants
///
/// A post pass is handed the shaded image and the depth that produced it, and writes a
/// third image. It has no projection and no points of its own beyond the three corners of
/// a fullscreen triangle, so its uniform is the time and nothing else: what a post stage
/// reads is the frame, not the scene.
///
/// A post pass declares no depth of its own. It has to be able to sample the depth buffer
/// it is drawn over, and a pass that both tested against that buffer and sampled it would
/// be reading and writing one texture at once.
pub fn post_pass_schema() -> Schema {
    interface(
        "A post pass, which draws over the whole frame.",
        vec![
            (
                VERTICES,
                buffer_binding_schema()
                    .with_documentation("The fullscreen triangle this pass draws."),
            ),
            (
                COLOR_TARGET,
                texture_binding_schema().with_documentation("The shaded image this pass reads."),
            ),
            (
                DEPTH_BUFFER,
                texture_binding_schema()
                    .with_documentation("The depth that produced the shaded image."),
            ),
            (
                PRESENT_TARGET,
                texture_binding_schema()
                    .with_documentation("The image this pass renders into, and the frame."),
            ),
        ],
        vec![
            (
                CAMERA,
                camera_schema().with_documentation(
                    "The camera this pass draws through, which is what a sky is built from.",
                ),
            ),
            (
                PROJECTION,
                projection_schema().with_documentation("The camera's own projection."),
            ),
            (
                TIME,
                time().with_documentation("Seconds since the scene started."),
            ),
            (
                DEPTH_RANGE,
                vector2().with_documentation(
                    "The distance to the near and far planes, which is what turns a depth into \
                     a distance.",
                ),
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

/// Builds the schema of a two component vector.
fn vector2() -> Schema {
    graphics::layout::components_schema(2).with_documentation("Two components.")
}

/// Builds the schema of a time in seconds.
///
/// One float rather than a list: a time is a single number, and a list of one component
/// would be read as a four component vector by anything that went by the schema alone.
fn time() -> Schema {
    Schema::new(SchemaKind::Float).with_documentation("Seconds since the scene started.")
}
