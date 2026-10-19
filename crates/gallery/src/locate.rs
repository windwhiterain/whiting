//! Answers "which authored part of the scene is this pixel?", from what a frame declared.
//!
//! # What this is
//!
//! The four hops of [`.agents/proposals/agent-content-identity.md`](../../../.agents/proposals/agent-content-identity.md),
//! composed into one call. A caller reads the three values off a rendered frame and gets a line
//! of code:
//!
//! ```text
//! landscape/valley_3/left_flank
//!   id 137  face 1804  u 0.612  v 0.831
//!   top    Ridge(RidgeId(5)), reached between the stations of valley 3 nodes 7 and 8
//!   the edit: massif.skeleton.valleys[3].nodes[7].elevation  (or node 8)
//! ```
//!
//! **The point of composing them here is that nobody can compose them wrongly.** Each hop is
//! owned by whatever knows it — the frame for the values, this module's table for the path, the
//! mountain crate for the belt, the skeleton for the lines — and a caller that chained them itself
//! would be able to chain them wrong, and would be wrong in the middle, where the answer looks
//! like an answer.
//!
//! # The registry
//!
//! **One row per scene, and the row is data rather than a branch.** A scene that names its own elements registers
//! a [`Naming`]: the resolver that turns an id into a handle, and the one clause that says what an id means in that
//! scene. `--locate` and `--region` ask the same table and want different sentences, so the table answers handles and
//! the asker writes the prose — which is also why a scene absent from it is refused rather than guessed at.
//!
//! **The table answers two questions, and only one of them is a sentence.** [`handle_in`] is the *handle* question —
//! one line, no pixel required — and [`locate`] is the *prose* question, whose first line is the handle. Both read
//! the same tables in the same order, so a caller printing a row of a rectangle and a caller printing one pixel are
//! looking at the same number and the same name. **The prose is the caller's concern**: which lines of the answer a
//! caller wants is a property of the question it asked, and a registry that returned a single string would force one
//! shape on both. The landscape is not in [`NAMINGS`] because it is answered by [`crate::identity`], which has its
//! own table and its own two questions.
//!
//! # Why it takes three numbers and not a frame
//!
//! **The dependency runs device → gallery, and a frame is the device's type.** Taking the three
//! values keeps that true: this crate asks nothing of the renderer, and a caller that has the
//! values from anywhere — a readback, a sidecar, a test — can ask the same question. It also
//! keeps the arithmetic here honest about what it actually needs, which is three floats.

use crate::identity::{ID_OUTPUT, NOT_AUTHORED, U_OUTPUT, V_OUTPUT};

/// A scene's own answer to *"which authored element is this id?"*.
pub type Resolver = fn(u32) -> Option<String>;

/// What a scene says about its own ids: how to resolve one, and what one means.
#[derive(Debug, Clone, Copy)]
pub struct Naming {
    /// The scene's name, which is the name a document carries and a frame is asked about.
    pub scene: &'static str,
    /// The id's handle, or `None` when the id names nothing in this scene's content.
    pub resolve: Resolver,
    /// What an id means here, as one clause: the sentence an asker puts after the handle.
    pub note: &'static str,
}

/// Every scene that names its own elements, keyed by name.
///
/// # Invariants
///
/// - **A scene appears once.** A name in this table is a scene whose pixels can be answered; a scene absent from it
///   falls through to [`crate::identity::locate`]'s own answer, which is the landscape's, and is refused by a caller
///   that has no naming for it.
/// - **The resolver answers the handle and nothing else**, so the two askers — a pixel and a rectangle — cannot
///   disagree about what an id means: they read one function.
/// - **The four still scenes share one resolver**, because they draw one plant; the sculpted and grove scenes have
///   their own because their trees differ from the reference tree by at least one node.
pub const NAMINGS: &[Naming] = &[
    Naming {
        scene: crate::vegetation_picture::SCENE,
        resolve: crate::vegetation_picture::handle_of,
        note: "this pixel is that node of the plant, and a leaf drawn by the blade mesh carries its twig's",
    },
    Naming {
        scene: crate::vegetation_picture::WINDY,
        resolve: crate::vegetation_picture::handle_of,
        note: "this pixel is that node of the plant under its wind, and a leaf drawn by the blade mesh carries its twig's",
    },
    Naming {
        scene: crate::vegetation_picture::CROWNED,
        resolve: crate::vegetation_picture::handle_of,
        note: "this pixel is that node of the crowned plant, and a leaf drawn by the blade mesh carries its twig's",
    },
    Naming {
        scene: crate::vegetation_picture::KNOTTED,
        resolve: crate::vegetation_picture::handle_of,
        note: "this pixel is that node of the knotted plant, and a leaf or a knot drawn by an instanced mesh carries its node's",
    },
    Naming {
        scene: crate::vegetation_picture::SCULPTED,
        resolve: crate::vegetation_picture::sculpted_handle_of,
        note: "this pixel is that node of the sculpted plant, and a leaf drawn by the blade mesh carries its twig's",
    },
    Naming {
        scene: crate::vegetation_picture::SEGMENTATION,
        resolve: crate::vegetation_picture::handle_of,
        note: "one colour per element: this pixel is that node of the plant, and a leaf wears the colour of the twig it grew from",
    },
    Naming {
        scene: crate::vegetation_laws::ARCHING,
        resolve: crate::vegetation_laws::arching_handle_of,
        note: "the trunk is a baked S-curve and every bough, branch and leaf hangs off its stations",
    },
    Naming {
        scene: crate::vegetation_laws::PANELLED,
        resolve: crate::vegetation_laws::panelled_handle_of,
        note: "the left crown is trimmed to a circle and the right one to the same radius law drawn with an elliptic section",
    },
    Naming {
        scene: crate::vegetation_laws::BANDS,
        resolve: crate::vegetation_laws::bands_handle_of,
        note: "the left planting has the rule's leaf field and the right one a narrower band, a rising density and a quarter-turn phyllotaxy",
    },
    Naming {
        scene: crate::vegetation_laws::GARDEN_WALL,
        resolve: crate::vegetation_laws::garden_wall_handle_of,
        note: "the left planting has no field and the right one is the plant that knows the wall is there",
    },
    Naming {
        scene: crate::vegetation_laws::ASPIRING,
        resolve: crate::vegetation_laws::aspiring_handle_of,
        note: "the left planting is the rule's own and the right one is under the ancestor field, with its leaves read in the trunk's frame and turned toward the sky",
    },
    Naming {
        scene: crate::vegetation_laws::THINNED,
        resolve: crate::vegetation_laws::thinned_handle_of,
        note: "three clumps left to right at no spacing, a quarter, and most of one; each planting's nodes are numbered from its own base",
    },
    Naming {
        scene: crate::vegetation_laws::BRUSHED,
        resolve: crate::vegetation_laws::brushed_handle_of,
        note: "the left planting is unedited and the right one carries two brush strokes, one by station band and one by generation",
    },
    Naming {
        scene: crate::southwest::YUNNAN_PINE,
        resolve: crate::southwest::yunnan_pine_handle_of,
        note: "this pixel is that node of the pine, and a needle drawn by the blade mesh carries its twig's",
    },
    Naming {
        scene: crate::southwest::YUNNAN_PINE_BOUGH,
        resolve: crate::southwest::yunnan_pine_bough_handle_of,
        note: "one bough at arm's length: this pixel is that node of the bough, and a needle carries its twig's",
    },
    Naming {
        scene: crate::southwest::YUNNAN_PINE_BASE,
        resolve: crate::southwest::yunnan_pine_handle_of,
        note: "the foot of the bole: this pixel is that node of the pine, whose flare and fissures are the subject",
    },
    Naming {
        scene: crate::southwest_flag::DOVE_TREE,
        resolve: crate::southwest_flag::dove_tree_handle_of,
        note: "this pixel is that node of the dove tree; a leaf carries its twig's id and a bract its spur's, and only the white bracts are a second blade",
    },
    Naming {
        scene: crate::southwest_flag::DOVE_TREE_BOUGH,
        resolve: crate::southwest_flag::dove_tree_bough_handle_of,
        note: "one first-order bough at arm's length, in flower: this pixel is that node of the bough",
    },
    Naming {
        scene: crate::southwest_flag::GIANT_RHODODENDRON,
        resolve: crate::southwest_flag::giant_rhododendron_handle_of,
        note: "this pixel is that node of the giant rhododendron; a rosette leaf and the truss in the middle of it both carry their shoot's id",
    },
    Naming {
        scene: crate::southwest_flag::GIANT_RHODODENDRON_BOUGH,
        resolve: crate::southwest_flag::giant_rhododendron_bough_handle_of,
        note: "one crown branch at arm's length: this pixel is that node of the branch",
    },
    Naming {
        scene: crate::southwest_flag::DOVE_TREE_BASE,
        resolve: crate::southwest_flag::dove_tree_handle_of,
        note: "the same tree two metres from its own foot: this pixel is that node, and the bole carries its own id",
    },
    Naming {
        scene: crate::southwest_flag::GIANT_RHODODENDRON_BASE,
        resolve: crate::southwest_flag::giant_rhododendron_handle_of,
        note: "the same tree at its own foot: this pixel is that node, and the bole's flare is this node's taper",
    },
    Naming {
        scene: crate::southwest_oak::ALPINE_OAK,
        resolve: crate::southwest_oak::alpine_oak_handle_of,
        note: "this pixel is that node of the oak, and a leaf drawn by the blade mesh carries its shoot's",
    },
    Naming {
        scene: crate::southwest_oak::ALPINE_OAK_BOUGH,
        resolve: crate::southwest_oak::alpine_oak_bough_handle_of,
        note: "one limb at arm's length: this pixel is that node of the limb, and a leaf carries its shoot's",
    },
    Naming {
        scene: crate::southwest_oak::ALPINE_OAK_BASE,
        resolve: crate::southwest_oak::alpine_oak_handle_of,
        note: "the same tree two metres from its own foot: this pixel is that node, and the bole's flare and flutes are the subject",
    },
    Naming {
        scene: crate::southwest_spire::LIJIAN_SPRUCE,
        resolve: crate::southwest_spire::lijian_spruce_handle_of,
        note: "this pixel is that node of the spruce, and a needle drawn by the blade mesh carries its shoot's",
    },
    Naming {
        scene: crate::southwest_spire::LIJIAN_SPRUCE_BOUGH,
        resolve: crate::southwest_spire::lijian_spruce_bough_handle_of,
        note: "one tier at arm's length: this pixel is that node of the bough, and a needle carries its shoot's",
    },
    Naming {
        scene: crate::southwest_spire::LIJIAN_SPRUCE_BASE,
        resolve: crate::southwest_spire::lijian_spruce_base_handle_of,
        note: "the same tree two and a half metres from its own foot: this pixel is that node, and the bole's plating is this node's section",
    },
    Naming {
        scene: crate::southwest_spire::CANGSHAN_FIR,
        resolve: crate::southwest_spire::cangshan_fir_handle_of,
        note: "this pixel is that node of the fir, and a flat needle drawn by the blade mesh carries its shoot's",
    },
    Naming {
        scene: crate::southwest_spire::CANGSHAN_FIR_BOUGH,
        resolve: crate::southwest_spire::cangshan_fir_bough_handle_of,
        note: "one upright shoot at arm's length: this pixel is that node, and a flat needle carries its spray's",
    },
    Naming {
        scene: crate::southwest_site::SITE_MONTANE,
        resolve: crate::southwest_site::site_montane_handle_of,
        note: "the planting is the id's quotient by the stride and the node its remainder: three heroes and their companions on one slope",
    },
    Naming {
        scene: crate::southwest_sites::SITE_VALLEY,
        resolve: crate::southwest_sites::site_valley_handle_of,
        note: "the planting is the id's quotient by the stride and the node its remainder: a kapok in flower at eight hundred metres, its own kind around it, and three ferns in the gully at z near eleven; the fallen log beside the lens is four and a half metres long and carries the reserved zero, because a log is not an authored element of any plant",
    },
    Naming {
        scene: crate::southwest_sites::SITE_SUBALPINE,
        resolve: crate::southwest_sites::site_subalpine_handle_of,
        note: "the planting is the id's quotient by the stride and the node its remainder: a spruce and a fir as co-heroes on the shoulder at three thousand two hundred metres, four companions of each, and four rhododendrons under them; the stone is one point two metres and carries the reserved zero",
    },
    Naming {
        scene: crate::southwest_sites::SITE_TRANSECT,
        resolve: crate::southwest_sites::site_transect_handle_of,
        note: "the planting is the id's quotient by the stride and the node its remainder: every species in its own belt on one slope, and the frame's own compression is about fifty-six to one -- two thousand four hundred metres of altitude are drawn across forty-two metres of ground, so a belt reads by what grows in it and not by how far above the belt below it stands; the stone is one point two metres and carries the reserved zero",
    },
    Naming {
        scene: crate::southwest_spire::CANGSHAN_FIR_BASE,
        resolve: crate::southwest_spire::cangshan_fir_base_handle_of,
        note: "the same tree two and a half metres from its own foot: this pixel is that node, and the bole's furrows are this node's section",
    },
    Naming {
        scene: crate::vegetation_picture::GROVE,
        resolve: crate::vegetation_picture::grove_handle_of,
        note: "the planting is the id's quotient by the stride and the node is its remainder",
    },
    Naming {
        scene: crate::southwest_valley::TREE_FERN,
        resolve: crate::southwest_valley::tree_fern_handle_of,
        note: "there is no branch graph here: this pixel is the column or one of the crown's fronds, and a pinna drawn by the blade mesh carries its frond's",
    },
    Naming {
        scene: crate::southwest_valley::TREE_FERN_FROND,
        resolve: crate::southwest_valley::tree_fern_frond_handle_of,
        note: "one frond at arm's length: this pixel is that frond's rachis, and a pinna carries its own",
    },
    Naming {
        scene: crate::southwest_valley::TREE_FERN_BASE,
        resolve: crate::southwest_valley::tree_fern_base_handle_of,
        note: "the foot of the same column: this pixel is that node, whose root mantle and packed bases are the subject",
    },
    Naming {
        scene: crate::southwest_valley::KAPOK,
        resolve: crate::southwest_valley::kapok_handle_of,
        note: "in leaf: this pixel is that node of the kapok, and a palmate hand drawn by a blade mesh carries its twig's",
    },
    Naming {
        scene: crate::southwest_valley::KAPOK_FLOWER,
        resolve: crate::southwest_valley::kapok_flower_handle_of,
        note: "the same tree leafless in red flower: this pixel is that node, and a flower drawn by the decorator mesh carries its shoot's",
    },
    Naming {
        scene: crate::southwest_valley::KAPOK_BOUGH,
        resolve: crate::southwest_valley::kapok_bough_handle_of,
        note: "one first-order limb in leaf at eight and a half metres: this pixel is that node of the limb",
    },
    Naming {
        scene: crate::southwest_valley::KAPOK_BASE,
        resolve: crate::southwest_valley::kapok_base_handle_of,
        note: "the plank buttresses of the same tree: this pixel is that node, and the foot's own section is this node's",
    },
    Naming {
        scene: crate::southwest_wave2::SOUTHWEST_BIRCH,
        resolve: crate::southwest_wave2::southwest_birch_handle_of,
        note: "this pixel is that node of the birch, and a leaf or a catkin drawn by a blade mesh carries its branchlet's",
    },
    Naming {
        scene: crate::southwest_wave2::SOUTHWEST_BIRCH_BOUGH,
        resolve: crate::southwest_wave2::southwest_birch_bough_handle_of,
        note: "one bough at arm's length: this pixel is that node of the bough, and a leaf or a catkin carries its branchlet's",
    },
    Naming {
        scene: crate::southwest_wave2::SOUTHWEST_BIRCH_BASE,
        resolve: crate::southwest_wave2::southwest_birch_base_handle_of,
        note: "the foot of the same bole: this pixel is that node, whose flare and lobed outline are the subject",
    },
    Naming {
        scene: crate::southwest_wave2::EVELYN_KETELEERIA,
        resolve: crate::southwest_wave2::evelyn_keteleeria_handle_of,
        note: "this pixel is that node of the Keteleeria; a plume drawn by a blade mesh carries its shoot's, and a cone its own",
    },
    Naming {
        scene: crate::southwest_wave2::EVELYN_KETELEERIA_BOUGH,
        resolve: crate::southwest_wave2::evelyn_keteleeria_bough_handle_of,
        note: "one second-order branch at arm's length: this pixel is that node, and a needle or a cone carries its shoot's",
    },
    Naming {
        scene: crate::southwest_wave2::EVELYN_KETELEERIA_BASE,
        resolve: crate::southwest_wave2::evelyn_keteleeria_base_handle_of,
        note: "the foot of the same bole: this pixel is that node, whose flare and blocky fissures are the subject",
    },
    Naming {
        scene: crate::southwest_wave3::ARMAND_PINE,
        resolve: crate::southwest_wave3::armand_pine_handle_of,
        note: "this pixel is that node of the Armand pine, and a needle drawn by the blade mesh carries its twig's; a cone carries its twig's too",
    },
    Naming {
        scene: crate::southwest_wave3::ARMAND_PINE_BOUGH,
        resolve: crate::southwest_wave3::armand_pine_bough_handle_of,
        note: "one bough at arm's length in the true five-needle fascicle: this pixel is that node of the bough, and a needle carries its twig's",
    },
    Naming {
        scene: crate::southwest_wave3::ARMAND_PINE_BASE,
        resolve: crate::southwest_wave3::armand_pine_base_handle_of,
        note: "the same tree two and a half metres from its own foot: this pixel is that node, and the bole's square plates are this node's section",
    },
    Naming {
        scene: crate::southwest_wave3::QIONG_BAMBOO,
        resolve: crate::southwest_wave3::qiong_bamboo_handle_of,
        note: "twelve culms from one crown: this pixel is that node of the clump, and a leaf drawn by the blade mesh carries its twig's",
    },
    Naming {
        scene: crate::southwest_wave3::QIONG_BAMBOO_CULM,
        resolve: crate::southwest_wave3::qiong_bamboo_culm_handle_of,
        note: "one culm at one metre: this pixel is that node of the culm, and its swollen node is this node's own taper",
    },
    Naming {
        scene: crate::southwest_wave3::QIONG_BAMBOO_BASE,
        resolve: crate::southwest_wave3::qiong_bamboo_base_handle_of,
        note: "the same clump at its own foot: this pixel is that node, and the rhizome crown under the first node is the root's own taper",
    },
];

/// The row a scene registered, or `None` when it names nothing.
pub fn naming(scene: &str) -> Option<&'static Naming> {
    NAMINGS.iter().find(|naming| naming.scene == scene)
}

/// Resolves an `authored_id` written by **any scene this crate names** into the handle of the element it shows.
///
/// This is the **handle** question; [`locate`] is the **prose** question. Both are asked of the same tables, in the
/// same order, and the answer of this one is the first line of that one's — so a caller that wants a handle and a
/// caller that wants a sentence are reading the same number and cannot disagree about what it is.
///
/// # Invariants
///
/// - **The order is [`locate`]'s, and each domain is asked by the content that owns it**: a building's own
///   [`crate::building_identity`] table first, then the naming registry for a scene that registered there, then the
///   landscape's own [`crate::identity`], which is the last because it is the one every other domain falls through
///   to. Neither answers for a scene it does not know, so a scene none of them names answers `None` here exactly as
///   it does there.
/// - **The reserved zero has no handle, and says so rather than naming the first element.** It is not an authored
///   element and no table allocates it; the sentence about it is a *pixel's*, and it belongs to [`locate`].
/// - **The prose around a handle is the caller's, not this table's.** A rectangle of the frame is a count of ids and
///   carries no pixel to place, so a row of one wants the handle alone; a single pixel wants the handle and the face,
///   the parameters and the edit beneath it. One registry answering two questions is what keeps those from being two
///   tables that can disagree.
pub fn handle_in(scene: &str, id: u32) -> Option<String> {
    if let Some(path) = crate::building_identity::locate_in(scene, id) {
        return Some(path);
    }
    if let Some(naming) = naming(scene) {
        return (naming.resolve)(id);
    }
    crate::identity::handle_of(scene, id)
}

/// Resolves the three values a frame declared at one pixel to the authored part they name.
///
/// # Invariants
///
/// - **A value of zero is answered as itself, not as "unknown".** It is a real answer about a
///   real pixel — this one is the plain — and a caller told "not an authored element" knows not
///   to go looking for a crest there. `None` is reserved for a scene whose content names nothing
///   at all, which is a different thing entirely.
/// - **An id above the count is reported, not clamped.** A frame and a mesh from different builds
///   produce exactly that, and the number in the message is what tells the two apart.
/// - **The first line of an answer is its handle, and [`handle_in`] gives that line alone.** The lines beneath it
///   are about a pixel — a face, a place on it, an edit — and are this function's to add; a caller holding only an
///   id asks [`handle_in`] rather than being handed a sentence with a pixel it does not have in it.
pub fn locate(scene: &str, values: &[(String, f32)]) -> Option<String> {
    let one = |name: &str| {
        values
            .iter()
            .find(|(held, _)| held == name)
            .map(|(_, value)| *value)
    };
    // **A building's frame answers through its own id**, and the two domains are asked in the order their
    // channels exist: a building scene declares `authored_id` and nothing else, the landscape declares the id
    // and a belt coordinate. Asking both and taking whichever answers is what keeps this one function rather
    // than one per subsystem — and neither answers for a scene it does not know.
    if let Some(authored) = one(ID_OUTPUT) {
        let id = authored.round() as u32;
        // **The reserved zero is answered before anything refuses, and by the subsystem that owns the
        // sentence.** Zero is what a surface that is not an authored element writes — the ground, the
        // sky — and the reservation is one across the gallery rather than one per content, so the
        // answer belongs to whoever else would have answered a non-zero id: the landscape's own for
        // the plain, and the sentence below for a scene whose content names its elements some other
        // way. Asked only inside the naming below, this arm was unreachable for every building scene,
        // and a pixel of sky was handed the sentence that says the scene does not name its own —
        // false of a scene whose walls are all named.
        if id == NOT_AUTHORED && naming(scene).is_none() {
            return Some(
                crate::identity::locate(
                    scene,
                    Some([authored, 0.0, 0.0, 0.0]),
                    one(U_OUTPUT).unwrap_or(0.0),
                    one(V_OUTPUT).unwrap_or(0.0),
                )
                .unwrap_or_else(|| {
                    String::from(
                        "not an authored element: this pixel is the ground the scene stands on, or \
                         the sky above it",
                    )
                }),
            );
        }
        if let Some(answer) = crate::building_identity::locate_in(scene, id) {
            return Some(answer);
        }
        if let Some(naming) = naming(scene) {
            return Some(match (naming.resolve)(id) {
                Some(handle) => format!("{handle}\n  authored_id {id}: {}", naming.note),
                None if id == NOT_AUTHORED => {
                    "not an authored element: this pixel is the ground the plant stands on, or the sky \
                     above it"
                        .to_owned()
                }
                None => format!("authored_id {id} is not one of this scene's elements"),
            });
        }
    }
    crate::identity::locate(
        scene,
        one(ID_OUTPUT).map(|value| [value, 0.0, 0.0, 0.0]),
        one(U_OUTPUT).unwrap_or(0.0),
        one(V_OUTPUT).unwrap_or(0.0),
    )
}
