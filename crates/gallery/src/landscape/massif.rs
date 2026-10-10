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
        nodes: &[
            [1080.0, 370.0, 1420.0],
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
    // **The crests are taken down to the plain before a single floor is walked**, because a floor is
    // written against the wall above it and the wall it is written against has to be the wall that
    // gets built. Walking the floors first and levelling the crests afterwards — the ordering this
    // function had — leaves a valley's last rib standing at 165 over a crest tip that has since been
    // taken to 0, which is an inverted belt: a surface folded back on itself, and the source of every
    // four-digit facet aspect in the report.
    level_the_crests(&mut massif);
    let (valleys, footprint) = drain(&massif.skeleton);
    massif.skeleton.valleys = valleys;
    massif.skeleton.footprint = footprint;
    stand_on_the_plain(&mut massif);
    massif
}

/// Takes every crest's rim end down to the plain. Done before the floors exist, so it touches ridges
/// only; [`stand_on_the_plain`] finishes the job on the rim and the valleys.
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
/// **A crest is tapered only toward the end that reaches the rim, and a junction end is never
/// tapered at all.** This is the rule the first version of this function did not have, and what it
/// cost: a hierarchy authors its cols as one crest's *far* end, so tapering every crest's far end
/// took nine of this massif's saddles down to the plain's height and left the summits between them
/// standing. The range became nine fangs rooted in a web at ground level — which validates, because
/// `check_elevations` reads each line on its own and the taper is monotone on every one of them, and
/// which renders as stacked rectangular teeth with a dark gash down each flank. Nothing in the mesh
/// disagrees, because a welded vertex simply keeps whichever elevation was written last.
fn stand_on_the_plain(massif: &mut Massif) {
    for node in 0..massif.skeleton.footprint.len() {
        massif.set_elevation(LineRef::Footprint, node, super::RIM as f32);
    }
    // The crests were taken down before the floors were walked — see [`level_the_crests`] — so what
    // is left is the rim and the valley mouths, which cannot be levelled until they exist.
    for valley in massif.skeleton.valley_ids() {
        // A valley's far end is its mouth, which is a rim vertex; its near end is a head, which is a
        // summit or a col and is welded to the crests it names, so it is left alone either way.
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
/// **Asked in both directions**, and this asked in one. A junction is authored as one crest's *far end*
/// naming another line, and the naming is written on the crest that arrives rather than on the one that
/// owns the point, so a crest can meet another either by **naming** it or by **being named** by it.
/// Reading only the second case leaves every crest that is itself the child of a junction with its far
/// end looking like a tip: its last node is taken down to the ground, and since the col is one welded
/// vertex shared with the parent, the mesh keeps whichever elevation was written last. The range loses
/// that saddle and nothing reports it, because `check_elevations` reads each line on its own and the
/// taper is monotone on every one of them.
///
/// It does not bite the crests authored today, whose junctions all happen to be named from the child's
/// side already. It is a trap for the next one written the other way round, which is why it is fixed
/// rather than noted.
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
    /// The lines being walked, read for the point and height a station names. Held rather than
    /// copied: a wall's ceiling has to be asked of the skeleton **as it now stands**, and the
    /// skeleton it stood as when the graph was welded has already had its foot taken down.
    skeleton: &'a Skeleton,
}

/// One welded vertex.
struct Vertex {
    /// Plan position and elevation.
    position: Vec2,
    elevation: f32,
    /// The crests standing here, each with the direction it leaves in.
    at: Vec<Branch>,
    /// The vertex this one was reached from, and the crest walked to get here. Set by [`Graph::order`],\n    /// read by [`Graph::chain`].
    parent: Option<(usize, usize)>,
    /// How many crests deep this vertex is from the summit.
    depth: usize,
}

/// One crest standing at one vertex.
#[derive(Clone, Copy)]
struct Branch {
    /// The crest.
    crest: usize,
    /// The vertex it leads to.
    to: usize,
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
                        parent: None,
                        depth: 0,
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
                    bearing: leaving(index, !far),
                });
            }
        }
        for vertex in &mut vertices {
            vertex
                .at
                .sort_by(|left, right| left.bearing.total_cmp(&right.bearing));
        }
        Self { vertices, skeleton }
    }

    fn vertex(&self, index: usize) -> &Vertex {
        &self.vertices[index]
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

    /// Roots the graph and returns its tips in the cyclic order they lie round the massif.
    ///
    /// **A wall-following tour, not a depth-first search.** A search lists a vertex's children in
    /// bearing order and calls that the cyclic order, and it is not: the branch back to the parent
    /// sits inside the circle too, so the pair that wraps from the last child to the first has the
    /// parent's own ridge in its sector and is not a face at all. Walking the tree with a wall kept
    /// on one side — at each vertex leaving by the branch next after the one arrived on — passes every
    /// branch of every vertex once, records each tip once, and the order it records them in is the
    /// order they lie round the mountain. Consecutive pairs of that, cyclically closed, are exactly
    /// the basins: one per tip.
    fn order(&mut self, root: usize) -> Vec<usize> {
        let mut tips = Vec::new();
        // The tour starts by walking *into* the tree from the root's first branch, and the root's own
        // branches are taken in bearing order: the leaves come out in the cyclic order they have round
        // the massif, and the last and first of them are adjacent because the root's branch list is a
        // circle and the tour closes there.
        let first = self.vertices[root].at.clone();
        for index in 0..first.len() {
            let branch = first[index];
            self.vertices[branch.to].parent = Some((root, branch.crest));
            self.vertices[branch.to].depth = 1;
            self.tour(branch.to, root, &mut tips);
        }
        if tips.is_empty() {
            tips.push(root);
        }
        tips
    }

    /// Walks on from `at`, having come from `from`, and records the leaves in the order the wall
    /// reaches them.
    ///
    /// At each vertex the branches are taken in bearing order **starting just after the one come in
    /// on**, which is the wall-following step: it is what makes a vertex's parent branch occupy its
    /// place in the circle rather than being skipped, and skipping it is what pairs two tips whose
    /// sector has the ridge back to the summit running through it.
    fn tour(&mut self, at: usize, from: usize, tips: &mut Vec<usize>) {
        let branches = self.vertices[at].at.clone();
        let arrived = branches
            .iter()
            .position(|branch| branch.to == from)
            .unwrap_or(branches.len());
        if branches.len() <= 1 {
            tips.push(at);
            return;
        }
        for step in 1..branches.len() {
            let branch = branches[(arrived + step) % branches.len()];
            if branch.to == from {
                continue;
            }
            self.vertices[branch.to].parent = Some((at, branch.crest));
            self.vertices[branch.to].depth = self.vertices[at].depth + 1;
            self.tour(branch.to, at, tips);
        }
    }

    /// The vertex where the paths from two tips part — which is the basin's head, and the col when it
    /// is not the summit.
    fn ancestor(&self, a: usize, b: usize) -> usize {
        let (mut x, mut y) = (a, b);
        while self.vertices[x].depth > self.vertices[y].depth {
            x = self.vertices[x].parent.map_or(x, |(parent, _)| parent);
        }
        while self.vertices[y].depth > self.vertices[x].depth {
            y = self.vertices[y].parent.map_or(y, |(parent, _)| parent);
        }
        while x != y {
            x = self.vertices[x].parent.map_or(x, |(parent, _)| parent);
            y = self.vertices[y].parent.map_or(y, |(parent, _)| parent);
        }
        x
    }

    /// The chain of crests from a vertex out to one of its tips.
    fn chain(&self, head: usize, tip: usize) -> Chain {
        let mut path: Vec<(usize, usize)> = Vec::new();
        let mut at = tip;
        while at != head {
            let Some((parent, crest)) = self.vertices[at].parent else {
                break;
            };
            path.push((crest, parent));
            at = parent;
        }
        path.reverse();
        let crests: Vec<usize> = path.iter().map(|(crest, _)| *crest).collect();
        let spans: Vec<f32> = crests
            .iter()
            .map(|index| {
                let nodes = CRESTS[*index].nodes;
                let mut total = 0.0;
                for pair in nodes.windows(2) {
                    total += distance([pair[0][0], pair[0][1]], [pair[1][0], pair[1][1]]);
                }
                total
            })
            .collect();
        let total: f32 = spans.iter().sum();
        Chain {
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

/// One wall of a basin: a run of crests from the head out to a tip, walked by the fraction of its own
/// length.
struct Chain {
    /// The crests, head first.
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
    /// arc-length parameter, while `nodes.len()` counts *nodes*, so on a crest whose authored nodes
    /// are unevenly spaced the two disagree about where along the crest the station is — the ceiling
    /// gets evaluated at a different point than the floor stands at. And the authored table is what the
    /// crest was *before* the foot was taken down to the plain, so a ceiling read from it can sit above
    /// a wall that has since been levelled, which inverts the belt.
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
                let station = Station::ridge(RidgeId(self.crests[index]), within);
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
            Station::ridge(RidgeId(self.crests[0]), 1.0),
        )
    }
}

/// The basin floors and the rim.
fn drain(skeleton: &Skeleton) -> (Vec<Valley>, Vec<FootprintNode>) {
    let mut graph = Graph::of(skeleton);
    let root = graph.summit();
    let tips = graph.order(root);
    let mut valleys: Vec<Valley> = Vec::new();
    let mut rim: Vec<(Vec2, f32)> = Vec::new();
    let mut pending: Vec<(usize, Vec2)> = Vec::new();

    for index in 0..tips.len() {
        let (a, b) = (tips[index], tips[(index + 1) % tips.len()]);
        let head = graph.ancestor(a, b);
        let (left, right) = (graph.chain(head, a), graph.chain(head, b));
        if left.crests.is_empty() || right.crests.is_empty() {
            continue;
        }
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
        ribs.push(1.0);
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
        let (head_plan, head_height, start_left) = left.at(&graph, 0.0);
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
            let reach = (distance(a_plan, head_plan) + distance(b_plan, head_plan)) * 0.5;
            // The floor's height is the lowest of three descents: a straight run from the head to the
            // foot, the ceiling it must stay under, and **the plain, which is base level**. A valley
            // carves down to where it meets the ground it drains into and no further — digging past it
            // is what made every floor here dip below zero and then climb back to the rim at its mouth,
            // a profile that rises at the end and so fails the elevation rule as well as looking like
            // a trench with a step in it.
            nodes.push(ValleyNode::new(
                offset(head_plan, [direction[0] * reach, direction[1] * reach]),
                (head_height - fall * fraction.powf(0.85))
                    .min(a_height.min(b_height) - fall * depth * *fraction)
                    .max(super::RIM as f32),
                a_station,
                b_station,
            ));
        }
        let corner = nodes.last().map_or(head_height, |node| node.elevation);
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
        let mouth_elevation = corner * 0.6;
        nodes.push(ValleyNode::new(
            mouth,
            mouth_elevation,
            left.at(&graph, 1.0).2,
            right.at(&graph, 1.0).2,
        ));
        pending.push((valleys.len(), mouth));
        // The rim is written in the order the basins were walked, not sorted by bearing afterwards.
        // Basin `i` lies between tip `i` and tip `i + 1` and drains past both to mouth `i`, so the
        // boundary round the whole massif is exactly `tip, mouth, tip, mouth …` — and each tip appears
        // once, as the left wall's end of one basin and the right wall's end of the one before.
        // Sorting those points by bearing about their own centroid asks a different question and gets
        // a different answer: it interleaves a wide basin's mouth with a neighbour's tips, and the
        // polygon that comes out cuts across the massif it was meant to enclose.
        let at = graph.vertex(left.tip);
        rim.push((at.position, at.elevation));
        rim.push((mouth, mouth_elevation));
        let walked = nodes.len();
        valleys.push(Valley::new(nodes).with_ends(End::On(start_left), End::Free));
        if std::env::var("BASIN_TRACE").is_ok() {
            println!(
                "  basin {index}: head {:.0}m, walls [{}] and [{}], {} nodes",
                head_height,
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
