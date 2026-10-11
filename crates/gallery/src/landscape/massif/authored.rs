//! The massif, written down.
//!
//! # What is authored and what is generated
//!
//! **The control structure is authored**: the ridge polylines, the valley polylines, the
//! correspondence from every valley node to the two ridge nodes it faces, and the rim. That is what
//! this file is. **The surface is generated**: the expansion walks each pair of consecutive valley
//! nodes as a belt bounded by two known stations and emits the facets between them. There is no
//! triangulation, no search, and — this is the point — **no derivation of the valleys from the
//! ridges**.
//!
//! # The shape: a dissected upland, and how one is written
//!
//! The reference is a dissected upland: **one long divide with spurs branching off it at several
//! scales**, and a valley cut in between every neighbouring pair. Three rules make one writable:
//!
//! - **A crest is written peak-first and is split at every col and every junction.** Nothing may
//!   rise as it runs away from a summit, so a divide that undulates is a *chain* of segments, each
//!   falling from its peak — a junction, or the summit — into the col beside it.
//! - **A junction is a segment end.** A belt changes line only at a line's end, so a spur born at a
//!   shoulder names the crest that ends or starts there, and [`name_junctions`] writes the naming.
//! - **A valley's wall is a chain, and the chain turns at the junctions.** The wall below the col
//!   `cW2` walks `w2` back up to `J_W1` and then out along `nW1a`; the belt that straddles the
//!   junction walks through it, because the two segments end there.
//!
//! A pocket of the tree is the wedge between two neighbouring ridge tips; its valley heads at the
//! point where its two wall chains converge — a col, a spur's own junction, or the summit — and its
//! floor is written between the walls down to the rim.
//!
//! # The stations are node indices
//!
//! A valley node faces a crest **node**, not a fraction of a crest: the tables name a
//! `(ridge, node)` pair and [`station`] turns it into the parameter the crate reads, which is that
//! node's own arc-length share. A round number written by hand lands a hair off the node it meant,
//! and a station a hair off its node is a sliver facet.
//!
//! # The size
//!
//! One divide of twelve segments and twenty spurs at three levels of branching — thirty-two crests
//! and seventeen valleys. The plan reads as one main divide with its tributaries: every pocket
//! between two neighbouring tips reaches a head that is a col, a junction, or the summit.

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
/// ```text
///  0 w0      summit -> cW1          16 nW2a   J_W2 -> M_nW2
///  1 w1      J_W1   -> cW1          17 nW2b   M_nW2 -> north-west tip
///  2 w2      J_W1   -> cW2          18 sub_nW2 M_nW2 -> its own spur, west
///  3 w3      J_W2   -> cW2          19 sW2    J_W2 -> south-west tip
///  4 w4      J_W2   -> cW3          20 nW3    J_W3 -> north-west tip
///  5 w5      J_W3   -> cW3          21 sW3    J_W3 -> south-west tip
///  6 w6      J_W3   -> west tip     22 nE1a   J_E1 -> M_nE1
///  7 e0      summit -> cE1          23 nE1b   M_nE1 -> north-east tip
///  8 e1      J_E1   -> cE1          24 sub_nE1 M_nE1 -> its own spur, north
///  9 e2      J_E1   -> cE2          25 sE1a   J_E1 -> M_sE1
/// 10 e3      J_E2   -> cE2          26 sE1b   M_sE1 -> south-east tip
/// 11 e4      J_E2   -> east tip     27 sub_sE1 M_sE1 -> its own spur, south
/// 12 nW1a    J_W1   -> M_nW1        28 nE2a   J_E2 -> M_nE2
/// 13 nW1b    M_nW1  -> north-west tip  29 nE2b M_nE2 -> north-east tip
/// 14 sub_nW1 M_nW1  -> its own spur 30 sub_nE2 M_nE2 -> its own spur, east
/// 15 sW1     J_W1   -> south-west tip  31 sE2  J_E2 -> south-east tip
/// ```
const RIDGES: &[RidgeLine] = &[
    // 0 w0: the summit down into the first col.
    RidgeLine {
        nodes: &[
            [0.0, 0.0, SUMMIT],
            [-135.0, 30.0, 2325.0],
            [-265.0, 65.0, 2245.0],
        ],
        sharpness: 0.85,
    },
    // 1 w1: the first shoulder back down into the same col.
    RidgeLine {
        nodes: &[
            [-535.0, 35.0, 2335.0],
            [-400.0, 50.0, 2290.0],
            [-265.0, 65.0, 2245.0],
        ],
        sharpness: 0.80,
    },
    // 2 w2: the first shoulder down into the second col.
    RidgeLine {
        nodes: &[
            [-535.0, 35.0, 2335.0],
            [-665.0, 10.0, 2250.0],
            [-795.0, -15.0, 2165.0],
        ],
        sharpness: 0.85,
    },
    // 3 w3: the second shoulder back down into that col.
    RidgeLine {
        nodes: &[
            [-1035.0, -70.0, 2265.0],
            [-915.0, -42.0, 2215.0],
            [-795.0, -15.0, 2165.0],
        ],
        sharpness: 0.80,
    },
    // 4 w4: the second shoulder down into the third col.
    RidgeLine {
        nodes: &[
            [-1035.0, -70.0, 2265.0],
            [-1150.0, -98.0, 2175.0],
            [-1265.0, -125.0, 2085.0],
        ],
        sharpness: 0.85,
    },
    // 5 w5: the third shoulder back down into that col.
    RidgeLine {
        nodes: &[
            [-1425.0, -165.0, 2185.0],
            [-1345.0, -145.0, 2135.0],
            [-1265.0, -125.0, 2085.0],
        ],
        sharpness: 0.80,
    },
    // 6 w6: the third shoulder out to the west tip.
    RidgeLine {
        nodes: &[
            [-1425.0, -165.0, 2185.0],
            [-1490.0, -182.0, 1945.0],
            [-1555.0, -198.0, 1700.0],
            [-1620.0, -215.0, GROUND],
        ],
        sharpness: 0.85,
    },
    // 7 e0: the summit down into the east col.
    RidgeLine {
        nodes: &[
            [0.0, 0.0, SUMMIT],
            [135.0, -28.0, 2330.0],
            [270.0, -55.0, 2255.0],
        ],
        sharpness: 0.85,
    },
    // 8 e1: the east shoulder back down into that col.
    RidgeLine {
        nodes: &[
            [550.0, -20.0, 2345.0],
            [410.0, -38.0, 2300.0],
            [270.0, -55.0, 2255.0],
        ],
        sharpness: 0.80,
    },
    // 9 e2: the east shoulder down into the second east col.
    RidgeLine {
        nodes: &[
            [550.0, -20.0, 2345.0],
            [680.0, 5.0, 2265.0],
            [810.0, 30.0, 2185.0],
        ],
        sharpness: 0.85,
    },
    // 10 e3: the second east shoulder back down into that col.
    RidgeLine {
        nodes: &[
            [1065.0, 85.0, 2275.0],
            [938.0, 58.0, 2230.0],
            [810.0, 30.0, 2185.0],
        ],
        sharpness: 0.80,
    },
    // 11 e4: the second east shoulder out to the east tip.
    RidgeLine {
        nodes: &[
            [1065.0, 85.0, 2275.0],
            [1190.0, 112.0, 2015.0],
            [1312.0, 138.0, 1735.0],
            [1435.0, 165.0, GROUND],
        ],
        sharpness: 0.85,
    },
    // 12 nW1a: a spur off the first shoulder, up to the peak where it splits.
    RidgeLine {
        nodes: &[[-535.0, 35.0, 2335.0], [-612.0, 188.0, 2160.0]],
        sharpness: 0.70,
    },
    // 13 nW1b: the split's north branch.
    RidgeLine {
        nodes: &[
            [-612.0, 188.0, 2160.0],
            [-695.0, 352.0, 1930.0],
            [-785.0, 528.0, GROUND],
        ],
        sharpness: 0.70,
    },
    // 14 sub_nW1: the split's own spur — the tree's third level.
    RidgeLine {
        nodes: &[
            [-612.0, 188.0, 2160.0],
            [-455.0, 335.0, 1985.0],
            [-325.0, 475.0, 1780.0],
            [-215.0, 595.0, GROUND],
        ],
        sharpness: 0.70,
    },
    // 15 sW1: the first shoulder's other side, south-west.
    RidgeLine {
        nodes: &[
            [-535.0, 35.0, 2335.0],
            [-598.0, -135.0, 2155.0],
            [-668.0, -310.0, 1925.0],
            [-745.0, -495.0, GROUND],
        ],
        sharpness: 0.70,
    },
    // 16 nW2a: a spur off the second shoulder, up to its split.
    RidgeLine {
        nodes: &[[-1035.0, -70.0, 2265.0], [-1092.0, 60.0, 2105.0]],
        sharpness: 0.70,
    },
    // 17 nW2b: the split's north branch.
    RidgeLine {
        nodes: &[
            [-1092.0, 60.0, 2105.0],
            [-1152.0, 205.0, 1890.0],
            [-1215.0, 355.0, GROUND],
        ],
        sharpness: 0.70,
    },
    // 18 sub_nW2: the split's own spur.
    RidgeLine {
        nodes: &[
            [-1092.0, 60.0, 2105.0],
            [-1010.0, 180.0, 1935.0],
            [-950.0, 300.0, 1735.0],
            [-905.0, 425.0, GROUND],
        ],
        sharpness: 0.70,
    },
    // 19 sW2: the second shoulder's other side.
    RidgeLine {
        nodes: &[
            [-1035.0, -70.0, 2265.0],
            [-1082.0, -180.0, 2100.0],
            [-1132.0, -300.0, 1880.0],
            [-1185.0, -430.0, GROUND],
        ],
        sharpness: 0.70,
    },
    // 20 nW3: a spur off the third shoulder, north.
    RidgeLine {
        nodes: &[
            [-1425.0, -165.0, 2185.0],
            [-1478.0, -55.0, 2035.0],
            [-1535.0, 65.0, 1830.0],
            [-1595.0, 195.0, GROUND],
        ],
        sharpness: 0.70,
    },
    // 21 sW3: the third shoulder's other side.
    RidgeLine {
        nodes: &[
            [-1425.0, -165.0, 2185.0],
            [-1462.0, -272.0, 2025.0],
            [-1502.0, -390.0, 1810.0],
            [-1545.0, -515.0, GROUND],
        ],
        sharpness: 0.70,
    },
    // 22 nE1a: a spur off the east shoulder, up to the peak where it splits.
    RidgeLine {
        nodes: &[[550.0, -20.0, 2345.0], [618.0, 138.0, 2175.0]],
        sharpness: 0.70,
    },
    // 23 nE1b: the split's north branch.
    RidgeLine {
        nodes: &[
            [618.0, 138.0, 2175.0],
            [692.0, 308.0, 1950.0],
            [775.0, 490.0, GROUND],
        ],
        sharpness: 0.70,
    },
    // 24 sub_nE1: the split's own spur, north.
    RidgeLine {
        nodes: &[
            [618.0, 138.0, 2175.0],
            [795.0, 215.0, 1985.0],
            [955.0, 295.0, 1775.0],
            [1030.0, 500.0, GROUND],
        ],
        sharpness: 0.70,
    },
    // 25 sE1a: the east shoulder's other side, up to its split.
    RidgeLine {
        nodes: &[[550.0, -20.0, 2345.0], [632.0, -172.0, 2165.0]],
        sharpness: 0.70,
    },
    // 26 sE1b: the split's south branch.
    RidgeLine {
        nodes: &[
            [632.0, -172.0, 2165.0],
            [722.0, -338.0, 1935.0],
            [820.0, -515.0, GROUND],
        ],
        sharpness: 0.70,
    },
    // 27 sub_sE1: the split's own spur, south.
    RidgeLine {
        nodes: &[
            [632.0, -172.0, 2165.0],
            [805.0, -255.0, 1975.0],
            [965.0, -345.0, 1775.0],
            [1115.0, -435.0, GROUND],
        ],
        sharpness: 0.70,
    },
    // 28 nE2a: a spur off the second east shoulder, up to its split.
    RidgeLine {
        nodes: &[[1065.0, 85.0, 2275.0], [1122.0, 212.0, 2115.0]],
        sharpness: 0.70,
    },
    // 29 nE2b: the split's north branch.
    RidgeLine {
        nodes: &[
            [1122.0, 212.0, 2115.0],
            [1185.0, 350.0, 1900.0],
            [1255.0, 500.0, GROUND],
        ],
        sharpness: 0.70,
    },
    // 30 sub_nE2: the split's own spur, east.
    RidgeLine {
        nodes: &[
            [1122.0, 212.0, 2115.0],
            [1275.0, 265.0, 1935.0],
            [1415.0, 315.0, 1735.0],
            [1545.0, 365.0, GROUND],
        ],
        sharpness: 0.70,
    },
    // 31 sE2: the second east shoulder's other side.
    RidgeLine {
        nodes: &[
            [1065.0, 85.0, 2275.0],
            [1128.0, -42.0, 2115.0],
            [1198.0, -180.0, 1895.0],
            [1275.0, -330.0, GROUND],
        ],
        sharpness: 0.70,
    },
];

/// One authored valley node: where the floor stands, how high, and **what it faces**.
///
/// Each pair is a crest node — the node this rib climbs to — so a node says "I sit between the
/// east shoulder's middle node on one side and the spur's second node on the other". That sentence
/// is the mesh's topology.
///
/// A wall that **chains** — one crest then another — is written by the node indices changing crest
/// at the junction, and the belt between those two nodes walks through it. The node index on a
/// chained wall therefore runs backwards where the chain does.
struct Floor {
    position: [f32; 2],
    elevation: f32,
    /// The crest node this node faces on its left, read down the valley.
    left: (usize, usize),
    /// The same for the right.
    right: (usize, usize),
}

/// One authored valley: its nodes, head first.
struct ValleyLine {
    nodes: &'static [Floor],
}

/// The seventeen valleys — one per pocket of the tree — in the order the rim walks them.
///
/// Each heads where its two walls converge: a **col**, a spur's own **junction**, or the **summit**.
/// The head names its own point on both sides, which is what makes the first belt row collapse to a
/// triangle and the head close. The mouth stands outside the chord between its two flanking tips,
/// roughly on the bisector of the two walls, so the rim bulges rather than bow-ties.
const VALLEYS: &[ValleyLine] = &[
    // 0 east_rim: between the east tip and the second east spur's own spur.
    //   head M_nE2, walls sub_nE2 (left) and nE2a+e4 (right).
    ValleyLine {
        nodes: &[
            Floor {
                position: [1122.0, 212.0],
                elevation: 2115.0,
                left: (30, 0),
                right: (28, 1),
            },
            Floor {
                position: [1300.0, 235.0],
                elevation: 1930.0,
                left: (30, 1),
                right: (28, 0),
            },
            Floor {
                position: [1520.0, 262.0],
                elevation: 1730.0,
                left: (30, 2),
                right: (11, 1),
            },
            Floor {
                position: [1819.0, 281.0],
                elevation: GROUND,
                left: (30, 3),
                right: (11, 3),
            },
        ],
    },
    // 1 sub_e2: between that spur's own spur and the second east spur's north branch.
    //   head M_nE2, walls nE2b (left) and sub_nE2 (right).
    ValleyLine {
        nodes: &[
            Floor {
                position: [1122.0, 212.0],
                elevation: 2115.0,
                left: (29, 0),
                right: (30, 0),
            },
            Floor {
                position: [1330.0, 395.0],
                elevation: 1870.0,
                left: (29, 1),
                right: (30, 1),
            },
            Floor {
                position: [1578.0, 631.0],
                elevation: GROUND,
                left: (29, 2),
                right: (30, 3),
            },
        ],
    },
    // 2 east_col: between the second east spur's north branch and the first east spur's own spur.
    //   head cE2, walls e2+nE1a+sub_nE1 (left) and e3+nE2a+nE2b (right).
    ValleyLine {
        nodes: &[
            Floor {
                position: [810.0, 30.0],
                elevation: 2185.0,
                left: (9, 2),
                right: (10, 2),
            },
            Floor {
                position: [975.0, 175.0],
                elevation: 2010.0,
                left: (9, 1),
                right: (10, 1),
            },
            Floor {
                position: [1135.0, 330.0],
                elevation: 1830.0,
                left: (22, 1),
                right: (28, 1),
            },
            Floor {
                position: [1185.0, 495.0],
                elevation: 1650.0,
                left: (24, 2),
                right: (29, 1),
            },
            Floor {
                position: [1232.0, 650.0],
                elevation: GROUND,
                left: (24, 3),
                right: (29, 2),
            },
        ],
    },
    // 3 sub_e1: between the first east spur's two branches.
    //   head M_nE1, walls nE1b (left) and sub_nE1 (right).
    ValleyLine {
        nodes: &[
            Floor {
                position: [618.0, 138.0],
                elevation: 2175.0,
                left: (23, 0),
                right: (24, 0),
            },
            Floor {
                position: [795.0, 375.0],
                elevation: 1900.0,
                left: (23, 1),
                right: (24, 1),
            },
            Floor {
                position: [986.0, 637.0],
                elevation: GROUND,
                left: (23, 2),
                right: (24, 3),
            },
        ],
    },
    // 4 north: the long one — from the summit, between the two north spur chains.
    //   head the summit, walls w0+w1+nW1a+sub_nW1 (left) and e0+e1+nE1a+nE1b (right).
    ValleyLine {
        nodes: &[
            Floor {
                position: [0.0, 0.0],
                elevation: SUMMIT,
                left: (0, 0),
                right: (7, 0),
            },
            Floor {
                position: [-40.0, 230.0],
                elevation: 2185.0,
                left: (0, 1),
                right: (7, 1),
            },
            Floor {
                position: [-30.0, 485.0],
                elevation: 1965.0,
                left: (1, 1),
                right: (8, 1),
            },
            Floor {
                position: [250.0, 640.0],
                elevation: 1745.0,
                left: (12, 1),
                right: (22, 1),
            },
            Floor {
                position: [300.0, 900.0],
                elevation: 1590.0,
                left: (14, 1),
                right: (23, 1),
            },
            Floor {
                position: [356.0, 1041.0],
                elevation: GROUND,
                left: (14, 3),
                right: (23, 2),
            },
        ],
    },
    // 5 sub_w1: between the first west spur's two branches.
    //   head M_nW1, walls nW1b (left) and sub_nW1 (right).
    ValleyLine {
        nodes: &[
            Floor {
                position: [-612.0, 188.0],
                elevation: 2160.0,
                left: (13, 0),
                right: (14, 0),
            },
            Floor {
                position: [-585.0, 470.0],
                elevation: 1880.0,
                left: (13, 1),
                right: (14, 1),
            },
            Floor {
                position: [-519.0, 801.0],
                elevation: GROUND,
                left: (13, 2),
                right: (14, 3),
            },
        ],
    },
    // 6 west_col: between the first west spur's north branch and the second west spur's own spur.
    //   head cW2, walls w2+nW1a+nW1b (left) and w3+nW2a+sub_nW2 (right).
    ValleyLine {
        nodes: &[
            Floor {
                position: [-795.0, -15.0],
                elevation: 2165.0,
                left: (2, 2),
                right: (3, 2),
            },
            Floor {
                position: [-775.0, 175.0],
                elevation: 1990.0,
                left: (2, 1),
                right: (3, 1),
            },
            Floor {
                position: [-800.0, 365.0],
                elevation: 1820.0,
                left: (12, 1),
                right: (16, 1),
            },
            Floor {
                position: [-820.0, 470.0],
                elevation: 1650.0,
                left: (13, 1),
                right: (18, 1),
            },
            Floor {
                position: [-860.0, 625.0],
                elevation: GROUND,
                left: (13, 2),
                right: (18, 3),
            },
        ],
    },
    // 7 sub_w2: between the second west spur's two branches.
    //   head M_nW2, walls nW2b (left) and sub_nW2 (right).
    ValleyLine {
        nodes: &[
            Floor {
                position: [-1092.0, 60.0],
                elevation: 2105.0,
                left: (17, 0),
                right: (18, 0),
            },
            Floor {
                position: [-1035.0, 330.0],
                elevation: 1850.0,
                left: (17, 1),
                right: (18, 1),
            },
            Floor {
                position: [-950.0, 602.0],
                elevation: GROUND,
                left: (17, 2),
                right: (18, 3),
            },
        ],
    },
    // 8 west_col_far: between the second west spur's north branch and the third's north spur.
    //   head cW3, walls w5+nW3 (left) and w4+nW2a+nW2b (right).
    ValleyLine {
        nodes: &[
            Floor {
                position: [-1265.0, -125.0],
                elevation: 2085.0,
                left: (5, 2),
                right: (4, 2),
            },
            Floor {
                position: [-1315.0, 75.0],
                elevation: 1920.0,
                left: (5, 1),
                right: (4, 1),
            },
            Floor {
                position: [-1375.0, 275.0],
                elevation: 1760.0,
                left: (20, 1),
                right: (16, 1),
            },
            Floor {
                position: [-1440.0, 415.0],
                elevation: 1620.0,
                left: (20, 2),
                right: (17, 1),
            },
            Floor {
                position: [-1504.0, 533.0],
                elevation: GROUND,
                left: (20, 3),
                right: (17, 2),
            },
        ],
    },
    // 9 west_north: between the third west spur and the west tip.
    //   head J_W3, walls w6 (left) and nW3 (right).
    ValleyLine {
        nodes: &[
            Floor {
                position: [-1425.0, -165.0],
                elevation: 2185.0,
                left: (6, 0),
                right: (20, 0),
            },
            Floor {
                position: [-1640.0, -75.0],
                elevation: 1930.0,
                left: (6, 1),
                right: (20, 1),
            },
            Floor {
                position: [-1878.0, 48.0],
                elevation: GROUND,
                left: (6, 3),
                right: (20, 3),
            },
        ],
    },
    // 10 west_south: the same shoulder's other pocket — the west tip and sW3.
    //   head J_W3, walls sW3 (left) and w6 (right).
    ValleyLine {
        nodes: &[
            Floor {
                position: [-1425.0, -165.0],
                elevation: 2185.0,
                left: (21, 0),
                right: (6, 0),
            },
            Floor {
                position: [-1615.0, -340.0],
                elevation: 1920.0,
                left: (21, 1),
                right: (6, 1),
            },
            Floor {
                position: [-1793.0, -504.0],
                elevation: GROUND,
                left: (21, 3),
                right: (6, 3),
            },
        ],
    },
    // 11 west_col_south: between the two south spurs on the far west.
    //   head cW3, walls w4+sW2 (left) and w5+sW3 (right).
    ValleyLine {
        nodes: &[
            Floor {
                position: [-1265.0, -125.0],
                elevation: 2085.0,
                left: (4, 2),
                right: (5, 2),
            },
            Floor {
                position: [-1295.0, -320.0],
                elevation: 1880.0,
                left: (4, 1),
                right: (5, 1),
            },
            Floor {
                position: [-1335.0, -530.0],
                elevation: 1680.0,
                left: (19, 1),
                right: (21, 1),
            },
            Floor {
                position: [-1378.0, -734.0],
                elevation: GROUND,
                left: (19, 3),
                right: (21, 3),
            },
        ],
    },
    // 12 west_col_mid: between the two south spurs on the middle west.
    //   head cW2, walls w2+sW1 (left) and w3+sW2 (right).
    ValleyLine {
        nodes: &[
            Floor {
                position: [-795.0, -15.0],
                elevation: 2165.0,
                left: (2, 2),
                right: (3, 2),
            },
            Floor {
                position: [-865.0, -215.0],
                elevation: 1945.0,
                left: (2, 1),
                right: (3, 1),
            },
            Floor {
                position: [-945.0, -450.0],
                elevation: 1700.0,
                left: (15, 1),
                right: (19, 1),
            },
            Floor {
                position: [-1019.0, -679.0],
                elevation: GROUND,
                left: (15, 3),
                right: (19, 3),
            },
        ],
    },
    // 13 south: the long one's twin, from the summit down the south side.
    //   head the summit, walls e0+e1+sE1a+sE1b (left) and w0+w1+sW1 (right).
    ValleyLine {
        nodes: &[
            Floor {
                position: [0.0, 0.0],
                elevation: SUMMIT,
                left: (7, 0),
                right: (0, 0),
            },
            Floor {
                position: [25.0, -235.0],
                elevation: 2185.0,
                left: (7, 1),
                right: (0, 1),
            },
            Floor {
                position: [45.0, -490.0],
                elevation: 1965.0,
                left: (8, 1),
                right: (1, 1),
            },
            Floor {
                position: [70.0, -745.0],
                elevation: 1745.0,
                left: (25, 1),
                right: (15, 1),
            },
            Floor {
                position: [10.0, -935.0],
                elevation: 1590.0,
                left: (26, 1),
                right: (15, 2),
            },
            Floor {
                position: [14.0, -1099.0],
                elevation: GROUND,
                left: (26, 2),
                right: (15, 3),
            },
        ],
    },
    // 14 sub_s1: between the first east spur's south branch and its own spur.
    //   head M_sE1, walls sub_sE1 (left) and sE1b (right).
    ValleyLine {
        nodes: &[
            Floor {
                position: [632.0, -172.0],
                elevation: 2165.0,
                left: (27, 0),
                right: (26, 0),
            },
            Floor {
                position: [835.0, -380.0],
                elevation: 1870.0,
                left: (27, 1),
                right: (26, 1),
            },
            Floor {
                position: [1028.0, -567.0],
                elevation: GROUND,
                left: (27, 3),
                right: (26, 2),
            },
        ],
    },
    // 15 east_col_south: between the first east spur's own spur and the second's south spur.
    //   head cE2, walls e3+sE2 (left) and e2+sE1a+sub_sE1 (right).
    ValleyLine {
        nodes: &[
            Floor {
                position: [810.0, 30.0],
                elevation: 2185.0,
                left: (10, 2),
                right: (9, 2),
            },
            Floor {
                position: [920.0, -155.0],
                elevation: 1985.0,
                left: (10, 1),
                right: (9, 1),
            },
            Floor {
                position: [1035.0, -315.0],
                elevation: 1790.0,
                left: (31, 1),
                right: (25, 1),
            },
            Floor {
                position: [1165.0, -415.0],
                elevation: 1620.0,
                left: (31, 2),
                right: (27, 1),
            },
            Floor {
                position: [1299.0, -498.0],
                elevation: GROUND,
                left: (31, 3),
                right: (27, 3),
            },
        ],
    },
    // 16 east_south: between the second east spur's south spur and the east tip.
    //   head J_E2, walls e4 (left) and sE2 (right).
    ValleyLine {
        nodes: &[
            Floor {
                position: [1065.0, 85.0],
                elevation: 2275.0,
                left: (11, 0),
                right: (31, 0),
            },
            Floor {
                position: [1300.0, -25.0],
                elevation: 2050.0,
                left: (11, 1),
                right: (31, 1),
            },
            Floor {
                position: [1535.0, -139.0],
                elevation: GROUND,
                left: (11, 3),
                right: (31, 3),
            },
        ],
    },
];

/// The ridge tips of the rim, in the order the rim is walked.
///
/// Counterclockwise, which is the winding the mesh wants: the inside is on the left of the walk.
/// The walk alternates a tip and the mouth of the pocket between it and the next tip, so this list
/// and [`RIM_MOUTHS`] are two halves of one round trip.
const RIM_TIPS: &[usize] = &[
    11, 30, 29, 24, 23, 14, 13, 18, 17, 20, 6, 21, 19, 15, 26, 27, 31,
];

/// The valley mouths of the rim, in the same walk order: each is the pocket between the tip above
/// it and the tip below it in [`RIM_TIPS`].
const RIM_MOUTHS: &[usize] = &[0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16];

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
                        station(floor.left),
                        station(floor.right),
                    )
                })
                .collect();
            Valley::new(nodes)
        })
        .collect()
}

/// Turns a named crest node into the station the crate reads.
///
/// The parameter is the node's own arc-length share of its crest, which is the one number that
/// lands the station exactly on the node.
fn station(named: (usize, usize)) -> Station {
    let (ridge, node) = named;
    Station::ridge(RidgeId(ridge), parameter_of_node(RIDGES[ridge].nodes, node))
}

/// Returns the arc-length parameter of one of a ridge's nodes.
fn parameter_of_node(nodes: Written, node: usize) -> f32 {
    let mut total = 0.0;
    let mut walked = 0.0;
    for (index, pair) in nodes.windows(2).enumerate() {
        let along = plan_distance(pair[0], pair[1]);
        if index == node {
            walked = total;
        }
        total += along;
    }
    if node + 1 >= nodes.len() {
        walked = total;
    }
    if total <= f32::EPSILON {
        return 0.0;
    }
    walked / total
}

/// Returns the plan distance between two written nodes.
fn plan_distance(from: [f32; 3], to: [f32; 3]) -> f32 {
    ((to[0] - from[0]).powi(2) + (to[1] - from[1]).powi(2)).sqrt()
}

/// Writes every junction as a name.
///
/// # Invariants
///
/// A junction is one crest's end standing on another's, and the crate states that as an **end naming
/// a station** rather than as two points that happen to coincide. Whichever crest comes first in the
/// table owns a point, and any crest after it whose end lands there names it.
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
/// the ground it meets.
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
