//! The content's own naming: what a scene's ids mean, and what they name.
//!
//! # What is here
//!
//! - [`NAMINGS`] — every scene that names its own elements, with the sentence it says about an
//!   id.
//! - [`building_of`] — the building a scene draws, which is the gallery's own fact and not the
//!   document's.
//! - the handle tables: a building's elements, the landscape's belts, and a scene of plantings.
//!
//! # Why the naming is here and not in the document
//!
//! **The document carries the naming; this is where the naming comes from.** A renderer reads
//! the naming out of the document it was handed, and this module is the producer's side of that:
//! the words are the gallery's and the ids are the gallery's own walks. Nothing here is read by
//! a renderer, and nothing here is needed to draw a document.
//!
//! # Why the tables are built where they are
//!
//! **An id and the name that goes with it are produced by one walk.** A plant's ids are its
//! nodes' positions in one walk over the plant, and a building's are its elements' positions in
//! one walk over the mesh. A table built by a second walk would be a second thing to disagree
//! with the first, and the disagreement would show as a pixel that names the wrong node.

use building::Building;
use building::identity::Identity;
use definition::Value;
use document::identity::Identity as MassifIdentity;
use document::naming::Table;

use crate::vegetation_mesh::{LEAF_BASE_ID, node_ids_from};
use crate::vegetation_picture::{PLANTING_STRIDE, Planting};

/// A scene's own answer to *"which authored element is this id?"*, in the gallery's words.
///
/// # Invariants
///
/// - **The scene appears once.** A name in this table is a scene whose pixels can be answered; a
///   scene absent from it names nothing, and is answered in the caller's own words.
/// - **The note is one clause and the zero is one sentence.** They are the sentences a caller
///   puts under a handle, and the one a caller is given for the id no table holds.
pub struct SceneNaming {
    /// The scene's name, which is the name a document carries and a frame is asked about.
    pub scene: &'static str,
    /// What an id means here, as one clause: the sentence an asker puts after the handle.
    pub note: &'static str,
    /// What the reserved zero means here, or none for the sentence every scene gets.
    pub zero: Option<&'static str>,
}

/// The sentence every scene gives the reserved zero, when it gives one of its own.
pub const DEFAULT_ZERO: &str =
    "not an authored element: the ground the scene stands on, or the sky above it";
pub const NAMINGS: &[SceneNaming] = &[
    SceneNaming {
        scene: crate::vegetation_picture::SCENE,
        note: "this pixel is that node of the plant, and a leaf drawn by the blade mesh carries its twig's",
        zero: None,
    },
    SceneNaming {
        scene: crate::vegetation_picture::WINDY,
        note: "this pixel is that node of the plant under its wind, and a leaf drawn by the blade mesh carries its twig's",
        zero: None,
    },
    SceneNaming {
        scene: crate::vegetation_picture::CROWNED,
        note: "this pixel is that node of the crowned plant, and a leaf drawn by the blade mesh carries its twig's",
        zero: None,
    },
    SceneNaming {
        scene: crate::vegetation_picture::KNOTTED,
        note: "this pixel is that node of the knotted plant, and a leaf or a knot drawn by an instanced mesh carries its node's",
        zero: None,
    },
    SceneNaming {
        scene: crate::vegetation_picture::SCULPTED,
        note: "this pixel is that node of the sculpted plant, and a leaf drawn by the blade mesh carries its twig's",
        zero: None,
    },
    SceneNaming {
        scene: crate::vegetation_picture::SEGMENTATION,
        note: "one colour per element: this pixel is that node of the plant, and a leaf wears the colour of the twig it grew from",
        zero: None,
    },
    SceneNaming {
        scene: crate::vegetation_laws::ARCHING,
        note: "the trunk is a baked S-curve and every bough, branch and leaf hangs off its stations",
        zero: None,
    },
    SceneNaming {
        scene: crate::vegetation_laws::PANELLED,
        note: "the left crown is trimmed to a circle and the right one to the same radius law drawn with an elliptic section",
        zero: None,
    },
    SceneNaming {
        scene: crate::vegetation_laws::BANDS,
        note: "the left planting has the rule's leaf field and the right one a narrower band, a rising density and a quarter-turn phyllotaxy",
        zero: None,
    },
    SceneNaming {
        scene: crate::vegetation_laws::GARDEN_WALL,
        note: "the left planting has no field and the right one is the plant that knows the wall is there",
        zero: None,
    },
    SceneNaming {
        scene: crate::vegetation_laws::ASPIRING,
        note: "the left planting is the rule's own and the right one is under the ancestor field, with its leaves read in the trunk's frame and turned toward the sky",
        zero: None,
    },
    SceneNaming {
        scene: crate::vegetation_laws::THINNED,
        note: "three clumps left to right at no spacing, a quarter, and most of one; each planting's nodes are numbered from its own base",
        zero: None,
    },
    SceneNaming {
        scene: crate::vegetation_laws::BRUSHED,
        note: "the left planting is unedited and the right one carries two brush strokes, one by station band and one by generation",
        zero: None,
    },
    SceneNaming {
        scene: crate::southwest::YUNNAN_PINE,
        note: "this pixel is that node of the pine, and a needle drawn by the blade mesh carries its twig's",
        zero: None,
    },
    SceneNaming {
        scene: crate::southwest::YUNNAN_PINE_BOUGH,
        note: "one bough at arm's length: this pixel is that node of the bough, and a needle carries its twig's",
        zero: None,
    },
    SceneNaming {
        scene: crate::southwest::YUNNAN_PINE_BASE,
        note: "the foot of the bole: this pixel is that node of the pine, whose flare and fissures are the subject",
        zero: None,
    },
    SceneNaming {
        scene: crate::southwest_flag::DOVE_TREE,
        note: "this pixel is that node of the dove tree; a leaf carries its twig's id and a bract its spur's, and only the white bracts are a second blade",
        zero: None,
    },
    SceneNaming {
        scene: crate::southwest_flag::DOVE_TREE_BOUGH,
        note: "one first-order bough at arm's length, in flower: this pixel is that node of the bough",
        zero: None,
    },
    SceneNaming {
        scene: crate::southwest_flag::GIANT_RHODODENDRON,
        note: "this pixel is that node of the giant rhododendron; a rosette leaf and the truss in the middle of it both carry their shoot's id",
        zero: None,
    },
    SceneNaming {
        scene: crate::southwest_flag::GIANT_RHODODENDRON_BOUGH,
        note: "one crown branch at arm's length: this pixel is that node of the branch",
        zero: None,
    },
    SceneNaming {
        scene: crate::southwest_flag::DOVE_TREE_BASE,
        note: "the same tree two metres from its own foot: this pixel is that node, and the bole carries its own id",
        zero: None,
    },
    SceneNaming {
        scene: crate::southwest_flag::GIANT_RHODODENDRON_BASE,
        note: "the same tree at its own foot: this pixel is that node, and the bole's flare is this node's taper",
        zero: None,
    },
    SceneNaming {
        scene: crate::southwest_oak::ALPINE_OAK,
        note: "this pixel is that node of the oak, and a leaf drawn by the blade mesh carries its shoot's",
        zero: None,
    },
    SceneNaming {
        scene: crate::southwest_oak::ALPINE_OAK_BOUGH,
        note: "one limb at arm's length: this pixel is that node of the limb, and a leaf carries its shoot's",
        zero: None,
    },
    SceneNaming {
        scene: crate::southwest_oak::ALPINE_OAK_BASE,
        note: "the same tree two metres from its own foot: this pixel is that node, and the bole's flare and flutes are the subject",
        zero: None,
    },
    SceneNaming {
        scene: crate::southwest_spire::LIJIAN_SPRUCE,
        note: "this pixel is that node of the spruce, and a needle drawn by the blade mesh carries its shoot's",
        zero: None,
    },
    SceneNaming {
        scene: crate::southwest_spire::LIJIAN_SPRUCE_BOUGH,
        note: "one tier at arm's length: this pixel is that node of the bough, and a needle carries its shoot's",
        zero: None,
    },
    SceneNaming {
        scene: crate::southwest_spire::LIJIAN_SPRUCE_BASE,
        note: "the same tree two and a half metres from its own foot: this pixel is that node, and the bole's plating is this node's section",
        zero: None,
    },
    SceneNaming {
        scene: crate::southwest_spire::CANGSHAN_FIR,
        note: "this pixel is that node of the fir, and a flat needle drawn by the blade mesh carries its shoot's",
        zero: None,
    },
    SceneNaming {
        scene: crate::southwest_spire::CANGSHAN_FIR_BOUGH,
        note: "one upright shoot at arm's length: this pixel is that node, and a flat needle carries its spray's",
        zero: None,
    },
    SceneNaming {
        scene: crate::southwest_site::SITE_MONTANE,
        note: "the planting is the id's quotient by the stride and the node its remainder: three heroes and their companions on one slope",
        zero: None,
    },
    SceneNaming {
        scene: crate::southwest_sites::SITE_VALLEY,
        note: "the planting is the id's quotient by the stride and the node its remainder: a kapok in flower at eight hundred metres, its own kind around it, and three ferns in the gully at z near eleven; the fallen log beside the lens is four and a half metres long and carries the reserved zero, because a log is not an authored element of any plant",
        zero: None,
    },
    SceneNaming {
        scene: crate::southwest_sites::SITE_SUBALPINE,
        note: "the planting is the id's quotient by the stride and the node its remainder: a spruce and a fir as co-heroes on the shoulder at three thousand two hundred metres, four companions of each, and four rhododendrons under them; the stone is one point two metres and carries the reserved zero",
        zero: None,
    },
    SceneNaming {
        scene: crate::southwest_sites::SITE_TRANSECT,
        note: "the planting is the id's quotient by the stride and the node its remainder: every species in its own belt on one slope, and the frame's own compression is about fifty-six to one -- two thousand four hundred metres of altitude are drawn across forty-two metres of ground, so a belt reads by what grows in it and not by how far above the belt below it stands; the stone is one point two metres and carries the reserved zero",
        zero: None,
    },
    SceneNaming {
        scene: crate::southwest_spire::CANGSHAN_FIR_BASE,
        note: "the same tree two and a half metres from its own foot: this pixel is that node, and the bole's furrows are this node's section",
        zero: None,
    },
    SceneNaming {
        scene: crate::vegetation_picture::GROVE,
        note: "the planting is the id's quotient by the stride and the node is its remainder",
        zero: None,
    },
    SceneNaming {
        scene: crate::southwest_valley::TREE_FERN,
        note: "there is no branch graph here: this pixel is the column or one of the crown's fronds, and a pinna drawn by the blade mesh carries its frond's",
        zero: None,
    },
    SceneNaming {
        scene: crate::southwest_valley::TREE_FERN_FROND,
        note: "one frond at arm's length: this pixel is that frond's rachis, and a pinna carries its own",
        zero: None,
    },
    SceneNaming {
        scene: crate::southwest_valley::TREE_FERN_BASE,
        note: "the foot of the same column: this pixel is that node, whose root mantle and packed bases are the subject",
        zero: None,
    },
    SceneNaming {
        scene: crate::southwest_valley::KAPOK,
        note: "in leaf: this pixel is that node of the kapok, and a palmate hand drawn by a blade mesh carries its twig's",
        zero: None,
    },
    SceneNaming {
        scene: crate::southwest_valley::KAPOK_FLOWER,
        note: "the same tree leafless in red flower: this pixel is that node, and a flower drawn by the decorator mesh carries its shoot's",
        zero: None,
    },
    SceneNaming {
        scene: crate::southwest_valley::KAPOK_BOUGH,
        note: "one first-order limb in leaf at eight and a half metres: this pixel is that node of the limb",
        zero: None,
    },
    SceneNaming {
        scene: crate::southwest_valley::KAPOK_BASE,
        note: "the plank buttresses of the same tree: this pixel is that node, and the foot's own section is this node's",
        zero: None,
    },
    SceneNaming {
        scene: crate::southwest_wave2::SOUTHWEST_BIRCH,
        note: "this pixel is that node of the birch, and a leaf or a catkin drawn by a blade mesh carries its branchlet's",
        zero: None,
    },
    SceneNaming {
        scene: crate::southwest_wave2::SOUTHWEST_BIRCH_BOUGH,
        note: "one bough at arm's length: this pixel is that node of the bough, and a leaf or a catkin carries its branchlet's",
        zero: None,
    },
    SceneNaming {
        scene: crate::southwest_wave2::SOUTHWEST_BIRCH_BASE,
        note: "the foot of the same bole: this pixel is that node, whose flare and lobed outline are the subject",
        zero: None,
    },
    SceneNaming {
        scene: crate::southwest_wave2::EVELYN_KETELEERIA,
        note: "this pixel is that node of the Keteleeria; a plume drawn by a blade mesh carries its shoot's, and a cone its own",
        zero: None,
    },
    SceneNaming {
        scene: crate::southwest_wave2::EVELYN_KETELEERIA_BOUGH,
        note: "one second-order branch at arm's length: this pixel is that node, and a needle or a cone carries its shoot's",
        zero: None,
    },
    SceneNaming {
        scene: crate::southwest_wave2::EVELYN_KETELEERIA_BASE,
        note: "the foot of the same bole: this pixel is that node, whose flare and blocky fissures are the subject",
        zero: None,
    },
    SceneNaming {
        scene: crate::southwest_wave3::ARMAND_PINE,
        note: "this pixel is that node of the Armand pine, and a needle drawn by the blade mesh carries its twig's; a cone carries its twig's too",
        zero: None,
    },
    SceneNaming {
        scene: crate::southwest_wave3::ARMAND_PINE_BOUGH,
        note: "one bough at arm's length in the true five-needle fascicle: this pixel is that node of the bough, and a needle carries its twig's",
        zero: None,
    },
    SceneNaming {
        scene: crate::southwest_wave3::ARMAND_PINE_BASE,
        note: "the same tree two and a half metres from its own foot: this pixel is that node, and the bole's square plates are this node's section",
        zero: None,
    },
    SceneNaming {
        scene: crate::southwest_wave3::QIONG_BAMBOO,
        note: "twelve culms from one crown: this pixel is that node of the clump, and a leaf drawn by the blade mesh carries its twig's",
        zero: None,
    },
    SceneNaming {
        scene: crate::southwest_wave3::QIONG_BAMBOO_CULM,
        note: "one culm at one metre: this pixel is that node of the culm, and its swollen node is this node's own taper",
        zero: None,
    },
    SceneNaming {
        scene: crate::southwest_wave3::QIONG_BAMBOO_BASE,
        note: "the same clump at its own foot: this pixel is that node, and the rhizome crown under the first node is the root's own taper",
        zero: None,
    },
    SceneNaming {
        scene: "landscape",
        note: "this pixel is that belt of the valley, and the face it shows is the one the belt is made of",
        zero: Some("not an authored element: the pixel is the plain"),
    },
];

/// Returns the naming a scene registered, or none when it names nothing.
pub fn naming(scene: &str) -> Option<&'static SceneNaming> {
    NAMINGS.iter().find(|naming| naming.scene == scene)
}

/// Writes the naming a scene's document carries.
///
/// # Invariants
///
/// **The document is the only place the naming is read from.** A renderer that asked the gallery
/// would link the gallery, and a renderer that asked a table of its own would answer about the
/// content *now* rather than about the document in hand.
pub fn with_elements(scene: Value, scene_name: &str, handles: Table) -> Value {
    let registered = naming(scene_name);
    let elements = document::naming::Naming {
        note: registered
            .map(|naming| naming.note.to_owned())
            .unwrap_or_default(),
        zero: registered
            .and_then(|naming| naming.zero)
            .unwrap_or(DEFAULT_ZERO)
            .to_owned(),
        handles,
    };
    let mut fields = scene.as_field_map().cloned().unwrap_or_default();
    fields.insert(
        scene::standard::ELEMENTS.to_owned(),
        document::naming::elements_field(&elements),
    );
    Value::Struct(fields)
}
/// The handles a scene's ids name, or none when the scene's own builder has already written them.
///
/// # Invariants
///
/// **A scene that writes its own handles is left alone.** The plant scenes write theirs where the
/// plantings are, because that is where the ids were numbered; a table built here would be a
/// second walk of the same plant and a second thing to disagree with the first.
pub fn handles_of(scene: &str) -> Option<Table> {
    if let Some(handles) = building_handles(scene) {
        return Some(handles);
    }
    if scene == "landscape" {
        return landscape_handles();
    }
    None
}

/// Returns the building a scene draws, or none for a scene that draws no building.
///
/// # Invariants
///
/// **One table, and it was two.** The name of a building and the id of one of its elements used
/// to be two matches over the same list, which is a table that can be extended on one side and
/// not the other: a scene added to one alone answers a name and not an id, and the identity
/// probe's own circle is what finds that.
pub fn building_of(scene: &str) -> Option<Building> {
    Some(match scene {
        "identity" => crate::authored::document(),
        "shed" => crate::shed::document(),
        "chuandou_house" => crate::chuandou::document(),
        "compound" => crate::compound::document(),
        // **Two cameras, one document**: the court view is the same expansion.
        "compound_court" => crate::compound::document(),
        "stilt" => crate::stilt::document(),
        "dai_temple" => crate::dai_temple::document(),
        "dong_village" => crate::dong_village::document(),
        "drum_tower" => crate::drum_tower::document(),
        "earthen_house" => crate::earthen::document(),
        "bai_court" => crate::bai_court::document(),
        "bai_lane" => crate::bai_lane::document(),
        "benzu_miao" => crate::benzu_miao::document(),
        "benzu_miao_court" => crate::benzu_miao::document(),
        "bamboo_house" => crate::bamboo_house::document(),
        "log_house" => crate::log_house::document(),
        "village_west" => crate::village::document(),
        "village_square" => crate::village::document(),
        "hillside_village" => crate::village::hillside_document(),
        "mushroom_house" => crate::mushroom_house::document(),
        "plateau_village" => crate::plateau::document(),
        "qiang_tower" => crate::qiang_tower::document(),
        "rain_bridge" => crate::rain_bridge::document(),
        "stilt_gallery" => crate::stilt::document(),
        "street" => crate::street::document(),
        "street_arcade" => crate::street::document(),
        "terrace" => crate::terrace::document(),
        "slate_court" => crate::slate_court::document(),
        "slate_street" => crate::slate_street::document(),
        "slate_house" => crate::tunpu::document(),
        "tunpu_village" => crate::tunpu_village::document(),
        "tibetan_house" => crate::tibetan::document(),
        "wing" => crate::wing::document(),
        "workshop" => crate::workshop::document(),
        "yard" => crate::yard::document(),
        "yikeyin" => crate::yikeyin::document(),
        // **Two cameras, one document**: the court view is the same expansion, so it names the same rows.
        "yikeyin_court" => crate::yikeyin::document(),
        "house" => crate::house::document(),
        "hall" => crate::hall::document(),
        "dai_hamlet" => crate::hamlet::document(),
        "hall_large" => crate::hall::document_at(14.0, 20.0, 1),
        _ => return None,
    })
}

/// The handles a building scene's ids name, or none for a scene that draws no building.
///
/// # Invariants
///
/// **The table is the mesh's own list of the elements its faces name.** The number a fragment
/// writes and the row a reader looks up are one walk, so a pixel and a name are the same number
/// by construction rather than by two orderings that have not diverged yet.
pub fn building_handles(scene: &str) -> Option<Table> {
    let building = building_of(scene)?;
    let built = building.build().ok()?;
    let numbering = Identity::of(&built, 1);
    Some(Table::of(
        numbering
            .rows()
            .iter()
            .map(|row| (row.id, row.path.clone())),
    ))
}

/// The handles the landscape's ids name.
///
/// # Invariants
///
/// **Both ranges are numbered by one walk, and this is the only walk that numbers them.** Each
/// range runs its own table from where the one before it ended, because two tables that both
/// began at one would hand the same number to two different flanks.
pub fn landscape_handles() -> Option<Table> {
    let massif = crate::landscape::massif();
    let mut rows = Vec::new();
    let mut next = 1u32;
    for range in crate::landscape::ranges() {
        let identity = MassifIdentity::of(&massif, next, |belt| {
            crate::landscape::belt_path(range.name, belt)
        });
        next += identity.rows().len() as u32;
        rows.extend(identity.rows().iter().map(|row| (row.id, row.path.clone())));
    }
    Some(Table::of(rows))
}

/// The handles a scene of plantings names.
///
/// # Invariants
///
/// - **One planting's node is named by its handle alone, and a scene of several names the
///   planting it is in.** The prefix says which plant, and a scene that draws one plant has
///   nothing to say about it.
/// - **The id is split the way it was written**: the planting index is the id's quotient by
///   [`PLANTING_STRIDE`] and the node is its remainder, so a frame answers *which plant* and
///   *which node* with one number and no table of its own.
pub fn planting_handles(plantings: &[Planting]) -> Table {
    let one = plantings.len() == 1;
    Table::of(plantings.iter().enumerate().flat_map(|(index, planting)| {
        node_ids_from(
            &planting.species.plant,
            LEAF_BASE_ID + index as u32 * PLANTING_STRIDE,
        )
        .into_iter()
        .map(move |(handle, id)| {
            let name = handle.to_string();
            (
                id,
                if one {
                    name
                } else {
                    format!("planting {index}, {name}")
                },
            )
        })
    }))
}
