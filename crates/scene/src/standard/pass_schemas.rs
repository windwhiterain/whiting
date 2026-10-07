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

use crate::lighting::{
    CLUSTER_COUNTS_ENTRY, CLUSTER_INDICES_ENTRY, LIGHT_RECORDS_ENTRY, SPOT_CONES_ENTRY,
};

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
/// scene's `frame` block declares, and two different things in one name is a name an
/// author reads as one of them.
pub const SHADOW_POOL: &str = "shadow_pool";

/// Binding point holding the **atlas** a light's virtual shadow pages are drawn into.
///
/// # Invariants
///
/// It sits beside [`SHADOW_POOL`] rather than replacing it, because the two are one texture array
/// apiece and the migration is per **kind**: a spot light's shadow lives in the atlas and a
/// directional or point light's still lives in the pool, and a scene with both draws both kinds
/// with one shader. The pool goes when the last kind moves, and until then a shader branches on
/// which of a light's two addresses was written rather than on its kind, so a kind that moves
/// later needs no shader edit.
///
/// It is bound by **every** shadowing pass, including one drawing a pool page and one belonging to
/// a scene that allocated no pages at all, for the reason [`SPOT_CONES_ENTRY`] is bound by a
/// lighting pass whose scene names no spot light: a binding point is numbered by its name's
/// position in the pass, so a pass that left this one out would move every point after it.
///
/// One instance entry for the whole scene, layers named on a shadowing pass's **declaration** of
/// it exactly as a pool page is — and the same argument applies: a payload naming a layer would
/// name it for the whole image. Which layer a light's pages live in is its record's
/// `virtual_page.y`, and a fragment that read the wrong one would shade against a neighbour lamp.
pub const SHADOW_ATLAS: &str = "shadow_atlas";

/// Field holding which rectangle of a light's own normalised plane one shadow page covers.
///
/// # Invariants
///
/// It is `[ minimum x, minimum y, width, height ]` in the light's device coordinates, which is the
/// form [`crate::vshadow::Page::window`] is stored in and the form a vertex stage wants: the pass
/// carries the **light's whole projection** rather than a projection fitted to the page, so the
/// page's job is to map the clip output of that one projection into the slot it was given. A page
/// of its own would mean a near and far plane per page derived from a depth range that belongs to
/// the light, which is the second copy of one arithmetic this pipeline refuses.
///
/// Absent from no shadowing pass: one that draws a light's whole volume into one pool page carries
/// the whole plane, `[-1, -1, 2, 2]`, which is the identity of this mapping. Which of the two a
/// pass is told apart by the texture it names as its target, never by whether this field is there.
pub const VIEW_PAGE: &str = "view_page";

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

/// Field holding how many of the scene's lights every fragment evaluates whatever cluster it
/// is in.
///
/// # Invariants
///
/// This is a **count and not a list**, because the lights it counts are already written: they
/// are the first records of the light record run and the first entries of the cone run beside
/// it, and the two are indexed by the same number. A fragment loops over `[0, global_count)`
/// and reads the same pair for every index, which is why a global light needs nothing in the
/// uniform but its own number: everything else about it is in the two runs the clustered loop
/// already reads.
///
/// It is written for every lighting pass, present or not, for the reason [`ENVIRONMENT`] is
/// written for every pass: the uniform's layout is the same for every scene. A scene with no
/// global light writes zero, and the loop above it does not run.
pub const GLOBAL_COUNT: &str = "global_count";

/// Field holding the environment a lighting pass is shaded against.
///
/// # Invariants
///
/// It is **four** numbers and not three: the light's colour scaled by its intensity, and then
/// whether the scene named an `Environment` light at all. The flag is in the same vector rather
/// than beside it because two numbers a shader could disagree about are one too many — the
/// shader reads the vector and the flag is already in it — and because a colour of zero is a
/// perfectly good environment to want, so a shader that inferred presence from a black map
/// would be reading the wrong question.
///
/// The field is written for every lighting pass, whether the scene names an environment or
/// not, because the uniform's layout is the same for every scene. A field that came and went
/// with the document would put every field after it somewhere else.
pub const ENVIRONMENT: &str = "environment";

/// Field holding the scene's directional light, which every lighting pass is shaded against.
///
/// # Invariants
///
/// It is a **stated input of the pass** and not a light record, for the same reason
/// [`ENVIRONMENT`] is: the record is read through a cluster list and this light is in none, so
/// a fragment has no path to a record of it. It differs from the environment in the one way
/// that matters — it casts a shadow — so it carries a page and an orthographic projection
/// where the environment carries neither.
///
/// A nested struct rather than three fields of its own, because one light is one thing. The
/// light record run has one entry per light for exactly this reason, and a uniform reading as
/// `directional_colour` beside `directional_projection` would be one light spelled three times.
///
/// The fields sort as `colour`, `direction`, `shadow_projection`.
///
/// The field is written for every lighting pass, whether the scene names a directional light
/// or not, for the reason [`ENVIRONMENT`] is written for every pass: the uniform's layout is
/// the same for every scene.
pub const DIRECTIONAL: &str = "directional";

/// Builds the interface of a directional light as a lighting pass's uniform carries it.
///
/// The page is the **fourth component of the direction** rather than a field of its own, so
/// that a fragment reads and tests it in one comparison exactly as it reads and tests a
/// record's own page. The flag is the fourth component of the colour, following [`ENVIRONMENT`]
/// for the same reason: a colour of zero is a perfectly good sun to want, so a shader that
/// inferred presence from the colour would be reading the wrong question.
pub fn directional_schema() -> Schema {
    Schema::new(SchemaKind::structure([
        (
            "colour".to_owned(),
            components_schema(4).with_documentation(
                "The directional light's colour scaled by its intensity, and whether the scene \
                 named one at all.",
            ),
        ),
        (
            "direction".to_owned(),
            components_schema(4).with_documentation(
                "The unit vector the light travels along, which is the way a surface faces it, \
                 and the page of the shadow pool its shadow was rendered into. The page is \
                 minus one for a light the pool gave none, which is lit unshadowed by design.",
            ),
        ),
        (
            "shadow_projection".to_owned(),
            matrix().with_documentation(
                "The orthographic matrix a fragment carries its own world position through to \
                 find its page, and the eye the shadowing pass drew that page from. Written for \
                 every lighting pass, so it is the identity in a scene naming no directional \
                 light.",
            ),
        ),
    ]))
}

/// Binding point the environment map is read at.
///
/// The name is what puts it at its binding number: a pass's binders are numbered in name
/// order, so this point sits between `depth_buffer` and any image a material names. It is
/// bound by every lighting pass — see [`ENVIRONMENT`] for why — and it declares no sampler,
/// because the map is read with `textureLoad` at a texel the shader computes.
pub const ENVIRONMENT_MAP: &str = "environment_map";

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

/// Field holding what a post pass multiplies the radiance it read by, before its own grade.
///
/// It is the **name the post uniform's own field is written under**, which is the same string
/// the scene names its exposure by: a scene that says `exposure: 0.5` and a shader that says
/// `uniforms.exposure` are agreeing about one number, and two spellings of it would be one
/// more thing to keep in step. So there is one constant and not two — it is re-exported from
/// [`crate::standard::scene_schema`], where the document's own field lives. See
/// [`post_pass_schema_exposed`] for the interface that carries it.
pub(crate) const EXPOSURE: &str = "exposure";

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
/// `shadow_page_width` and `shadow_page_height` on the scene's `frame` block, beside the
/// size of the image and the depth buffer, and the same rule their sizes follow. See
/// [`crate::standard::frame`].
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
/// A shadowing pass draws **one light's depth into one page**, and the page is named on the
/// declaration of the texture rather than anywhere else. That is why this interface declares the
/// target as a texture array: a flat texture is a `D2` view and an array is a `D2Array` view, they
/// are different types to a shader, and a schema that accepted both would let a document draw into
/// a pool no shader can read.
///
/// - **Both arrays are bound by every shadowing pass, and both are required.** A light whose
///   shadow is one page of the pool and a light whose shadow is a grid of atlas pages are drawn by
///   two passes of one shader, and a binding point is numbered by its **name's position** in the
///   pass's own list: a pass that dropped one of the two would put `vertices` at a different
///   number than the shader declares it at, which is not a lighter document but a wrong one. So
///   the array a pass does not draw into is still there, named with no layer, read by nothing —
///   exactly how [`SPOT_CONES_ENTRY`] is bound by a lighting pass whose scene names no spot light.
/// - **`view_page` is always supplied.** A pass that draws a whole volume into one page carries the
///   whole plane, `[-1, -1, 2, 2]`, which is the identity of the mapping a page's vertex stage
///   applies — so one shader body serves both paths and the field is not a branch a scene can
///   forget to declare. The alternative was an optional uniform field, which is a field a packed
///   upload has to guess the size of.
pub fn shadow_pass_schema() -> Schema {
    interface(
        "A shadowing pass, which renders one light's depth into one page of the pool or one page \
         of a light's own virtual atlas.",
        vec![
            (
                VERTICES.to_owned(),
                buffer_binding_schema()
                    .with_documentation("Vertex data of the shadow casting meshes."),
            ),
            (
                INSTANCE_DATA.to_owned(),
                buffer_binding_schema().with_documentation("Per instance data of those meshes."),
            ),
            (
                SHADOW_ATLAS.to_owned(),
                texture_array_binding_schema().with_documentation(
                    "The atlas of virtual pages. Bound by every shadowing pass, and drawn into by \
                     the ones that record a virtual page; a scene that allocated no pages at all \
                     still carries one layer of it, because a binding point is numbered by name \
                     and a pass that bound nothing here would move every point after it.",
                ),
            ),
            (
                SHADOW_POOL.to_owned(),
                texture_array_binding_schema().with_documentation(
                    "The pool of fixed pages, and the page a pool pass draws \
                                         into. Read by nothing that draws a virtual page, and \
                                         bound for the same reason the atlas is.",
                ),
            ),
        ],
        vec![
            (
                PROJECTION.to_owned(),
                projection_schema().with_documentation("Looking down the light's direction."),
            ),
            (
                VIEW_PAGE.to_owned(),
                vector4().with_documentation(
                    "Which rectangle of the light's own normalised plane this page covers, as \
                     minimum x, minimum y and the two extents. The whole plane for a pass that \
                     draws a light's one pool page.",
                ),
            ),
            (
                TIME.to_owned(),
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
                VERTICES.to_owned(),
                buffer_binding_schema().with_documentation("Vertex data of the meshes."),
            ),
            (
                INSTANCE_DATA.to_owned(),
                buffer_binding_schema().with_documentation("Per instance data of those meshes."),
            ),
            (
                DEPTH_BUFFER.to_owned(),
                texture_binding_schema()
                    .with_documentation("The depth buffer this pass renders into."),
            ),
        ],
        vec![
            (
                PROJECTION.to_owned(),
                projection_schema().with_documentation("Looking at the scene from the camera."),
            ),
            (
                TIME.to_owned(),
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
                VERTICES.to_owned(),
                buffer_binding_schema().with_documentation("Vertex data of the meshes."),
            ),
            (
                INSTANCE_DATA.to_owned(),
                buffer_binding_schema().with_documentation("Per instance data of those meshes."),
            ),
            (
                COLOR_TARGET.to_owned(),
                texture_binding_schema().with_documentation("The image this pass adds into."),
            ),
            (
                DEPTH_BUFFER.to_owned(),
                texture_binding_schema()
                    .with_documentation("The depth the earlier passes produced."),
            ),
        ],
        vec![
            (
                PROJECTION.to_owned(),
                projection_schema().with_documentation("Looking at the scene from the camera."),
            ),
            (
                TIME.to_owned(),
                time().with_documentation("Seconds since the scene started."),
            ),
        ],
    )
}

/// Builds the interface of a lighting pass.
///
/// # Invariants
///
/// **It declares the frame and the depth buffer even though no lighting shader reads either**,
/// because a target is a binding point that takes a number and emits no layout entry. A pass
/// that binds something the shader's interface does not declare is a compile error from
/// `verify_pass_inputs`, which is the point: the two halves have to agree rather than one of them
/// being generous.
pub fn light_pass_schema() -> Schema {
    light_pass_schema_over(&[])
}

/// The same interface, for a pass that also renders into the scene's own declared outputs.
///
/// # Invariants
///
/// **One entry per declared output, by the point [`output_point`] names it under** — the same
/// function the compiler creates the entry with, so a shader cannot declare a point the emitter
/// did not build. The entries are **appended after everything else**, which is what keeps every
/// other binding at the number it already had: a target takes its number from its name's
/// position among the pass's points, and inserting one in the middle would move the light records
/// under a shader that declares them by hand.
pub fn light_pass_schema_over(outputs: &[String]) -> Schema {
    let mut binders = vec![
        (
            VERTICES.to_owned(),
            buffer_binding_schema().with_documentation("Vertex data of the meshes."),
        ),
        (
            INSTANCE_DATA.to_owned(),
            buffer_binding_schema().with_documentation("Per instance data of those meshes."),
        ),
        (
            COLOR_TARGET.to_owned(),
            texture_binding_schema().with_documentation("The image this pass renders into."),
        ),
        (
            DEPTH_BUFFER.to_owned(),
            texture_binding_schema().with_documentation("The depth the earlier passes produced."),
        ),
        (
            LIGHT_RECORDS_ENTRY.to_owned(),
            buffer_binding_schema().with_documentation(
                "One record per light, which is what a cluster's indices point into.",
            ),
        ),
        (
            SPOT_CONES_ENTRY.to_owned(),
            buffer_binding_schema().with_documentation(
                "One cone per light, indexed by the same numbers as the records. Bound by \
                 every lighting pass including one whose scene names no spot light, which \
                 then binds a single whole sphere that no cluster points at, so the \
                 binding numbers are the same either way.",
            ),
        ),
        (
            CLUSTER_COUNTS_ENTRY.to_owned(),
            buffer_binding_schema().with_documentation(
                "How many lights each cluster kept, which is the bound \
                 of the loop.",
            ),
        ),
        (
            CLUSTER_INDICES_ENTRY.to_owned(),
            buffer_binding_schema().with_documentation(
                "The lights of each cluster, a row of the cap wide for each one.",
            ),
        ),
        (
            SHADOW_ATLAS.to_owned(),
            texture_array_binding_schema().with_documentation(
                "The pages of every light recorded as a grid, which the shader indexes with \
                 the address each record carries. Bound by every lighting pass, including \
                 one whose scene allocated no pages at all, because a binding point is \
                 numbered by its name's position and one that came and went would move \
                 [`SHADOW_POOL`] and [`SPOT_CONES_ENTRY`] off the numbers the shader \
                 declares them at.",
            ),
        ),
        (
            SHADOW_POOL.to_owned(),
            texture_array_binding_schema().with_documentation(
                "The pages of every light that was given one, which the shader indexes \
                 itself with the page each record carries.",
            ),
        ),
        (
            ENVIRONMENT_MAP.to_owned(),
            texture_binding_schema().with_documentation(
                "The scene's environment, as a two dimensional octahedral map read with \
                 `textureLoad`. A scene naming no environment light binds a one texel black \
                 map here rather than nothing, so the binding numbers are the same either way.",
            ),
        ),
    ];
    for point in outputs {
        binders.push((
            point.clone(),
            texture_binding_schema().with_documentation(
                "An image this scene's own fragment shader writes beside the frame. The shader \
                 reads nothing here; a target is a binding point that takes a number and emits \
                 no layout entry, and the number is what has to agree with the pass's.",
            ),
        ));
    }
    interface(
        "A lighting pass, which shades every mesh in the scene.",
        binders,
        vec![
            (
                PROJECTION.to_owned(),
                projection_schema().with_documentation("Looking at the scene from the camera."),
            ),
            (
                CLUSTER_GRID.to_owned(),
                components_schema(4).with_documentation(
                    "How the view is divided: how many clusters across, how many up, how many \
                     depth slices, and how many lights one cluster keeps.",
                ),
            ),
            (
                ENVIRONMENT.to_owned(),
                components_schema(4).with_documentation(
                    "The environment light's colour scaled by its intensity, and whether the \
                     scene named one at all. Written for every lighting pass.",
                ),
            ),
            (
                DIRECTIONAL.to_owned(),
                directional_schema().with_documentation(
                    "The directional light, which stands nowhere and so is in no cluster list. \
                     Written for every lighting pass, present or not.",
                ),
            ),
            (
                FRAME_SIZE.to_owned(),
                vector2().with_documentation(
                    "The frame's width and height in pixels, which is what turns a fragment's \
                     own position into a column and a row.",
                ),
            ),
            (
                GLOBAL_COUNT.to_owned(),
                Schema::new(SchemaKind::Float).with_documentation(
                    "How many of the leading light records are global lights, which every \
                     fragment evaluates whatever cluster it is in. Zero for a scene whose \
                     lights are all clustered.",
                ),
            ),
            (
                TIME.to_owned(),
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
    post_interface(false)
}

/// Builds the interface of a post pass of a scene that names an `exposure`.
///
/// # Why this is a second function and not a flag
///
/// **The uniform is the layout the pass is uploaded with**, so a field that came and went
/// with the document would not be a field that came and went: it would move every field
/// after it, and a scene that names no exposure would be drawn with a layout its own shader
/// does not describe. `post_pass_schema` therefore answers for a scene that names none, and
/// this answers for a scene that does, and the two are the same function with one more field.
///
/// # Invariants
///
/// - **The field sits where the layout puts it**, which is by name: `camera`, `depth_range`,
///   `exposure`, `projection`, `time`. A shader that declares the same field anywhere else
///   is a shader whose offsets are wrong, and the interface check is what says so.
/// - **It is a float and not a vector.** An exposure is one number for the whole frame, and a
///   three component vector would be three exposures a scene cannot tell apart.
pub fn post_pass_schema_exposed() -> Schema {
    post_interface(true)
}

/// Builds either post interface, which is one function so the two cannot drift apart.
fn post_interface(exposed: bool) -> Schema {
    let mut uniform = vec![
        (
            CAMERA.to_owned(),
            camera_schema().with_documentation(
                "The camera this pass draws through, which is what a sky is built from.",
            ),
        ),
        (
            PROJECTION.to_owned(),
            projection_schema().with_documentation("The camera's own projection."),
        ),
        (
            TIME.to_owned(),
            time().with_documentation("Seconds since the scene started."),
        ),
        (
            DEPTH_RANGE.to_owned(),
            vector2().with_documentation(
                "The distance to the near and far planes, which is what turns a depth into a \
                 distance.",
            ),
        ),
    ];
    if exposed {
        // Placed in name order rather than appended, so the vector reads the way the layout
        // it produces does. It is a `BTreeMap` either way; this is for the reader.
        uniform.push((
            EXPOSURE.to_owned(),
            Schema::new(SchemaKind::Float).with_documentation(
                "What this pass multiplies the radiance it read by, before its own grade: a \
                 linear multiplier where 1 is no change. It is not a parameter of the curve \
                 that follows, and it cannot brighten a value the shaded image already holds \
                 at one.",
            ),
        ));
        uniform.sort_by(|left, right| left.0.cmp(&right.0));
    }
    interface(
        "A post pass, which draws over the whole frame.",
        vec![
            (
                VERTICES.to_owned(),
                buffer_binding_schema()
                    .with_documentation("The fullscreen triangle this pass draws."),
            ),
            (
                COLOR_TARGET.to_owned(),
                texture_binding_schema().with_documentation("The shaded image this pass reads."),
            ),
            (
                DEPTH_BUFFER.to_owned(),
                texture_binding_schema()
                    .with_documentation("The depth that produced the shaded image."),
            ),
            (
                PRESENT_TARGET.to_owned(),
                texture_binding_schema()
                    .with_documentation("The image this pass renders into, and the frame."),
            ),
        ],
        uniform,
    )
}

/// Builds a shader interface of a `uniform` and a `binders` block.
///
/// Shared with [`super::debug`], whose pass interface is built outside this file because the
/// channel's shader is the compiler's own rather than one a scene declares: the binding point
/// names here are the contract between the compiler and a scene author's shaders, and a channel
/// has no author on the other side.
pub(crate) fn interface(
    documentation: &str,
    binders: Vec<(String, Schema)>,
    uniform: Vec<(String, Schema)>,
) -> Schema {
    Schema::new(SchemaKind::structure([
        (
            UNIFORM.to_owned(),
            Schema::new(SchemaKind::structure(uniform))
                .with_documentation("Uniform data, uploaded verbatim."),
        ),
        (
            BINDERS.to_owned(),
            Schema::new(SchemaKind::structure(binders))
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

/// Builds the schema of a four component vector.
fn vector4() -> Schema {
    graphics::layout::components_schema(4).with_documentation("Four components.")
}

/// Builds the schema of a time in seconds.
///
/// One float rather than a list: a time is a single number, and a list of one component
/// would be read as a four component vector by anything that went by the schema alone.
fn time() -> Schema {
    Schema::new(SchemaKind::Float).with_documentation("Seconds since the scene started.")
}
