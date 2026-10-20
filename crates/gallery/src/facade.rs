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
        let window = Opening::new(face, column, 1, Inset::new(margin, margin, 0.9, 0.5));
        // **The bars are placed from the opening's own resolved hole**, not from a second reading of the
        // inset: `extents` is the same rectangle the cut uses, and the face's ground turns its elevations
        // into the heights `Anchor::OnFace` measures. A bar computed from the mass's height instead would
        // stand in the wrong place on any face whose top is not the mass's own top — which is every range
        // whose gable end was crowned.
        let hole = skeleton
            .bay(face, 1, column)
            .and_then(|bay| window.extents(bay));
        let ground = skeleton.face_extent(face).map(|(ground, _)| ground);
        if ops::add_opening(skeleton, window).is_some() {
            if let (Some(hole), Some(ground)) = (hole, ground) {
                // **A 直棂窗, the barred window**, which is what an unglazed window of this region is: two
                // vertical bars standing in the opening, anchored to the wall's face so they travel with it.
                lattice(skeleton, face, hole, ground);
            }
        }
    }
    true
}

/// Stands the bars of a **直棂窗** in one opening.
///
/// # Invariants
///
/// **A bar is anchored to the wall's own face**, not placed at a world coordinate: `Anchor::OnFace` names the
/// face, how far along its edge from the edge's start, and how far above the wall's base at that station — so
/// a bar travels with the wall when the building is seated, and a wall on a slope carries its bars at the
/// slope's own height rather than at one level.
///
/// **The bars are placed from the hole the cut actually made.** `hole` is what the opening resolves to and
/// `ground` is the face's own ground line, which is the datum `Anchor::OnFace` measures `up` from; the bars
/// therefore run between the sill and the head of the hole and nowhere else. Computing them from the mass's
/// height instead put them in the wrong place on every range whose gable end was crowned, because a crowned
/// mass's top is its apex and not the top of the wall the window is cut in.
///
/// **Two bars, at the opening's thirds**, because a 直棂窗 is read as a division of the light and three lights
/// are what one bar-pair makes. The section is square-ish — five centimetres wide, four deep: thick enough to
/// cast a shadow at a metre, thin enough to leave the opening open.
///
/// **A hole too narrow to hold them is left open**, rather than given overlapping bars or bars outside its
/// own reveal.
fn lattice(
    skeleton: &mut BuildingSkeleton,
    face: FaceRef,
    hole: building::openings::Extents,
    ground: f32,
) {
    const BARS: usize = 2;
    const WIDTH: f32 = 0.05;
    const DEPTH: f32 = 0.04;
    if hole.width() <= 0.3 || hole.height() <= 0.3 {
        return;
    }
    let (low, high) = (hole.low - ground, hole.high - ground);
    if high - low <= 0.3 {
        return;
    }
    let Some(mass) = skeleton.masses.get(face.mass.0) else {
        return;
    };
    let (Some(from), Some(to)) = (
        mass.outline.get(face.edge),
        mass.outline.get((face.edge + 1) % mass.outline.len()),
    ) else {
        return;
    };
    let along = [
        to.position[0] - from.position[0],
        to.position[1] - from.position[1],
    ];
    let length = (along[0] * along[0] + along[1] * along[1]).sqrt();
    if length <= 1.0e-3 {
        return;
    }
    let across = [along[0] / length, along[1] / length];
    for bar in 1..=BARS {
        let at = hole.from + hole.width() * bar as f32 / (BARS as f32 + 1.0);
        skeleton.members.push(building::Member::new(
            building::Anchor::OnFace {
                face,
                along: at,
                up: low,
            },
            building::Anchor::OnFace {
                face,
                along: at,
                up: high,
            },
            across,
            WIDTH,
            DEPTH,
        ));
    }
}
