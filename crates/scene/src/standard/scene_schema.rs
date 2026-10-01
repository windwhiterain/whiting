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

use definition::{Schema, SchemaKind, SchemaMap};

use crate::lighting::{SHADOW_PAGE_HEIGHT, SHADOW_PAGE_WIDTH, SHADOW_PAGES};

/// Field holding the name of the scene.
pub const NAME: &str = "name";

/// Field holding the lights of the scene.
pub const LIGHTS: &str = "lights";

/// Field holding the shaders of the scene.
pub const SHADERS: &str = "shaders";

/// Field holding the meshes of the scene.
pub const MESHES: &str = "meshes";

/// Field holding the list of images a scene declares.
pub const IMAGES: &str = "images";

/// Field holding how many texels across an image is.
pub const WIDTH: &str = "width";

/// Field holding how many texels down an image is.
pub const HEIGHT: &str = "height";

/// Field holding how a texel of an image is spelled.
pub const FORMAT: &str = "format";

/// Field holding the pixels of an image.
pub const DATA: &str = "data";

/// Field holding the instances of the scene.
pub const INSTANCES: &str = "instances";

/// Field holding the camera the scene is drawn through.
/// Field holding the cameras a scene is drawn from.
pub const CAMERAS: &str = "cameras";

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

/// Field holding how the view frustum is divided for clustered lighting.
pub const CLUSTERING: &str = "clustering";

/// Field holding how many clusters the frame is divided into across.
pub const WIDTH_EXTENTS: &str = "width_extents";

/// Field holding how many clusters the frame is divided into up.
pub const HEIGHT_EXTENTS: &str = "height_extents";

/// Field holding how many slices the frustum is divided into, deep.
pub const DEPTH_EXTENTS: &str = "depth_extents";

/// Field holding how many lights one cluster keeps.
pub const LIGHTS_PER_CLUSTER: &str = "lights_per_cluster";

/// Field holding how far a light reaches.
pub const RADIUS: &str = "radius";

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

/// The list of mesh materials a scene declares.
pub const MESH_MATERIALS: &str = "mesh_materials";

/// Field holding the data a material's own shader reads.
pub const CUSTOM_DATA: &str = "custom_data";

/// The list of screen materials a scene declares.
pub const SCREEN_MATERIALS: &str = "screen_materials";

/// Field holding the ordered list of post processes a scene runs.
pub const POST_PROCESSES: &str = "post_processes";

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
                .with_documentation("One shadowing pass is emitted per point light."),
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
            IMAGES.to_owned(),
            Schema::new(SchemaKind::list(image_schema().reference()))
                .optional()
                .with_documentation(
                    "Pixels a material samples, listed once per image. A material names one by \
                     label inside its custom data, so two materials that draw the same surface \
                     share the one texture. An image may be written here or named from another \
                     definition, which is what keeps a large one out of the document that is \
                     re-sent to the device on every reload. Absent means no material in this \
                     scene samples an image.",
                ),
        ),
        (
            SCREEN_MATERIALS.to_owned(),
            Schema::new(SchemaKind::list(screen_material_schema()))
                .optional()
                .with_documentation(
                    "Screen materials, listed once per material and named by a post process. A \
                 screen material is a shader over the frame rather than over a mesh, so it \
                 declares no point schema and no instance row.",
                ),
        ),
        (
            POST_PROCESSES.to_owned(),
            Schema::new(SchemaKind::list(Schema::new(SchemaKind::String)))
                .optional()
                .with_documentation(
                    "The post processes to run, in order, each naming a screen material by label. \
                 The first reads the shaded image and the last writes the frame, and each \
                 between them reads what the one before it wrote.",
                ),
        ),
        (
            MESH_MATERIALS.to_owned(),
            Schema::new(SchemaKind::list(mesh_material_schema())).with_documentation(
                "Materials, listed once per material. An instance names one by label, so two \
                 objects drawn the same way carry one copy of the material.",
            ),
        ),
        (
            INSTANCES.to_owned(),
            Schema::new(SchemaKind::dictionary(Schema::new(SchemaKind::list(
                instance_schema(),
            ))))
            .with_documentation(
                "The instances drawn in each pass, keyed by the pass's stage: `shadowing`, \
                 `depthing`, `lighting` or `emission`. An instance listed under several \
                 passes is drawn once per pass, so a mesh drawn in three stages is three \
                 entries with the same transform and a material per stage. A key that is not \
                 a mesh stage is an error.",
            ),
        ),
        (
            CAMERAS.to_owned(),
            Schema::new(SchemaKind::list(camera_schema().reference()))
                .optional()
                .with_documentation(
                    "Where the scene is drawn from, listed once per view. **Every camera is \
                 rendered**: a scene that names two is compiled and drawn twice, because a \
                 camera decides the projection every pass draws through and so changes the \
                 emitted documents rather than being something a renderer picks afterwards. A \
                 scene without a camera is drawn by the stated convention: parallel, dead \
                 ahead, framing everything that is drawn.",
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
        (
            CLUSTERING.to_owned(),
            clustering_schema().optional().with_documentation(
                "How the view frustum is divided into the boxes a light is assigned to, and how \
                 many lights one box keeps. Absent is the grid below, which is what every scene \
                 gets until it asks for another: the shape of the culling is a per scene \
                 decision, because the number of lights a scene holds and the shape of what it \
                 draws are both the author's.",
            ),
        ),
    ]))
    .with_documentation("The conventional lit scene: lights, shaders, geometry, and what draws it.")
}

/// Builds the schema of how the frustum is divided for clustered lighting.
///
/// # What the grid is
///
/// The frame is divided into `width_extents` by `height_extents` boxes across and up, and the
/// frustum into `depth_extents` slices deep, which is 3456 boxes at the defaults. Every box
/// holds the lights that reach it, and `lights_per_cluster` is how many of them it keeps: the
/// ones that contribute most to that box, rather than the first ones to arrive. A shading pass
/// that draws one box's worth of lights per fragment is what lets a scene hold thousands of
/// lights without a pass apiece.
///
/// # What each number costs
///
/// The box list is `width_extents * height_extents * depth_extents` rows of
/// `lights_per_cluster` four byte indices, so the defaults are 3456 rows of 64, which is 864
/// KiB, whatever the scene holds. A finer grid buys a tighter fit and costs that much more, and
/// a larger cap buys fewer dropped lights at four bytes apiece per cluster. Both are the
/// author's to trade, which is why they are fields.
///
/// # Invariants
///
/// Every field is optional, and an absent field falls back to the documented default rather
/// than to a number the author has to know. A grid is a decision about how much a frame is
/// worth dividing, and a scene that names none gets the division every other scene gets.
pub fn clustering_schema() -> Schema {
    Schema::new(SchemaKind::structure([
        (
            WIDTH_EXTENTS.to_owned(),
            Schema::new(SchemaKind::Int).optional().with_documentation(
                "How many clusters the frame is divided into across. Defaults to 16. A \
                     finer division fits a light's volume more tightly, so fewer lights are \
                     listed for a box than for a larger one, at the cost of one count and one \
                     row of the cap per extra cluster.",
            ),
        ),
        (
            HEIGHT_EXTENTS.to_owned(),
            Schema::new(SchemaKind::Int).optional().with_documentation(
                "How many clusters the frame is divided into up. Defaults to 9, which is \
                     three rows for every four of a sixteen by twenty four frame.",
            ),
        ),
        (
            DEPTH_EXTENTS.to_owned(),
            Schema::new(SchemaKind::Int).optional().with_documentation(
                "How many slices the frustum is divided into, deep. Defaults to 24. The \
                     slices are equal in depth rather than in distance, so each covers the same \
                     part of the frame however far away it is, and a fragment's own depth is \
                     the one number that says which slice it is in.",
            ),
        ),
        (
            LIGHTS_PER_CLUSTER.to_owned(),
            Schema::new(SchemaKind::Int).optional().with_documentation(
                "How many lights one cluster keeps. Defaults to 64. A cluster with more \
                     lights reaching it than this keeps the ones that contribute most to it \
                     and reports the rest, because a list cut off at the cap is the one thing a \
                     sparse scheme must not do quietly. Four bytes per cluster per extra light \
                     is what a larger cap costs.",
            ),
        ),
        (
            SHADOW_PAGE_WIDTH.to_owned(),
            Schema::new(SchemaKind::Int).optional().with_documentation(
                "How wide one shadow page is, in pixels. Defaults to 1024. This is what a \
                     shadow map's size was when it was a Rust constant, and it is a scene's \
                     decision because a scene that draws its steps two units wide wants a \
                     different answer from one that draws a mountain.",
            ),
        ),
        (
            SHADOW_PAGE_HEIGHT.to_owned(),
            Schema::new(SchemaKind::Int).optional().with_documentation(
                "How tall one shadow page is, in pixels. Defaults to 1024.",
            ),
        ),
        (
            SHADOW_PAGES.to_owned(),
            Schema::new(SchemaKind::Int).optional().with_documentation(
                "How many shadow pages the pool holds, which is how many lights can cast one \
                     at all. Defaults to 64, and a scene may not ask for more than the 256 \
                     layers a texture array can have. A light the pool did not buy a page for \
                     is lit unshadowed rather than wrongly shadowed, and a scene that wants \
                     more real shadows raises this.",
            ),
        ),
    ]))
    .with_documentation("How the view is divided, how many lights one box keeps, and how many \
                         shadows the scene can afford.")
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

/// Builds the schema of one light: an enumeration of the kinds of light.
///
/// # Why a light names its kind rather than carrying an optional field for it
///
/// A light has two kinds of field: the ones every light has — a name, a colour and a
/// brightness — and the ones that make a spot a spot and a directional light something
/// other than a point. Written as one structure, `position` would be optional, because a
/// directional light has none, and a document saying which shape of light it meant would be
/// a document whose author had to be inferred from which fields were left out.
///
/// The same shape a pass has, in `graphics::pass`: `Rasterize` declares `targets` and
/// `Compute` declares a `workgroup`, and neither declares an optional field for the other's
/// work. What a thing **is** is its tag, and the fields beside the tag are the ones that
/// kind of thing has.
///
/// # A light that names no kind is refused, not read as a point light
///
/// A missing kind has no default. In an offline render a wrongly classified light is a wrong
/// image that nobody sees until the render finishes, and statelessness makes that worse
/// rather than better: there is no previous frame in which the mistake was visible, so the
/// only thing that catches it is a refusal at compile time. The cost of the error is a whole
/// render; the cost of a guess is an image that was wrong for the whole render.
///
/// # Document shape
///
/// ```text
/// Point(
///     label: "key",
///     position: [ 4.0, 6.0, 2.0 ],
///     color: [ 1.0, 0.95, 0.9 ],
///     intensity: 1.0,
///     radius: 6.0,
///     shadow_extent: 2.5,
/// )
/// Directional(
///     label: "sun",
///     direction: [ -0.3, -0.9, -0.2 ],
///     color: [ 1.0, 0.95, 0.85 ],
///     intensity: 2.0,
///     shadow_extent: 60.0,
/// )
/// ```
pub fn light_schema() -> Schema {
    Schema::new(SchemaKind::enumeration([
        ("Point".to_owned(), point_light()),
        ("Spot".to_owned(), spot_light()),
        ("Directional".to_owned(), directional_light()),
        ("Environment".to_owned(), environment_light()),
    ]))
    .with_documentation("A light, and which kind of light it is.")
}

/// The fields every light carries.
///
/// A field belongs here when every light has it. A field that only the kinds that stand
/// somewhere have — a position, a direction, a cone, an image — belongs in the variant that
/// declares it, so a document carrying a directional light cannot also carry a position.
fn shared_fields() -> SchemaMap {
    [
        (
            "label".to_owned(),
            Schema::new(SchemaKind::String)
                .with_documentation("Name used in pass labels and in errors."),
        ),
        ("color".to_owned(), float_triple("Colour in zero to one.")),
        (
            "intensity".to_owned(),
            Schema::new(SchemaKind::Float).with_documentation("Brightness multiplier."),
        ),
    ]
    .into_iter()
    .collect()
}

/// Builds the schema of a point light.
///
/// A light at a position, brightest at the position and falling off with distance from it.
/// This is the kind every light in the standard pipeline was before kinds existed, and the
/// one a shadow volume is a box around.
fn point_light() -> Schema {
    light_variant(shared_fields(), [position(), radius(), shadow_extent()]).with_documentation(
        "A light at a position, falling off with distance from it. Shadows a box of \
             half the given extent, centred on what is drawn.",
    )
}

/// Builds the schema of a spot light.
///
/// A point light with a cone, which is what a stage lamp, a torch and a street light are.
/// The cone is what the two angles describe: one the light is at full strength inside, and
/// the one it has fallen to nothing by.
fn spot_light() -> Schema {
    light_variant(
        shared_fields(),
        [
            position(),
            direction(),
            inner_angle(),
            outer_angle(),
            shadow_extent(),
        ],
    )
    .with_documentation(
        "A point light that shines inside a cone. What it lights is the cone, and what it \
         shadows is the volume that cone cuts out of the scene.",
    )
}

/// Builds the schema of a directional light.
///
/// A light that is a direction rather than a place: every surface facing it is lit the same
/// however far from it that surface stands, which is the sun as far as a shaded image is
/// concerned. It has no position because a direction is not somewhere, and a document that
/// gave it a position would be a document asking for a point light by another name.
fn directional_light() -> Schema {
    light_variant(shared_fields(), [direction(), shadow_extent()]).with_documentation(
        "A light from a direction, reaching every distance equally. Shadows a box of \
             half the given extent, which is where its shadowing pass looks at all.",
    )
}

/// Builds the schema of an environment light.
///
/// A light that is an image: what a surface sees where no light reaches it, and what a
/// smooth surface reflects. It stands nowhere and shines no direction, and it casts no
/// shadow, so it declares neither a position nor a volume.
fn environment_light() -> Schema {
    light_variant(shared_fields(), [image()]).with_documentation(
        "Light from an image the scene declares, sampled by a material rather than cast \
         from anywhere. It shadows nothing.",
    )
}

/// Builds the schema of where a light stands.
fn position() -> (String, Schema) {
    (
        "position".to_owned(),
        float_triple("Position in world space."),
    )
}

/// Builds the schema of which way a light shines.
///
/// A direction, not a point to look at: the vector a surface is lit by, which is the same
/// however far along it the light is written to be.
fn direction() -> (String, Schema) {
    (
        "direction".to_owned(),
        float_triple("Which way the light shines, in world space. Length does not matter."),
    )
}

/// Builds the schema of how far a light reaches.
///
/// # Invariants
///
/// The radius is what a light is assigned by, so it is the one number that decides whether a
/// scene is lit at all: a light whose radius reaches nothing is in no cluster, and no shading
/// pass can find it. It is separate from `shadow_extent` because the two answer different
/// questions — how far a light's *light* falls off, and how much of the scene its shadow map
/// covers — and a scene that wants a small light with a wide shadow volume, or the other way
/// round, has to be able to say so.
///
/// Absent is one box radius of what is drawn, which is the measure the camera and the shadow
/// volume already use: a light the size of the scene it stands in reaches across it.
fn radius() -> (String, Schema) {
    (
        RADIUS.to_owned(),
        Schema::new(SchemaKind::Float)
            .optional()
            .with_documentation(
                "How far the light reaches, in world units, as a sphere around its position. \
                 It is what decides which clusters the light is assigned to, so a radius that \
                 covers the whole frustum is a light in every cluster and one that culls \
                 nothing. Defaults to one box radius of the scene that is drawn.",
            ),
    )
}

/// Builds the schema of the volume a light shadows.
///
/// A point light's shadow map covers a box of this half width centred on what is drawn, and
/// a directional light's covers the same box seen along its direction. One number, because
/// one shadow map covers it either way.
fn shadow_extent() -> (String, Schema) {
    (
        "shadow_extent".to_owned(),
        Schema::new(SchemaKind::Float)
            .with_documentation("Half the width of the volume the light shadows."),
    )
}

/// Builds the schema of the angle a spot light is at full strength inside.
fn inner_angle() -> (String, Schema) {
    (
        "inner_angle".to_owned(),
        Schema::new(SchemaKind::Float).with_documentation(
            "Half the angle of the cone the light is at full strength within, in radians.",
        ),
    )
}

/// Builds the schema of the angle a spot light falls to nothing at.
///
/// # Invariants
///
/// Both angles are required rather than one being derived, because nothing yet reads them
/// and a default angle would be a number chosen once in this source rather than a number
/// the author of a scene chooses. The pair describes a cone, and which cone is the
/// author's to draw.
fn outer_angle() -> (String, Schema) {
    (
        "outer_angle".to_owned(),
        Schema::new(SchemaKind::Float).with_documentation(
            "Half the angle of the cone the light has fallen to nothing at, in radians. \
             Between this angle and the inner one the light is scaled down.",
        ),
    )
}

/// Builds the schema of the image an environment light is read from.
///
/// A label rather than pixels, for the reason images are a list a document names: the same
/// image is sampled by every surface of a scene, and a scene that inlined it would send it
/// again on every reload.
fn image() -> (String, Schema) {
    (
        "image".to_owned(),
        Schema::new(SchemaKind::String).with_documentation(
            "Label of an image this scene declares, which is what the material samples.",
        ),
    )
}

/// Merges the fields every light carries with the fields of one kind.
fn light_variant(shared: SchemaMap, own: impl IntoIterator<Item = (String, Schema)>) -> Schema {
    let mut fields = shared;
    fields.extend(own);
    Schema::new(SchemaKind::Struct { fields })
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

/// Identifier an image in another definition is read with.
///
/// A scene may write an image's pixels itself or name a definition to read them from, and a
/// definition named that way is read with [`image_schema`]. Which identifier it is registered
/// under is the client's to choose, and this is the conventional one.
pub const IMAGE_SCHEMA: &str = "whiting.scene.image";

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

/// Builds the schema of one image: pixels, and nothing about what samples them.
///
/// # Why this is a list a material names, and not pixels a material carries
///
/// Two materials that draw the same surface have to share the one texture, or the same pixels
/// are uploaded once per material. Sharing needs a name, and a list of labelled images is
/// where a label lives — the same shape `meshes` already has for geometry.
///
/// It is also what makes a large image possible at all: a definition travels to the device as
/// json, so a scene that inlined a photograph would re-send it on every reload. A scene may
/// write its pixels here or reference them from a definition of their own.
pub fn image_schema() -> Schema {
    Schema::new(SchemaKind::structure([
        (LABEL.to_owned(), Schema::new(SchemaKind::String)),
        (
            WIDTH.to_owned(),
            Schema::new(SchemaKind::Int).with_documentation("Texels across."),
        ),
        (
            HEIGHT.to_owned(),
            Schema::new(SchemaKind::Int).with_documentation("Texels down."),
        ),
        (
            FORMAT.to_owned(),
            Schema::new(SchemaKind::String)
                .with_documentation("How a texel is spelled, as the device names formats."),
        ),
        (
            DATA.to_owned(),
            Schema::new(SchemaKind::Bytes)
                .optional()
                .with_documentation(
                    "The pixels, one texel per four bytes, top row first. Absent is an image \
                     with nothing to sample, which the compiler refuses rather than binding a \
                     texture that reads as garbage.",
                ),
        ),
    ]))
    .with_documentation("Pixels a material samples, listed once and referenced by label.")
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
        (
            MATERIAL.to_owned(),
            Schema::new(SchemaKind::String).with_documentation(
                "The label of a mesh material this scene declares. The stage this instance is \
                 drawn in is the bucket it is listed under, not the material: the material \
                 says how it is drawn there.",
            ),
        ),
    ]))
    .with_documentation("A mesh placed in the scene, listed under every pass that draws it.")
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

/// Builds the schema of one screen material.
///
/// # Invariants
///
/// A screen material names one `ScreenShader` and the data that shader reads. It is a
/// material rather than a shader because the same grade may be run over several frames with
/// different data, and it is separate from the mesh materials because a shader over the
/// screen reads no points and no instance row.
pub fn screen_material_schema() -> Schema {
    Schema::new(SchemaKind::structure([
        (
            LABEL.to_owned(),
            Schema::new(SchemaKind::String)
                .with_documentation("Name a post process refers to this material by."),
        ),
        (
            SHADER.to_owned(),
            Schema::new(SchemaKind::String)
                .with_documentation("The label of a screen shader this scene declares."),
        ),
        (
            CUSTOM_DATA.to_owned(),
            Schema::new(SchemaKind::Any).optional().with_documentation(
                "The data this grade reads, verified against the shader's custom schema. \
                     Every screen shader the standard pipeline emits declares the unit schema \
                     and so reads none, which is why this is optional: a grade whose parameters \
                     live in its own source has no data to carry.",
            ),
        ),
    ]))
    .with_documentation("A grade over the frame, run once by a post process.")
}

/// Builds the schema of one mesh material.
///
/// # Invariants
///
/// A material names one shader, and that shader's stage is the pass it can be drawn in. Which
/// passes an instance is actually drawn in is the buckets it is listed under in `instances`,
/// so the two are checked against each other: a material whose shader is a `DepthShader`,
/// listed under `lighting`, is an error rather than a depth pass drawn into the shaded image.
pub fn mesh_material_schema() -> Schema {
    Schema::new(SchemaKind::structure([
        (
            LABEL.to_owned(),
            Schema::new(SchemaKind::String)
                .with_documentation("Name an instance refers to this material by."),
        ),
        (
            SHADER.to_owned(),
            Schema::new(SchemaKind::String).with_documentation(
                "The label of a shader this scene declares, whose stage is the pass this \
                 material can be drawn in.",
            ),
        ),
        (
            CUSTOM_DATA.to_owned(),
            Schema::new(SchemaKind::Any)
                .with_documentation("Verified against the shader's custom schema."),
        ),
    ]))
    .with_documentation("How an instance is drawn, in one pass.")
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
        ("LightShader".to_owned(), mesh()),
        ("EmissionShader".to_owned(), mesh()),
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
