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
use crate::shaders::{Authored, Extras};

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
        Authored::Field {
            name: ID_OUTPUT.to_owned(),
            components: 1,
            flat: true,
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
