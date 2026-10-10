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
//! # The shape: a tree, and how one is written
//!
//! The reference is a dissected upland: **one main divide with spurs hanging off it**, and a valley
//! cut in between every neighbouring pair. That is a tree, and three rules make one writable here:
//!
//! - **A crest is written peak-first and is split at every col and every junction.** The rule the
//!   crate checks is that nothing rises as it runs away from a summit, so a divide that undulates is
//!   a *chain* of segments: each descends from its peak (a junction, or the summit) into the col
//!   beside it. The divide above is `w0` (summit into `cW1`) and `w1` (`J_W1` into the same col) —
//!   two crests ending at one point, which is what a col is.
//! - **A junction is a segment end.** A belt changes line only at a line's end, so a spur born at
//!   `J_W1` names the crest that *ends* there: `w1` starts at `J_W1`, and the spurs name it. Every
//!   split in this file is at a point where four lines meet and one of them owns the vertex.
//! - **A valley's wall is a chain, and the chain turns at the junctions.** The valley below the col
//!   `cW2` has one wall walking `w2` back up to `J_W1` and then out along `nW1`; the belt where the
//!   wall changes line walks through the junction, because the two segments end there. The
//!   correspondence names the crests; the expansion does the walking.
//!
//! The valley floors descend from their heads — a col, a junction, or the summit — to the rim, and
//! every elevation in this file is written. Nothing is levelled afterwards: a pass that took the
//! foot down to the ground would have to know which ends are the massif's edge and which are cols,
//! and the author knows that and the pass does not.
//!
//! # The size
//!
//! One divide, six spurs and one spur off a spur — three levels of branching, sixteen crests and
//! nine valleys. It is small enough that the whole correspondence is this one table and large
//! enough that a wall has to chain and a valley has to head at a col rather than at a summit.

use mountain::line::{LineRef, Station};
use mountain::skeleton::{
    End, FootprintNode, Ridge, RidgeId, RidgeNode, Skeleton, Valley, ValleyNode,
};

/// The elevation the country around the massif stands at.
const GROUND: f32 = super::super::RIM as f32;

/// The summit, at the head of the divide.
const SUMMIT: f32 = 2400.0;

/// A ridge's nodes, from where it starts to where it ends.
///
/// Written as triples so the tables below read as the polylines they are: a plan position and an
/// elevation, in the order the crest runs — peak first, falling away.
type Written = &'static [[f32; 3]];

/// One authored crest: its line and how sharp its edge is.
struct RidgeLine {
    nodes: Written,
    sharpness: f32,
}

/// The crests of the tree, in the order the junction naming reads them.
///
/// Indices are named in the comments because the correspondence below is written in them:
///
/// ```text
///  0 w0     summit  -> cW1          8  nW1   J_W1  -> north-west tip
///  1 w1     J_W1    -> cW1          9  sW1   J_W1  -> south-west tip
///  2 w2     J_W1    -> cW2         10  nW2   J_W2  -> north-west tip
///  3 w3     J_W2    -> cW2         11  sW2   J_W2  -> south-west tip
///  4 w4     J_W2    -> west tip    12  nE1a  J_E1  -> M
///  5 e0     summit  -> cE1         13  nE1b  M     -> north-east tip
///  6 e1     J_E1    -> cE1         14  nE1c  M     -> the spur's own spur, east
///  7 e2     J_E1    -> east tip    15  sE1   J_E1  -> south-east tip
/// ```
///
/// The divide undulates like a real one: `cW1` and `cW2` are cols at `2235` and `2140` between the
/// shoulders `J_W1` and `J_W2`, and the peaks carry the spurs. Each segment falls from its peak, so
/// every crest in the chain runs away from the summit without ever rising.
const RIDGES: &[RidgeLine] = &[
    // 0 w0: the summit down into the first col.
    RidgeLine {
        nodes: &[
            [0.0, 0.0, SUMMIT],
            [-170.0, 60.0, 2325.0],
            [-330.0, 95.0, 2235.0],
        ],
        sharpness: 0.85,
    },
    // 1 w1: the first shoulder back down into the same col.
    RidgeLine {
        nodes: &[
            [-680.0, 35.0, 2330.0],
            [-510.0, 65.0, 2285.0],
            [-330.0, 95.0, 2235.0],
        ],
        sharpness: 0.80,
    },
    // 2 w2: the first shoulder down into the second col.
    RidgeLine {
        nodes: &[
            [-680.0, 35.0, 2330.0],
            [-835.0, -5.0, 2235.0],
            [-985.0, -45.0, 2140.0],
        ],
        sharpness: 0.85,
    },
    // 3 w3: the second shoulder back down into that col.
    RidgeLine {
        nodes: &[
            [-1235.0, -95.0, 2235.0],
            [-1110.0, -70.0, 2188.0],
            [-985.0, -45.0, 2140.0],
        ],
        sharpness: 0.80,
    },
    // 4 w4: the second shoulder out to the west tip.
    RidgeLine {
        nodes: &[
            [-1235.0, -95.0, 2235.0],
            [-1310.0, -115.0, 1985.0],
            [-1385.0, -132.0, 1720.0],
            [-1470.0, -150.0, GROUND],
        ],
        sharpness: 0.85,
    },
    // 5 e0: the summit down into the east col.
    RidgeLine {
        nodes: &[
            [0.0, 0.0, SUMMIT],
            [165.0, -42.0, 2325.0],
            [330.0, -85.0, 2245.0],
        ],
        sharpness: 0.85,
    },
    // 6 e1: the east shoulder back down into that col.
    RidgeLine {
        nodes: &[
            [695.0, -20.0, 2340.0],
            [512.0, -52.0, 2292.0],
            [330.0, -85.0, 2245.0],
        ],
        sharpness: 0.80,
    },
    // 7 e2: the east shoulder out to the east tip.
    RidgeLine {
        nodes: &[
            [695.0, -20.0, 2340.0],
            [930.0, 12.0, 2185.0],
            [1165.0, 52.0, 1950.0],
            [1395.0, 95.0, GROUND],
        ],
        sharpness: 0.85,
    },
    // 8 nW1: a spur off the first shoulder, north-west.
    RidgeLine {
        nodes: &[
            [-680.0, 35.0, 2330.0],
            [-762.0, 212.0, 2130.0],
            [-862.0, 392.0, 1870.0],
            [-985.0, 585.0, GROUND],
        ],
        sharpness: 0.70,
    },
    // 9 sW1: the same shoulder's other side, south-west.
    RidgeLine {
        nodes: &[
            [-680.0, 35.0, 2330.0],
            [-742.0, -185.0, 2140.0],
            [-828.0, -402.0, 1880.0],
            [-920.0, -620.0, GROUND],
        ],
        sharpness: 0.70,
    },
    // 10 nW2: a spur off the second shoulder, north.
    RidgeLine {
        nodes: &[
            [-1235.0, -95.0, 2235.0],
            [-1288.0, 32.0, 2085.0],
            [-1352.0, 172.0, 1835.0],
            [-1435.0, 320.0, GROUND],
        ],
        sharpness: 0.70,
    },
    // 11 sW2: that shoulder's other side, south-west — short, because the west tip is near.
    RidgeLine {
        nodes: &[
            [-1235.0, -95.0, 2235.0],
            [-1288.0, -170.0, 2075.0],
            [-1342.0, -248.0, 1820.0],
            [-1395.0, -330.0, GROUND],
        ],
        sharpness: 0.70,
    },
    // 12 nE1a: a spur off the east shoulder, up to the peak where it splits.
    RidgeLine {
        nodes: &[
            [695.0, -20.0, 2340.0],
            [760.0, 118.0, 2185.0],
            [812.0, 232.0, 2055.0],
            [865.0, 330.0, 1950.0],
        ],
        sharpness: 0.70,
    },
    // 13 nE1b: the split's north branch.
    RidgeLine {
        nodes: &[
            [865.0, 330.0, 1950.0],
            [918.0, 468.0, 1785.0],
            [968.0, 610.0, 1590.0],
            [1010.0, 745.0, GROUND],
        ],
        sharpness: 0.70,
    },
    // 14 nE1c: the split's own spur — the tree's third level — running east.
    RidgeLine {
        nodes: &[
            [865.0, 330.0, 1950.0],
            [1025.0, 348.0, 1832.0],
            [1185.0, 375.0, 1645.0],
            [1345.0, 415.0, GROUND],
        ],
        sharpness: 0.70,
    },
    // 15 sE1: the east shoulder's south side.
    RidgeLine {
        nodes: &[
            [695.0, -20.0, 2340.0],
            [832.0, -182.0, 2130.0],
            [1008.0, -372.0, 1830.0],
            [1185.0, -560.0, GROUND],
        ],
        sharpness: 0.70,
    },
];

/// One authored valley node: where the floor stands, how high, and **what it faces**.
///
/// The two pairs are stations on the crests — which crest, and how far along it — so a node says
/// "I sit between the east shoulder a third of the way out and the spur's north branch half way
/// up". That sentence is the mesh's topology; there is nothing else to it.
///
/// A wall that **chains** — one crest then another — is written by the stations changing crest at
/// the junction, and the belt between those two nodes walks through it. The parameter on a chained
/// wall therefore runs backwards where the chain does: the valley below `cW2` walks `w2` from its
/// far end back to the junction, so its stations fall from `1.0` to `0.0`.
struct Floor {
    position: [f32; 2],
    elevation: f32,
    /// The station this node faces on its left, read down the valley.
    left: (usize, f32),
    /// The same for the right.
    right: (usize, f32),
}

/// One authored valley: its nodes, head first.
struct ValleyLine {
    nodes: &'static [Floor],
}

/// The nine valleys — one per pocket of the tree — in the order the rim walks them.
///
/// Each heads where its two walls meet: a **col** (`cW2` heads two of them), a **junction** (`J_W1`
/// heads the one between the divide's west arm and its own spur), or the **summit** (the two long
/// ones, north and south). The head names its own point on both sides, which is what makes the
/// first belt collapse to a triangle and the head close.
///
/// The walls are named going **counterclockwise** round the valley's own direction, so `left` is
/// the crest on the higher bearing. A valley written the other way round would be wound backwards
/// and the facets beside it would face into the hill.
const VALLEYS: &[ValleyLine] = &[
    // 0 east_rim: the pocket between the east tip and the spur's own spur.
    //   head at J_E1, walls nE1a+nE1c (left) and e2 (right).
    ValleyLine {
        nodes: &[
            Floor {
                position: [695.0, -20.0],
                elevation: 2340.0,
                left: (12, 0.0),
                right: (7, 0.0),
            },
            Floor {
                position: [955.0, 40.0],
                elevation: 2140.0,
                left: (12, 0.35),
                right: (7, 0.32),
            },
            Floor {
                position: [1215.0, 130.0],
                elevation: 1870.0,
                left: (14, 0.30),
                right: (7, 0.62),
            },
            Floor {
                position: [1465.0, 225.0],
                elevation: 1650.0,
                left: (14, 0.68),
                right: (7, 0.88),
            },
            Floor {
                position: [1620.0, 290.0],
                elevation: GROUND,
                left: (14, 1.0),
                right: (7, 1.0),
            },
        ],
    },
    // 1 sub_east: the pocket the third level cuts — between the spur's two branches.
    //   head at the split M, walls nE1b (left) and nE1c (right).
    ValleyLine {
        nodes: &[
            Floor {
                position: [865.0, 330.0],
                elevation: 1950.0,
                left: (13, 0.0),
                right: (14, 0.0),
            },
            Floor {
                position: [1035.0, 428.0],
                elevation: 1830.0,
                left: (13, 0.32),
                right: (14, 0.34),
            },
            Floor {
                position: [1215.0, 535.0],
                elevation: 1690.0,
                left: (13, 0.64),
                right: (14, 0.66),
            },
            Floor {
                position: [1390.0, 690.0],
                elevation: GROUND,
                left: (13, 1.0),
                right: (14, 1.0),
            },
        ],
    },
    // 2 north: the long one — from the summit, between the two north spurs' chains.
    //   head at the summit, walls w0+w1+nW1 (left) and e0+e1+nE1a+nE1b (right).
    ValleyLine {
        nodes: &[
            Floor {
                position: [0.0, 0.0],
                elevation: SUMMIT,
                left: (0, 0.0),
                right: (5, 0.0),
            },
            Floor {
                position: [-35.0, 210.0],
                elevation: 2180.0,
                left: (0, 0.5),
                right: (5, 0.5),
            },
            Floor {
                position: [-20.0, 450.0],
                elevation: 1950.0,
                left: (1, 0.6),
                right: (6, 0.55),
            },
            Floor {
                position: [0.0, 690.0],
                elevation: 1730.0,
                left: (1, 0.25),
                right: (12, 0.55),
            },
            Floor {
                position: [8.0, 920.0],
                elevation: 1580.0,
                left: (8, 0.35),
                right: (13, 0.45),
            },
            Floor {
                position: [20.0, 1140.0],
                elevation: GROUND,
                left: (8, 1.0),
                right: (13, 1.0),
            },
        ],
    },
    // 3 nw_col: the valley between the two north spurs, headed at the col cW2.
    //   walls w3+nW2 (left) and w2+nW1 (right) — both chains turn at their junction.
    ValleyLine {
        nodes: &[
            Floor {
                position: [-985.0, -45.0],
                elevation: 2140.0,
                left: (3, 1.0),
                right: (2, 1.0),
            },
            Floor {
                position: [-1105.0, 115.0],
                elevation: 1990.0,
                left: (3, 0.5),
                right: (2, 0.55),
            },
            Floor {
                position: [-1215.0, 285.0],
                elevation: 1830.0,
                left: (10, 0.35),
                right: (8, 0.3),
            },
            Floor {
                position: [-1310.0, 435.0],
                elevation: 1650.0,
                left: (10, 0.7),
                right: (8, 0.68),
            },
            Floor {
                position: [-1395.0, 545.0],
                elevation: GROUND,
                left: (10, 1.0),
                right: (8, 1.0),
            },
        ],
    },
    // 4 w_north: between the divide's west arm and the second shoulder's north spur.
    //   head at J_W2, walls w4 (left) and nW2 (right).
    ValleyLine {
        nodes: &[
            Floor {
                position: [-1235.0, -95.0],
                elevation: 2235.0,
                left: (4, 0.0),
                right: (10, 0.0),
            },
            Floor {
                position: [-1395.0, -48.0],
                elevation: 2020.0,
                left: (4, 0.35),
                right: (10, 0.35),
            },
            Floor {
                position: [-1555.0, 0.0],
                elevation: 1790.0,
                left: (4, 0.7),
                right: (10, 0.72),
            },
            Floor {
                position: [-1682.0, 99.0],
                elevation: GROUND,
                left: (4, 1.0),
                right: (10, 1.0),
            },
        ],
    },
    // 5 w_south: the same shoulder's other pocket — the west arm and sW2.
    //   head at J_W2, walls sW2 (left) and w4 (right).
    ValleyLine {
        nodes: &[
            Floor {
                position: [-1235.0, -95.0],
                elevation: 2235.0,
                left: (11, 0.0),
                right: (4, 0.0),
            },
            Floor {
                position: [-1370.0, -155.0],
                elevation: 2010.0,
                left: (11, 0.35),
                right: (4, 0.35),
            },
            Floor {
                position: [-1520.0, -222.0],
                elevation: 1760.0,
                left: (11, 0.7),
                right: (4, 0.7),
            },
            Floor {
                position: [-1680.0, -282.0],
                elevation: GROUND,
                left: (11, 1.0),
                right: (4, 1.0),
            },
        ],
    },
    // 6 sw_col: the valley between the two south spurs, headed at the same col cW2.
    //   walls w2+sW1 (left) and w3+sW2 (right).
    ValleyLine {
        nodes: &[
            Floor {
                position: [-985.0, -45.0],
                elevation: 2140.0,
                left: (2, 1.0),
                right: (3, 1.0),
            },
            Floor {
                position: [-1075.0, -195.0],
                elevation: 1975.0,
                left: (2, 0.55),
                right: (3, 0.5),
            },
            Floor {
                position: [-1165.0, -345.0],
                elevation: 1810.0,
                left: (9, 0.3),
                right: (11, 0.35),
            },
            Floor {
                position: [-1250.0, -475.0],
                elevation: 1640.0,
                left: (9, 0.68),
                right: (11, 0.7),
            },
            Floor {
                position: [-1320.0, -580.0],
                elevation: GROUND,
                left: (9, 1.0),
                right: (11, 1.0),
            },
        ],
    },
    // 7 south: the long one's twin, from the summit down the south side.
    //   head at the summit, walls e0+e1+sE1 (left) and w0+w1+sW1 (right).
    ValleyLine {
        nodes: &[
            Floor {
                position: [0.0, 0.0],
                elevation: SUMMIT,
                left: (5, 0.0),
                right: (0, 0.0),
            },
            Floor {
                position: [35.0, -215.0],
                elevation: 2180.0,
                left: (5, 0.5),
                right: (0, 0.5),
            },
            Floor {
                position: [60.0, -450.0],
                elevation: 1950.0,
                left: (6, 0.55),
                right: (1, 0.55),
            },
            Floor {
                position: [95.0, -690.0],
                elevation: 1730.0,
                left: (15, 0.5),
                right: (9, 0.5),
            },
            Floor {
                position: [150.0, -915.0],
                elevation: 1580.0,
                left: (15, 0.78),
                right: (9, 0.78),
            },
            Floor {
                position: [202.0, -1130.0],
                elevation: GROUND,
                left: (15, 1.0),
                right: (9, 1.0),
            },
        ],
    },
    // 8 se_rim: the pocket between the south-east tip and the east tip.
    //   head at J_E1, walls e2 (left) and sE1 (right).
    ValleyLine {
        nodes: &[
            Floor {
                position: [695.0, -20.0],
                elevation: 2340.0,
                left: (7, 0.0),
                right: (15, 0.0),
            },
            Floor {
                position: [985.0, -115.0],
                elevation: 2130.0,
                left: (7, 0.33),
                right: (15, 0.33),
            },
            Floor {
                position: [1290.0, -208.0],
                elevation: 1850.0,
                left: (7, 0.68),
                right: (15, 0.68),
            },
            Floor {
                position: [1584.0, -286.0],
                elevation: GROUND,
                left: (7, 1.0),
                right: (15, 1.0),
            },
        ],
    },
];

/// The ridge tips of the rim, in the order the rim is walked.
///
/// Counterclockwise, which is the winding the mesh wants: the inside is on the left of the walk.
/// The walk alternates a tip and the mouth of the pocket between it and the next tip, so this list
/// and [`RIM_MOUTHS`] are two halves of one round trip.
const RIM_TIPS: &[usize] = &[7, 14, 13, 8, 10, 4, 11, 9, 15];

/// The valley mouths of the rim, in the same walk order: each is the pocket between the tip above
/// it and the tip below it in [`RIM_TIPS`].
const RIM_MOUTHS: &[usize] = &[0, 1, 2, 3, 4, 5, 6, 7, 8];

/// Builds the authored massif's skeleton.
pub fn skeleton() -> Skeleton {
    let mut skeleton = Skeleton::new(ridge_lines(), valley_lines(), Vec::new());
    name_junctions(&mut skeleton);
    skeleton.footprint = rim(&skeleton);
    skeleton
}

/// The crests, as the crate's lines.
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
/// A junction is one crest's end standing on another's, and the crate states that as an **end naming
/// a station** rather than as two points that happen to coincide: coincidence is what the expansion
/// welds on, but it is the *naming* that says a junction is a junction — and on a tree it is what
/// makes the junction walkable, because a belt changes line only at a line's end. So the col `cW2`,
/// where `w2` and `w3` both end, is written as `w3` naming `w2`, and the four lines of `J_W1` all
/// name the crest that starts there.
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

/// The rim, as the crate's footprint: every ridge tip and every valley mouth, in walk order.
///
/// The elevations come from the lines the points already stand on, so the rim cannot disagree with
/// the ground it meets — a rim node given its own height would be a second copy of a number the
/// crest or the valley already carries, and the copy is the one that goes stale. Each mouth was
/// placed outside the chord between the two tips beside it, which is what keeps the outline a
/// polygon instead of a bow tie.
fn rim(skeleton: &Skeleton) -> Vec<FootprintNode> {
    let mut outline: Vec<FootprintNode> = Vec::new();
    for (tip, mouth) in RIM_TIPS.iter().zip(RIM_MOUTHS) {
        let ridge = &skeleton.ridges[*tip];
        let last = ridge.nodes[ridge.nodes.len() - 1];
        outline.push(FootprintNode::new(last.position, last.elevation));
        let valley = &skeleton.valleys[*mouth];
        let floor = valley.nodes[valley.nodes.len() - 1];
        outline.push(FootprintNode::new(floor.position, floor.elevation));
    }
    outline
}

/// Whether two plan points are the same place.
fn near(one: [f32; 2], other: [f32; 2]) -> bool {
    (one[0] - other[0]).abs() < 1.0e-3 && (one[1] - other[1]).abs() < 1.0e-3
}
