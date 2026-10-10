//! The massif: a ridge **tree**, walked into the basins it drains to.
//!
//! # What this replaces
//!
//! The scene used to author a fan — ten crests out of one summit, a valley wedged between each
//! neighbouring pair — and a fan is a cone. Its silhouette is a triangle however its crests are
//! profiled; it has no main ridge, no col, no spur off a spur, and standing it twice gives two tents
//! with a road between them, which is what the picture was.
//!
//! Here the ridges are a **graph**: a spine of crest segments running away from the summit and
//! bending as it falls, spurs born at the spine's cols, and sub-spurs born at the spurs' cols. Three
//! levels of that is a hierarchy, and a hierarchy is what "the same shape at every size" is made of —
//! which is the reading the picture is for.
//!
//! ```text
//!      tip ◀── sub-spur ──◓ col ── spur ──◓ J1 ━━━━ spine ━━━━▶ tip
//!                         (level 2)              ╲                (level 0)
//!                    (level 1)                   ╲━━ spur ━━━━━━▶ tip
//!     ◉ SUMMIT ─━━━━━╱  ╲
//!                    ╲    ╲━━ spur ──◓ col ── sub-spur ──▶ tip
//!                     ╲━━ spine ━━━━▶ tip
//! ```
//!
//! # The two conventions an author has to keep
//!
//! - **Every crest is written outwards.** Its first node is the end nearer the summit, its last is
//!   its tip. That is what makes a basin's wall a plain list of crests walked forwards, and what
//!   makes a junction a place in the table rather than an index to keep straight.
//! - **A junction is always one crest's end and another's start or end**, never a point in the middle
//!   of a line. The expansion turns a belt at a line's end and nowhere else, so a spur born
//!   "halfway along" its parent is authored by splitting the parent there. That is the one rule the
//!   model asks of an author, and it is why the spine is four segments rather than one long one.
//!
//! # Why the basins are walked and not written
//!
//! `SPEC` R4.1 says the correspondence is authored rather than inferred, and this module honours
//! that in the way that matters: **which crest bounds which basin is a statement about the mountain,
//! and it is read out of the plan** — the tips' cyclic order round the massif, and the col where two
//! tips' paths part. What is *not* read out of a search is the surface: no triangulation, no
//! constraint insertion, no intersection test. The first attempt at this scene did the basins by
//! hand, and of fifteen authored junctions five were interior to a crest and therefore not junctions
//! at all, which is the failure mode hand-binding a hierarchy has and the reason the walk is here
//! instead.
//!
//! # The three rules the floors obey
//!
//! - **A rib stands at every junction.** A valley node faces one station on each side, so if a wall
//!   passes from one crest to the next *between* two ribs, the tail of the first and the head of the
//!   second are faced by nobody — a hole in the flank that the picture will not show.
//! - **The floor stands on the sector's bisector, found by rotating.** The midpoint of a point on one
//!   wall and a point on the other leaves the basin as soon as the sector at the head opens past a
//!   right angle, which is most of them. Rotating one wall's outward bearing through half the swept
//!   angle to the other's is correct for a reflex sector too; summing the two unit directions — the
//!   obvious arithmetic — degenerates at 180° and puts the floor outside the basin past it.
//! - **A mouth stands past the further tip.** A rim drawn inside the chord between two tips folds
//!   back there, and a folded rim encloses a negative area, which is the one number that would
//!   otherwise be read as a massif covering its own footprint.

use crate::geometry::smoothstep;
use mountain::line::{LineRef, Station};
use mountain::math::{Vec2, distance, offset};
use mountain::{
    End, FootprintNode, Massif, Ridge, RidgeId, RidgeNode, Skeleton, Tessellation, Valley,
    ValleyNode,
};

/// A full turn, in radians.
const TAU: f32 = std::f32::consts::TAU;

/// How far apart a crest is sampled along its own length.
///
/// It is measured against the **rib**, not against the mountain's width: a belt's cells are bounded
/// across by a rib and along by this step, so a facet's shape is this number over the rib's length.
/// Forty-six units against ribs of two to four hundred gives a face with a shape rather than a
/// splinter, which is the difference between a massif cut from stone and one folded from paper.
const STEP: f32 = 46.0;

/// How many ribs a basin's floor is walked with, between its head and its corner.
///
/// It is a count and not a distance because it is the flank's resolution across the fall: too few and
/// the flank is one ruled plane with nothing to read on it, too many and the facets stop being faces.
/// The same number in a big basin and a small one is the point — a sub-spur's flank is a smaller
/// thing, and it is tiled with the same count of smaller blocks, which is what puts the same shape at
/// every size.
const RIBS: usize = 9;

/// One authored crest: a run of nodes written outwards from the summit, and how sharp it is cut.
struct Crest {
    /// The name this crest's belts answer to.
    name: &'static str,
    /// Its nodes, first to last: plan position and elevation.
    nodes: &'static [[f32; 3]],
    /// How sharp the crest is: 0 rounded, 1 a knife edge. Also decides how finely it is sampled,
    /// because the crests cut to read as rock are the ones whose facets must be small enough to be
    /// rock, and a rounded shoulder must not be chopped into steps it cannot hold.
    sharpness: f32,
    /// How far below the wall above it a floor stands at its furthest, as a fraction of the fall from
    /// head to tip. Read against the crest it belongs to rather than against the massif, because a
    /// sub-spur's gully is not a scaled-down version of a spine's valley — it is shallower, and it is
    /// the shallowness that keeps its own two walls from meeting.
    depth: f32,
    /// Which way this crest's own basin leans across its sector, as a fraction of the way from one
    /// wall to the other.
    ///
    /// A floor on the bisector makes every rib point at the head, and a flank whose ribs all meet at
    /// one point is a fold of paper rather than a rock face. Lean is what turns the ribs into fall
    /// lines that disagree about where down is.
    lean: f32,
}

/// The crests. Plan positions and elevations in metres.
///
/// The spine runs west to east, falls, and bends; the spurs leave its cols to the north and the
/// south; the sub-spurs leave the spurs. Every one of them descends, because elevation is
/// non-increasing along every path from the summit and that is what keeps one vertex the high point
/// of the whole massif.
///
/// The cols are where the junctions are, and every one of them is written as the end of one crest
/// and the start of two or three others. `spine_w` and `spine_w2` are **one ridge line cut in half**,
/// and that is the whole of the difference between a hierarchy and a cone.
const CRESTS: &[Crest] = &[
    // ------------------------------------------------------------------ the shoulder, west of the top
    //
    // The spine runs *through* the summit rather than starting at it, and it is written **from** the
    // summit outwards like every other crest, because elevation is non-increasing along a path away
    // from the root and a crest authored uphill is a crest the checker refuses. A summit with one
    // crest on it is a leaf of the graph, and the face that wraps round a leaf is bounded by the same
    // crest on both sides, which no pair of outward chains can state. A range whose high point stands
    // at the end of its own ridge is a cone; one whose ridge carries on past the top is a range.
    Crest {
        name: "shoulder_top",
        nodes: &[
            [0.0, 0.0, 2400.0],
            [-100.0, 20.0, 2250.0],
            [-190.0, 40.0, 2120.0],
        ],
        sharpness: 0.85,
        depth: 0.28,
        lean: -0.12,
    },
    Crest {
        name: "shoulder_w",
        nodes: &[
            [-190.0, 40.0, 2120.0],
            [-360.0, 80.0, 1900.0],
            [-520.0, 120.0, 1720.0],
        ],
        sharpness: 0.8,
        depth: 0.28,
        lean: 0.14,
    },
    Crest {
        name: "spur_w",
        nodes: &[
            [-190.0, 40.0, 2120.0],
            [-300.0, 220.0, 1880.0],
            [-400.0, 400.0, 1640.0],
            [-480.0, 560.0, 1400.0],
        ],
        sharpness: 0.85,
        depth: 0.30,
        lean: -0.12,
    },
    // ------------------------------------------------------------------ the spine, level 0
    Crest {
        name: "spine_w",
        nodes: &[
            [0.0, 0.0, 2400.0],
            [150.0, -30.0, 2300.0],
            [290.0, -30.0, 2200.0],
            [400.0, -20.0, 2120.0],
        ],
        sharpness: 0.9,
        depth: 0.30,
        lean: 0.16,
    },
    Crest {
        name: "spine_w2",
        nodes: &[
            [400.0, -20.0, 2120.0],
            [500.0, 10.0, 2020.0],
            [580.0, 40.0, 1940.0],
            [640.0, 60.0, 1880.0],
        ],
        sharpness: 0.85,
        depth: 0.28,
        lean: -0.14,
    },
    Crest {
        name: "spine_m",
        nodes: &[
            [640.0, 60.0, 1880.0],
            [760.0, 140.0, 1720.0],
            [900.0, 240.0, 1580.0],
            [1000.0, 310.0, 1480.0],
            [1080.0, 370.0, 1420.0],
        ],
        sharpness: 0.8,
        depth: 0.26,
        lean: 0.12,
    },
    Crest {
        name: "spine_e1",
        nodes: &[
            [1080.0, 370.0, 1420.0],
            [1200.0, 470.0, 1280.0],
            [1340.0, 570.0, 1140.0],
            [1440.0, 640.0, 1040.0],
        ],
        sharpness: 0.7,
        depth: 0.24,
        lean: -0.10,
    },
    Crest {
        name: "spine_e2",
        nodes: &[
            [1440.0, 640.0, 1040.0],
            [1560.0, 700.0, 900.0],
            [1690.0, 750.0, 780.0],
            [1800.0, 780.0, 660.0],
        ],
        sharpness: 0.55,
        depth: 0.22,
        lean: 0.08,
    },
    // ------------------------------------------------------------------ the north side, level 1
    Crest {
        name: "spur_n1",
        nodes: &[
            [400.0, -20.0, 2120.0],
            [370.0, -170.0, 1900.0],
            [320.0, -320.0, 1700.0],
            [250.0, -460.0, 1540.0],
            [210.0, -610.0, 1380.0],
        ],
        sharpness: 0.95,
        depth: 0.32,
        lean: -0.18,
    },
    Crest {
        name: "spur_n2",
        nodes: &[
            [640.0, 60.0, 1880.0],
            [660.0, -90.0, 1680.0],
            [670.0, -230.0, 1500.0],
            [660.0, -340.0, 1340.0],
        ],
        sharpness: 0.9,
        depth: 0.30,
        lean: 0.14,
    },
    Crest {
        name: "spur_s1",
        nodes: &[
            [640.0, 60.0, 1880.0],
            [650.0, 220.0, 1700.0],
            [650.0, 380.0, 1540.0],
            [620.0, 520.0, 1400.0],
        ],
        sharpness: 0.9,
        depth: 0.30,
        lean: -0.16,
    },
    // The continuation of `spur_s1` past the col its gully is born at. **The split is the whole of
    // the difference between a junction and a coincidence**: a gully named at `spur_s1`'s interior
    // node welds geometrically but is walkable in one direction only, so the parent is cut where the
    // child is born and both ends of the cut are ends.
    Crest {
        name: "spur_s1_mid",
        nodes: &[
            [620.0, 520.0, 1400.0],
            [600.0, 600.0, 1320.0],
            [580.0, 660.0, 1260.0],
        ],
        sharpness: 0.85,
        depth: 0.28,
        lean: 0.12,
    },
    Crest {
        name: "spur_s2",
        nodes: &[[1080.0, 370.0, 1420.0], [1130.0, 520.0, 1260.0]],
        sharpness: 0.75,
        depth: 0.26,
        lean: 0.12,
    },
    Crest {
        name: "spur_s2_low",
        nodes: &[
            [1130.0, 520.0, 1260.0],
            [1170.0, 660.0, 1100.0],
            [1190.0, 800.0, 940.0],
        ],
        sharpness: 0.75,
        depth: 0.26,
        lean: 0.12,
    },
    Crest {
        name: "spur_s3",
        nodes: &[
            [1440.0, 640.0, 1040.0],
            [1500.0, 780.0, 900.0],
            [1540.0, 920.0, 760.0],
            [1560.0, 1040.0, 620.0],
        ],
        sharpness: 0.6,
        depth: 0.24,
        lean: -0.10,
    },
    // ------------------------------------------------------------------ level 2, off the spurs
    Crest {
        name: "spur_n1_low",
        nodes: &[
            [210.0, -610.0, 1380.0],
            [110.0, -740.0, 1180.0],
            [-10.0, -860.0, 980.0],
            [-140.0, -950.0, 780.0],
        ],
        sharpness: 0.8,
        depth: 0.30,
        lean: 0.18,
    },
    Crest {
        name: "gully_n",
        nodes: &[
            [660.0, -340.0, 1340.0],
            [780.0, -420.0, 1160.0],
            [890.0, -490.0, 990.0],
            [990.0, -540.0, 820.0],
        ],
        sharpness: 0.7,
        depth: 0.28,
        lean: -0.14,
    },
    Crest {
        name: "spur_n2_low",
        nodes: &[
            [660.0, -340.0, 1340.0],
            [620.0, -470.0, 1160.0],
            [570.0, -590.0, 980.0],
        ],
        sharpness: 0.75,
        depth: 0.26,
        lean: 0.12,
    },
    Crest {
        name: "spur_s1_low",
        nodes: &[
            [580.0, 660.0, 1260.0],
            [500.0, 800.0, 1060.0],
            [400.0, 920.0, 860.0],
            [300.0, 1010.0, 660.0],
        ],
        sharpness: 0.7,
        depth: 0.28,
        lean: -0.14,
    },
    Crest {
        name: "gully_s",
        nodes: &[
            [620.0, 520.0, 1400.0],
            [760.0, 580.0, 1220.0],
            [880.0, 640.0, 1040.0],
        ],
        sharpness: 0.65,
        depth: 0.26,
        lean: 0.10,
    },
    /// The continuation of gully_s past the col its tie is born at, so the tie's far end is a
    /// **through-vertex** and not a tip. A tie between two tips makes a lollipop hanging off the
    /// graph by one vertex, and the pocket wrapping it is an annulus - see [asins].
    Crest {
        name: "gully_s_low",
        nodes: &[
            [880.0, 640.0, 1040.0],
            [940.0, 700.0, 900.0],
            [980.0, 760.0, 740.0],
        ],
        sharpness: 0.65,
        depth: 0.26,
        lean: 0.10,
    },
    // ------------------------------------------------------------------ the tie, which makes it a network
    // Both its ends stand on other crests — the north spur's shoulder and the gully's mouth — so the
    // graph closes a ring, and the ring encloses a face that reaches no tip. That face is a
    // **cirque**: it has no rim to drain to, so its floor runs to the low point of its own wall and
    // spills over the lip into the basin beyond. A tree cannot express that, and a picture of a
    // mountain without one is a picture of a cone with branches on it.
    Crest {
        name: "tie_n",
        nodes: &[
            [1130.0, 520.0, 1260.0],
            [1030.0, 590.0, 1150.0],
            [880.0, 640.0, 1040.0],
        ],
        sharpness: 0.4,
        depth: 0.22,
        lean: 0.08,
    },
];

/// The index a crest is held at, by name.
fn crest_index(name: &str) -> usize {
    CRESTS
        .iter()
        .position(|crest| crest.name == name)
        .unwrap_or_else(|| panic!("`{name}` is not one of the authored crests"))
}

/// Builds the massif.
pub fn massif() -> Massif {
    let mut massif = Massif::new(Skeleton::new(ridges(), Vec::new(), Vec::new()))
        .with_tessellation(Tessellation::default().with_step(STEP));
    // **The crests are taken down to the plain before a single floor is walked.** A floor is written
    // against the wall above it, and the wall it is written against has to be the wall that gets
    // built: walking the floors first and levelling the crests afterwards leaves a valley's last rib
    // standing above a crest tip that has since been taken to the plain — an inverted belt, a surface
    // folded back on itself, and a flank that nothing covers.
    level_the_crests(&mut massif);
    let (valleys, footprint) = drain(&massif.skeleton);
    massif.skeleton.valleys = valleys;
    massif.skeleton.footprint = footprint;
    stand_on_the_plain(&mut massif);
    massif
}

/// Takes every crest's rim end down to the plain, before any floor exists to be written against it.
fn level_the_crests(massif: &mut Massif) {
    for ridge in massif.skeleton.ridge_ids() {
        if is_junction(massif, ridge, true) {
            continue;
        }
        shelf(massif, LineRef::Ridge(ridge));
    }
}

/// Levels everything that reaches the rim down to the plain.
///
/// # Invariants
///
/// The fall is **tapered over the outer part of every line** rather than set at its last node.
/// Cutting only the last node would leave a cliff at the foot of every flank: the rim is at the
/// plain's height and the node before it is where the ridge's own profile left it, so the drop
/// happens across one segment. A shelf multiplied along the line takes the last fifth of every
/// flank down instead, and the product of two non-increasing non-negative functions is still
/// non-increasing, so the edit cannot make the massif invalid.
///
/// Every ridge tip and every valley mouth **is** a footprint vertex, so all three are set together.
/// Levelling only the footprint would leave each ridge ending above the ground it is supposed to
/// stand on, and the gap is at the one place in the frame the eye is trained on — and a point with
/// two elevations is two points, which is what splits a surface open along its own foot.
///
/// The crests are **not** touched here: they were taken down by [`level_the_crests`] before the floors
/// were walked, because a floor is written against the wall above it and the wall has to be the one
/// that gets built. Doing it again here would taper them twice.
fn stand_on_the_plain(massif: &mut Massif) {
    for node in 0..massif.skeleton.footprint.len() {
        massif.set_elevation(LineRef::Footprint, node, super::RIM as f32);
    }
    for valley in massif.skeleton.valley_ids() {
        shelf(massif, LineRef::Valley(valley));
    }
}

/// Whether one end of a crest is a junction — a point another crest's end stands on.
///
/// Read from the authored naming rather than from the plan, because the naming is the fact: two ends
/// at one point that do not name each other are two vertices, and the expansion builds a slit there
/// rather than a saddle.
/// Whether one end of a crest is a junction — a col, where it meets another crest.
///
/// **Asked in both directions**, and the first version asked in one. A junction is authored as one
/// crest's *far end* naming another line, and the naming is written on the crest that arrives rather
/// than on the one that owns the point, so a crest can meet another either by **naming** it or by
/// **being named** by it. Reading only the second case leaves every crest that is itself the child of
/// a junction with its far end looking like a tip: its last node is taken down to the plain, and since
/// the col is one welded vertex shared with the parent, the mesh keeps whichever elevation was written
/// last. That is a saddle at ground level under a crest that is not — and it is exactly what a wall
/// that walks out to `t = 1.0` on one crest and finds height zero, then steps onto another crest,
/// is looking at.
///
/// Read from the authored naming rather than from the plan, because the naming is the fact: two ends
/// at one point that do not name each other are two vertices, and the expansion builds a slit there
/// rather than a saddle.
fn is_junction(massif: &Massif, ridge: RidgeId, far: bool) -> bool {
    let parameter = if far { 1.0 } else { 0.0 };
    let line = LineRef::Ridge(ridge);
    let own = if far {
        massif.skeleton.ridges[ridge.0].end
    } else {
        massif.skeleton.ridges[ridge.0].start
    };
    // This crest arrives on another line: a col.
    if own.station().is_some() {
        return true;
    }
    // Another crest arrives on this one: the same col, authored from the other side.
    massif.skeleton.ridges.iter().any(|other| {
        [other.start, other.end]
            .iter()
            .filter_map(End::station)
            .any(|station| station.line == line && (station.t - parameter).abs() < 1.0e-3)
    })
}

/// Takes the outer part of a line down to the rim's height.
fn shelf(massif: &mut Massif, line: LineRef) {
    let count = massif.skeleton.node_count(line);
    for node in 0..count {
        let fraction = node as f64 / (count - 1) as f64;
        let current = elevation(massif, line, node);
        let height = current as f64 * (1.0 - smoothstep(0.85, 1.0, fraction));
        massif.set_elevation(line, node, height as f32);
    }
}

/// Returns the elevation of one node of a line.
fn elevation(massif: &Massif, line: LineRef, node: usize) -> f32 {
    match line {
        LineRef::Ridge(ridge) => massif.skeleton.ridges[ridge.0].nodes[node].elevation,
        LineRef::Valley(valley) => massif.skeleton.valleys[valley.0].nodes[node].elevation,
        LineRef::Footprint => massif.skeleton.footprint[node].elevation,
    }
}

/// The crests, with every junction **named** rather than coincided with.
///
/// The naming is read off the table rather than written beside it: a junction stated twice is a
/// junction that can be wrong twice, and the point of `End::On` is that one vertex owns the place, so
/// the ring of triangles round a col shares a single mesh vertex instead of leaving a slit whose
/// zero area no coverage check would notice.
fn ridges() -> Vec<Ridge> {
    CRESTS
        .iter()
        .enumerate()
        .map(|(index, crest)| {
            let nodes: Vec<RidgeNode> = crest
                .nodes
                .iter()
                .map(|[x, y, elevation]| {
                    RidgeNode::new([*x, *y], *elevation).with_sharpness(crest.sharpness)
                })
                .collect();
            let ridge = Ridge::root(nodes).with_step_scale(1.25 - 0.6 * crest.sharpness);
            let mut ends = [None, None];
            for (other_index, other) in CRESTS.iter().enumerate() {
                if other_index >= index {
                    continue;
                }
                for far in [false, true] {
                    let own = end_point(crest.name, far).0;
                    for other_far in [false, true] {
                        if distance(own, end_point(other.name, other_far).0) > 0.5 {
                            continue;
                        }
                        ends[usize::from(far)] = Some(Station::ridge(
                            RidgeId(other_index),
                            if other_far { 1.0 } else { 0.0 },
                        ));
                    }
                }
            }
            match ends {
                [None, None] => ridge,
                [start, end] => ridge.with_ends(
                    start.map_or(End::Free, End::On),
                    end.map_or(End::Free, End::On),
                ),
            }
        })
        .collect()
}

/// The plan point and elevation one end of a crest stands at.
fn end_point(name: &str, far: bool) -> (Vec2, f32) {
    let nodes = CRESTS[crest_index(name)].nodes;
    let node = if far {
        nodes[nodes.len() - 1]
    } else {
        nodes[0]
    };
    ([node[0], node[1]], node[2])
}

/// One vertex of the ridge graph, and one wall of one basin.
///
/// The graph is welded from the authored ends: two crest ends at one point are one vertex, which is
/// the same fact `ridges()` states with `End::On` and the same fact the expansion welds on.
struct Graph<'a> {
    /// The vertices, welded.
    vertices: Vec<Vertex>,
    /// For each crest, the vertex its authored start stands at and the vertex its end stands at.
    /// The face walk steps on these: a dart's start and end are read off them rather than stored
    /// twice.
    ends: Vec<[usize; 2]>,
    /// The lines being walked, read for the point and height a station names. Held rather than
    /// copied: a wall's ceiling has to be asked of the skeleton **as it now stands**, because the
    /// skeleton as it was authored still has its foot up in the air where the plain has since taken
    /// it down.
    skeleton: &'a Skeleton,
}

/// One welded vertex.
struct Vertex {
    /// Plan position and elevation.
    position: Vec2,
    elevation: f32,
    /// The crests standing here, each with the direction it leaves in, counterclockwise by bearing.
    /// The order is what the face walk turns on, so it is the one piece of state the walk cannot
    /// recover from anything else.
    at: Vec<Branch>,
}

/// One crest standing at one vertex.
#[derive(Clone, Copy)]
struct Branch {
    /// The crest.
    crest: usize,
    /// The vertex it leads to.
    to: usize,
    /// Whether the crest's authored start stands here, which is the direction of travel that keeps
    /// its parameters reading outwards from this vertex.
    forward: bool,
    /// The bearing it leaves at.
    bearing: f32,
}

impl<'a> Graph<'a> {
    /// Welds the authored ends into vertices.
    fn of(skeleton: &'a Skeleton) -> Self {
        let count = CRESTS.len();
        let mut parent: Vec<usize> = (0..count * 2).collect();
        fn find(parent: &mut [usize], index: usize) -> usize {
            let mut index = index;
            while parent[index] != index {
                parent[index] = parent[parent[index]];
                index = parent[index];
            }
            index
        }
        let union = |parent: &mut Vec<usize>, left: usize, right: usize| {
            let (a, b) = (find(parent, left), find(parent, right));
            if a != b {
                parent[a.max(b)] = a.min(b);
            }
        };
        // Junctions are named in the table, so the welding here reads the naming rather than
        // measuring the plan: two ends are one vertex when one of them stands on the other.
        for (index, crest) in skeleton.ridges.iter().enumerate() {
            for far in [false, true] {
                let standing = if far { crest.end } else { crest.start };
                let Some(station) = standing.station() else {
                    continue;
                };
                let LineRef::Ridge(named) = station.line else {
                    continue;
                };
                let named_far = if station.t <= 0.5 { false } else { true };
                union(
                    &mut parent,
                    index * 2 + usize::from(far),
                    named.0 * 2 + usize::from(named_far),
                );
            }
        }
        let mut ids: std::collections::BTreeMap<usize, usize> = std::collections::BTreeMap::new();
        let mut vertices: Vec<Vertex> = Vec::new();
        let mut ends: Vec<[usize; 2]> = Vec::with_capacity(count);
        for index in 0..count {
            let mut pair = [0usize; 2];
            for far in [false, true] {
                let root = find(&mut parent, index * 2 + usize::from(far));
                let vertex = *ids.entry(root).or_insert_with(|| {
                    let (position, elevation) = end_point(CRESTS[root / 2].name, root % 2 == 1);
                    vertices.push(Vertex {
                        position,
                        elevation,
                        at: Vec::new(),
                    });
                    vertices.len() - 1
                });
                pair[usize::from(far)] = vertex;
            }
            ends.push(pair);
        }
        for (index, pair) in ends.iter().enumerate() {
            for far in [false, true] {
                let at = pair[usize::from(far)];
                vertices[at].at.push(Branch {
                    crest: index,
                    to: pair[usize::from(!far)],
                    forward: !far,
                    bearing: leaving(index, !far),
                });
            }
        }
        for vertex in &mut vertices {
            vertex
                .at
                .sort_by(|left, right| left.bearing.total_cmp(&right.bearing));
        }
        Self {
            vertices,
            ends,
            skeleton,
        }
    }

    fn vertex(&self, index: usize) -> &Vertex {
        &self.vertices[index]
    }

    /// How many crests stand at a vertex. One is a tip, which is where a crest reaches the rim.
    fn degree(&self, index: usize) -> usize {
        self.vertices[index].at.len()
    }

    /// The vertices a run of crest ends passes through, in order, without repeats.
    fn vertices_of(&self, walk: &[Dart]) -> Vec<usize> {
        let mut found: Vec<usize> = Vec::new();
        for dart in walk {
            for vertex in [self.start(*dart), self.end(*dart)] {
                if !found.contains(&vertex) {
                    found.push(vertex);
                }
            }
        }
        found
    }

    /// The vertex with the most height: the root the walk starts from.
    fn summit(&self) -> usize {
        (0..self.vertices.len())
            .max_by(|left, right| {
                self.vertices[*left]
                    .elevation
                    .total_cmp(&self.vertices[*right].elevation)
            })
            .unwrap_or(0)
    }

    /// Every face of the ridge graph, as the run of crest ends that bounds it.
    ///
    /// **The half-edge walk, which is what makes a network and a tree the same case.** A basin is a
    /// face; a face is found by walking its boundary with the land on one side: arrive along a crest
    /// end, and leave by the end immediately clockwise of the one taken back. Every direction of
    /// every crest is the start of exactly one such walk, and the walks that come back to where they
    /// started are the faces.
    ///
    /// A **tree has one face**, and it is the outside: the walk runs down one side of every crest and
    /// back up the other, passing each tip. Splitting that single walk at its degree-one vertices is
    /// what recovers the basins of a tree — one between each consecutive pair of tips — so the tree
    /// is not a special case of the algorithm, it is the case where there is nothing but the outer
    /// face. A **tie** ridge, joining two crest ends that were already connected, cuts a face in two
    /// and the new one reaches no tip at all: that is a cirque, and it is handled by the same walk
    /// with no branch anywhere for it.
    ///
    /// The order is deterministic because the crests are walked in index order and the directions in
    /// the order `start` then `end`, which is what makes an id mean the same thing across builds.
    fn faces(&self) -> Vec<Face> {
        let darts = self.ends.len() * 2;
        let mut seen = vec![false; darts];
        let mut faces: Vec<Face> = Vec::new();
        for start in 0..darts {
            if seen[start] {
                continue;
            }
            let mut walk: Vec<Dart> = Vec::new();
            let mut dart = Dart {
                crest: start / 2,
                forward: start % 2 == 0,
            };
            for _ in 0..=darts {
                // Mark **every dart the walk touches**, not just the one it started on. Marking only
                // the start leaves the other darts of the same boundary unclaimed, so each of them
                // begins a fresh "face" that is really the same cycle entered later, and a graph with
                // twenty crests reports thirty-eight basins whose walls are one crest long.
                seen[dart.slot()] = true;
                walk.push(dart);
                let Some(onward) = self.turn(dart) else {
                    break;
                };
                if onward.slot() == start {
                    break;
                }
                dart = onward;
            }
            faces.push(Face { walk });
        }
        faces
    }

    /// The crest end taken after `came`, keeping the land on the walk's left.
    fn turn(&self, came: Dart) -> Option<Dart> {
        let at = self.end(came);
        let branches = &self.vertices[at].at;
        // The end that leaves here and runs back along the crest just walked.
        let back = branches
            .iter()
            .position(|other| other.crest == came.crest)?;
        // Immediately **clockwise** of it, which is the previous one in the counterclockwise order
        // the branches are held in, wrapping at the front. Written as an index rather than with
        // `checked_sub` and a `?`, because a `?` there ends the walk instead of wrapping it and every
        // face that crosses index zero comes back truncated — which reads as too many faces.
        let previous = (back + 1) % branches.len();
        let branch = branches.get(previous)?;
        // A branch's `forward` already says which way its crest runs **from this vertex** — the
        // authored start stands here, so leaving here runs outwards — and the dart leaving here is
        // therefore the same direction, not its negation. Negating it makes the walk step back along
        // the crest it came in on, which bounces between two darts and returns a "face" one crest
        // long for every crest in the graph.
        Some(Dart {
            crest: branch.crest,
            forward: branch.forward,
        })
    }

    /// Where a crest end stands: the vertex at the end the dart runs to.
    fn end(&self, dart: Dart) -> usize {
        let pair = self.ends[dart.crest];
        if dart.forward { pair[1] } else { pair[0] }
    }

    /// Where a crest end starts from.
    fn start(&self, dart: Dart) -> usize {
        let pair = self.ends[dart.crest];
        if dart.forward { pair[0] } else { pair[1] }
    }

    /// The chain of crests a run of darts makes, walked in the order it is given.
    ///
    /// The caller has already put the darts in the order one wall walks them, which is why this takes
    /// a list rather than a boundary and two endpoints: a basin's two walls are the **same stretch of
    /// boundary traversed in opposite senses**, and a chain that scanned the stretch forward from its
    /// beginning for both would hand back the same wall twice. That scan is what this replaced.
    fn chain(&self, walk: &[Dart]) -> Chain {
        // **A wall has to be a path**, and asserting it is the single check whose absence let a
        // wrong wall through as a plausible mesh for a whole session. `Chain::at` reads a wall as a
        // run of legs joined end to end and interpolates along it without ever asking whether the
        // legs actually join; when they do not, it happily reports a height from somewhere the wall
        // never visited, the floor is written against it, and the surface folds. A wall that is not a
        // path is not a detail to be found later by looking at the render — it is a wrong answer
        // stated confidently, and the only cheap way to catch it is to say so at the moment it is
        // built.
        for pair in walk.windows(2) {
            let (arrived, leaves) = (self.end(pair[0]), self.start(pair[1]));
            debug_assert!(
                arrived == leaves,
                "a wall is not a path: it arrives at vertex {arrived} on `{}` and leaves vertex \
                 {leaves} on `{}`, which are different points. A crest was walked to an end that is \
                 not where the next crest begins.",
                CRESTS[pair[0].crest].name,
                CRESTS[pair[1].crest].name,
            );
        }
        let mut legs: Vec<Leg> = Vec::new();
        for dart in walk {
            let nodes = CRESTS[dart.crest].nodes;
            let mut span = 0.0;
            for pair in nodes.windows(2) {
                span += distance([pair[0][0], pair[0][1]], [pair[1][0], pair[1][1]]);
            }
            // Every crest is authored outwards, so a dart running back along one is walked with its
            // parameters falling. The leg carries which way, so `Chain::at` reads the crest the same
            // way its author wrote it and the station it names is the station the wall stands on.
            legs.push(Leg {
                crest: dart.crest,
                from: if dart.forward { 0.0 } else { 1.0 },
                to: if dart.forward { 1.0 } else { 0.0 },
                span,
            });
        }
        let spans: Vec<f32> = legs.iter().map(|leg| leg.span).collect();
        let total: f32 = spans.iter().sum();
        let crests: Vec<usize> = legs.iter().map(|leg| leg.crest).collect();
        let tip = legs.last().map_or(0, |leg| {
            let pair = self.ends[leg.crest];
            if leg.to > leg.from { pair[1] } else { pair[0] }
        });
        Chain {
            legs,
            crests,
            spans,
            total,
            tip,
        }
    }
}

/// The bearing a crest leaves one of its ends at.
fn leaving(crest: usize, forward: bool) -> f32 {
    let nodes = CRESTS[crest].nodes;
    let last = nodes.len() - 1;
    let (from, to) = if forward {
        (nodes[0], nodes[1])
    } else {
        (nodes[last], nodes[last - 1])
    };
    (to[1] - from[1]).atan2(to[0] - from[0]).rem_euclid(TAU)
}

/// One direction of one crest. The same data as a [`Branch`] read from the other end, but a dart has
/// exactly one start and one end, which is what a face walk steps on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Dart {
    /// The crest.
    crest: usize,
    /// Whether the dart runs from the crest's authored start to its authored end.
    forward: bool,
}

impl Dart {
    /// The slot a dart is remembered by: one per direction of every crest.
    ///
    /// Written once and used for both the `seen` index and the test for the walk's own closing,
    /// because two spellings of the same number is one that can be inverted — and inverted it was,
    /// which made every face come back one dart long.
    fn slot(self) -> usize {
        self.crest * 2 + usize::from(!self.forward)
    }
}

/// One face of the ridge graph: the run of crest ends that bound it, in walk order.
struct Face {
    /// The darts round the boundary, each starting where the one before it ended.
    walk: Vec<Dart>,
}

/// One crest of a wall, with the parameters it is walked between.
#[derive(Debug, Clone, Copy)]
struct Leg {
    /// The crest.
    crest: usize,
    /// The parameter the wall enters it at.
    from: f32,
    /// The parameter it leaves at.
    to: f32,
    /// Its plan length.
    span: f32,
}

/// One wall of a basin: a run of crests from the head out to its far end, walked by the fraction of
/// its own length.
struct Chain {
    /// The legs, head first, each carrying which way its crest is walked.
    legs: Vec<Leg>,
    /// The crests, head first. The same list as `legs` reduced to names, kept because the trace and
    /// the junction arithmetic both want only that.
    crests: Vec<usize>,
    /// The plan length of each.
    spans: Vec<f32>,
    /// The whole wall's length.
    total: f32,
    /// The vertex it ends at.
    tip: usize,
}

impl Chain {
    /// The fractions at which this wall passes a col.
    fn junctions(&self) -> Vec<f32> {
        let mut walked = 0.0;
        let mut found = Vec::new();
        for span in self.spans.iter().take(self.spans.len().saturating_sub(1)) {
            walked += span;
            if self.total > f32::EPSILON {
                found.push(walked / self.total);
            }
        }
        found
    }

    /// Where this wall stands at a fraction of its own length.
    ///
    /// Read **through the skeleton**, at the station the wall names, rather than by interpolating the
    /// authored table by node index. Two reasons, and the first is a live bug: a station names an
    /// arc-length parameter while `nodes.len()` counts *nodes*, so on a crest whose authored nodes are
    /// unevenly spaced the two disagree about where along the crest the station is, and the ceiling
    /// gets evaluated somewhere other than where the floor stands. And the authored table is what the
    /// crest was **before** its foot was taken down to the plain, so a ceiling read from it can sit
    /// above a wall that has since been levelled — which inverts the belt.
    fn at(&self, graph: &Graph, fraction: f32) -> (Vec2, f32, Station) {
        let target = self.total * fraction.clamp(0.0, 1.0);
        let mut walked = 0.0;
        for (index, span) in self.spans.iter().enumerate() {
            let last = index + 1 == self.crests.len();
            if target <= walked + span || last {
                let within = if *span > f32::EPSILON {
                    ((target - walked) / span).clamp(0.0, 1.0)
                } else {
                    0.0
                };
                let leg = self.legs[index];
                let t = leg.from + (leg.to - leg.from) * within;
                let station = Station::ridge(RidgeId(leg.crest), t);
                let point = graph
                    .skeleton
                    .station_point(station)
                    .unwrap_or([0.0, 0.0, 0.0]);
                return ([point[0], point[2]], point[1], station);
            }
            walked += span;
        }
        let at = graph.vertex(self.tip);
        (
            at.position,
            at.elevation,
            Station::ridge(RidgeId(self.crests[0]), self.legs[0].to),
        )
    }
}
/// One basin: the two runs of crest that bound it, from the head out to each end.
struct Basin {
    /// The vertex the two walls part at: the basin's highest, and a col when it is not the summit.
    head: usize,
    /// The wall on the walk's left, head first.
    left: Vec<Dart>,
    /// The wall on the walk's right, head first.
    right: Vec<Dart>,
    /// Whether this basin is a **cirque**: a closed ring that reaches no rim, so both walls end at one
    /// vertex and its floor spills over that lip instead of running out to a mouth.
    closed: bool,
}

/// Split an open stretch at the vertex its two walls part at.
///
/// A pocket of the outer face is a path from one tip to the next, and the head — its highest vertex —
/// lies somewhere along it. The two walls are the way out from the head toward each tip: one is the
/// path's own order, the other is reversed **and turned around**, because a dart read backwards leaves
/// from the vertex it arrived at rather than the one it started at.
fn split(graph: &Graph, walk: &[Dart], head: usize) -> Option<[Vec<Dart>; 2]> {
    let at = walk
        .iter()
        .position(|dart| graph.start(*dart) == head)
        .unwrap_or(walk.len());
    let left: Vec<Dart> = walk[at..].to_vec();
    let right: Vec<Dart> = walk[..at]
        .iter()
        .rev()
        .map(|dart| Dart {
            crest: dart.crest,
            forward: !dart.forward,
        })
        .collect();
    if left.is_empty() || right.is_empty() {
        return None;
    }
    Some([left, right])
}

/// Split a **closed** ring into the two arcs between its highest and its lowest vertex.
///
/// A cirque is the one basin whose boundary comes back to where it started, so splitting it at its
/// head alone produces two walls that both end at the head — `foot` equals `head`, the fall is zero,
/// and the floor is written flat at the summit's own height with a mouth inside the mountain. The two
/// arcs have to run between **two** vertices: the head, where the water parts, and the foot, the lowest
/// point of the ring, which is the col the floor spills over into the basin beyond.
fn split_ring(graph: &Graph, walk: &[Dart], head: usize, foot: usize) -> Option<[Vec<Dart>; 2]> {
    let at = walk
        .iter()
        .position(|dart| graph.start(*dart) == head)
        .unwrap_or(walk.len());
    let to = walk
        .iter()
        .position(|dart| graph.start(*dart) == foot)
        .unwrap_or(at);
    if at == to {
        return None;
    }
    // Round the ring the long way and the short way, both read from the head toward the foot.
    let mut forward: Vec<Dart> = Vec::new();
    let mut index = at;
    while index != to {
        forward.push(walk[index]);
        index = (index + 1) % walk.len();
        if forward.len() > walk.len() {
            return None;
        }
    }
    let mut backward: Vec<Dart> = Vec::new();
    let mut index = at;
    while index != to {
        index = (index + walk.len() - 1) % walk.len();
        let dart = walk[index];
        backward.push(Dart {
            crest: dart.crest,
            forward: !dart.forward,
        });
        if backward.len() > walk.len() {
            return None;
        }
    }
    if forward.is_empty() || backward.is_empty() {
        return None;
    }
    Some([forward, backward])
}

/// Every basin of the ridge graph, from its faces.
///
/// A face that reaches the massif's edge is the outside of the whole graph, and the basins inside it
/// are the stretches between its tips — the land between one tip and the next. A face that reaches no
/// tip is already exactly one basin: a cirque, closed by a tie ridge, with nothing but its own ring
/// round it.
///
/// The head of a basin is the highest vertex on its boundary, and that is not a guess at where water
/// parts: it is the same fact stated as an elevation. For a face that reaches the rim the two walls
/// end at two different tips; for a cirque they end at the one low point where its floor spills over
/// the lip into the basin beyond.
fn basins(graph: &Graph) -> Vec<Basin> {
    let mut found: Vec<Basin> = Vec::new();
    if std::env::var("BASIN_TRACE").is_ok() {
        for face in graph.faces() {
            let visits = face
                .walk
                .iter()
                .filter(|dart| graph.degree(graph.start(**dart)) <= 1)
                .count();
            let names: Vec<&str> = face
                .walk
                .iter()
                .map(|dart| CRESTS[dart.crest].name)
                .collect();
            println!(
                "  face: {} darts, {visits} tip visits, simple {}, [{}]",
                face.walk.len(),
                is_simple(graph, &face.walk),
                names.join(" "),
            );
        }
    }
    for face in graph.faces() {
        if face.walk.is_empty() {
            continue;
        }
        let tips: Vec<usize> = face
            .walk
            .iter()
            .enumerate()
            .filter(|(_, dart)| graph.degree(graph.start(**dart)) <= 1)
            .map(|(index, _)| index)
            .collect();
        // The stretches to split the boundary into: one between each consecutive pair of tips for a
        // face that reaches the massif's edge, and the whole boundary for one that does not — a
        // cirque, which is already exactly one basin.
        let stretches: Vec<Vec<Dart>> = if tips.len() < 2 {
            vec![face.walk.clone()]
        } else {
            let mut built: Vec<Vec<Dart>> = (0..tips.len())
                .map(|slot| {
                    let from = tips[slot];
                    let to = tips[(slot + 1) % tips.len()];
                    let mut stretch: Vec<Dart> = Vec::new();
                    let mut index = from;
                    while index != to {
                        stretch.push(face.walk[index]);
                        index = (index + 1) % face.walk.len();
                        if stretch.len() > face.walk.len() {
                            break;
                        }
                    }
                    stretch
                })
                .collect();
            // **Each pocket is then mirrored**: the same land, walked the other way round. The
            // half-edge walk keeps the land on one side and the basins have to be found in its order,
            // but the rim is a *second* closed walk parallel to the face's, and the mesh wants it wound
            // with the inside on its left — the opposite sense. Taken in the walk's own order the rim
            // comes out wound backwards, its signed area is negative, and the plan comparison reads
            // that as a massif that is not there. Mirroring is the one degree of freedom the two
            // conventions differ by, not a correction to a wrong walk: the other turn direction in
            // [`Graph::turn`] hands back exactly this order.
            for stretch in &mut built {
                stretch.reverse();
                for dart in stretch.iter_mut() {
                    dart.forward = !dart.forward;
                }
            }
            built.reverse();
            built
        };
        for stretch in stretches {
            if stretch.is_empty() {
                continue;
            }
            // **A stretch is a basin only if it visits no vertex twice.** This is the check that says
            // which graphs the weave can handle, and it is not about the weave at all: a rim is a
            // second closed walk parallel to the outer face's, sharing its tips, and a stretch that
            // returns to a vertex it already stood on encloses a hole. A tie between two **tips** makes
            // exactly that — a lollipop hanging off the graph by one vertex, which the outer walk
            // enters and leaves by the same place — and the pocket wrapping it is an annulus, for
            // which "two walls, a head, a bisector, a mouth" has no meaning. Its mouth lands inside
            // the massif, and because welding is by position a misplaced rim vertex does not merely
            // look wrong, it glues the outline to whatever sample it fell on and shatters the mesh.
            // Tie two **cols** instead and every stretch is a simple arc.
            debug_assert!(
                is_simple(graph, &stretch),
                "a basin's boundary revisits a vertex, so the region it bounds has a hole in it: the \
                 ridge graph has a cycle attached to the rest by a single vertex, and the pocket \
                 around that cycle is an annulus which cannot be drained to one mouth. Tie two cols, \
                 not two tips.",
            );
            let Some(head) = highest(graph, &stretch) else {
                continue;
            };
            // A tipless face is a ring, and a ring's two arcs run between its highest and its lowest
            // vertex. An open pocket's two walls run from its head out to its two tips.
            let closed = tips.is_empty();
            let split = if closed {
                let Some(foot) = lowest(graph, &stretch) else {
                    continue;
                };
                split_ring(graph, &stretch, head, foot)
            } else {
                split(graph, &stretch, head)
            };
            let Some([left, right]) = split else {
                continue;
            };
            found.push(Basin {
                head,
                left,
                right,
                closed,
            });
        }
    }
    found
}

/// Whether a run of crest ends walks a boundary that meets itself nowhere.
///
/// The vertices are taken **with multiplicity** — a run of `n` darts stands on `n + 1` of them, each
/// where the one before it ended — and then the closing point of a closed run is dropped, because a
/// ring's last vertex is its first and that repeat is the ring closing rather than the ring crossing
/// itself. What is left must be all different.
///
/// A first version of this compared the count of *distinct* vertices to the dart count plus one, which
/// is right for an open run and wrong for a closed one: a cirque's five-dart ring stands on five
/// vertices, not six, so the check refused the very face it was written to protect.
fn is_simple(graph: &Graph, walk: &[Dart]) -> bool {
    let mut visited: Vec<usize> = Vec::with_capacity(walk.len() + 1);
    for dart in walk {
        visited.push(graph.start(*dart));
    }
    if let Some(last) = walk.last() {
        visited.push(graph.end(*last));
    }
    // A closed run's last vertex is its first; that repeat is not a crossing.
    if visited.len() > 1 && visited.first() == visited.last() {
        visited.pop();
    }
    let mut seen: Vec<usize> = Vec::with_capacity(visited.len());
    for vertex in visited {
        if seen.contains(&vertex) {
            return false;
        }
        seen.push(vertex);
    }
    true
}

/// The highest vertex on a boundary.
fn highest(graph: &Graph, walk: &[Dart]) -> Option<usize> {
    let vertices = graph.vertices_of(walk);
    vertices
        .iter()
        .max_by(|left, right| {
            graph
                .vertex(**left)
                .elevation
                .total_cmp(&graph.vertex(**right).elevation)
        })
        .copied()
}

/// The lowest vertex on a boundary.
fn lowest(graph: &Graph, walk: &[Dart]) -> Option<usize> {
    let vertices = graph.vertices_of(walk);
    vertices
        .iter()
        .min_by(|left, right| {
            graph
                .vertex(**left)
                .elevation
                .total_cmp(&graph.vertex(**right).elevation)
        })
        .copied()
}
fn drain(skeleton: &Skeleton) -> (Vec<Valley>, Vec<FootprintNode>) {
    let graph = Graph::of(skeleton);
    let mut valleys: Vec<Valley> = Vec::new();
    let mut rim: Vec<(Vec2, f32)> = Vec::new();
    let mut pending: Vec<(usize, Vec2)> = Vec::new();

    // One basin per wall pair, and the pairs come out of the faces. A face that reaches the massif's
    // edge is split at its tips and gives one basin between each consecutive pair; a face that does
    // not reach it — a cirque closed by a tie ridge — is one basin whole, draining to the low point
    // of its own ring.
    for (index, basin) in basins(&graph).iter().enumerate() {
        let (left, right) = (graph.chain(&basin.left), graph.chain(&basin.right));
        if left.crests.is_empty() || right.crests.is_empty() {
            continue;
        }
        // A basin whose two walls end at the one vertex is a cirque: it reaches no rim, and its floor
        // spills over the low point of its own ring.
        // A ring's two walls both end at the same col, which is how it was split; that is the fact the
        // basin carries rather than one to be read off the two tips.
        let closed = basin.closed;
        let (head_plan, head_height, _) = left.at(&graph, 0.0);
        let bearing = |chain: &Chain| {
            let at = graph.vertex(chain.tip).position;
            (at[1] - head_plan[1]).atan2(at[0] - head_plan[0])
        };
        let swept = (bearing(&right) - bearing(&left)).rem_euclid(TAU);
        let lean = CRESTS[*right.crests.last().unwrap()].lean;
        let bisector = bearing(&left) + swept * (0.5 + lean * 0.5);
        let direction = [bisector.cos(), bisector.sin()];

        let mut ribs: Vec<f32> = (1..RIBS).map(|rib| rib as f32 / RIBS as f32).collect();
        for chain in [&left, &right] {
            ribs.extend(chain.junctions());
        }
        // **A rim basin ends at its mouth, a cirque ends at its col.** The far rib is the last point
        // the floor is *computed* at; for a basin with a mouth that is where the mouth is hung, and for
        // a cirque it is the lip, whose node the branch below writes with the col's own elevation
        // because the floor has to arrive at a height it does not get to choose. Keeping the rib as
        // well writes the same point twice: a belt of zero area, whose triangles collapse to a point,
        // which encloses no area and so passes the plan comparison while leaving the surface pinched —
        // two boundary loops meeting at one vertex, found by counting boundary edges per vertex.
        if !closed {
            ribs.push(1.0);
        }
        ribs.sort_by(f32::total_cmp);
        ribs.dedup_by(|first, second| (*first - *second).abs() < 1.0e-4);

        let foot = [
            graph.vertex(left.tip).elevation,
            graph.vertex(right.tip).elevation,
        ]
        .into_iter()
        .fold(f32::INFINITY, f32::min);
        let fall = (head_height - foot).max(0.0);
        let depth = CRESTS[*left.crests.last().unwrap()].depth;
        let (_, _, start_left) = left.at(&graph, 0.0);
        let (_, _, start_right) = right.at(&graph, 0.0);
        let mut nodes = vec![ValleyNode::new(
            head_plan,
            head_height,
            start_left,
            start_right,
        )];
        for fraction in &ribs {
            let (a_plan, a_height, a_station) = left.at(&graph, *fraction);
            let (b_plan, b_height, b_station) = right.at(&graph, *fraction);
            if std::env::var("RIB_TRACE").is_ok() && index == 2 {
                println!(
                    "    rib {fraction:.3}: L {a_height:.0} on {:?} / R {b_height:.0} on {:?}",
                    a_station, b_station,
                );
            }
            let reach = (distance(a_plan, head_plan) + distance(b_plan, head_plan)) * 0.5;
            // The floor's height is the lower of two descents: a straight run from the head to the
            // foot, and the ceiling it must stay under. The minimum of two non-increasing functions
            // is itself non-increasing, which is what keeps the elevation rule true without a clamp
            // that would flatten the whole valley to one number.
            //
            // The floor is laid out along the head's bisector at the **average** of the two walls'
            // distances, and it is the only place it can be: the plan position of a floor node is what
            // decides which side of a crest each belt beside it lies on, so a floor moved to, say, the
            // midpoint of its two stations — which reads as more obviously "between the walls" — puts
            // belts on the wrong sides and takes the coverage of whole crests with it. Tried; 24
            // violations. The consequence is a floor that can land on a wall where a wedge bends
            // sharply, which pinches the surface at that one point (see the junction note in
            // `.agents/proposals/network-derivation.md`).
            nodes.push(ValleyNode::new(
                offset(head_plan, [direction[0] * reach, direction[1] * reach]),
                // The floor is the lowest of three descents: a straight run from the head to the foot,
                // the ceiling it must stay under, and **the plain, which is base level**. A valley
                // carves down to where it meets the ground it drains into and no further; digging past
                // it makes the floor dip below zero and climb back to the rim at its mouth, a profile
                // that rises at the end and fails the elevation rule as well as looking like a trench
                // with a step in it.
                (head_height - fall * fraction.powf(0.85))
                    .min(a_height.min(b_height) - fall * depth * *fraction)
                    .max(super::RIM as f32),
                a_station,
                b_station,
            ));
        }
        let corner = nodes.last().map_or(head_height, |node| node.elevation);
        if closed {
            // A **cirque**: both walls end at the same vertex, so the floor reaches the low point of
            // its own ring and stops. Its last node's two stations name that one point, which makes
            // the belts collapse there exactly as they do at a head, and the water goes over the lip
            // into whatever basin the col belongs to. No rim vertex is written, because the cirque
            // has no rim to reach — and that is the whole of why a network is worth more than a tree:
            // it is the only one of the two that can hold a corrie.
            let at = graph.vertex(left.tip);
            nodes.push(ValleyNode::new(
                at.position,
                at.elevation,
                left.at(&graph, 1.0).2,
                right.at(&graph, 1.0).2,
            ));
            valleys.push(Valley::new(nodes).with_ends(End::On(start_left), End::Free));
            continue;
        }
        // The mouth stands beyond **both** tips, and the distance that does it is the fan's own
        // formula: the corner of a sector of half-angle θ whose walls reach `r` is `r / cos θ`,
        // because the chord between the two tips cuts inside that corner by exactly that much. A
        // mouth set at the plain reach therefore sits inside the rim's own polygon whenever the
        // sector is wide, and the rim folds back across the basin it was meant to close — which
        // shows up as a floor sampled outside the footprint, not as a wrong picture.
        let past = [
            distance(graph.vertex(left.tip).position, head_plan),
            distance(graph.vertex(right.tip).position, head_plan),
        ]
        .into_iter()
        .fold(0.0f32, f32::max);
        let across = (swept * 0.5).cos();
        let stand = if across > 0.1 { past / across } else { past };
        let mouth = offset(
            head_plan,
            [direction[0] * stand * 1.12, direction[1] * stand * 1.12],
        );
        // The mouth stands at base level, which is where the floor has already reached: a valley
        // arriving at the rim above its own floor leaves a step for the plain to fill, and one below it
        // digs under the ground it drains into.
        let mouth_elevation = corner.max(super::RIM as f32);
        nodes.push(ValleyNode::new(
            mouth,
            mouth_elevation,
            left.at(&graph, 1.0).2,
            right.at(&graph, 1.0).2,
        ));
        pending.push((valleys.len(), mouth));
        // The rim is written in the order the basins were walked, not sorted by bearing afterwards.
        // Basin `i` lies between tip `i` and tip `i + 1` and drains past both to mouth `i`, so the
        // boundary round the whole massif is exactly `tip, mouth, tip, mouth …`. Sorting those points
        // by bearing about their own centroid asks a different question and gets a different answer:
        // it interleaves a wide basin's mouth with a neighbour's tips, and the polygon that comes out
        // cuts across the massif it was meant to enclose.
        //
        // The tip pushed is the one the basin **starts** at, which is the *right* wall's end: the left
        // wall runs forward along the stretch from the head to the far tip, so its end is the next
        // basin's starting tip. Pushing that one puts each mouth after the tip that follows it, which
        // winds the outline one place out of step and folds a whole side of the range back on itself.
        let at = graph.vertex(right.tip);
        rim.push((at.position, at.elevation));
        rim.push((mouth, mouth_elevation));
        let walked = nodes.len();
        valleys.push(Valley::new(nodes).with_ends(End::On(start_left), End::Free));
        if std::env::var("BASIN_TRACE").is_ok() {
            println!(
                "  basin {index}: head {:.0}m at vertex {} (degree {}), walls [{}] and [{}], {} nodes",
                head_height,
                basin.head,
                graph.degree(basin.head),
                left.crests
                    .iter()
                    .map(|crest| CRESTS[*crest].name)
                    .collect::<Vec<_>>()
                    .join("+"),
                right
                    .crests
                    .iter()
                    .map(|crest| CRESTS[*crest].name)
                    .collect::<Vec<_>>()
                    .join("+"),
                walked,
            );
        }
    }

    // **The rim is wound opposite to the order the basins were walked.** The half-edge walk keeps the
    // land on its left, and a rim written in that same sense comes out wound clockwise — a correct,
    // simple outline of exactly the right area, and a negative one, which the plan comparison reads as
    // a massif that is not there. The mesh wants the inside on the rim's left, so the outline is the
    // basins' order reversed. Reversing the list and not the pairs keeps every `tip, mouth` adjacent.
    rim.reverse();
    let outline = weave_rim(rim);
    let walked = rim_parameters(&outline);
    for (valley, mouth) in pending {
        let Some(parameter) = walked
            .iter()
            .find(|(position, _)| distance(*position, mouth) <= 0.5)
            .map(|(_, parameter)| *parameter)
        else {
            continue;
        };
        let station = Station::new(LineRef::Footprint, parameter);
        let last = valleys[valley].nodes.len() - 1;
        valleys[valley].nodes[last].left = station;
        valleys[valley].nodes[last].right = station;
    }
    (valleys, outline)
}

/// The rim, in the order the basins handed it over.
///
/// Two neighbouring entries can be the same point — a basin whose mouth stands where its neighbour's
/// tip does, which is what happens where two flanks of one crest reach the rim together — and a
/// repeated vertex on a closed loop is a zero-length segment the coverage check splits its arc
/// across. Only consecutive duplicates are folded, because that is all the walk can produce.
fn weave_rim(points: Vec<(Vec2, f32)>) -> Vec<FootprintNode> {
    let mut outline: Vec<FootprintNode> = Vec::with_capacity(points.len());
    for (position, elevation) in points {
        if outline
            .last()
            .is_some_and(|last| distance(last.position, position) <= 0.5)
        {
            continue;
        }
        outline.push(FootprintNode::new(position, elevation));
    }
    if outline.len() > 1
        && distance(outline[0].position, outline[outline.len() - 1].position) <= 0.5
    {
        outline.pop();
    }
    outline
}

/// Each rim vertex's arc-length parameter round the loop.
fn rim_parameters(outline: &[FootprintNode]) -> Vec<(Vec2, f32)> {
    let mut walked = Vec::with_capacity(outline.len());
    let mut total = 0.0;
    for (index, node) in outline.iter().enumerate() {
        walked.push((node.position, total));
        total += distance(node.position, outline[(index + 1) % outline.len()].position);
    }
    if total <= f32::EPSILON {
        return Vec::new();
    }
    walked
        .into_iter()
        .map(|(position, along)| (position, along / total))
        .collect()
}
