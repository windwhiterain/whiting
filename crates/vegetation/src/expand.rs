//! The walk that turns a plant's tree into triangles: one closed solid per node.
//!
//! # The one idea
//!
//! **Expansion is a walk, not a solve.** A node's world spine is its authored jets moved by its placement,
//! its taper is its own law, and [`shape::Sweep`] does the rest — so nothing is reconciled between
//! neighbours and no pass looks for the place a branch ought to meet its parent. The count of triangles is
//! a function of the resolution dial and of nothing else.
//!
//! # The dial is one pair of numbers
//!
//! [`Detail`] is `stations` rings along every spine and `around` vertices around every section, for the
//! whole plant. There is no level-of-detail chain: a coarser plant is the same plant expanded at a smaller
//! dial, which is one number an author can turn rather than a second document to keep in step.
//!
//! # What is not here
//!
//! No welding between a child's base and its parent's skin: a branch is its own closed solid, sunk into
//! the parent by its [`crate::node::Attachment::sink`], and the two surfaces meet by construction rather
//! than by a merge pass. That is the same answer a socket gets in `building` — a joint is one subtraction,
//! not a transform — and it is why every wood loft is a solid that can be graded on its own.
//!
//! # What a walk derives once
//!
//! **The whole expansion holds one [`crate::plant::Derived`] table, and the table is this walk's own local.**
//! Every node's placed spine is a function of the node, its ancestors' placements and the plant's fields, so
//! the wood and every leaf field read the same answers rather than re-deriving them; and the table is dropped
//! when the call returns, so there is no cache inside [`crate::plant::Plant`] that a mutation could leave
//! behind. A stale placed spine would be a wrong picture with no error, so the invalidation is structural:
//! **the table cannot outlive the immutable borrow of the plant it was derived from.**

use shape::{Loft, Sweep};

use crate::handle::Handle;
use crate::leaf::{LeafField, LeafGroup};
use crate::plant::{Derived, Plant};

/// How finely a plant is expanded: rings along each spine, and vertices around each section.
///
/// # Invariants
///
/// **`stations` is a floor and `spacing` is the law**: a spine gets the rings its own length asks for and
/// never fewer than `stations`. A count alone makes a 2 cm grass blade and a 25 m trunk the same six
/// facets, and a 4 m facet on a trunk against a sky is a hexagon; a spacing alone makes a probe's grass
/// blade two rings. The two are one dial because they answer one question — how far apart may two rings
/// be — and the finer of the two answers it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Detail {
    /// Rings along every spine, however short it is.
    pub stations: usize,
    /// Vertices around every section.
    pub around: usize,
    /// The longest distance two rings may be apart, in the plant's units.
    pub spacing: f32,
}

impl Detail {
    /// Six rings and twelve vertices, half a metre between rings: enough for a branch to read as round
    /// and a probe to be fast, and coarse enough that no spine this crate's own species make is longer
    /// than six half-metres, so the floor is what every existing plant is drawn at.
    pub const DEFAULT: Detail = Detail {
        stations: 6,
        around: 12,
        spacing: 0.5,
    };

    /// The same dial, `factor` times finer along, around and between, at least at the minimum a sweep
    /// accepts.
    pub fn scaled(self, factor: usize) -> Self {
        Self {
            stations: (self.stations * factor).max(2),
            around: (self.around * factor).max(3),
            spacing: self.spacing / factor as f32,
        }
    }

    /// The rings one spine of `length` gets: the dial's floor, or what its length asks for, whichever is
    /// finer, and never fewer than the two a sweep accepts.
    pub fn rings(self, length: f32) -> usize {
        let asked = (length / self.spacing).ceil() as usize;
        self.stations.max(asked).max(2)
    }
}

/// One node's wood: where it is in the plant, and the closed solid built there.
#[derive(Debug, Clone, PartialEq)]
pub struct Wood {
    /// The node's handle, so a solid can be named the way the tree names it.
    pub handle: Handle,
    /// The solid, wound outward, with the `(u, v)` of every vertex.
    pub loft: Loft,
}

/// Every node's wood, in the order the plant's walk yields the nodes.
///
/// # Invariants
///
/// Every loft is [`crate::node::Node`]'s own spine moved by its placement, swept with its own section and
/// taper and capped, so each is a closed solid and the whole plant's wood is a list of solids rather than
/// one surface with seams. The walk's handles always resolve, so a failure here is a defect in the tree and
/// not data a caller has to handle.
///
/// **One derived table for the whole walk**: each node's placed spine is a function of the node and of the
/// plant's fields alone, so the walk asks [`crate::plant::Plant::spine_in`] once per node and reuses every
/// ancestor's answer. The table is this call's own local, dropped when it returns, so it cannot outlive the
/// immutable borrow of the plant it was derived from.
pub fn wood(plant: &Plant, detail: Detail) -> Vec<Wood> {
    wood_in(plant, detail, &mut Derived::default())
}

/// [`wood`] with a derived table the caller already holds, so an expansion shares one walk's derivations
/// with its leaf fields rather than re-deriving every ancestor chain per field.
pub(crate) fn wood_in(plant: &Plant, detail: Detail, derived: &mut Derived) -> Vec<Wood> {
    plant
        .live()
        .map(|(handle, node)| {
            let spine = plant
                .spine_in(&handle, derived)
                .expect("a handle the walk yielded resolves");
            let rings = detail.rings(spine.length());
            // **A section's authored corners survive the sampling.** An outline drawn at fewer
            // vertices than it was authored loses its lobes silently — a bark column with a hard
            // terminator where a flute used to be — so the dial is a floor and the section's own
            // corner count is the law above it. An ellipse authors nought corners and is unchanged.
            let around = detail.around.max(node.section().corners());
            let mut sweep = Sweep::new(spine, node.section().clone());
            sweep.scale = node.taper_now();
            Wood {
                handle,
                loft: sweep.sample(rings, around),
            }
        })
        .collect()
}

/// A plant's whole geometry: the wood as solids, the leaves as groups of rigid placements.
#[derive(Debug, Clone, PartialEq)]
pub struct Expansion {
    /// One closed solid per node, in walk order.
    pub wood: Vec<Wood>,
    /// One group per node a field put leaves on, in walk order and then in field order.
    pub leaves: Vec<LeafGroup>,
}

/// The whole plant at one dial: every node's solid, and every field's groups.
///
/// # Invariants
///
/// **The leaves are numbers, not geometry.** A group holds a template index and one rigid placement per
/// leaf, so `k` leaves of one template cost the template's own triangles once and `k` transforms — which is
/// the whole reason a field is a distribution rather than a generator. The wood is the opposite by design: it
/// is baked, one solid per node, because a branch's shape is the document and a leaf's is a template.
///
/// **One derived table for the wood and every field**, because a leaf field's leaves hang on the same nodes
/// the wood sweeps and every field asks for the same placed spines. The table is this call's own local and
/// is dropped when the call returns: the whole expansion holds one immutable borrow of one plant, so there
/// is nothing a mutation could be observed through.
pub fn expand(plant: &Plant, detail: Detail, fields: &[LeafField]) -> Expansion {
    expand_in(plant, detail, fields, &mut Derived::default())
}

/// [`expand`] with the caller's derived table, so a caller that expands and then measures reads one set of
/// placements rather than two.
pub(crate) fn expand_in(
    plant: &Plant,
    detail: Detail,
    fields: &[LeafField],
    derived: &mut Derived,
) -> Expansion {
    let wood = wood_in(plant, detail, derived);
    let mut leaves = Vec::new();
    for field in fields {
        leaves.extend(field.groups_in(plant, derived));
    }
    Expansion { wood, leaves }
}
