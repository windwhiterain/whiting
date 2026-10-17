//! Content identity for a building: which authored element a pixel shows.
//!
//! # What this is for
//!
//! The mountain side of the gallery answers *"which belt is this pixel on"* from a rendered frame; this
//! module is its sibling for buildings, and it is the half of the crate's own [`Identity`] that a picture
//! can use. An agent looking at a frame of a building has nothing to look at but the frame: *"the third
//! window's reveal is too deep"* is a sentence about a picture, and the join from a pixel to the authored
//! element is what makes such a sentence checkable.
//!
//! # The four hops, and who owns each
//!
//! | hop | owned by |
//! | --- | --- |
//! | a pixel → `authored_id` | the frame, read off the image |
//! | `authored_id` → a row of the table | [`Identity`], in this crate's `identity` module |
//! | the row → an element, a mass, an opening | the row itself |
//! | an element → a name an author reads | [`Identity::rows`]' own `path` |
//!
//! # Invariants
//!
//! - **The id is written per corner and is the same at all three.** An element is a property of a *face*,
//!   not of a vertex; the mesh is not indexed, so a value written at one corner is a value the rasteriser
//!   interpolates across the face, and the other two corners would pull it somewhere between two elements.
//! - **A face whose element has no row is refused, not written as zero.** Zero means "not an authored
//!   element" — the sky, the ground — and a pixel showing a wall that answered zero would be a picture
//!   lying about what it shows. The table is built from the same mesh, so a miss is a bug rather than a
//!   case, and it panics with that sentence.
//! - **The layout is the schema's own**, read through the same `packed_layout` the renderer reads it with.
//!   Writing the field order here would be a second copy of the rule that a field's position is its name's
//!   position, and the two would agree until a field was added whose name sorted somewhere else.

use building::BuildingMesh;
use building::identity::Identity;
use definition::{PackedBuilder, PackedList, Value};

use crate::identity::ID_OUTPUT;
use crate::shaders::{
    Authored, Block, Extras, Projection, lighting_shader_declaring, mesh_shaders_over,
};
use definition::Schema;
use scene::standard::Stage;

/// Returns the extra output a building's shader writes beside the frame.
///
/// # Invariants
///
/// **One output, where the landscape declares three.** A building's element is a discrete thing — a wall,
/// a reveal, a floor — and the frame needs a number for it and nothing else; the mountain's `u` and `v`
/// exist because *where on a flank* a pixel sits is a continuous question that a building's surfaces do not
/// ask. A scene that wants both declares both.
pub fn building_extras() -> Extras {
    Extras::new([(
        ID_OUTPUT.to_owned(),
        1,
        Authored::Sum {
            name: ID_OUTPUT.to_owned(),
            components: 1,
            flat: true,
            custom: crate::kit::AUTHORED_OFFSET.to_owned(),
        },
    )])
}

/// Returns the scene's own `outputs` field: the one image a building writes beside the frame.
///
/// # Invariants
///
/// **Location 1, type `f32`.** `f32` is `r32float`; an `rgba16float` is exact only to 2048, so an id of
/// 4093 would come back as 4092 with nothing saying so. The frame is location zero and the pipeline's.
pub fn outputs_field() -> Value {
    Value::dictionary([(
        ID_OUTPUT.to_owned(),
        Value::structure([
            ("location".to_owned(), Value::Int(1)),
            ("type".to_owned(), Value::String("f32".to_owned())),
        ]),
    )])
}

/// Returns one id per **point** the mesh's triangles name: the same id three times per face.
///
/// # Invariants
///
/// It is the definition [`packed`] writes, kept separate so a probe or a tool can read the numbers without
/// decoding a buffer, and so there is one derivation of "which id a corner carries" rather than two.
pub fn ids(built: &BuildingMesh, identity: &Identity) -> Vec<u32> {
    let mut ids = Vec::with_capacity(built.faces.len() * 3);
    for face in &built.faces {
        let id = identity
            .id_of(face.element)
            .expect("every face's element was numbered by the same walk that built the mesh");
        for _ in 0..3 {
            ids.push(id);
        }
    }
    ids
}

/// Returns the points of a building's mesh, each corner carrying its element's id.
///
/// It is [`crate::building_mesh::points_where`] with the id added: the same corner normals, the same
/// positions, and the plan position as the texture coordinate, which is what the building materials read.
pub fn packed(built: &BuildingMesh, identity: &Identity) -> PackedList {
    packed_where(built, identity, |_| true)
}

/// Returns the points of every face an element filter keeps, each corner carrying its element's id.
///
/// # Invariants
///
/// The filter is applied to a **face** after its three ids are consumed, so a filtered mesh holds whole
/// triangles and its ids stay in step with the table whatever the filter keeps.
pub fn packed_where(
    built: &BuildingMesh,
    identity: &Identity,
    keep: impl Fn(&building::Element) -> bool,
) -> PackedList {
    let schema = building_extras().point_schema();
    let layout = graphics::layout::packed_layout(&schema)
        .expect("the building's own point schema is a struct of components");
    let mut points = PackedBuilder::new(layout);
    points.reserve(built.faces.len() * 3);
    let normals = built.corner_normals();
    let mut ids = ids(built, identity).into_iter();
    for (index, face) in built.faces.iter().enumerate() {
        let id = f64::from(ids.next().expect("one id per corner"));
        let _ = ids.next();
        let _ = ids.next();
        if !keep(&face.element) {
            continue;
        }
        let triangle = built.triangle(face);
        for corner in 0..3 {
            let position = triangle[corner];
            let normal = normals.at(index, corner);
            points
                .field(ID_OUTPUT, &[id])
                .and_then(|points| {
                    points.field(
                        "normal",
                        &[
                            f64::from(normal[0]),
                            f64::from(normal[1]),
                            f64::from(normal[2]),
                        ],
                    )
                })
                .and_then(|points| {
                    points.field(
                        "position",
                        &[
                            f64::from(position[0]),
                            f64::from(position[1]),
                            f64::from(position[2]),
                        ],
                    )
                })
                .and_then(|points| {
                    points.field("texture", &[f64::from(position[0]), f64::from(position[2])])
                })
                .expect("a generator writes whole elements of its own layout");
        }
    }
    points
        .finish()
        .expect("every generator writes whole elements of its own layout")
}

/// Returns the extras a surface that is **not** an authored element writes: the reserved zero.
///
/// # Invariants
///
/// **A scene's second surface has to write the outputs too.** A stage that drew geometry and did not would
/// leave the image holding whatever an earlier fragment put there, and the ground a building stands on is
/// the one thing in such a frame that is not an element of it. Every value is the constant zero for the
/// reason zero is reserved.
pub fn not_authored_extras() -> Extras {
    Extras::new([(
        ID_OUTPUT.to_owned(),
        1,
        Authored::Constant(f64::from(building::identity::NOT_AUTHORED)),
    )])
}

/// Returns the four stage shaders of one mesh, with the lighting one **declaring the extra outputs**.
///
/// # Invariants
///
/// **The lighting stage is the only one that writes the outputs**, and it is built here rather than
/// filtered out of [`mesh_shaders_over`](crate::shaders::mesh_shaders_over)'s four by re-deriving the
/// stage table: the one comparison this needs is a tag, and the tag is [`Stage::tag`]'s own.
///
/// The point schema handed in must be the **extras' own** ([`building_extras().point_schema`]) when the
/// mesh carries ids, because a varying numbered by one schema and a fragment output by another is a
/// picture of somebody else's numbers.
pub fn stage_set<'text>(
    prefix: &str,
    body: impl Into<Block<'text>>,
    helpers: impl Into<Block<'text>>,
    point: &Schema,
    extras: &Extras,
) -> Vec<Value> {
    let body = body.into();
    let helpers = helpers.into();
    let mut shaders = mesh_shaders_over(prefix, "", body, helpers, point.clone());
    shaders.retain(
        |shader| !matches!(shader, Value::Enum { tag, .. } if tag == Stage::Lighting.tag()),
    );
    shaders.push(lighting_shader_declaring(
        &format!("{prefix}_lighting"),
        "",
        body,
        helpers,
        crate::kit::gallery_custom_schema(),
        extras,
        Projection::default(),
    ));
    shaders
}

/// Resolves a **name** back to the `authored_id` a frame writes for it.
///
/// # Invariants
///
/// - **The table is the one [`locate_in`] reads**, so a name that round-trips through a pixel answers the same
///   number the pixel carried: the two directions are one table and one document.
/// - **O(rows) per call**, which is what a tool costs and a frame never pays: the harness asks once per query
///   and a scene asks nothing.
/// - A name the table does not hold answers `None` rather than zero, because *"no such element"* and *"an
///   element off screen"* are different answers and a caller that conflated them would read the second as the
///   first.
pub fn id_in(scene: &str, name: &str) -> Option<u32> {
    let building = match scene {
        "identity" => crate::authored::document(),
        "shed" => crate::shed::document(),
        "workshop" => crate::workshop::document(),
        "yard" => crate::yard::document(),
        "house" => crate::house::document(),
        _ => return None,
    };
    let built = building.build().ok()?;
    let numbering = Identity::of(&built, 1);
    numbering
        .rows()
        .iter()
        .find(|row| row.path == name)
        .map(|row| row.id)
}

/// Resolves an `authored_id` written by **any building scene this crate knows** into the name of the element
/// it shows.
///
/// # Invariants
///
/// - **The document is rebuilt from the scene's own builder**, so a caller holding a number from a frame is
///   handed the name the *same* document gives it: rebuilding is what makes the two agree by construction
///   rather than by hope.
/// - **A scene this module cannot name answers `None`**, and so does an id the table does not hold — zero, the
///   sky's, the ground's, and any number from another build. That is the reserved zero doing its job: *"not an
///   element of this building"* is an answer, and it is different from *"an element I cannot name"*.
/// - It is the building side of [`crate::identity::locate`], with the same argument: the hops happen here and
///   nowhere else, because a caller that chained them itself would be wrong in the middle where the answer
///   looks like an answer.
pub fn locate_in(scene: &str, id: u32) -> Option<String> {
    let building = match scene {
        "identity" => crate::authored::document(),
        "shed" => crate::shed::document(),
        "workshop" => crate::workshop::document(),
        "yard" => crate::yard::document(),
        "house" => crate::house::document(),
        _ => return None,
    };
    let built = building.build().ok()?;
    let numbering = Identity::of(&built, 1);
    numbering.row(id).map(|row| row.path.clone())
}

/// Resolves an `authored_id` written by the **identity scene** into the name of the element it shows.
///
/// It is [`locate_in`] with the scene this module was built for, kept because that scene is the one the crate
/// documents as its identity picture.
pub fn locate(id: u32) -> Option<String> {
    locate_in("identity", id)
}
