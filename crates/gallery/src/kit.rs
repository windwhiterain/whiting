//! The parts a gallery scene is assembled from.
//!
//! A standard scene document has a field for every thing a scene decides — the frame it is
//! drawn into, its lights, its geometry, its shaders — and each one is written here once. A
//! scene that builds itself says `light("key", [4.0, 6.0, 2.0], ...)`, which reads as the
//! scene it is, rather than repeating the shape of a light at every call site.
//!
//! # Invariants
//!
//! A builder here produces the field names the scene schema declares and the emitter
//! reads. Nothing in this module knows what a stage does, only which tag names it.

use definition::{PackedList, Schema, SchemaKind, Value};
use graphics::layout::components_schema;

use crate::shaders::Projection;
use scene::standard::PROJECTION;

/// The tint a material's shader multiplies its albedo by.
pub const TINT: &str = "tint";
/// The colour a surface emits on its own, which no light reaches.
pub const EMISSIVE: &str = "emissive";
/// How metallic a surface is, from zero to one.
pub const METALLIC: &str = "metallic";
/// How much of the sky a surface shows, from zero to one.
pub const REFLECT: &str = "reflect";
/// What an instanced mesh's own authored ids are offset by, one **material per placement**.
///
/// # Invariants
///
/// **It lives on the material rather than on the instance**, because a scene's instance carries no custom
/// values at all — it is a label, a mesh, a transform and a material — and a material already carries them. So
/// one material per placement is what an instanced identity needs, and the value is the number a bake would
/// have written for that placement: `base − 1`, measured by `building`'s `instance_identity_probe`.
pub const AUTHORED_OFFSET: &str = "authored_offset";
/// How rough a surface is, from zero to one.
pub const ROUGHNESS: &str = "roughness";
/// The height of a fog bank, and how thick it is.
pub const FOG: &str = "fog";

/// Builds a three component list of floats.
pub fn triple(components: [f64; 3]) -> Value {
    Value::List(components.into_iter().map(Value::Float).collect())
}

/// Builds one image of a scene, from pixels the scene generated.
///
/// # Why a scene generates its own pixels
///
/// Nothing in this framework reads the filesystem: a shader carries its own source, and an image
/// carries its own pixels. A scene compiler is therefore where a texture comes from, and a
/// pattern a loop can write is a texture — which is the same reason the points of a dune field
/// are generated rather than typed out.
///
/// The pixels are four bytes a texel, top row first, which is what the device uploads and what it
/// derives a level chain from.
///
/// # `projection` is how a **direction** is read out of it
///
/// It is [`Projection::Latlong`] by default, and that default is the whole point of the
/// parameter: a latlong map is the one a path tracer reads, so a scene that says nothing is
/// carried by every backend rather than by the one that happens to like squares.
///
/// A map that is only ever sampled by **uv** — a tile texture on a floor, say — is not read by
/// direction at all, and its projection is never consulted by anyone. `tiles` passes the
/// default for the same reason a scene with no sky does: it is the answer that is right when the
/// question is not being asked.
pub fn image(
    label: &str,
    width: i64,
    height: i64,
    pixels: impl IntoIterator<Item = u8>,
    projection: Projection,
) -> Value {
    Value::structure([
        ("label".to_owned(), Value::String(label.to_owned())),
        ("width".to_owned(), Value::Int(width)),
        ("height".to_owned(), Value::Int(height)),
        ("format".to_owned(), Value::String("rgba8unorm".to_owned())),
        (PROJECTION.to_owned(), Value::variant(projection.tag(), [])),
        (
            "data".to_owned(),
            Value::Bytes(pixels.into_iter().collect()),
        ),
    ])
}

/// Builds one **global** point light of a scene.
///
/// # Invariants
///
/// A light names the kind it is, so this builder writes the `Point` tag rather than taking
/// a kind: every point light in the gallery is a point light, and a builder that made each
/// call site name a kind they all share would be a kind spelled nine times. A kind with
/// fields of its own is a sibling of this one rather than a parameter of it.
///
/// [`environment_light`], [`spot`] and [`directional`] are those siblings, and they are here
/// now because those paths have landed. Every light kind the standard pipeline draws has a
/// builder, which is what makes a gallery of scenes that compile the whole of what can be
/// drawn.
///
/// # Why this builder writes no `radius`
///
/// A point light that names no reach is a **global** light: it takes no cluster slot, every
/// fragment evaluates it, and it cannot be culled short of the scene it is lighting. That is
/// the whole of what saying nothing means, and it is the answer to a light standing **outside**
/// the scene it lights — a sun over a valley, a fill on the far side. Such a light culled per
/// cluster is dropped from every cluster past its own reach, and the boundary between a
/// cluster that has the light and one that does not is a plane through the eye, so the
/// terminator comes out as a staircase of screen-aligned rectangles with nothing in the frame
/// to say a number was wrong.
///
/// A scene whose light only reaches part of what it draws wants [`light_with_radius`] instead,
/// and wants the number to be at least the distance from the light to the furthest surface it
/// is meant to light. A **spot** light needs no such parameter, because its cone already says
/// how far it carries — see [`spot`].
pub fn light(
    label: &str,
    position: [f64; 3],
    color: [f64; 3],
    power: f64,
    shadow_extent: f64,
) -> Value {
    Value::variant(
        "Point",
        [
            ("label".to_owned(), Value::String(label.to_owned())),
            ("position".to_owned(), triple(position)),
            ("color".to_owned(), triple(color)),
            ("power".to_owned(), Value::Float(power)),
            ("shadow_extent".to_owned(), Value::Float(shadow_extent)),
        ],
    )
}

/// Builds one point light that says how far it reaches.
///
/// # Why a scene says it
///
/// The cluster assignment tests a light against the boxes it can reach, and the cluster grid
/// is a grid: a light culled out of one box and kept in the next has an edge that runs along
/// a box boundary, which on the frame is a straight line at a multiple of the grid. So a
/// reach that is shorter than the light's own light draws that light's own edge as a
/// staircase rather than as the curve the falloff is.
///
/// The value is the distance the light is meant to carry over, so it wants to be at least the
/// distance from the light to the furthest surface it is supposed to light. A reach that
/// already holds the whole scene is **promoted** to a global light by the compiler and changes
/// no pixel, so a scene that names a reach which reaches is saying something true about
/// itself whether or not it is load-bearing.
pub fn light_with_radius(
    label: &str,
    position: [f64; 3],
    color: [f64; 3],
    power: f64,
    shadow_extent: f64,
    radius: f64,
) -> Value {
    let Value::Enum { tag, mut fields } = light(label, position, color, power, shadow_extent)
    else {
        unreachable!("a light is an enum value")
    };
    fields.insert(scene::standard::RADIUS.to_owned(), Value::Float(radius));
    Value::Enum { tag, fields }
}

/// **The two angles of a spot light's cone**, both half angles in radians, the inner at full strength
/// and the outer at nothing. They are named together because the pair is the cone: two `f64`s in
/// sequence at a call site read as two numbers, and transposing them draws the same cone with its
/// falloff the wrong way round.
#[derive(Debug, Clone, Copy)]
pub struct Cone {
    /// The half angle at full strength.
    pub inner: f64,
    /// The half angle at nothing.
    pub outer: f64,
}

/// Builds one spot light of a scene: a point light with a cone.
///
/// # Invariants
///
/// The two angles are **half** angles in radians, and both are written rather than one being
/// derived: the pair is what draws the cone, and a default angle would be a number chosen once
/// in this source rather than a number the author of a scene chooses. `direction` is the way
/// the light shines, so the cone opens away from the light rather than towards it, and its
/// length does not matter.
///
/// The builder takes no `radius`. A spot light's reach is its cone, and the compiler culls it
/// by the reach of that cone rather than by a number this builder would have to restate: the
/// schema's `radius` is optional for a `Spot` precisely so that a cone is never culled shorter
/// than the light it is drawing. A spot that stands **inside** the volume it lights is a
/// different question — the cone reaches past the scene — and that is the one case a `Spot`
/// should name a radius for, by writing the field beside the ones below.
pub fn spot(
    label: &str,
    position: [f64; 3],
    direction: [f64; 3],
    color: [f64; 3],
    power: f64,
    cone: Cone,
    shadow_extent: f64,
) -> Value {
    Value::variant(
        "Spot",
        [
            ("label".to_owned(), Value::String(label.to_owned())),
            ("position".to_owned(), triple(position)),
            ("direction".to_owned(), triple(direction)),
            ("color".to_owned(), triple(color)),
            ("power".to_owned(), Value::Float(power)),
            ("inner_angle".to_owned(), Value::Float(cone.inner)),
            ("outer_angle".to_owned(), Value::Float(cone.outer)),
            ("shadow_extent".to_owned(), Value::Float(shadow_extent)),
        ],
    )
}

/// Builds one directional light of a scene: a light from a direction rather than a place.
///
/// # Invariants
///
/// It declares **no position**, because a direction is not somewhere. A builder that wrote one
/// would be a builder for a point light under another name, and the compiler would read the
/// position and never ask the direction that the shading actually uses.
///
/// `direction` is the way the light **travels**, the same convention a spot light's cone uses,
/// so a light shining down at a floor is written `[0, -1, 0]`. Its length does not matter: the
/// compiler normalises it where it reads it and refuses a direction of zero, which is the one
/// refusal this builder cannot make for the author.
///
/// The builder takes no `radius`. A directional light reaches every distance equally, so it
/// takes no cluster slot and is culled by nothing — `radius` would be a number no question is
/// ever asked of.
pub fn directional(
    label: &str,
    direction: [f64; 3],
    color: [f64; 3],
    irradiance: f64,
    shadow_extent: f64,
) -> Value {
    Value::variant(
        "Directional",
        [
            ("label".to_owned(), Value::String(label.to_owned())),
            ("direction".to_owned(), triple(direction)),
            ("color".to_owned(), triple(color)),
            ("irradiance".to_owned(), Value::Float(irradiance)),
            ("shadow_extent".to_owned(), Value::Float(shadow_extent)),
        ],
    )
}

/// The flux a point light needs so that a surface at `distance` from it receives `irradiance`.
///
/// A point light's `power` is a flux spread over the sphere at each distance, so the number a
/// scene wants — how brightly a surface at a known distance is lit — is not the number the
/// document takes. A scene that writes a flux by taste writes the hermite sheet's old `3.2`,
/// which at ten units of distance is two thousandths of an irradiance and drew the sheet
/// black. The sphere's area is the whole of the conversion, and stating it here keeps every
/// scene's point light a statement about its subject rather than about the inverse-square law.
pub fn flux_for_irradiance(irradiance: f64, distance: f64) -> f64 {
    irradiance * 4.0 * std::f64::consts::PI * distance * distance
}

/// Builds one environment light of a scene, reading the image the label names.
///
/// # Invariants
///
/// It declares neither a position nor a volume, because it stands nowhere and casts no shadow:
/// the fields the kind has are the fields it writes, and a document carrying one of the others
/// would be a document asking for a point light by another name.
///
/// The image is a **label** into the scene's own image list rather than pixels, which is the
/// same reason a material's image is: one map is shared by every surface of a scene, and the
/// scene's `images` list is where it is written down once.
pub fn environment_light(label: &str, image: &str, color: [f64; 3], radiance: f64) -> Value {
    Value::variant(
        "Environment",
        [
            ("label".to_owned(), Value::String(label.to_owned())),
            ("image".to_owned(), Value::String(image.to_owned())),
            ("color".to_owned(), triple(color)),
            ("radiance".to_owned(), Value::Float(radiance)),
        ],
    )
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
            scene::standard::LABEL.to_owned(),
            Value::String(label.to_owned()),
        ),
        (
            scene::standard::TOPOLOGY.to_owned(),
            Value::variant(topology, []),
        ),
        (
            scene::standard::POINT_SCHEMA.to_owned(),
            point_schema.to_value(),
        ),
        (scene::standard::POINTS.to_owned(), Value::Packed(points)),
    ])
}

/// Builds one shader of a scene, in one of the four stages.
pub fn shader(
    stage: scene::standard::Stage,
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
                scene::standard::LABEL.to_owned(),
                Value::String(label.to_owned()),
            ),
            (
                scene::standard::SOURCE.to_owned(),
                Value::String(source.to_owned()),
            ),
            (
                scene::standard::PASS_SCHEMA.to_owned(),
                pass_schema.to_value(),
            ),
            (
                scene::standard::INSTANCE_SCHEMA.to_owned(),
                instance_schema.to_value(),
            ),
            (
                scene::standard::POINT_SCHEMA.to_owned(),
                point_schema.to_value(),
            ),
            (
                scene::standard::CUSTOM_SCHEMA.to_owned(),
                custom_schema.to_value(),
            ),
        ],
    )
}

/// The custom field a material names its image in.
///
/// # Why the name is fixed
///
/// The binding point an image is bound at is derived from the field it was named in, so the field
/// is what a shader and a material agree on without either naming a binding number. A scene is
/// free to call it anything; the gallery calls it this, and its shaders read it here.
pub const IMAGE_FIELD: &str = "image";

/// Builds the custom schema a gallery material gets when it may name an image.
///
/// # Invariants
///
/// This is [`gallery_custom_schema`] with one optional field added, so a material that names no
/// image is packed exactly as it was before images existed. The field is a marker rather than a
/// value the row carries: see [`graphics::image`].
pub fn gallery_custom_schema_with_image() -> Schema {
    let base = gallery_custom_schema();
    let SchemaKind::Struct { fields } = base.kind else {
        return base;
    };
    let mut fields = fields;
    fields.insert(IMAGE_FIELD.to_owned(), graphics::image_field());
    Schema::new(SchemaKind::structure(fields))
}

/// Builds a screen material, which a post process names by label.
///
/// A screen material is a grade over the frame rather than a drawing of geometry, so it has
/// no instance row and no points, and the only thing that distinguishes it from the shader it
/// names is that several post processes may run one grade.
pub fn screen_material(label: &str, shader: &str) -> Value {
    Value::structure([
        (
            scene::standard::LABEL.to_owned(),
            Value::String(label.to_owned()),
        ),
        (
            scene::standard::SHADER.to_owned(),
            Value::String(shader.to_owned()),
        ),
    ])
}

/// Builds the ordered post processes a scene runs, each naming a screen material.
pub fn post_processes(labels: &[&str]) -> Value {
    Value::List(
        labels
            .iter()
            .map(|label| Value::String((*label).to_owned()))
            .collect(),
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
            scene::standard::LABEL.to_owned(),
            Value::String(label.to_owned()),
        ),
        (
            scene::standard::SHADER.to_owned(),
            Value::String(shader.to_owned()),
        ),
        (scene::standard::CUSTOM_DATA.to_owned(), custom_data),
    ])
}

/// Builds one instance: a mesh, where it is, how it is turned, and what draws it.
///
/// # Invariants
///
/// The label is taken as anything a string can be made from rather than as a `&str`,
/// because a scene that places the same geometry in several passes needs a distinct label
/// for each placement — a label names one instance in the **whole** scene, not one per pass —
/// and a label that has to be assembled is one every call site then writes the same way.
pub fn instance(
    label: impl Into<String>,
    mesh: &str,
    translation: [f64; 3],
    rotation: [f64; 3],
    scale: [f64; 3],
    material: &str,
) -> Value {
    Value::structure([
        (
            scene::standard::LABEL.to_owned(),
            Value::String(label.into()),
        ),
        (
            scene::standard::MESH.to_owned(),
            Value::String(mesh.to_owned()),
        ),
        (
            scene::standard::TRANSFORM.to_owned(),
            Value::structure([
                ("translation".to_owned(), triple(translation)),
                ("rotation".to_owned(), triple(rotation)),
                ("scale".to_owned(), triple(scale)),
            ]),
        ),
        (
            scene::standard::MATERIAL.to_owned(),
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
///
/// # Why the key is a [`scene::standard::Stage`] and not a string
///
/// This is the argument list where a scene decides **which stage a surface is drawn in**, and
/// it is therefore where the most expensive mistake in the gallery is available: a surface
/// whose colour comes from a reflection rather than from a light.
///
/// The **lighting** stage runs once per light and its passes add, so anything a lighting body
/// computes is computed and added *per light*. A mirror, a body of water, a pane of glass — a
/// surface lit by what it reflects — drawn there comes out once per light. Under two lights it
/// comes out twice as bright as the sky above it: white, clipped, no depth, no Fresnel
/// gradient. The **emission** stage runs exactly once, which is the whole of why it exists, so
/// it is the correct home for such a surface and it costs a third of the passes as well.
///
/// Nothing in the framework says so. The name "emission" is about light a material gives off,
/// and a mirror gives off nothing, so a scene author draws water where everything else is
/// drawn and finds a white lake. Taking the stage as a type rather than a string does not make
/// an author choose correctly — but it puts four choices side by side in one signature, where
/// the fourth is a different *word*, instead of four interchangeable strings that a typo turns
/// into a different pass.
///
/// The framework half of this is a material field saying "none of that is a light's business",
/// which belongs to whoever owns the emitter and is not invented here. See
/// [the proposal](../../../.agents/proposals/photographic-scenes.md).
pub fn instances(buckets: Vec<(scene::standard::Stage, Vec<Value>)>) -> Value {
    let buckets: Vec<(String, Value)> = buckets
        .into_iter()
        .map(|(pass, instances)| (pass.suffix().to_owned(), Value::List(instances)))
        .collect();
    Value::dictionary(buckets)
}

/// Builds the size of the image a scene is drawn into, in pixels.
///
/// # Why a builder and not a number each scene spells
///
/// The frame is **the scene's** rather than the harness's, and it was a scene field before it
/// was a builder: `scene::standard::frame::Frame::read` takes it out of the document, and a
/// document that names none is drawn into `DEFAULT_WIDTH` by `DEFAULT_HEIGHT` — the square the
/// pipeline has always drawn into. So a scene that says nothing here is drawn exactly as it was
/// before this builder existed, which is what lets every scene but one keep its own frame.
///
/// The two numbers are written rather than an aspect, because the aspect is what they are for
/// and the pair is what the document is read for: a scene that wants a row four times as wide as
/// it is tall names a frame that shape and lets the lens be worked out against it, and a scene
/// that wants a square names nothing at all.
///
/// # Invariants
///
/// Only the two sizes are written. The frame field also carries the shadow page pool and the two
/// atlas ceilings, and a scene that has an opinion about those — `foldyard` pages its shadow map
/// at 4096 square — writes them beside this rather than through it, because a pool is a
/// question about how far a light reaches and only that scene asks it.
pub fn frame(width: i64, height: i64) -> Value {
    Value::structure([
        ("width".to_owned(), Value::Int(width)),
        ("height".to_owned(), Value::Int(height)),
    ])
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
    camera_placed_up(eye, target, [0.0, 1.0, 0.0], field_of_view, near, far)
}

/// Builds a placed camera whose **up in the world** is written, rather than taken as world-up.
///
/// # Invariants
///
/// A camera looking down its own up axis has no orientation, and the one view that does look
/// nearly straight down — a plan — is exactly the one that needs it written: its up is a
/// horizontal direction, the one the frame's top edge runs along, and that is a statement about
/// the world and not a roll. See [`camera_placed`] for the depth planes.
pub fn camera_placed_up(
    eye: [f64; 3],
    target: [f64; 3],
    up: [f64; 3],
    field_of_view: f64,
    near: f64,
    far: f64,
) -> Value {
    Value::structure([
        ("eye".to_owned(), triple(eye)),
        ("target".to_owned(), triple(target)),
        ("up".to_owned(), triple(up)),
        ("field_of_view".to_owned(), Value::Float(field_of_view)),
        ("projection".to_owned(), Value::variant("Perspective", [])),
        ("near".to_owned(), Value::Float(near)),
        ("far".to_owned(), Value::Float(far)),
    ])
}

/// Builds a camera that names the lens it looks through rather than an angle.
///
/// # Invariants
///
/// A lens is a focal length, and a focal length is meaningless without the film size it was
/// measured against — so both are written here rather than the angle they work out to. A
/// scene that says `35.0` says what a photographer would have said, and the angle is
/// arrived at from the frame the scene is drawn into rather than by the author doing the
/// arithmetic in their head and getting it slightly wrong for a frame that is not square.
///
/// `roll` is the camera turned about its own axis, which is a different thing from `up`:
/// `up` is a statement about the world and a roll is a statement about the frame. A
/// photograph whose horizon is level has made a decision, and this is the field that
/// decision is written in.
pub fn camera_placed_on_lens(
    eye: [f64; 3],
    target: [f64; 3],
    focal_length: f64,
    sensor_width: f64,
    roll: f64,
    near: f64,
    far: f64,
) -> Value {
    Value::structure([
        ("eye".to_owned(), triple(eye)),
        ("target".to_owned(), triple(target)),
        ("focal_length".to_owned(), Value::Float(focal_length)),
        ("sensor_width".to_owned(), Value::Float(sensor_width)),
        ("roll".to_owned(), Value::Float(roll)),
        ("projection".to_owned(), Value::variant("Perspective", [])),
        ("near".to_owned(), Value::Float(near)),
        ("far".to_owned(), Value::Float(far)),
    ])
}

/// Builds the camera a scene is drawn through.
///
/// # Invariants
///
/// **`eye` is where the camera stands, not where it started.** A scene document carries the
/// transform it will be drawn from rather than the arithmetic that produced it, so nothing
/// downstream moves the eye: a scene that wants its framing worked out against its own
/// geometry works it out here, in the scene that knows the geometry, and writes the answer
/// down. A reader that refitted the eye would draw a different picture from every other
/// backend reading the same document, and none of them could say why.
///
/// `field_of_view` is the full **vertical** angle, which is what this engine measures a
/// camera's angle on; see `scene::standard::camera::vertical_angle`.
pub fn camera(eye: [f64; 3], target: [f64; 3], field_of_view: f64) -> Value {
    Value::structure([
        ("eye".to_owned(), triple(eye)),
        ("target".to_owned(), triple(target)),
        ("field_of_view".to_owned(), Value::Float(field_of_view)),
        ("projection".to_owned(), Value::variant("Perspective", [])),
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
/// the order the shaders in [`crate::shaders`] read them at.
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
            AUTHORED_OFFSET.to_owned(),
            Schema::new(SchemaKind::Float).with_documentation(
                "What an instanced mesh's own authored ids are offset by. Zero for a scene that \
                 bakes its geometry, which is every scene before a building draws one template \
                 in many places.",
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
        // **Zero unless a scene offsets it.** A baked mesh's ids are already the table's, so this default
        // changes every existing scene by nothing at all.
        (AUTHORED_OFFSET.to_owned(), Value::Float(0.0)),
    ])
}

/// Returns a surface's custom data with its authored-id offset set.
///
/// # Invariants
///
/// One material per **placement**: the offset belongs to a row rather than to a point, so an instanced mesh's
/// corners answer their template's id plus this number — which is the id a bake would have written for that
/// placement, measured by `building`'s `instance_identity_probe`.
pub fn offset_by(custom_data: Value, offset: f64) -> Value {
    let Value::Struct(mut fields) = custom_data else {
        return custom_data;
    };
    fields.insert(AUTHORED_OFFSET.to_owned(), Value::Float(offset));
    Value::Struct(fields)
}
