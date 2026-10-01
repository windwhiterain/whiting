//! The schema of what each stage of the standard pipeline supplies to its shader.
//!
//! A pass schema is an interface: the `uniform` and the `binders` a pass of that stage
//! carries, described as one struct. That is exactly the shape a shader's `pass_schema`
//! is checked against, so a material that wants the standard shadowing pass writes
//! [`shadow_pass_schema`] and the compiler is on the hook to supply exactly that.
//!
//! # Invariants
//!
//! The binding point names here are the contract between the compiler and the shaders
//! that read it. Renaming one is a change to this file and to every shader that declared
//! it, which is why they are constants rather than literals spread across the crate.

use definition::{Schema, SchemaKind};
use graphics::binders::{
    buffer_binding_schema, texture_array_binding_schema, texture_binding_schema,
};
use graphics::layout::components_schema;
use graphics::pass::{BINDERS, UNIFORM};

use crate::lighting::{CLUSTER_COUNTS_ENTRY, CLUSTER_INDICES_ENTRY, LIGHT_RECORDS_ENTRY};

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

/// Binding point holding the pool of shadow pages a scene's lights were given.
///
/// # Invariants
///
/// This is one instance entry for the whole scene, not one per light, which is the whole
/// point of a pool: a thousand lights that could each have a page share one texture array
/// and one binding point. A shadowing pass names the page it draws into on its **declaration**
/// of this point, and a lighting pass names none, because a shader indexes the layers itself
/// with the page its own light record carries.
///
/// It is called a pool rather than `shadow_pages` on purpose: `shadow_pages` is the count a
/// scene's `clustering` block declares, and two different things in one name is a name an
/// author reads as one of them.
pub const SHADOW_POOL: &str = "shadow_pool";

/// Field holding how the view is divided for clustered lighting, as four whole numbers.
///
/// Across, up, deep, and how many lights one cluster keeps, in that order. They are one
/// vector rather than four fields because a fragment multiplies its own position by them
/// four times and a vector says so; a shader field per count would be four scalars padded to
/// four slots each, which is the same sixteen bytes for no reason.
pub const CLUSTER_GRID: &str = "cluster_grid";

/// Field holding the size of the frame in pixels, as width then height.
///
/// A fragment resolves which cluster it is in from its own position, and a position in
/// pixels only means a column and a row once the frame's size divides it. The depth needs no
/// size, because the view's depth runs zero to one whatever the frame is.
pub const FRAME_SIZE: &str = "frame_size";

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

/// Format every shadow page is rendered in.
///
/// A page is a depth map like any other. Its **size** is not a constant: it is
/// `shadow_page_width` and `shadow_page_height` on the scene's `clustering` block, and the
/// same rule the depth buffer's size follows. See [`crate::standard::frame`].
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
///
/// # Invariants
///
/// A shadowing pass draws **one light's depth into one page of the pool**, and the page is
/// named on the declaration of [`SHADOW_POOL`] rather than anywhere else. That is why this
/// interface declares the pool as a texture array: a flat texture is a `D2` view and an
/// array is a `D2Array` view, they are different types to a shader, and a schema that
/// accepted both would let a document draw into a pool no shader can read.
pub fn shadow_pass_schema() -> Schema {
    interface(
        "A shadowing pass, which renders one light's depth into one page of the pool.",
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
                SHADOW_POOL,
                texture_array_binding_schema()
                    .with_documentation("The pool of pages, and the page this pass draws into."),
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
pub fn depth_pass_schema() -> Schema {
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

/// Builds the interface of an emission pass.
///
/// # Invariants
///
/// An emission pass carries no pass data of its own. What a surface emits is not lit, so the
/// stage reads no light, no shadow map and no projection of a light's: a projection, the two
/// buffers every mesh pass reads, and the depth it tests against are the whole of it. That is
/// why emission is the one stage whose interface has nothing of its own to supply — the
/// reason it exists is to be drawn once rather than once per light, and a pass that read a
/// light would be drawn once per light again.
pub fn emission_pass_schema() -> Schema {
    interface(
        "An emission pass, which adds what every mesh in the scene emits.",
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
                texture_binding_schema().with_documentation("The image this pass adds into."),
            ),
            (
                DEPTH_BUFFER,
                texture_binding_schema()
                    .with_documentation("The depth the earlier passes produced."),
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
pub fn light_pass_schema() -> Schema {
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
                LIGHT_RECORDS_ENTRY,
                buffer_binding_schema().with_documentation(
                    "One record per light, which is what a cluster's indices point into.",
                ),
            ),
            (
                CLUSTER_COUNTS_ENTRY,
                buffer_binding_schema().with_documentation(
                    "How many lights each cluster kept, which is the bound \
                     of the loop.",
                ),
            ),
            (
                CLUSTER_INDICES_ENTRY,
                buffer_binding_schema().with_documentation(
                    "The lights of each cluster, a row of the cap wide for each one.",
                ),
            ),
            (
                SHADOW_POOL,
                texture_array_binding_schema().with_documentation(
                    "The pages of every light that was given one, which the shader indexes \
                     itself with the page each record carries.",
                ),
            ),
        ],
        vec![
            (
                PROJECTION,
                projection_schema().with_documentation("Looking at the scene from the camera."),
            ),
            (
                CLUSTER_GRID,
                components_schema(4).with_documentation(
                    "How the view is divided: how many clusters across, how many up, how many \
                     depth slices, and how many lights one cluster keeps.",
                ),
            ),
            (
                FRAME_SIZE,
                vector2().with_documentation(
                    "The frame's width and height in pixels, which is what turns a fragment's \
                     own position into a column and a row.",
                ),
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
