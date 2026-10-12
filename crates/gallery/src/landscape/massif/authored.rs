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
//! # The control points are placed by hand
//!
//! Every node in this file was put where a person put it. **No formula, grid, ellipse, spacing rule
//! or generator places a point here**, and that includes the elevations and the correspondences:
//! a shoulder is as high as it was made, not as high as a schedule computed, and a valley node
//! faces the crest node an author chose, not the one a picker found. The test is not whether the
//! numbers look regular — a hand can be regular by choice — it is whether an equation produced
//! them. Regularity that arrives on its own is the smell of a generator, and a table that can be
//! reproduced from a seed is not created, it is re-created. Scratch arithmetic to find where a
//! point *should* go is a ruler; anything that writes this file is not.
//!
//! # The shape: a dissected upland, and how one is written
//!
//! The reference is a dissected upland: **one long divide with spurs branching off it**, and a
//! valley cut in between every neighbouring pair of tips. Four rules make one writable:
//!
//! - **A crest is written peak-first and is one segment between two forks.** Nothing may rise as it
//!   runs away from the summit, so a crest falls from the peak it starts at to the fork, the col or
//!   the tip it ends at — and the divide's undulation is exactly that pair of moves: a shoulder
//!   falls into the col beside it, and the next shoulder's crest falls into the same col. A crest
//!   carries a node at each end and one between them, because a belt is one facet wide and as tall
//!   as the crest it faces falls by between two stations: a crest of two nodes puts the whole of a
//!   shoulder's fall into one row of facets, and a crest of three halves it.
//! - **A junction is a segment end.** A belt changes line only at a line's end, so the crests that
//!   leave a shoulder name the crest that ends there, and [`name_junctions`] writes the naming.
//! - **A valley's wall is a chain, and the chain turns at the junctions.** The wall below the col
//!   `colW2` walks that crest back up to `W2` and then out along the crest to `F2`; the belt that
//!   straddles the junction walks through it, because the two segments end there.
//! - **The spurs are visited in the order they hang off the divide.** The rim's walk leaves the
//!   north-east tip, takes the north spurs in order as it goes west, turns at the west tip, and
//!   comes back along the south spurs — which is what keeps every pocket a wedge that opens
//!   outward.
//!
//! A pocket of the tree is the wedge between two neighbouring ridge tips; its valley heads where
//! the two tips' wall chains converge, which is a spur's fork when both tips hang off one spur, a
//! shoulder or the summit where a pocket's wedge closes, and a col where the divide dips inside a
//! pocket. Its floor is written between the walls, down to the rim.
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
//! **Sixteen tips** — seven spurs of two tips each off the seven shoulders of the divide, plus the
//! west and east ends of the divide itself — joined by **thirty-five crests and sixteen valleys**.
//! The divide is the first fourteen of them: six saddles hung between the seven shoulders, strung
//! from the summit to each end, so the range's own length is the plan's and no pocket is wider
//! than a flank is deep. **Seven spurs** hang off those shoulders, each a stem and two branches,
//! which is where the second scale is: two branches forking off one stem. The tips of each flank
//! are handed to the spurs **nearest them in plan**, so a spur never leans along the divide. The
//! tips are therefore bearing-contiguous along the flanks, the pockets between them tile the plan,
//! and the plan reads as a backbone with ribs rather than as a cone.
//!
//! **The relief is a range's and not a needle's.** The summit stands at 2400 and the ground at
//! 1450, and the divide undulates away from it shoulder and saddle by shoulder and saddle, so the
//! shoulders beside the summit stay above 2200 and the two ends come down to about 1985 — a
//! continuous ridge line rather than an obelisk over a tabletop. Every col sits below the two
//! shoulders beside it, every spur stem falls from its own shoulder to its own fork, and every
//! floor descends from its own head to the ground at its own mouth.

use mountain::line::{LineRef, Station};
use mountain::skeleton::{
    End, FootprintNode, Ridge, RidgeId, RidgeNode, Skeleton, Valley, ValleyNode,
};

/// The elevation the country around the massif stands at.
const GROUND: f32 = super::super::RIM as f32;

/// The summit, at the root of the tree.
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

/// The thirty-five crests, in the order the junction naming reads them.
///
/// The first fourteen are the divide, written outward from the summit: a peak falls into the col
/// beside it, and the next peak's crest falls into the same col, so the divide is a chain of
/// segments that undulates between shoulders and saddles. The rest are seven spurs and their
/// branches, in the order the rim visits them.
///
/// Every crest is one segment between two forks — or a fork and a tip, or a fork and a col — so
/// every junction is an end of a line, which is what `Sampling::corner` requires of one.
///
/// ```text
///  0 dc0   summit -> colWs
///  1 dc1   W3     -> colWs
///  2 dc2   W3     -> colW2
///  3 dc3   W2     -> colW2
///  4 dc4   W2     -> colW1
///  5 dc5   W1     -> colW1
///  6 dc6   W1     -> west tip
///  7 dc7   summit -> colEs
///  8 dc8   E1     -> colEs
///  9 dc9   E1     -> colE2
/// 10 dc10  E2     -> colE2
/// 11 dc11  E2     -> colE3
/// 12 dc12  E3     -> colE3
/// 13 dc13  E3     -> east tip
/// 14 s1stem W1  -> F1
/// 15 s1A   F1    -> south-west tip
/// 16 s1B   F1    -> tip
/// 17 s2stem W2  -> F2
/// 18 s2A   F2    -> north-west tip
/// 19 s2B   F2    -> tip
/// 20 s3stem W3  -> F3
/// 21 s3A   F3    -> south tip
/// 22 s3B   F3    -> tip
/// 23 s4stem summit -> F4
/// 24 s4A   F4    -> north tip
/// 25 s4B   F4    -> tip
/// 26 s5stem E1  -> F5
/// 27 s5A   F5    -> south-east tip
/// 28 s5B   F5    -> tip
/// 29 s6stem E2  -> F6
/// 30 s6A   F6    -> north-east tip
/// 31 s6B   F6    -> tip
/// 32 s7stem E3  -> F7
/// 33 s7A   F7    -> east-south tip
/// 34 s7B   F7    -> tip
/// ```
const RIDGES: &[RidgeLine] = &[
    // 0 dc0: the summit falls west into the col beside it.
    RidgeLine {
        nodes: &[
            [0.0, 150.0, SUMMIT],
            [-108.0, 140.0, 2232.0],
            [-215.0, 130.0, 2050.0],
        ],
        sharpness: 0.50,
    },
    // 1 dc1: W3 falls east into that same col.
    RidgeLine {
        nodes: &[
            [-430.0, 95.0, 2215.0],
            [-322.0, 113.0, 2132.0],
            [-215.0, 130.0, 2050.0],
        ],
        sharpness: 0.50,
    },
    // 2 dc2: W3 falls west into the col below it.
    RidgeLine {
        nodes: &[
            [-430.0, 95.0, 2215.0],
            [-535.0, 70.0, 2075.0],
            [-640.0, 45.0, 1935.0],
        ],
        sharpness: 0.50,
    },
    // 3 dc3: W2 falls east into that same col.
    RidgeLine {
        nodes: &[
            [-830.0, -20.0, 2105.0],
            [-735.0, 12.0, 2020.0],
            [-640.0, 45.0, 1935.0],
        ],
        sharpness: 0.50,
    },
    // 4 dc4: W2 falls west into the col below it.
    RidgeLine {
        nodes: &[
            [-830.0, -20.0, 2105.0],
            [-925.0, -58.0, 1958.0],
            [-1020.0, -95.0, 1810.0],
        ],
        sharpness: 0.50,
    },
    // 5 dc5: W1 falls east into that same col.
    RidgeLine {
        nodes: &[
            [-1180.0, -140.0, 1985.0],
            [-1100.0, -118.0, 1898.0],
            [-1020.0, -95.0, 1810.0],
        ],
        sharpness: 0.50,
    },
    // 6 dc6: W1 runs out west to the west tip and down to the ground.
    RidgeLine {
        nodes: &[
            [-1180.0, -140.0, 1985.0],
            [-1282.0, -185.0, 1792.0],
            [-1384.0, -230.0, 1598.0],
            [-1450.0, -260.0, GROUND],
        ],
        sharpness: 0.50,
    },
    // 7 dc7: the summit falls east into the col beside it.
    RidgeLine {
        nodes: &[
            [0.0, 150.0, SUMMIT],
            [118.0, 138.0, 2258.0],
            [235.0, 125.0, 2120.0],
        ],
        sharpness: 0.50,
    },
    // 8 dc8: E1 falls west into that same col.
    RidgeLine {
        nodes: &[
            [455.0, 85.0, 2285.0],
            [345.0, 105.0, 2202.0],
            [235.0, 125.0, 2120.0],
        ],
        sharpness: 0.50,
    },
    // 9 dc9: E1 falls east into the col below it.
    RidgeLine {
        nodes: &[
            [455.0, 85.0, 2285.0],
            [558.0, 58.0, 2152.0],
            [660.0, 30.0, 2020.0],
        ],
        sharpness: 0.50,
    },
    // 10 dc10: E2 falls west into that same col.
    RidgeLine {
        nodes: &[
            [855.0, -35.0, 2155.0],
            [758.0, -2.0, 2088.0],
            [660.0, 30.0, 2020.0],
        ],
        sharpness: 0.50,
    },
    // 11 dc11: E2 falls east into the col below it.
    RidgeLine {
        nodes: &[
            [855.0, -35.0, 2155.0],
            [945.0, -65.0, 2020.0],
            [1035.0, -95.0, 1885.0],
        ],
        sharpness: 0.50,
    },
    // 12 dc12: E3 falls west into that same col.
    RidgeLine {
        nodes: &[
            [1215.0, -165.0, 1995.0],
            [1125.0, -130.0, 1940.0],
            [1035.0, -95.0, 1885.0],
        ],
        sharpness: 0.50,
    },
    // 13 dc13: E3 runs out east to the east tip and down to the ground.
    RidgeLine {
        nodes: &[
            [1215.0, -165.0, 1995.0],
            [1310.0, -215.0, 1802.0],
            [1405.0, -265.0, 1608.0],
            [1465.0, -295.0, GROUND],
        ],
        sharpness: 0.50,
    },
    // 14 s1stem: the first spur leaves W1 and runs south to its fork.
    RidgeLine {
        nodes: &[[-1180.0, -140.0, 1985.0], [-1245.0, -335.0, 1810.0]],
        sharpness: 0.42,
    },
    // 15 s1A: that fork falls south-west to its tip.
    RidgeLine {
        nodes: &[
            [-1245.0, -335.0, 1810.0],
            [-1330.0, -520.0, 1620.0],
            [-1425.0, -640.0, GROUND],
        ],
        sharpness: 0.42,
    },
    // 16 s1B: the same fork falls south-east to its tip.
    RidgeLine {
        nodes: &[
            [-1245.0, -335.0, 1810.0],
            [-1105.0, -530.0, 1585.0],
            [-1040.0, -685.0, GROUND],
        ],
        sharpness: 0.42,
    },
    // 17 s2stem: the second spur leaves W2 and runs north to its fork.
    RidgeLine {
        nodes: &[[-830.0, -20.0, 2105.0], [-905.0, 175.0, 1930.0]],
        sharpness: 0.42,
    },
    // 18 s2A: that fork falls north-west to its tip.
    RidgeLine {
        nodes: &[
            [-905.0, 175.0, 1930.0],
            [-1015.0, 320.0, 1745.0],
            [-1120.0, 470.0, GROUND],
        ],
        sharpness: 0.42,
    },
    // 19 s2B: the same fork falls north-east to its tip.
    RidgeLine {
        nodes: &[
            [-905.0, 175.0, 1930.0],
            [-805.0, 355.0, 1720.0],
            [-720.0, 525.0, GROUND],
        ],
        sharpness: 0.42,
    },
    // 20 s3stem: the third spur leaves W3 and runs south to its fork.
    RidgeLine {
        nodes: &[[-430.0, 95.0, 2215.0], [-480.0, -145.0, 2020.0]],
        sharpness: 0.42,
    },
    // 21 s3A: that fork falls south-west to its tip.
    RidgeLine {
        nodes: &[
            [-480.0, -145.0, 2020.0],
            [-585.0, -315.0, 1815.0],
            [-665.0, -485.0, GROUND],
        ],
        sharpness: 0.42,
    },
    // 22 s3B: the same fork falls south-east to its tip.
    RidgeLine {
        nodes: &[
            [-480.0, -145.0, 2020.0],
            [-375.0, -345.0, 1780.0],
            [-295.0, -525.0, GROUND],
        ],
        sharpness: 0.42,
    },
    // 23 s4stem: the middle spur leaves the summit and runs north to its fork.
    RidgeLine {
        nodes: &[[0.0, 150.0, SUMMIT], [-35.0, 385.0, 2165.0]],
        sharpness: 0.42,
    },
    // 24 s4A: that fork falls north-west to its tip.
    RidgeLine {
        nodes: &[
            [-35.0, 385.0, 2165.0],
            [-155.0, 545.0, 1935.0],
            [-265.0, 690.0, GROUND],
        ],
        sharpness: 0.42,
    },
    // 25 s4B: the same fork falls north-east to its tip.
    RidgeLine {
        nodes: &[
            [-35.0, 385.0, 2165.0],
            [120.0, 575.0, 1905.0],
            [245.0, 720.0, GROUND],
        ],
        sharpness: 0.42,
    },
    // 26 s5stem: the fifth spur leaves E1 and runs south to its fork.
    RidgeLine {
        nodes: &[[455.0, 85.0, 2285.0], [545.0, -145.0, 2035.0]],
        sharpness: 0.42,
    },
    // 27 s5A: that fork falls south-east to its tip.
    RidgeLine {
        nodes: &[
            [545.0, -145.0, 2035.0],
            [680.0, -305.0, 1810.0],
            [795.0, -475.0, GROUND],
        ],
        sharpness: 0.42,
    },
    // 28 s5B: the same fork falls south-west to its tip.
    RidgeLine {
        nodes: &[
            [545.0, -145.0, 2035.0],
            [445.0, -355.0, 1775.0],
            [355.0, -530.0, GROUND],
        ],
        sharpness: 0.42,
    },
    // 29 s6stem: the sixth spur leaves E2 and runs north to its fork.
    RidgeLine {
        nodes: &[[855.0, -35.0, 2155.0], [925.0, 165.0, 1930.0]],
        sharpness: 0.42,
    },
    // 30 s6A: that fork falls north-east to its tip.
    RidgeLine {
        nodes: &[
            [925.0, 165.0, 1930.0],
            [1055.0, 315.0, 1740.0],
            [1175.0, 465.0, GROUND],
        ],
        sharpness: 0.42,
    },
    // 31 s6B: the same fork falls north-west to its tip.
    RidgeLine {
        nodes: &[
            [925.0, 165.0, 1930.0],
            [835.0, 350.0, 1710.0],
            [745.0, 505.0, GROUND],
        ],
        sharpness: 0.42,
    },
    // 32 s7stem: the seventh spur leaves E3 and runs south-east to its fork.
    RidgeLine {
        nodes: &[[1215.0, -165.0, 1995.0], [1305.0, -375.0, 1810.0]],
        sharpness: 0.42,
    },
    // 33 s7A: that fork falls east-south-east to its tip.
    RidgeLine {
        nodes: &[
            [1305.0, -375.0, 1810.0],
            [1435.0, -525.0, 1645.0],
            [1555.0, -665.0, GROUND],
        ],
        sharpness: 0.42,
    },
    // 34 s7B: the same fork falls south-west to its tip, kept on its bearing so the
    // two tips of this spur stay together in the walk around the rim.
    RidgeLine {
        nodes: &[
            [1305.0, -375.0, 1810.0],
            [1195.0, -545.0, 1615.0],
            [1230.0, -560.0, GROUND],
        ],
        sharpness: 0.42,
    },
];

/// One authored floor node: where it stands, how high it stands, and the two crest nodes it faces.
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

/// The sixteen valleys — one per pocket of the tree — in the order the rim walks them.
///
/// Each heads where its two tips' wall chains converge: at the spur's fork when both tips hang off
/// one spur, at the shoulder or the summit where the pocket's wedge closes. The head names its own
/// point on both sides, which is what makes the first belt row collapse to a triangle and the head
/// close. A wall is the chain of crests from the head down to its tip, and its stations are the
/// chain's own nodes: the head, one for every node it turns or bends at, and the tip. The floor
/// runs from the head to the **chord between its two tips** and the mouth stands a hundred or so
/// beyond that, on the bisector of the tips' directions from the head, so the rim bulges rather
/// than bow-ties.
///
/// # Invariants
///
/// **The stations advance by one or not at all, and the tip is named twice.** The longer wall's
/// nodes are named one to a floor node, the shorter wall's are then repeated at its tip, and the
/// floor carries one node **more** than the longer wall has nodes — so its last station, the tip, is
/// named at the last interior node and again at the mouth. The rib that faces a tip crest is
/// therefore drawn from the floor's own interior, and never from a mouth that stands beyond the tip,
/// where the crest and the rib are nearly parallel and the flank's side, read from their cross
/// product, comes out of the noise. **Each floor is the running low of what the walls allow**,
/// because the ceiling a wall puts over the floor is not monotone — a col is lower than the peak
/// beyond it — so a floor that held one share of the lower wall would rise under a shoulder and
/// stop being a valley.
const VALLEYS: &[ValleyLine] = &[
    // 0: between the two tips of the sixth spur, heading at its fork F6.
    ValleyLine {
        nodes: &[
            Floor {
                position: [925.0, 165.0],
                elevation: 1760.0,
                left: (30, 0),
                right: (31, 0),
            },
            Floor {
                position: [952.0, 352.0],
                elevation: 1620.0,
                left: (30, 1),
                right: (31, 1),
            },
            Floor {
                position: [965.0, 470.0],
                elevation: 1450.0,
                left: (30, 2),
                right: (31, 2),
            },
            Floor {
                position: [978.0, 655.0],
                elevation: GROUND,
                left: (30, 2),
                right: (31, 2),
            },
        ],
    },
    // 1: between tip 31 and tip 25, heading at the summit — the whole north-east face.
    ValleyLine {
        nodes: &[
            Floor {
                position: [0.0, 150.0],
                elevation: 2160.0,
                left: (7, 0),
                right: (23, 0),
            },
            Floor {
                position: [72.0, 292.0],
                elevation: 2010.0,
                left: (7, 1),
                right: (23, 1),
            },
            Floor {
                position: [118.0, 352.0],
                elevation: 1925.0,
                left: (8, 2),
                right: (25, 0),
            },
            Floor {
                position: [172.0, 408.0],
                elevation: 1810.0,
                left: (8, 1),
                right: (25, 1),
            },
            Floor {
                position: [232.0, 458.0],
                elevation: 1715.0,
                left: (9, 0),
                right: (25, 2),
            },
            Floor {
                position: [290.0, 502.0],
                elevation: 1640.0,
                left: (9, 1),
                right: (25, 2),
            },
            Floor {
                position: [345.0, 540.0],
                elevation: 1585.0,
                left: (10, 2),
                right: (25, 2),
            },
            Floor {
                position: [392.0, 572.0],
                elevation: 1540.0,
                left: (10, 1),
                right: (25, 2),
            },
            Floor {
                position: [424.0, 598.0],
                elevation: 1508.0,
                left: (29, 0),
                right: (25, 2),
            },
            Floor {
                position: [460.0, 620.0],
                elevation: 1487.0,
                left: (29, 1),
                right: (25, 2),
            },
            Floor {
                position: [500.0, 638.0],
                elevation: 1478.0,
                left: (31, 0),
                right: (25, 2),
            },
            Floor {
                position: [540.0, 654.0],
                elevation: 1470.0,
                left: (31, 1),
                right: (25, 2),
            },
            Floor {
                position: [580.0, 667.0],
                elevation: 1462.0,
                left: (31, 2),
                right: (25, 2),
            },
            Floor {
                position: [605.0, 714.0],
                elevation: GROUND,
                left: (31, 2),
                right: (25, 2),
            },
        ],
    },
    // 2: between the two tips of the fourth spur, heading at its fork F4.
    ValleyLine {
        nodes: &[
            Floor {
                position: [-35.0, 385.0],
                elevation: 1990.0,
                left: (25, 0),
                right: (24, 0),
            },
            Floor {
                position: [-18.0, 570.0],
                elevation: 1790.0,
                left: (25, 1),
                right: (24, 1),
            },
            Floor {
                position: [-12.0, 712.0],
                elevation: 1450.0,
                left: (25, 2),
                right: (24, 2),
            },
            Floor {
                position: [-20.0, 875.0],
                elevation: GROUND,
                left: (25, 2),
                right: (24, 2),
            },
        ],
    },
    // 3: between tip 24 and tip 19, heading at the summit — the north-west face.
    ValleyLine {
        nodes: &[
            Floor {
                position: [0.0, 150.0],
                elevation: 2128.0,
                left: (23, 0),
                right: (0, 0),
            },
            Floor {
                position: [-98.0, 268.0],
                elevation: 2036.0,
                left: (23, 1),
                right: (0, 1),
            },
            Floor {
                position: [-158.0, 314.0],
                elevation: 1948.0,
                left: (24, 0),
                right: (1, 2),
            },
            Floor {
                position: [-218.0, 356.0],
                elevation: 1864.0,
                left: (24, 1),
                right: (1, 1),
            },
            Floor {
                position: [-272.0, 395.0],
                elevation: 1792.0,
                left: (24, 2),
                right: (2, 0),
            },
            Floor {
                position: [-318.0, 428.0],
                elevation: 1726.0,
                left: (24, 2),
                right: (2, 1),
            },
            Floor {
                position: [-356.0, 462.0],
                elevation: 1668.0,
                left: (24, 2),
                right: (3, 2),
            },
            Floor {
                position: [-390.0, 490.0],
                elevation: 1614.0,
                left: (24, 2),
                right: (3, 1),
            },
            Floor {
                position: [-420.0, 518.0],
                elevation: 1562.0,
                left: (24, 2),
                right: (17, 0),
            },
            Floor {
                position: [-442.0, 548.0],
                elevation: 1512.0,
                left: (24, 2),
                right: (17, 1),
            },
            Floor {
                position: [-470.0, 572.0],
                elevation: 1476.0,
                left: (24, 2),
                right: (19, 0),
            },
            Floor {
                position: [-498.0, 596.0],
                elevation: 1460.0,
                left: (24, 2),
                right: (19, 1),
            },
            Floor {
                position: [-526.0, 618.0],
                elevation: 1453.0,
                left: (24, 2),
                right: (19, 2),
            },
            Floor {
                position: [-602.0, 710.0],
                elevation: GROUND,
                left: (24, 2),
                right: (19, 2),
            },
        ],
    },
    // 4: between the two tips of the second spur, heading at its fork F2.
    ValleyLine {
        nodes: &[
            Floor {
                position: [-905.0, 175.0],
                elevation: 1790.0,
                left: (19, 0),
                right: (18, 0),
            },
            Floor {
                position: [-908.0, 342.0],
                elevation: 1600.0,
                left: (19, 1),
                right: (18, 1),
            },
            Floor {
                position: [-918.0, 502.0],
                elevation: 1450.0,
                left: (19, 2),
                right: (18, 2),
            },
            Floor {
                position: [-943.0, 666.0],
                elevation: GROUND,
                left: (19, 2),
                right: (18, 2),
            },
        ],
    },
    // 5: between tip 18 and the west tip, heading at W2 — the wedge the second spur and the
    // divide's western end leave open.
    ValleyLine {
        nodes: &[
            Floor {
                position: [-830.0, -20.0],
                elevation: 1900.0,
                left: (17, 0),
                right: (4, 0),
            },
            Floor {
                position: [-918.0, 52.0],
                elevation: 1810.0,
                left: (17, 1),
                right: (4, 1),
            },
            Floor {
                position: [-968.0, 32.0],
                elevation: 1710.0,
                left: (18, 0),
                right: (5, 2),
            },
            Floor {
                position: [-1062.0, 92.0],
                elevation: 1650.0,
                left: (18, 1),
                right: (5, 1),
            },
            Floor {
                position: [-1155.0, 158.0],
                elevation: 1590.0,
                left: (18, 2),
                right: (6, 0),
            },
            Floor {
                position: [-1215.0, 195.0],
                elevation: 1540.0,
                left: (18, 2),
                right: (6, 1),
            },
            Floor {
                position: [-1268.0, 118.0],
                elevation: 1500.0,
                left: (18, 2),
                right: (6, 2),
            },
            Floor {
                position: [-1300.0, 105.0],
                elevation: 1470.0,
                left: (18, 2),
                right: (6, 3),
            },
            Floor {
                position: [-1428.0, 150.0],
                elevation: GROUND,
                left: (18, 2),
                right: (6, 3),
            },
        ],
    },
    // 6: between the west tip and tip 15, heading at W1.
    ValleyLine {
        nodes: &[
            Floor {
                position: [-1180.0, -140.0],
                elevation: 1810.0,
                left: (6, 0),
                right: (14, 0),
            },
            Floor {
                position: [-1268.0, -252.0],
                elevation: 1680.0,
                left: (6, 1),
                right: (14, 1),
            },
            Floor {
                position: [-1320.0, -285.0],
                elevation: 1560.0,
                left: (6, 2),
                right: (15, 0),
            },
            Floor {
                position: [-1395.0, -385.0],
                elevation: 1520.0,
                left: (6, 3),
                right: (15, 1),
            },
            Floor {
                position: [-1442.0, -448.0],
                elevation: 1480.0,
                left: (6, 3),
                right: (15, 2),
            },
            Floor {
                position: [-1533.0, -565.0],
                elevation: GROUND,
                left: (6, 3),
                right: (15, 2),
            },
        ],
    },
    // 7: between the two tips of the first spur, heading at its fork F1.
    ValleyLine {
        nodes: &[
            Floor {
                position: [-1245.0, -335.0],
                elevation: 1680.0,
                left: (15, 0),
                right: (16, 0),
            },
            Floor {
                position: [-1218.0, -520.0],
                elevation: 1510.0,
                left: (15, 1),
                right: (16, 1),
            },
            Floor {
                position: [-1230.0, -658.0],
                elevation: 1450.0,
                left: (15, 2),
                right: (16, 2),
            },
            Floor {
                position: [-1252.0, -831.0],
                elevation: GROUND,
                left: (15, 2),
                right: (16, 2),
            },
        ],
    },
    // 8: between tip 16 and tip 21, heading at W1 — the whole southern west face.
    ValleyLine {
        nodes: &[
            Floor {
                position: [-1180.0, -140.0],
                elevation: 1850.0,
                left: (14, 0),
                right: (5, 0),
            },
            Floor {
                position: [-1142.0, -228.0],
                elevation: 1740.0,
                left: (14, 1),
                right: (5, 1),
            },
            Floor {
                position: [-1130.0, -272.0],
                elevation: 1670.0,
                left: (16, 0),
                right: (4, 2),
            },
            Floor {
                position: [-1080.0, -308.0],
                elevation: 1615.0,
                left: (16, 1),
                right: (4, 1),
            },
            Floor {
                position: [-1030.0, -338.0],
                elevation: 1572.0,
                left: (16, 2),
                right: (3, 0),
            },
            Floor {
                position: [-985.0, -365.0],
                elevation: 1540.0,
                left: (16, 2),
                right: (3, 1),
            },
            Floor {
                position: [-950.0, -390.0],
                elevation: 1515.0,
                left: (16, 2),
                right: (2, 2),
            },
            Floor {
                position: [-925.0, -415.0],
                elevation: 1495.0,
                left: (16, 2),
                right: (2, 1),
            },
            Floor {
                position: [-905.0, -440.0],
                elevation: 1480.0,
                left: (16, 2),
                right: (1, 0),
            },
            Floor {
                position: [-878.0, -470.0],
                elevation: 1470.0,
                left: (16, 2),
                right: (20, 1),
            },
            Floor {
                position: [-848.0, -502.0],
                elevation: 1462.0,
                left: (16, 2),
                right: (21, 0),
            },
            Floor {
                position: [-818.0, -535.0],
                elevation: 1456.0,
                left: (16, 2),
                right: (21, 1),
            },
            Floor {
                position: [-790.0, -570.0],
                elevation: 1452.0,
                left: (16, 2),
                right: (21, 2),
            },
            Floor {
                position: [-787.0, -674.0],
                elevation: GROUND,
                left: (16, 2),
                right: (21, 2),
            },
        ],
    },
    // 9: between the two tips of the third spur, heading at its fork F3.
    ValleyLine {
        nodes: &[
            Floor {
                position: [-480.0, -145.0],
                elevation: 1880.0,
                left: (21, 0),
                right: (22, 0),
            },
            Floor {
                position: [-482.0, -326.0],
                elevation: 1690.0,
                left: (21, 1),
                right: (22, 1),
            },
            Floor {
                position: [-478.0, -502.0],
                elevation: 1450.0,
                left: (21, 2),
                right: (22, 2),
            },
            Floor {
                position: [-498.0, -674.0],
                elevation: GROUND,
                left: (21, 2),
                right: (22, 2),
            },
        ],
    },
    // 10: between tip 22 and tip 28, heading at the summit — the whole southern face.
    ValleyLine {
        nodes: &[
            Floor {
                position: [0.0, 150.0],
                elevation: 2180.0,
                left: (0, 0),
                right: (7, 0),
            },
            Floor {
                position: [4.0, 20.0],
                elevation: 2048.0,
                left: (0, 1),
                right: (7, 1),
            },
            Floor {
                position: [10.0, -35.0],
                elevation: 1932.0,
                left: (1, 2),
                right: (8, 2),
            },
            Floor {
                position: [14.0, -90.0],
                elevation: 1830.0,
                left: (1, 1),
                right: (8, 1),
            },
            Floor {
                position: [18.0, -148.0],
                elevation: 1725.0,
                left: (2, 0),
                right: (9, 0),
            },
            Floor {
                position: [22.0, -205.0],
                elevation: 1620.0,
                left: (20, 1),
                right: (26, 1),
            },
            Floor {
                position: [28.0, -265.0],
                elevation: 1530.0,
                left: (22, 0),
                right: (28, 0),
            },
            Floor {
                position: [33.0, -340.0],
                elevation: 1487.0,
                left: (22, 1),
                right: (28, 1),
            },
            Floor {
                position: [35.0, -440.0],
                elevation: 1462.0,
                left: (22, 2),
                right: (28, 2),
            },
            Floor {
                position: [37.0, -677.0],
                elevation: GROUND,
                left: (22, 2),
                right: (28, 2),
            },
        ],
    },
    // 11: between the two tips of the fifth spur, heading at its fork F5.
    ValleyLine {
        nodes: &[
            Floor {
                position: [545.0, -145.0],
                elevation: 1890.0,
                left: (28, 0),
                right: (27, 0),
            },
            Floor {
                position: [565.0, -328.0],
                elevation: 1680.0,
                left: (28, 1),
                right: (27, 1),
            },
            Floor {
                position: [578.0, -498.0],
                elevation: 1450.0,
                left: (28, 2),
                right: (27, 2),
            },
            Floor {
                position: [596.0, -671.0],
                elevation: GROUND,
                left: (28, 2),
                right: (27, 2),
            },
        ],
    },
    // 12: between tip 27 and tip 34, heading at E1 — the whole southern east face.
    ValleyLine {
        nodes: &[
            Floor {
                position: [455.0, 85.0],
                elevation: 2080.0,
                left: (26, 0),
                right: (9, 0),
            },
            Floor {
                position: [548.0, -38.0],
                elevation: 1940.0,
                left: (26, 1),
                right: (9, 1),
            },
            Floor {
                position: [598.0, -72.0],
                elevation: 1862.0,
                left: (27, 0),
                right: (10, 2),
            },
            Floor {
                position: [715.0, -150.0],
                elevation: 1775.0,
                left: (27, 1),
                right: (10, 1),
            },
            Floor {
                position: [818.0, -248.0],
                elevation: 1690.0,
                left: (27, 2),
                right: (11, 0),
            },
            Floor {
                position: [880.0, -315.0],
                elevation: 1612.0,
                left: (27, 2),
                right: (11, 1),
            },
            Floor {
                position: [935.0, -360.0],
                elevation: 1548.0,
                left: (27, 2),
                right: (12, 2),
            },
            Floor {
                position: [985.0, -400.0],
                elevation: 1497.0,
                left: (27, 2),
                right: (12, 1),
            },
            Floor {
                position: [1030.0, -435.0],
                elevation: 1468.0,
                left: (27, 2),
                right: (32, 0),
            },
            Floor {
                position: [1065.0, -462.0],
                elevation: 1459.0,
                left: (27, 2),
                right: (32, 1),
            },
            Floor {
                position: [1085.0, -492.0],
                elevation: 1455.0,
                left: (27, 2),
                right: (34, 0),
            },
            Floor {
                position: [1100.0, -515.0],
                elevation: 1452.0,
                left: (27, 2),
                right: (34, 1),
            },
            Floor {
                position: [1112.0, -532.0],
                elevation: 1450.0,
                left: (27, 2),
                right: (34, 2),
            },
            Floor {
                position: [1141.0, -657.0],
                elevation: GROUND,
                left: (27, 2),
                right: (34, 2),
            },
        ],
    },
    // 13: between the two tips of the seventh spur, heading at its fork F7.
    ValleyLine {
        nodes: &[
            Floor {
                position: [1305.0, -375.0],
                elevation: 1680.0,
                left: (34, 0),
                right: (33, 0),
            },
            Floor {
                position: [1317.0, -530.0],
                elevation: 1520.0,
                left: (34, 1),
                right: (33, 1),
            },
            Floor {
                position: [1390.0, -608.0],
                elevation: 1450.0,
                left: (34, 2),
                right: (33, 2),
            },
            Floor {
                position: [1444.0, -753.0],
                elevation: GROUND,
                left: (34, 2),
                right: (33, 2),
            },
        ],
    },
    // 14: between tip 33 and the east tip, heading at E3.
    ValleyLine {
        nodes: &[
            Floor {
                position: [1215.0, -165.0],
                elevation: 1830.0,
                left: (32, 0),
                right: (13, 0),
            },
            Floor {
                position: [1300.0, -288.0],
                elevation: 1690.0,
                left: (32, 1),
                right: (13, 1),
            },
            Floor {
                position: [1350.0, -322.0],
                elevation: 1590.0,
                left: (33, 0),
                right: (13, 2),
            },
            Floor {
                position: [1442.0, -395.0],
                elevation: 1520.0,
                left: (33, 1),
                right: (13, 3),
            },
            Floor {
                position: [1498.0, -452.0],
                elevation: 1480.0,
                left: (33, 2),
                right: (13, 3),
            },
            Floor {
                position: [1655.0, -620.0],
                elevation: GROUND,
                left: (33, 2),
                right: (13, 3),
            },
        ],
    },
    // 15: between the east tip and tip 30, heading at E2.
    ValleyLine {
        nodes: &[
            Floor {
                position: [855.0, -35.0],
                elevation: 1960.0,
                left: (11, 0),
                right: (29, 0),
            },
            Floor {
                position: [952.0, 52.0],
                elevation: 1850.0,
                left: (11, 1),
                right: (29, 1),
            },
            Floor {
                position: [992.0, 82.0],
                elevation: 1780.0,
                left: (12, 2),
                right: (30, 0),
            },
            Floor {
                position: [1032.0, 110.0],
                elevation: 1700.0,
                left: (12, 1),
                right: (30, 1),
            },
            Floor {
                position: [1072.0, 136.0],
                elevation: 1630.0,
                left: (13, 0),
                right: (30, 2),
            },
            Floor {
                position: [1112.0, 158.0],
                elevation: 1570.0,
                left: (13, 1),
                right: (30, 2),
            },
            Floor {
                position: [1155.0, 176.0],
                elevation: 1520.0,
                left: (13, 2),
                right: (30, 2),
            },
            Floor {
                position: [1250.0, 185.0],
                elevation: 1480.0,
                left: (13, 3),
                right: (30, 2),
            },
            Floor {
                position: [1417.0, 110.0],
                elevation: GROUND,
                left: (13, 3),
                right: (30, 2),
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
    30, 31, 25, 24, 19, 18, 6, 15, 16, 21, 22, 28, 27, 34, 33, 13,
];

/// The valley mouths of the rim, in the same walk order: each is the pocket between the tip above
/// it and the tip below it in [`RIM_TIPS`].
const RIM_MOUTHS: &[usize] = &[0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15];

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
/// The parameter is the node's own arc-length parameter of its crest, which is the one number that
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
