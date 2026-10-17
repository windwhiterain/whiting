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

use shape::{Loft, Sweep};

use crate::handle::Handle;
use crate::plant::Plant;

/// How finely a plant is expanded: rings along each spine, and vertices around each section.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Detail {
    /// Rings along every spine.
    pub stations: usize,
    /// Vertices around every section.
    pub around: usize,
}

impl Detail {
    /// Six rings and twelve vertices: enough for a branch to read as round and a probe to be fast.
    pub const DEFAULT: Detail = Detail {
        stations: 6,
        around: 12,
    };

    /// The same dial, `factor` times finer along and around, at least at the minimum a sweep accepts.
    pub fn scaled(self, factor: usize) -> Self {
        Self {
            stations: (self.stations * factor).max(2),
            around: (self.around * factor).max(3),
        }
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
pub fn wood(plant: &Plant, detail: Detail) -> Vec<Wood> {
    plant
        .walk()
        .map(|(handle, node)| {
            let spine = plant
                .spine(&handle)
                .expect("a handle the walk yielded resolves");
            let mut sweep = Sweep::new(spine, node.section().clone());
            sweep.scale = node.taper().clone();
            Wood {
                handle,
                loft: sweep.sample(detail.stations, detail.around),
            }
        })
        .collect()
}
