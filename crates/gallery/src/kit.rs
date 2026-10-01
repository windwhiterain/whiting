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
pub fn image(label: &str, width: i64, height: i64, pixels: impl IntoIterator<Item = u8>) -> Value {
    Value::structure([
        ("label".to_owned(), Value::String(label.to_owned())),
        ("width".to_owned(), Value::Int(width)),
        ("height".to_owned(), Value::Int(height)),
        ("format".to_owned(), Value::String("rgba8unorm".to_owned())),
        (
            "data".to_owned(),
            Value::Bytes(pixels.into_iter().collect()),
        ),
    ])
}

/// Builds one point light of a scene.
///
/// # Invariants
///
/// A light names the kind it is, so this builder writes the `Point` tag rather than taking
/// a kind: every point light in the gallery is a point light, and a builder that made each
/// call site name a kind they all share would be a kind spelled nine times. A kind with
/// fields of its own is a sibling of this one rather than a parameter of it.
///
/// [`environment_light`], [`spot`] and [`directional`] are those siblings, and they are here now
/// because those paths have landed. Every light kind the standard pipeline draws has a builder,
/// which is what makes a gallery of scenes that compile the whole of what can be drawn.
pub fn light(
    label: &str,
    position: [f64; 3],
    color: [f64; 3],
    intensity: f64,
    shadow_extent: f64,
) -> Value {
    Value::variant(
        "Point",
        [
            ("label".to_owned(), Value::String(label.to_owned())),
            ("position".to_owned(), triple(position)),
            ("color".to_owned(), triple(color)),
            ("intensity".to_owned(), Value::Float(intensity)),
            ("shadow_extent".to_owned(), Value::Float(shadow_extent)),
        ],
    )
}

/// Builds one point light that says how far it reaches.
///
/// # Why a scene has to say
///
/// The cluster assignment tests a light against the boxes it can reach, and a light that names
/// no `radius` is given one box radius of everything the scene draws. That default is right for
/// a lamp standing inside the scene and wrong for a light **outside** it: a window two and a
/// half metres from the table reaches the table and is not inside the box around it, so a scene
/// that leaves the radius out gets a light that lights nothing, which renders as a frame with
/// one light missing and no complaint.
///
/// The value is the distance the light is meant to carry over, so it wants to be at least the
/// distance from the light to the furthest surface it is supposed to light.
pub fn light_with_radius(
    label: &str,
    position: [f64; 3],
    color: [f64; 3],
    intensity: f64,
    shadow_extent: f64,
    radius: f64,
) -> Value {
    let Value::Enum { tag, mut fields } = light(label, position, color, intensity, shadow_extent)
    else {
        unreachable!("a light is an enum value")
    };
    fields.insert(scene::standard::RADIUS.to_owned(), Value::Float(radius));
    Value::Enum { tag, fields }
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
/// The builder takes no `radius`. A spot light's reach is its cone, and a `Spot` declares no
/// radius of its own to be inconsistent with: the compiler culls it by one box radius of the
/// scene, which is the whole scene, and the cone scales what survives. A scene that needs a
/// reach narrower than that is asking a spot light to do a point light's job.
pub fn spot(
    label: &str,
    position: [f64; 3],
    direction: [f64; 3],
    color: [f64; 3],
    intensity: f64,
    inner_angle: f64,
    outer_angle: f64,
    shadow_extent: f64,
) -> Value {
    Value::variant(
        "Spot",
        [
            ("label".to_owned(), Value::String(label.to_owned())),
            ("position".to_owned(), triple(position)),
            ("direction".to_owned(), triple(direction)),
            ("color".to_owned(), triple(color)),
            ("intensity".to_owned(), Value::Float(intensity)),
            ("inner_angle".to_owned(), Value::Float(inner_angle)),
            ("outer_angle".to_owned(), Value::Float(outer_angle)),
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
    intensity: f64,
    shadow_extent: f64,
) -> Value {
    Value::variant(
        "Directional",
        [
            ("label".to_owned(), Value::String(label.to_owned())),
            ("direction".to_owned(), triple(direction)),
            ("color".to_owned(), triple(color)),
            ("intensity".to_owned(), Value::Float(intensity)),
            ("shadow_extent".to_owned(), Value::Float(shadow_extent)),
        ],
    )
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
pub fn environment_light(label: &str, image: &str, color: [f64; 3], intensity: f64) -> Value {
    Value::variant(
        "Environment",
        [
            ("label".to_owned(), Value::String(label.to_owned())),
            ("image".to_owned(), Value::String(image.to_owned())),
            ("color".to_owned(), triple(color)),
            ("intensity".to_owned(), Value::Float(intensity)),
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
