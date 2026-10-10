//! Grows a ridge **network**: one summit, ridges radiating from it, each splitting into ridges that
//! split again.
//!
//! # Why this is generated and not authored
//!
//! A hand-written table of crests is a drawing. It can hold a dozen lines and a designed silhouette,
//! and it cannot hold the thing that makes a real mountain read as a mountain: **the same branching at
//! every scale, repeated until the detail runs out**. A dissected upland is one shape — a divide that
//! throws off spurs, each of which throws off spurs — and a table long enough to state that at four
//! scales is a table nobody can edit, whose every entry has to be checked by hand against the rule
//! that a junction is a crest's end.
//!
//! So the shape is stated once, as a rule, and applied until it is small enough to stop.
//!
//! # The two rules a generated network has to keep
//!
//! - **A junction is a crest's end.** A child is born at a *node index* of its parent, never at a
//!   fraction along it, so the emitter can cut the parent exactly there and the two crests share the
//!   point bit for bit. Birth by fraction would put the child's first node near the parent's line
//!   rather than on it, the expansion would refuse the weld, and the massif would come apart at every
//!   junction in it.
//! - **Two crests may not cross in plan.** The massif is expanded by walking paired lines, which
//!   assumes the ridge graph is drawn flat: two crests that cross have an intersection the walk has no
//!   representation for, and it shows up as belts folded through each other rather than as anything
//!   that can be reported. The generator therefore checks and retries, which is why it takes a seed
//!   and returns only a network that passes.

use super::Crest;

/// How many crest ends a first-order ridge is cut into by its children.
const ORDERS: u32 = 4;

/// How much shorter each order is than the one that grew it.
const SHRINK: f32 = 0.46;

/// How much of a branch's remaining fall is left when it reaches its tip.
const RELIEF: f32 = 0.34;

/// Nodes per generated crest, before it is cut. Odd, so a crest has a middle node to be born at.
const NODES: usize = 5;

/// How far a crest wanders sideways at its middle, as a fraction of the angular interval it owns times
/// its length.
///
/// **Scaled by the interval, not by the length alone.** A crest is confined to a wedge, and a crest
/// near the bottom of the network owns a wedge a few hundredths of a radian wide; a wander that was a
/// fixed fraction of its *length* would swing it further sideways than the wedge is wide, and it would
/// cross its own parent's continuation. The third and last way this generator produced a crossing, and
/// the only one where the culprit was shape rather than direction.
const WANDER: f32 = 0.16;

/// The smallest angle a child may leave its parent at, in radians.
const SPLAY_MIN: f32 = 0.08;

/// The largest angle a child may leave its parent at, in radians.
///
/// Kept well under the angle between two divides of a five-fold summit, because a child that leaves
/// by more than that heads into its parent's neighbour and the two cross.
const SPLAY_MAX: f32 = 0.85;

/// How far **in total** a ridge may lean away from the first divide it belongs to, in radians.
///
/// This is the number that makes the network planar, and it is not a shape parameter. A splay is
/// measured from the parent's own bearing, so a chain of them **accumulates**: four children each
/// turning the same way come round by three radians, which is most of a half circle, and a ridge that
/// has come round that far runs back across the divide it grew from. Bounding the *total* lean keeps
/// every descendant inside the wedge its first divide owns — a five-fold summit gives each divide
/// `2π/5`, so half of that with a margin is the whole budget — and inside a wedge nothing can cross
/// the spine of it, because the spine is the middle and its children are on either side of it.
const LEAN_MAX: f32 = 1.15;

/// How many attempts to find a network that does not cross itself.
///
/// A retry loop is the honest way to state "no two crests cross": the constraint is global — it is
/// about every pair of lines at once — and a generator that placed each branch to avoid all the
/// others would be solving the whole planarity problem to draw a mountain.
const ATTEMPTS: u32 = 64;

/// A tiny deterministic generator, so a seed is all a network needs to be reproduced.
///
/// SplitMix64: the standard mixer, used here because a scene's shapes must be **reproducible from a
/// number** rather than from a build's history. Anything that reads the clock would make a picture
/// that cannot be compared with the last one.
struct Random(u64);

impl Random {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// A number in `[0, 1)`.
    fn unit(&mut self) -> f32 {
        (self.next() >> 40) as f32 / (1u32 << 24) as f32
    }

    /// A number in `[-1, 1)`.
    fn signed(&mut self) -> f32 {
        self.unit() * 2.0 - 1.0
    }

    /// A number in `[low, high)`.
    fn between(&mut self, low: f32, high: f32) -> f32 {
        low + self.unit() * (high - low)
    }
}

/// One ridge, before it is cut into the crests a junction forces it to become.
struct Branch {
    /// The crest's own line, from where it is born to where it ends, descending throughout.
    nodes: Vec<[f32; 3]>,
    sharpness: f32,
    depth: f32,
    lean: f32,
    /// The children and the node index of this branch each is born at.
    children: Vec<(usize, Branch)>,
}

/// What a network is grown from: where the summit stands and how much country there is.
pub struct Ground {
    /// Plan position of the summit.
    pub summit: [f32; 2],
    /// The elevation the country around the massif stands at.
    pub base: f32,
    /// The elevation of the summit.
    pub height: f32,
    /// How far a first-order ridge reaches, in plan.
    pub reach: f32,
}

/// Grows a network and returns it as the crests the massif is built from.
///
/// # Invariants
///
/// The returned crests are **planar** — no two cross except where they share an end — and every
/// junction is a shared node, so `ridges()` names each one from the positions rather than by search.
/// Both are checkable and both are checked: the first by [`crosses`], the second by construction.
pub fn crests(ground: &Ground, seed: u64) -> Vec<Crest> {
    for attempt in 0..ATTEMPTS {
        let mut random = Random(seed ^ (attempt as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15));
        let branches = divide(ground, &mut random);
        let mut grown: Vec<Crest> = Vec::new();
        for (index, branch) in branches.iter().enumerate() {
            emit(branch, &format!("r{index}"), &mut grown);
        }
        if !self_crosses(&grown) {
            return grown;
        }
        if std::env::var("NETWORK_TRACE").is_ok() {
            for (index, one) in grown.iter().enumerate() {
                for other in grown.iter().skip(index + 1) {
                    if crossing_points(one, other) > 0 {
                        println!(
                            "  attempt {attempt}: `{}` crosses `{}`",
                            one.name, other.name
                        );
                    }
                }
            }
        }
    }
    // A network that never came out planar is a bug in the parameters rather than bad luck, and
    // silently returning a crossing one would put the failure somewhere else entirely.
    panic!("no planar ridge network in {ATTEMPTS} attempts from seed {seed}");
}

/// Lays the main divide across the plateau.
///
/// **One divide that crosses the country, not a fan out of a point.** The first version radiated six
/// ridges from a single summit, which is what the massif used to be and is the mistake this whole
/// change exists to undo: whatever is done to the branches, a fan of ridges descending from one place
/// is a cone, and the render said so — a smooth dome with black cuts in it. Real dissected country has
/// a **divide** that runs for tens of kilometres, with the spurs leaving it on either side and the
/// valleys between those cutting back into it. The divide is the trunk and the spurs are the
/// branching; there is no single point the land comes from.
///
/// So the interval it owns is most of a half circle, because its children are spurs on both flanks
/// rather than a continuation of it, and its reach is the whole plateau.
fn divide(ground: &Ground, random: &mut Random) -> Vec<Branch> {
    let bearing = random.between(0.0, std::f32::consts::TAU);
    let reach = ground.reach * 2.4;
    vec![grow(
        ground,
        // One end of the plateau, at the height the summit is authored at.
        [
            ground.summit[0] - bearing.cos() * reach * 0.5,
            ground.summit[1] - bearing.sin() * reach * 0.5,
        ],
        bearing,
        -LEAN_MAX,
        LEAN_MAX,
        reach,
        ground.height,
        ORDERS,
        random,
    )]
}

/// Grows one ridge and everything born on it.
///
/// # Invariants
///
/// The branch leans by `lean = (low + high) / 2` from its spine, and **every child gets a half of the
/// interval strictly on its own side of that**: the right child is confined to `(lean, high)` and the
/// left to `(low, lean)`. That is what makes the network planar, and it took two wrong versions to
/// find, both of which produced a ridge that ran back across the one it grew from:
///
/// - measuring a splay from the parent's own bearing lets the splay **accumulate**, so four children
///   each turning the same way come round by three radians and curl back;
/// - bounding the accumulated lean symmetrically still lets a descendant swing to the *other* side of
///   its parent's spine, which is the same crossing one generation further down.
///
/// Confining each child to a half whose boundary is its parent's own line is the version that holds:
/// a branch is the middle of its interval, its children are outside it, and the parent's continuation
/// — straight down the middle — is bounded away from all of them by construction rather than by luck.
fn grow(
    ground: &Ground,
    origin: [f32; 2],
    spine: f32,
    low: f32,
    high: f32,
    reach: f32,
    height: f32,
    order: u32,
    random: &mut Random,
) -> Branch {
    let lean = (low + high) * 0.5;
    let bearing = spine + lean;
    let sharpness = 0.35 + 0.4 * (order as f32 / ORDERS as f32);
    let fall = (height - ground.base) * RELIEF;
    // The line itself: a straight run with a gentle sideways wander and a fall that steepens as it
    // goes, because a ridge is convex — it holds its height and then drops away.
    let mut nodes: Vec<[f32; 3]> = Vec::with_capacity(NODES);
    let across = bearing + std::f32::consts::FRAC_PI_2;
    for node in 0..NODES {
        let along = node as f32 / (NODES - 1) as f32;
        let bend =
            (along * std::f32::consts::PI).sin() * WANDER * (high - low) * reach * random.signed();
        let distance = reach * along;
        nodes.push([
            origin[0] + bearing.cos() * distance + across.cos() * bend,
            origin[1] + bearing.sin() * distance + across.sin() * bend,
            ground.base + (height - ground.base) - fall * along.powf(1.35),
        ]);
    }

    let mut branch = Branch {
        nodes,
        sharpness,
        depth: 0.22 + 0.10 * (order as f32 / ORDERS as f32),
        lean: random.signed() * 0.16,
        children: Vec::new(),
    };
    if order == 0 {
        return branch;
    }
    // The children, born on the nodes between the first and the last so that cutting the parent there
    // leaves a crest on each side of every junction. One per flank, each taking the half of the
    // interval on its own side of the parent's line.
    let flanks = 2.min(NODES - 2);
    let mut taken: Vec<usize> = Vec::new();
    for child in 0..flanks {
        let slot = 1 + (child * (NODES - 2)) / flanks.max(1);
        if slot >= NODES - 1 || taken.contains(&slot) {
            continue;
        }
        taken.push(slot);
        let at = branch.nodes[slot];
        // Half the interval, on one side. `SPLAY_MIN` is the margin that keeps a child clear of its
        // parent's line instead of lying along it, and the interval is refused when the margin has
        // eaten it — which is how the recursion runs out of room to lean rather than of order.
        let (child_low, child_high) = if child % 2 == 0 {
            (lean + SPLAY_MIN, high)
        } else {
            (low, lean - SPLAY_MIN)
        };
        if child_high - child_low < SPLAY_MIN * 0.5 {
            continue;
        }
        // A child is a fraction **of what is left of its parent beyond the junction**, so a spur born
        // near the summit is a long ridge and one born near a tip is a short one. The first version of
        // this added a floor to that fraction, which made a child born early almost as long as the
        // whole ridge it was born on: it then reached out sideways across its own parent and its
        // siblings, and no arrangement of the rest of the network could avoid the crossing.
        let remaining = 1.0 - slot as f32 / (NODES - 1) as f32;
        let child_reach = (reach * remaining * SHRINK).max(ground.reach * 0.05);
        let child_height = at[2];
        let grown = grow(
            ground,
            [at[0], at[1]],
            spine,
            child_low,
            child_high,
            child_reach,
            child_height,
            order - 1,
            random,
        );
        branch.children.push((slot, grown));
    }
    branch
}

/// Writes a branch out as crests, cutting it at every node a child is born at.
///
/// The cut is the whole reason children are born at node indices: the span before a junction and the
/// span after it are two crests that share one node, and sharing it exactly is what lets the expansion
/// weld them into a saddle rather than build two lines that pass close by.
fn emit(branch: &Branch, name: &str, out: &mut Vec<Crest>) {
    let mut cuts: Vec<usize> = branch.children.iter().map(|(slot, _)| *slot).collect();
    cuts.sort_unstable();
    cuts.dedup();
    let mut from = 0;
    for (part, cut) in cuts
        .iter()
        .chain(std::iter::once(&(branch.nodes.len() - 1)))
        .enumerate()
    {
        if *cut <= from {
            continue;
        }
        out.push(Crest {
            name: format!("{name}_{part}"),
            nodes: branch.nodes[from..=*cut].to_vec(),
            sharpness: branch.sharpness,
            depth: branch.depth,
            lean: branch.lean,
        });
        from = *cut;
    }
    for (slot, child) in &branch.children {
        let piece = cuts.iter().position(|cut| cut == slot).unwrap_or(0);
        emit(child, &format!("{name}_{piece}c{}", child.nodes.len()), out);
    }
}

/// Whether any two crests cross in plan anywhere but at a shared end.
///
/// Endpoints are excluded because two crests *are* meant to meet at a junction; what is refused is a
/// crossing in the middle of both, which is a point the ridge graph has no vertex for.
fn self_crosses(crests: &[Crest]) -> bool {
    for (index, one) in crests.iter().enumerate() {
        for other in crests.iter().skip(index + 1) {
            if crossing_points(one, other) > 0 {
                return true;
            }
        }
    }
    false
}

/// How many times two crests cross in plan, ignoring shared ends.
fn crossing_points(one: &Crest, other: &Crest) -> usize {
    let mut count = 0;
    for first in one.nodes.windows(2) {
        for second in other.nodes.windows(2) {
            if segments_cross(
                [first[0][0], first[0][1]],
                [first[1][0], first[1][1]],
                [second[0][0], second[0][1]],
                [second[1][0], second[1][1]],
            ) {
                count += 1;
            }
        }
    }
    count
}

/// Whether two open segments cross **properly**, which is to say anywhere other than at an end they
/// share.
fn segments_cross(a: [f32; 2], b: [f32; 2], c: [f32; 2], d: [f32; 2]) -> bool {
    if same(a, c) || same(a, d) || same(b, c) || same(b, d) {
        return false;
    }
    let side = |from: [f32; 2], to: [f32; 2], at: [f32; 2]| {
        (to[0] - from[0]) * (at[1] - from[1]) - (to[1] - from[1]) * (at[0] - from[0])
    };
    let (d1, d2) = (side(a, b, c), side(a, b, d));
    let (d3, d4) = (side(c, d, a), side(c, d, b));
    (d1 > 0.0) != (d2 > 0.0) && (d3 > 0.0) != (d4 > 0.0)
}

/// Whether two plan points are the same place.
fn same(a: [f32; 2], b: [f32; 2]) -> bool {
    (a[0] - b[0]).abs() < 1.0e-3 && (a[1] - b[1]).abs() < 1.0e-3
}
