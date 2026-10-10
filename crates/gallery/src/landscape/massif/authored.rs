//! The massif, written down.
//!
//! # What is authored and what is generated
//!
//! **The control structure is authored**: the ridge polylines, the valley polylines, the
//! correspondence from every valley node to the two ridge stations it faces, and the rim. That is what
//! this file is. **The surface is generated**: the expansion walks each pair of consecutive valley
//! nodes as a belt bounded by two known stations and emits the facets between them. There is no
//! triangulation, no search, and — this is the point — **no derivation of the valleys from the ridges**.
//!
//! An earlier version of this file derived them: `drain()` read the ridge graph, found its faces,
//! decided where each basin's head and mouth were, and invented a floor along the head's bisector. That
//! is a generator wearing the author's clothes, and it went wrong exactly where a generated thing goes
//! wrong — the floor was a *fan*, which fits a mountain that widens away from one summit and does not
//! fit the long narrow troughs of a dissected upland, and no amount of moving it about fixed a shape
//! that was never asked for.
//!
//! The correspondence is the mesh's topology, and it belongs to the author.
//!
//! # The shape, and why it is this shape
//!
//! Four ridges leave one summit on four bearings. Between each neighbouring pair is a **valley**, and
//! the four valleys are what make the massif read as country rather than as a cross: a ridge is a
//! divide between two drainages, so a ridge with no valley beside it is a line on a plain.
//!
//! It is deliberately small. The shape of the *authoring* is what this is here to settle — where a
//! junction is written, how a valley names the ridges it faces, how the rim closes — and a small
//! massif shows that in twenty numbers where a large one buries it in two hundred.
//!
//! Every elevation is written, including the ridges' own descent to the plateau at
//! [`super::RIM`]: nothing is levelled afterwards, because a pass that takes the massif's foot down to
//! the ground has to know which ends are the massif's edge and which are cols, and the author knows
//! that and the pass does not.

use mountain::line::{LineRef, Station};
use mountain::skeleton::{
    End, FootprintNode, Ridge, RidgeId, RidgeNode, Skeleton, Valley, ValleyNode,
};

/// The elevation the country around the massif stands at.
const GROUND: f32 = super::super::RIM as f32;

/// The summit.
const SUMMIT: f32 = 2400.0;

/// A ridge's nodes, from where it starts to where it ends.
///
/// Written as triples so the tables below read as the polylines they are: a plan position and an
/// elevation, in the order the ridge runs.
type Written = &'static [[f32; 3]];

/// One authored ridge: its name, its line, and how sharp its crest is.
struct RidgeLine {
    name: &'static str,
    nodes: Written,
    sharpness: f32,
}

/// The four ridges, on four bearings, each descending from the summit to the plateau.
///
/// The profile is the same shape four times and the lengths are not: a massif whose ridges are all the
/// same length is a star, and the eye reads the irregularity before it reads anything else. Each one
/// reaches [`GROUND`] at its own end, over its own last two nodes rather than in a step, because a
/// ridge that falls off a cliff at its tip is a ridge on a table.
const RIDGES: &[RidgeLine] = &[
    RidgeLine {
        name: "east",
        nodes: &[
            [0.0, 0.0, SUMMIT],
            [300.0, 18.0, 2255.0],
            [610.0, 36.0, 2070.0],
            [900.0, 52.0, 1870.0],
            [1130.0, 66.0, 1650.0],
            [1330.0, 78.0, GROUND],
        ],
        sharpness: 0.85,
    },
    RidgeLine {
        name: "north",
        nodes: &[
            [0.0, 0.0, SUMMIT],
            [-26.0, 270.0, 2290.0],
            [-52.0, 540.0, 2120.0],
            [-74.0, 790.0, 1930.0],
            [-92.0, 990.0, 1690.0],
            [-106.0, 1160.0, GROUND],
        ],
        sharpness: 0.6,
    },
    RidgeLine {
        name: "west",
        nodes: &[
            [0.0, 0.0, SUMMIT],
            [-320.0, -22.0, 2230.0],
            [-650.0, -44.0, 2030.0],
            [-960.0, -64.0, 1810.0],
            [-1200.0, -80.0, 1610.0],
            [-1420.0, -96.0, GROUND],
        ],
        sharpness: 0.9,
    },
    RidgeLine {
        name: "south",
        nodes: &[
            [0.0, 0.0, SUMMIT],
            [30.0, -250.0, 2280.0],
            [60.0, -500.0, 2100.0],
            [86.0, -730.0, 1900.0],
            [108.0, -920.0, 1660.0],
            [124.0, -1080.0, GROUND],
        ],
        sharpness: 0.7,
    },
];

/// One authored valley node: where the floor stands, how high, and **what it faces**.
///
/// The two numbers on each side are parameters along the ridge named there, so a node says "I am
/// between the east ridge a quarter of the way out and the north ridge a third of the way out". That
/// sentence is the mesh's topology; there is nothing else to it.
struct Floor {
    position: [f32; 2],
    elevation: f32,
    /// Which ridge is on the left going outward, and how far along it.
    left: (usize, f32),
    /// The same for the right.
    right: (usize, f32),
}

/// One authored valley: its name and its nodes, head first.
struct ValleyLine {
    name: &'static str,
    nodes: &'static [Floor],
}

/// The four valleys, one between each neighbouring pair of ridges.
///
/// Each runs from the **summit**, where both of its walls meet, out to the rim. Its head names the
/// summit on both sides — the same point twice — which is what makes the first belt collapse to a
/// triangle and the summit close. Its mouth is a rim node, and the rim is written to include it.
///
/// The walls are named going **counterclockwise**, so `left` is the ridge on the higher bearing; a
/// valley written the other way round would be wound backwards and the facets beside it would face
/// into the hill.
const VALLEYS: &[ValleyLine] = &[
    ValleyLine {
        name: "east_of_east",
        nodes: &[
            Floor {
                position: [0.0, 0.0],
                elevation: SUMMIT,
                left: (1, 0.0),
                right: (0, 0.0),
            },
            Floor {
                position: [188.0, 168.0],
                elevation: 2030.0,
                left: (1, 0.33),
                right: (0, 0.29),
            },
            Floor {
                position: [380.0, 392.0],
                elevation: 1830.0,
                left: (1, 0.62),
                right: (0, 0.56),
            },
            Floor {
                position: [570.0, 636.0],
                elevation: 1620.0,
                left: (1, 0.88),
                right: (0, 0.86),
            },
            Floor {
                position: [760.0, 900.0],
                elevation: GROUND,
                left: (1, 1.0),
                right: (0, 1.0),
            },
        ],
    },
    ValleyLine {
        name: "north_of_north",
        nodes: &[
            Floor {
                position: [0.0, 0.0],
                elevation: SUMMIT,
                left: (2, 0.0),
                right: (1, 0.0),
            },
            Floor {
                position: [-300.0, 340.0],
                elevation: 2010.0,
                left: (2, 0.34),
                right: (1, 0.38),
            },
            Floor {
                position: [-600.0, 730.0],
                elevation: 1800.0,
                left: (2, 0.66),
                right: (1, 0.72),
            },
            Floor {
                position: [-880.0, 1160.0],
                elevation: 1590.0,
                left: (2, 0.94),
                right: (1, 1.0),
            },
            Floor {
                position: [-1130.0, 1630.0],
                elevation: GROUND,
                left: (2, 1.0),
                right: (1, 1.0),
            },
        ],
    },
    ValleyLine {
        name: "west_of_west",
        nodes: &[
            Floor {
                position: [0.0, 0.0],
                elevation: SUMMIT,
                left: (3, 0.0),
                right: (2, 0.0),
            },
            Floor {
                position: [-190.0, -200.0],
                elevation: 2020.0,
                left: (3, 0.30),
                right: (2, 0.30),
            },
            Floor {
                position: [-400.0, -450.0],
                elevation: 1810.0,
                left: (3, 0.58),
                right: (2, 0.58),
            },
            Floor {
                position: [-620.0, -740.0],
                elevation: 1600.0,
                left: (3, 0.86),
                right: (2, 0.86),
            },
            Floor {
                position: [-860.0, -1080.0],
                elevation: GROUND,
                left: (3, 1.0),
                right: (2, 1.0),
            },
        ],
    },
    ValleyLine {
        name: "south_of_south",
        nodes: &[
            Floor {
                position: [0.0, 0.0],
                elevation: SUMMIT,
                left: (0, 0.0),
                right: (3, 0.0),
            },
            Floor {
                position: [242.0, -186.0],
                elevation: 2040.0,
                left: (0, 0.28),
                right: (3, 0.34),
            },
            Floor {
                position: [500.0, -400.0],
                elevation: 1830.0,
                left: (0, 0.55),
                right: (3, 0.64),
            },
            Floor {
                position: [790.0, -640.0],
                elevation: 1610.0,
                left: (0, 0.84),
                right: (3, 0.92),
            },
            Floor {
                position: [1100.0, -890.0],
                elevation: GROUND,
                left: (0, 1.0),
                right: (3, 1.0),
            },
        ],
    },
];

/// The rim, in walk order round the massif.
///
/// **Written in the order it is walked**, one ridge tip and one valley mouth at a time. It is not
/// sorted by bearing afterwards: the ridge tips and the mouths are the same points either way and it is
/// the *order* that decides whether the outline is a polygon or a bow tie, and the order is the order
/// the land goes round in.
///
/// Each mouth stands **outside the chord between the two tips it lies between**, which is why the four
/// mouths sit further out than the tips they are beside. A mouth on the chord folds the rim back across
/// the ground it was meant to enclose, and the massif's foot is then inside its own outline.
const RIM: &[(usize, f32)] = &[
    (0, 1.0), // east tip
    (0, 1.0), // east_of_east mouth
    (1, 1.0), // north tip
    (1, 1.0), // north_of_north mouth
    (2, 1.0), // west tip
    (2, 1.0), // west_of_west mouth
    (3, 1.0), // south tip
    (3, 1.0), // south_of_south mouth
];

/// Builds the authored massif's skeleton.
pub fn skeleton() -> Skeleton {
    let mut skeleton = Skeleton::new(ridge_lines(), valley_lines(), Vec::new());
    name_junctions(&mut skeleton);
    skeleton.footprint = rim(&skeleton);
    skeleton
}

/// The ridges, as the crate's lines.
fn ridge_lines() -> Vec<Ridge> {
    RIDGES
        .iter()
        .map(|written| {
            let nodes = written
                .nodes
                .iter()
                .map(|node| {
                    RidgeNode::new([node[0], node[1]], node[2]).with_sharpness(written.sharpness)
                })
                .collect();
            Ridge::root(nodes)
        })
        .collect()
}

/// The valleys, as the crate's lines, with every node's correspondence written into it.
fn valley_lines() -> Vec<Valley> {
    VALLEYS
        .iter()
        .map(|written| {
            let nodes = written
                .nodes
                .iter()
                .map(|floor| {
                    ValleyNode::new(
                        floor.position,
                        floor.elevation,
                        Station::ridge(RidgeId(floor.left.0), floor.left.1),
                        Station::ridge(RidgeId(floor.right.0), floor.right.1),
                    )
                })
                .collect();
            Valley::new(nodes)
        })
        .collect()
}

/// Writes every junction as a name.
///
/// # Invariants
///
/// A junction is one crest's end standing on another's, and the crate states that as an **end naming a
/// station** rather than as two points that happen to coincide: coincidence is what the expansion welds
/// on, but it is the *naming* that says a junction is a junction, and it is what stops a levelling pass
/// taking a col for a ridge tip. So the summit, where all four ridges meet, is written as the other
/// three naming the first, and it does not matter which one owns the point.
///
/// The rule is one line long because the author placed every junction deliberately: whichever crest
/// comes first in the table owns a point, and any crest after it whose end lands there names it.
fn name_junctions(skeleton: &mut Skeleton) {
    let ends: Vec<[([f32; 2], f32); 2]> = skeleton
        .ridges
        .iter()
        .map(|ridge| {
            let first = ridge.nodes[0];
            let last = ridge.nodes[ridge.nodes.len() - 1];
            [(first.position, 0.0), (last.position, 1.0)]
        })
        .collect();
    for index in 0..skeleton.ridges.len() {
        for far in [false, true] {
            let (at, _) = ends[index][usize::from(far)];
            // The earliest crest whose *end* is at this point owns it.
            let owner =
                (0..index).find(|earlier| ends[*earlier].iter().any(|(their, _)| near(*their, at)));
            if let Some(owner) = owner {
                let parameter = if near(ends[owner][1].0, at) { 1.0 } else { 0.0 };
                skeleton.set_end(
                    LineRef::Ridge(RidgeId(index)),
                    far,
                    End::On(Station::ridge(RidgeId(owner), parameter)),
                );
            }
        }
    }
}

/// The rim, as the crate's footprint: four ridge tips and four valley mouths, in walk order.
///
/// A jaw of the rim is a ridge tip **and** the mouth of the valley beside it, and the two are written
/// here in the order they go round, so the outline closes on itself once. The elevations come from the
/// lines the points already stand on, so the rim cannot disagree with the ground it meets — a rim node
/// given its own height would be a second copy of a number the ridge or the valley already carries, and
/// the copy is the one that goes stale.
fn rim(skeleton: &Skeleton) -> Vec<FootprintNode> {
    let mut outline: Vec<FootprintNode> = Vec::new();
    let mut ridge_tip = 0;
    let mut valley_mouth = 0;
    for (index, _) in RIM.iter().enumerate() {
        if index % 2 == 0 {
            let ridge = &skeleton.ridges[ridge_tip];
            let last = ridge.nodes[ridge.nodes.len() - 1];
            outline.push(FootprintNode::new(last.position, last.elevation));
            ridge_tip += 1;
        } else {
            let valley = &skeleton.valleys[valley_mouth];
            let last = valley.nodes[valley.nodes.len() - 1];
            outline.push(FootprintNode::new(last.position, last.elevation));
            valley_mouth += 1;
        }
    }
    outline
}

/// Whether two plan points are the same place.
fn near(one: [f32; 2], other: [f32; 2]) -> bool {
    (one[0] - other[0]).abs() < 1.0e-3 && (one[1] - other[1]).abs() < 1.0e-3
}
