//! The court face: where a house's own fronts are, and what a range's front is made of.
//!
//! A courtyard house is not read from outside — it is read from its court, and its court is lined with
//! **bays**: a door in the middle of the ground storey and a window in every other one. Two scenes build
//! that (`一颗印` and the `三坊一照壁`) and this module is the one place that says how, so the two cannot
//! disagree about what a range's front is.

use building::grid::{FaceRef, Rhythm};
use building::openings::{Inset, Opening};
use building::{BuildingSkeleton, MassId, ops};

/// The bays a range's front is cut into, and the openings it carries.
///
/// # Invariants
///
/// **Three bays and two storeys**, which is the 明间 / 次间 division the sources describe: a door in the
/// middle bay of the ground storey, a window in each side bay, and a window in every bay above. The
/// **gate house** is the type's own exception — one storey, one bay, one door — and it is stated here
/// rather than at each call because a gate house is not a range in either type.
pub struct Front {
    /// How many storeys the face is divided into.
    pub storeys: usize,
    /// How many bays wide it is cut.
    pub bays: usize,
    /// Whether the ground storey's middle bay carries a door.
    pub door: bool,
}

impl Front {
    /// The front of a two-storey range.
    pub const RANGE: Self = Self {
        storeys: 2,
        bays: 3,
        door: true,
    };

    /// The front of a gate house: one storey, one bay, one door.
    pub const GATE: Self = Self {
        storeys: 1,
        bays: 3,
        door: true,
    };
}

/// Returns the edge of a mass that stands on the plan line `line`, matching on `z` or on `x`.
///
/// # Invariants
///
/// **The edge is matched by its two nodes' own coordinate**, both of them on the line, and the longest such
/// edge wins: a split loop has several short edges at a corner, and the wall a court opens onto is the long
/// one. An index would not survive the split that crowns a gable end.
pub fn edge_on(mass: &building::Mass, line: f32, on_z: bool) -> Option<usize> {
    let count = mass.outline.len();
    let mut best: Option<(usize, f32)> = None;
    for edge in 0..count {
        let from = mass.outline[edge].position;
        let to = mass.outline[(edge + 1) % count].position;
        let along = if on_z { 1 } else { 0 };
        if (from[along] - line).abs() < 1.0e-3 && (to[along] - line).abs() < 1.0e-3 {
            let length = (to[1 - along] - from[1 - along]).abs();
            if best.is_none_or(|(_, held)| length > held) {
                best = Some((edge, length));
            }
        }
    }
    best.filter(|(_, length)| *length > 1.0)
        .map(|(edge, _)| edge)
}

/// Cuts one mass's court face into bays and opens it, returning whether the face was there to open.
///
/// # Invariants
///
/// **The openings are authored in metres from the bay's own corners**, which is what `Inset` means: a
/// half-metre reveal at each side, a sill or a head measured from the bay's own edge. The door's head is
/// four tenths of a metre below the ground storey's head, and the upper windows hang nine tenths above
/// their own sill and five tenths below their head — numbers chosen so a bay reads as an opening rather
/// than as a hole punched through a wall.
pub fn open_court_face(
    skeleton: &mut BuildingSkeleton,
    mass: MassId,
    line: f32,
    on_z: bool,
    front: Front,
) -> bool {
    let Some(mass_ref) = skeleton.masses.get(mass.0) else {
        return false;
    };
    let Some(edge) = edge_on(mass_ref, line, on_z) else {
        return false;
    };
    let face = FaceRef::new(mass, edge);
    let bays = Rhythm::Even { count: front.bays };
    let storeys = Rhythm::Even {
        count: front.storeys,
    };
    if ops::split_face(skeleton, face, &bays, &storeys).is_none() {
        return false;
    }
    // **A jamb stands clear of the miter the wall's own thickness leaves.** The face an opening is measured
    // against is the *inner* one, whose ends the miter has already pulled in by the wall's thickness; a bay
    // at the end of the wall therefore has less room than its own width, and a margin of half a metre on a
    // half-metre wall leaves the jamb exactly on the corner — which the expansion refuses, by one
    // millimetre, as `OpeningOutsideWall`. The margin is the wall's own thickness plus five centimetres, so
    // a 0.45 m wall keeps the half metre it always had.
    let Some(wall) = skeleton.masses.get(mass.0).map(|mass| mass.wall) else {
        return false;
    };
    let margin = wall + 0.05;
    if front.door {
        let middle = front.bays / 2;
        let _ = ops::add_opening(
            skeleton,
            Opening::new(face, middle, 0, Inset::new(margin, margin, 0.0, 0.4)),
        );
    }
    if front.storeys < 2 {
        return true;
    }
    for column in 0..front.bays {
        let _ = ops::add_opening(
            skeleton,
            Opening::new(face, column, 1, Inset::new(margin, margin, 0.9, 0.5)),
        );
    }
    true
}
