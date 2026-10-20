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
        // **The door is in the middle bay of the ground storey**, and it is a plain hole: a door is what
        // you walk through, so it carries no bars.
        open_with_bars(
            skeleton,
            face,
            0,
            front.bays / 2,
            Inset::new(margin, margin, 0.0, 0.4),
            Bar::NONE,
        );
    }
    if front.storeys < 2 {
        return true;
    }
    // **A window in every bay of the upper storey, and none on the ground floor but the door.** The ground
    // floor was given side windows too, and the court face answered with an open wall: its two ends are the
    // *inner* loop, pulled in by the wall's own thickness, so an end bay is narrower than the margin plus a
    // window it was supposed to hold. The expansion does not refuse such an opening — it leaves the solid
    // open, which is `SolidOpen { loops: 4 }` on every range. A window that cannot fit is not written.
    for column in 0..front.bays {
        // **A 直棂窗 in every upper bay**: two bars standing in the hole, anchored to the wall's face so
        // they travel with it.
        open_with_bars(
            skeleton,
            face,
            1,
            column,
            Inset::new(margin, margin, 0.9, 0.5),
            Bar::WINDOW,
        );
    }
    true
}

/// The section of a **直棂窗**'s bars.
const BARS: usize = 2;
const BAR_WIDTH: f32 = 0.05;
const BAR_DEPTH: f32 = 0.04;

/// How many bars one opening carries, and of what section.
///
/// **One value, three callers**: the court window's 直棂窗, the shopfront's 门板 and its transom. A board is a
/// bar with a wide section and a small gap, a lattice bar is a bar with a narrow one, and neither is a
/// different construct — which is why this is a number pair and not three functions.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Bar {
    /// How many bars stand in the opening.
    pub count: usize,
    /// The width across the face, in metres.
    pub width: f32,
    /// The depth out of the face, in metres.
    pub depth: f32,
}

impl Bar {
    /// The two bars of a court window's 直棂窗.
    pub const WINDOW: Self = Self {
        count: BARS,
        width: BAR_WIDTH,
        depth: BAR_DEPTH,
    };

    /// No bars: a door is what you walk through.
    pub const NONE: Self = Self {
        count: 0,
        width: 0.0,
        depth: 0.0,
    };

    /// The boards of a shopfront: five planks, wide and thin, so the front reads as boards and not as a grille.
    pub const BOARDS: Self = Self {
        count: 5,
        width: 0.09,
        depth: 0.035,
    };
}

/// Opens one grid cell and stands `bar`'s bars in the hole it made.
///
/// # Invariants
///
/// **The bars are placed from the hole the cut actually made.** `Opening::extents` is the same rectangle the
/// cut uses and the face's own ground line is the datum `Anchor::OnFace` measures from, so the bars run
/// between the sill and the head of the hole and nowhere else. Computing them from the mass's height instead
/// put them in the wrong place on every range whose gable end was crowned, because a crowned mass's top is
/// its apex and not the top of the wall the window is cut in.
///
/// **An opening the expansion refuses is not given bars**, because a bar in a wall with no hole is a stick on
/// plaster.
pub fn open_with_bars(
    skeleton: &mut BuildingSkeleton,
    face: FaceRef,
    row: usize,
    column: usize,
    inset: Inset,
    bar: Bar,
) -> bool {
    let opening = Opening::new(face, row, column, inset);
    let hole = skeleton
        .bay(face, row, column)
        .and_then(|bay| opening.extents(bay));
    let ground = skeleton.face_extent(face).map(|(ground, _)| ground);
    if ops::add_opening(skeleton, opening).is_none() {
        return false;
    }
    if let (Some(hole), Some(ground)) = (hole, ground) {
        bars(skeleton, face, hole, ground, bar);
    }
    true
}

/// Stands a set of bars in one opening, between the hole's own sill and head.
///
/// # Invariants
///
/// **A bar is anchored to the wall's own face**, not placed at a world coordinate: `Anchor::OnFace` names the
/// face, how far along its edge from the edge's start, and how far above the wall's base at that station — so
/// a bar travels with the wall when the building is seated, and a wall on a slope carries its bars at the
/// slope's own height rather than at one level.
///
/// **A hole too narrow to hold them is left open**, rather than given overlapping bars or bars outside its own
/// reveal.
pub fn bars(
    skeleton: &mut BuildingSkeleton,
    face: FaceRef,
    hole: building::openings::Extents,
    ground: f32,
    bar: Bar,
) {
    if hole.width() <= 0.3 || hole.height() <= 0.3 || bar.count == 0 {
        return;
    }
    let (low, high) = (hole.low - ground, hole.high - ground);
    if high - low <= 0.3 {
        return;
    }
    let Some(along) = face_direction(skeleton, face) else {
        return;
    };
    for index in 1..=bar.count {
        let at = hole.from + hole.width() * index as f32 / (bar.count as f32 + 1.0);
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
            along,
            bar.width,
            bar.depth,
        ));
    }
}

/// Stands one **horizontal rail** across an opening at `up`, anchored to the same face.
///
/// # Invariants
///
/// **A rail runs along the face and is measured across it**, so its `across` is the face's own outward normal
/// and not its direction: the member's axis is the along-coordinate, and a section laid along the axis instead
/// would have no width at all. The normal is `up × along` read in plan — the walker's left on a positively
/// wound loop, which is the outer face — so a rail cannot be drawn inside the wall by a sign mistake.
///
/// `small` is how far the rail's section stands out, and `depth` how tall it is on the face.
pub fn rail(
    skeleton: &mut BuildingSkeleton,
    face: FaceRef,
    hole: building::openings::Extents,
    ground: f32,
    up: f32,
    small: f32,
    depth: f32,
) {
    if hole.width() <= 0.3 {
        return;
    }
    let Some(along) = face_direction(skeleton, face) else {
        return;
    };
    let across = [along[1], -along[0]];
    let at = hole.low - ground + up;
    skeleton.members.push(building::Member::new(
        building::Anchor::OnFace {
            face,
            along: hole.from,
            up: at,
        },
        building::Anchor::OnFace {
            face,
            along: hole.to,
            up: at,
        },
        across,
        depth,
        small,
    ));
}

/// Stands a horizontal rail across one grid cell, `up` metres above the hole's own sill.
///
/// # Invariants
///
/// **It resolves the hole the same way the cut does**, from the bay and the inset, so a rail cannot be drawn
/// across a cell whose opening was refused: if the rectangle has no area there is no rail either.
pub fn rail_across(
    skeleton: &mut BuildingSkeleton,
    face: FaceRef,
    row: usize,
    column: usize,
    inset: Inset,
    up: f32,
    small: f32,
    depth: f32,
) {
    let opening = Opening::new(face, row, column, inset);
    let hole = skeleton
        .bay(face, row, column)
        .and_then(|bay| opening.extents(bay));
    let ground = skeleton.face_extent(face).map(|(ground, _)| ground);
    if let (Some(hole), Some(ground)) = (hole, ground) {
        rail(skeleton, face, hole, ground, up, small, depth);
    }
}

/// Returns the face edge's own plan direction, normalized, or `None` when the edge has no length.
fn face_direction(skeleton: &BuildingSkeleton, face: FaceRef) -> Option<[f32; 2]> {
    let mass = skeleton.masses.get(face.mass.0)?;
    let from = mass.outline.get(face.edge)?;
    let to = mass.outline.get((face.edge + 1) % mass.outline.len())?;
    let along = [
        to.position[0] - from.position[0],
        to.position[1] - from.position[1],
    ];
    let length = (along[0] * along[0] + along[1] * along[1]).sqrt();
    if length <= 1.0e-3 {
        return None;
    }
    Some([along[0] / length, along[1] / length])
}
