//! What an edit may get wrong, and the check that names it.
//!
//! # Why this is a separate call and not a rule
//!
//! An edit may pass through invalid states, and it has to be allowed to: lowering a summit
//! before raising the spur that hangs off it is a legitimate two-step edit whose intermediate
//! state has the spur above its parent. Enforcing the invariants per step would make that edit
//! inexpressible, so they are checked when the author asks instead.
//!
//! # Why coverage is the check that matters
//!
//! Every other violation here is local to one line. Coverage is the one that decides whether
//! the mesh has a hole, and it is a statement about the whole correspondence: for each line, on
//! each side, the stations facing it must account for its whole length exactly once. A gap is a
//! hole in the massif; an overlap is a surface drawn twice. Both are invisible in a list of
//! lines and both are a number here.

use std::collections::BTreeMap;

use crate::line::{LineRef, Side, Station};
use crate::math::{distance, Vec2};
use crate::sampling::{Sampling, Unresolved, WELD};
use crate::skeleton::{RidgeId, Skeleton, ValleyId};
use crate::tessellation::{BeltRef, Tessellation};

/// How much of a line's length may be left over before it is a gap.
///
/// # Invariants
///
/// It is a **parameter** tolerance on a line's own `[0, 1]`, so it does not scale with the
/// massif. It is wide enough to absorb the rounding of a station computed from an arc length
/// and narrow enough that a gap a person would see is far above it.
const COVERAGE_EPSILON: f32 = 1.0e-4;

/// Something an author has got wrong.
#[derive(Debug, Clone, PartialEq)]
pub enum Violation {
    /// A line has too few nodes to have a direction.
    TooFewNodes {
        /// The line.
        line: LineRef,
        /// How many nodes it has.
        nodes: usize,
    },
    /// A line's end is outside the line it names.
    EndOutOfRange {
        /// The line whose end it is.
        line: LineRef,
        /// Whether it is the far end.
        far: bool,
        /// The parameter it asked for.
        t: f32,
    },
    /// A line's end names a line that is not in the skeleton.
    EndNamesMissingLine {
        /// The line whose end it is.
        line: LineRef,
        /// Whether it is the far end.
        far: bool,
        /// The line it named.
        named: LineRef,
    },
    /// A ring of ends closes on itself, so no end on it has a point to stand at.
    EndRing {
        /// A line on the ring.
        line: LineRef,
    },
    /// The ridge lines do not form one tree or network.
    RidgesDisconnected {
        /// How many separate pieces they fall into.
        components: usize,
    },
    /// The expanded mesh is not a closed disc.
    MeshOpen {
        /// How many boundary loops it has; a disc has one, and it is the rim.
        boundaries: usize,
        /// Its Euler characteristic; a disc's is one.
        euler: i64,
    },
    /// A line rises from one node to the next.
    ElevationRises {
        /// The line.
        line: LineRef,
        /// The node it rises from.
        node: usize,
        /// The lower elevation.
        from: f32,
        /// The higher one.
        to: f32,
    },
    /// A station names a parameter off its line.
    StationOutOfRange {
        /// The valley the node belongs to.
        valley: ValleyId,
        /// The node.
        node: usize,
        /// Which side.
        side: Side,
        /// The parameter it asked for.
        t: f32,
    },
    /// A station names a line that is not in the skeleton.
    StationNamesMissingLine {
        /// The valley the node belongs to.
        valley: ValleyId,
        /// The node.
        node: usize,
        /// Which side.
        side: Side,
        /// The line it named.
        line: LineRef,
    },
    /// Two stations a belt has to walk between name lines that do not meet.
    Unwalkable(Unresolved),
    /// A line's side is not accounted for exactly once by the stations facing it.
    NotCovered {
        /// The line.
        line: LineRef,
        /// Which side of it.
        side: Side,
        /// Where the uncovered or doubly covered stretch begins.
        from: f32,
        /// Where it ends.
        to: f32,
    },
    /// A rib crosses a line on the way up the flank.
    RibCrossesLine {
        /// The belt the rib belongs to.
        belt: BeltRef,
        /// The line it crossed.
        line: LineRef,
    },
    /// Two ribs cross, which is two flanks that intersect.
    RibCrossesRib {
        /// The belt one rib belongs to.
        first: BeltRef,
        /// The belt the other belongs to.
        second: BeltRef,
    },
    /// A polyline crosses itself.
    SelfIntersects {
        /// The line.
        line: LineRef,
        /// The edge the crossing starts on.
        from: usize,
        /// The edge it crosses.
        to: usize,
    },
    /// A line leaves the footprint.
    OutsideFootprint {
        /// The line.
        line: LineRef,
        /// The node that is outside.
        node: usize,
    },
}

/// Checks a skeleton, returning everything wrong with it.
///
/// # Invariants
///
/// The structural checks run first and, while any of **them** is failing, the rest are skipped:
/// every check below this point reads a line by index, and a station naming a line that is not
/// there is a violation to report rather than an index to follow. Reporting coverage for a
/// skeleton that cannot be sampled would be a report about a document that is not the one
/// written.
///
/// The result is ordered by the check that raised it and then by the line it is about, so two
/// runs over one skeleton report the same list in the same order and an author can diff two
/// edits by their violation lists.
///
/// The geometric checks — a rib crossing a line, a polyline leaving the footprint — compare
/// every rib against every segment, which is quadratic in the size of the skeleton. That is the
/// right cost here and the wrong one anywhere else: this is called by an author asking what is
/// wrong, a few times a second at most, and never by `build`.
pub fn validate(skeleton: &Skeleton, tessellation: Tessellation) -> Vec<Violation> {
    let mut violations = Vec::new();
    check_shape(skeleton, &mut violations);
    check_ends(skeleton, &mut violations);
    check_stations(skeleton, &mut violations);
    check_ridge_network(skeleton, &mut violations);
    if violations.iter().any(blocks_sampling) {
        return violations;
    }
    check_elevations(skeleton, &mut violations);

    let sampling = Sampling::of(skeleton, tessellation.clone());
    let (belts, problems) = crate::expand::enumerate(skeleton, &sampling);
    for problem in problems {
        if let crate::expand::Problem::Unresolved(unresolved) = problem {
            violations.push(Violation::Unwalkable(unresolved));
        }
    }
    check_coverage(skeleton, &sampling, &belts, &mut violations);
    if skeleton.footprint.len() >= 3 {
        check_self_intersection(skeleton, &mut violations);
        check_ribs(skeleton, &sampling, &belts, &mut violations);
        check_inside(skeleton, &sampling, &mut violations);
        check_mesh(skeleton, tessellation, &mut violations);
    }
    violations
}

/// Checks that the expanded mesh is a closed disc.
///
/// # Invariants
///
/// This is the check the **plan area cannot make**. A hole with no area — which is exactly what a
/// junction left unwelded produces, because the triangles that would have sewn it shut have two
/// corners at one point — contributes nothing to the area and so nothing to the difference the
/// expander's own check compares. Counting boundary loops and the Euler characteristic is not a
/// measurement of the picture; it is a measurement of what the surface *is*, and a disc has one
/// boundary loop and characteristic one.
fn check_mesh(skeleton: &Skeleton, tessellation: Tessellation, violations: &mut Vec<Violation>) {
    let Ok(mesh) = crate::expand::build(skeleton, tessellation) else {
        return;
    };
    let boundaries = mesh.boundary_loops();
    let euler = mesh.euler_characteristic();
    if boundaries != 1 || euler != 1 {
        violations.push(Violation::MeshOpen { boundaries, euler });
    }
}

/// Returns whether a violation stops the checks that read lines by index.
fn blocks_sampling(violation: &Violation) -> bool {
    matches!(
        violation,
        Violation::TooFewNodes { .. }
            | Violation::EndNamesMissingLine { .. }
            | Violation::StationNamesMissingLine { .. }
    )
}

/// Checks that every line has the nodes to have a direction.
fn check_shape(skeleton: &Skeleton, violations: &mut Vec<Violation>) {
    for line in skeleton.lines() {
        let nodes = skeleton.node_count(line);
        if nodes < 2 {
            violations.push(Violation::TooFewNodes { line, nodes });
        }
    }
}

/// Checks that the ridge set is one piece.
///
/// # Invariants
///
/// The spec asks for the ridge lines to form **a tree or a network**, and neither of those is a set
/// of lines that happen to meet. This is the check that distinguishes them: two ridges are
/// connected when one's *end names a station on the other*, not when the two ends stand at the same
/// coordinates. A generated fan whose eight ridges all stood at the origin without naming anything
/// would pass every other check in this file and still be eight ridge lines rather than a mountain's
/// ridge network.
fn check_ridge_network(skeleton: &Skeleton, violations: &mut Vec<Violation>) {
    let components = skeleton.ridge_components();
    if skeleton.ridges.len() > 1 && components > 1 {
        violations.push(Violation::RidgesDisconnected { components });
    }
}

/// Checks that every end names something that exists, and that no ring of ends closes on itself.
fn check_ends(skeleton: &Skeleton, violations: &mut Vec<Violation>) {
    for line in skeleton.lines() {
        for far in [false, true] {
            let Some(station) = skeleton.end(line, far).station() else {
                continue;
            };
            if !(0.0..=1.0).contains(&station.t) {
                violations.push(Violation::EndOutOfRange {
                    line,
                    far,
                    t: station.t,
                });
            }
            if !line_exists(skeleton, station.line) {
                violations.push(Violation::EndNamesMissingLine {
                    line,
                    far,
                    named: station.line,
                });
            }
        }
    }
    // A ring of ends — A's start on B's start, B's start on A's start — has no point to stand at,
    // and the walk that resolves ends would follow it as far as its bound allows and stop there.
    for line in skeleton.lines() {
        let Some(first) = skeleton.end(line, false).station() else {
            continue;
        };
        let mut current = first.line;
        for _ in 0..skeleton.lines().len() {
            if current == line {
                violations.push(Violation::EndRing { line });
                break;
            }
            let Some(next) = skeleton.end(current, false).station() else {
                break;
            };
            current = next.line;
        }
    }
}

/// Checks that nothing rises as it runs away from a summit.
///
/// # Invariants
///
/// The elevations checked are the **authored** ones, and an end that stands on a station is skipped
/// for the node it overrides — its elevation is the station's and is therefore already the other
/// line's. There is no "the child does not meet its parent" check any more, and that is not an
/// omission: an end naming a station *is* that station's point, so the two cannot disagree. The
/// check was possible only while the join was a stored position.
fn check_elevations(skeleton: &Skeleton, violations: &mut Vec<Violation>) {
    for (index, ridge) in skeleton.ridges.iter().enumerate() {
        let standing = [
            ridge.start.station().is_some(),
            ridge.end.station().is_some(),
        ];
        for node in 0..ridge.nodes.len().saturating_sub(1) {
            if (node == 0 && standing[0]) || (node + 1 == ridge.nodes.len() - 1 && standing[1]) {
                continue;
            }
            let (from, to) = (ridge.nodes[node].elevation, ridge.nodes[node + 1].elevation);
            if to > from {
                violations.push(Violation::ElevationRises {
                    line: LineRef::Ridge(RidgeId(index)),
                    node,
                    from,
                    to,
                });
            }
        }
    }
    for (index, valley) in skeleton.valleys.iter().enumerate() {
        for node in 0..valley.nodes.len().saturating_sub(1) {
            let (from, to) = (
                valley.nodes[node].elevation,
                valley.nodes[node + 1].elevation,
            );
            if to > from {
                violations.push(Violation::ElevationRises {
                    line: LineRef::Valley(ValleyId(index)),
                    node,
                    from,
                    to,
                });
            }
        }
    }
}

/// Checks that every station names something that exists.
fn check_stations(skeleton: &Skeleton, violations: &mut Vec<Violation>) {
    for (index, valley) in skeleton.valleys.iter().enumerate() {
        let valley_id = ValleyId(index);
        for (node, entry) in valley.nodes.iter().enumerate() {
            for side in [Side::Left, Side::Right] {
                let station = entry.station(side);
                if !(0.0..=1.0).contains(&station.t) {
                    violations.push(Violation::StationOutOfRange {
                        valley: valley_id,
                        node,
                        side,
                        t: station.t,
                    });
                }
                if !line_exists(skeleton, station.line) {
                    violations.push(Violation::StationNamesMissingLine {
                        valley: valley_id,
                        node,
                        side,
                        line: station.line,
                    });
                }
            }
        }
    }
}

/// Returns whether a skeleton holds a line.
fn line_exists(skeleton: &Skeleton, line: LineRef) -> bool {
    match line {
        LineRef::Ridge(ridge) => ridge.0 < skeleton.ridges.len(),
        LineRef::Valley(valley) => valley.0 < skeleton.valleys.len(),
        LineRef::Footprint => !skeleton.footprint.is_empty(),
    }
}

/// Checks that the stations facing each line's two sides account for it exactly once.
///
/// # Invariants
///
/// Only a line that **something faces** has to be accounted for by stations. A line nothing names
/// is a valley floor, and a valley floor is covered by the two belts that hang off it rather than
/// by ribs from elsewhere — requiring it to be faced would report every fan ever generated, in
/// which no valley names another. A valley that *is* faced on one side and not the other is
/// reported, because that is a region with a belt on one side of it and nothing on the other.
///
/// **Which side a span belongs to is declared by the rib, not measured from the floor.** A belt's
/// rib joins a valley node to the station it names, and the node stands on one side of the line it
/// faces — that is the author's own statement, and it is read at the rib's own station so a bending
/// line is asked at the place the rib meets it. Measuring instead from the midpoint of the two
/// valley nodes between the ribs asks a question of a point nobody placed, and on a wide basin, or
/// one whose two crests part at a shallow angle, that midpoint crosses the crest from one segment
/// to the next: the spans then land on alternating sides, and the report is a crowd of gaps in a
/// flank that the mesh covers once.
///
/// A node standing exactly on its own station declares nothing — that is a head or a corner
/// collapsed to a point — and its belt takes the side the other rib of the same span declared. A
/// span whose two ribs disagree is the valley crossing the line it faces, which is reported as
/// [`Violation::RibCrossesLine`] by [`check_ribs`] rather than guessed at here.
fn check_coverage(
    skeleton: &Skeleton,
    sampling: &Sampling,
    belts: &[crate::expand::Belt],
    violations: &mut Vec<Violation>,
) {
    let mut faced: std::collections::BTreeSet<LineRef> = std::collections::BTreeSet::new();
    for valley in &skeleton.valleys {
        for node in &valley.nodes {
            faced.insert(node.left.line);
            faced.insert(node.right.line);
        }
    }

    let mut tiled: BTreeMap<(LineRef, Side), Vec<(f32, f32)>> = BTreeMap::new();
    for belt in belts {
        let valley = &skeleton.valleys[belt.reference.valley.0];
        let node = belt.reference.segment;
        let (left, right) = (valley.nodes[node], valley.nodes[node + 1]);
        let from = left.station(belt.reference.side);
        let to = right.station(belt.reference.side);
        if from.line == to.line {
            let foot = side_foot(skeleton, from.line, from.t, left.position, right.position);
            let side = rib_side(skeleton, from.line, from.t, left.position)
                .or_else(|| rib_side(skeleton, to.line, to.t, right.position))
                .unwrap_or_else(|| skeleton.side_of(from.line, from.t, foot));
            for (low, high) in crate::line::arc_spans(from.t, to.t, from.line == LineRef::Footprint)
            {
                tiled
                    .entry((from.line, side))
                    .or_default()
                    .push((low, high));
            }
            continue;
        }
        // Two lines that meet: the belt's upper boundary leaves the first at the end that welds to
        // the second and joins the second at that same point, so each line is covered from its
        // station to the corner. The corner is asked of the **sampler**, which is where the weld
        // index is: reading it as the nearer end of the leaving line is the fan's rule again, and
        // it files the rim's arc under the far side of the loop whenever a crest is a segment of a
        // longer chain — which is every hierarchy.
        // On a loop the arc is the **shorter** of the two, the same one the expander walks, and it
        // is split at the seam when it crosses it. A set of parameters that wraps is not an
        // interval, and reading it as one would report the rim's seam as a hole and its far side
        // as covered twice.
        let Some(to_point) = skeleton.station_point(to) else {
            continue;
        };
        let Some((leaving, corner_on_to)) =
            sampling.corner_parameters(from.line, to.line, to_point)
        else {
            continue;
        };
        let closed = from.line == LineRef::Footprint;
        for (line, station, reach, line_closed) in [
            (from.line, from.t, leaving, closed),
            (to.line, to.t, corner_on_to, to.line == LineRef::Footprint),
        ] {
            // The side is asked **of each line and at that line's own station**, from the rib's foot
            // rather than from a point between two floors. Asking it once from the line the path
            // leaves would file the rim's arc under the side its ridge happens to be on, which
            // alternates round a fan and splits the rim's coverage across both halves of every line
            // in it.
            let foot = if line == from.line {
                left.position
            } else {
                right.position
            };
            let other = if line == from.line {
                right.position
            } else {
                left.position
            };
            let side = rib_side(skeleton, line, station, foot).unwrap_or_else(|| {
                skeleton.side_of(
                    line,
                    station,
                    side_foot(skeleton, line, station, foot, other),
                )
            });
            for (low, high) in crate::line::arc_spans(station, reach, line_closed) {
                tiled.entry((line, side)).or_default().push((low, high));
            }
        }
    }

    for ((line, side), spans) in &tiled {
        let mut spans = spans.clone();
        spans.sort_by(|left, right| left.0.total_cmp(&right.0));
        let mut reached = 0.0f32;
        for (low, high) in spans {
            // An interval of no length covers nothing and decides nothing: it is what a corner
            // leaves behind when both of a belt's stations name the same point, which is how a
            // summit and a spur junction close.
            if high - low <= COVERAGE_EPSILON {
                continue;
            }
            // A stretch starting after what is already covered is a hole; one starting before it
            // is a surface drawn twice over the overlap and no more.
            if low > reached + COVERAGE_EPSILON {
                violations.push(Violation::NotCovered {
                    line: *line,
                    side: *side,
                    from: reached,
                    to: low,
                });
            } else if low < reached - COVERAGE_EPSILON {
                violations.push(Violation::NotCovered {
                    line: *line,
                    side: *side,
                    from: low,
                    to: reached.min(high),
                });
            }
            reached = reached.max(high);
        }
        if reached < 1.0 - COVERAGE_EPSILON {
            violations.push(Violation::NotCovered {
                line: *line,
                side: *side,
                from: reached,
                to: 1.0,
            });
        }
    }
    // A line that something faces but that no interval mentions is uncovered from end to end,
    // and the walk above never starts on it because there was no interval to start one. The
    // footprint is exempt: it is the domain the massif is cut into, and the land reaches it
    // rather than facing it.
    for line in skeleton.lines() {
        if line == LineRef::Footprint || !faced.contains(&line) {
            continue;
        }
        for side in [Side::Left, Side::Right] {
            if !tiled.contains_key(&(line, side)) {
                violations.push(Violation::NotCovered {
                    line,
                    side,
                    from: 0.0,
                    to: 1.0,
                });
            }
        }
    }
}

/// Returns the foot a rib with no length declares its side from.
///
/// # Invariants
///
/// A rib of no length — a head or a corner collapsed to a point — points nowhere, so it declares no
/// side of its own and the side is read from the **valley's other floor node**, the direction the
/// valley descends in. Reading `side_of` at the coincident foot instead reads the sign of a zero
/// cross product, and two valleys sharing one head then file their spans under the same side of the
/// crest and leave the opposite flank bare.
fn side_foot(skeleton: &Skeleton, line: LineRef, t: f32, foot: Vec2, other: Vec2) -> Vec2 {
    if rib_side(skeleton, line, t, foot).is_some() {
        foot
    } else {
        other
    }
}

/// Returns the side of a line a rib's foot stands on, or nothing when the rib has no length.
///
/// A rib of no length is a node standing exactly on the station it names, which is how a head and a
/// corner close; it declares no side because it points nowhere, and its belt is attributed by the
/// rib at its other end.
fn rib_side(skeleton: &Skeleton, line: LineRef, t: f32, foot: Vec2) -> Option<Side> {
    let at = skeleton.station_position(Station::new(line, t))?;
    if distance(at, foot) <= crate::sampling::WELD {
        return None;
    }
    Some(skeleton.side_of(line, t, foot))
}

/// Checks that no polyline crosses itself.
fn check_self_intersection(skeleton: &Skeleton, violations: &mut Vec<Violation>) {
    for line in skeleton.lines() {
        let nodes = skeleton.nodes(line);
        if nodes.len() < 4 {
            continue;
        }
        let last = nodes.len() - 2;
        for from in 0..=last {
            for to in from + 2..=last {
                // Edges that share a node are neighbours and meet by construction.
                if from + 1 == to {
                    continue;
                }
                if crosses(
                    nodes[from].position,
                    nodes[from + 1].position,
                    nodes[to].position,
                    nodes[to + 1].position,
                ) {
                    violations.push(Violation::SelfIntersects { line, from, to });
                }
            }
        }
    }
}

/// Checks that no rib crosses a line on its way up the flank, and that no two ribs cross.
fn check_ribs(
    skeleton: &Skeleton,
    sampling: &Sampling,
    belts: &[crate::expand::Belt],
    violations: &mut Vec<Violation>,
) {
    let mut ribs: Vec<(BeltRef, LineRef, Vec2, Vec2)> = Vec::new();
    for belt in belts {
        let valley = &skeleton.valleys[belt.reference.valley.0];
        for (offset, station) in [
            (
                0usize,
                valley.nodes[belt.reference.segment].station(belt.reference.side),
            ),
            (
                1,
                valley.nodes[belt.reference.segment + 1].station(belt.reference.side),
            ),
        ] {
            let (Some(foot), Some(head)) = (
                sampling.node_vertex(
                    skeleton,
                    LineRef::Valley(belt.reference.valley),
                    belt.reference.segment + offset,
                ),
                sampling.vertex(station).ok(),
            ) else {
                continue;
            };
            ribs.push((
                belt.reference,
                station.line,
                sampling.sample(foot).position,
                sampling.sample(head).position,
            ));
        }
    }

    for (reference, named, foot, head) in &ribs {
        for line in skeleton.lines() {
            if line == *named {
                continue;
            }
            let nodes = skeleton.nodes(line);
            for pair in nodes.windows(2) {
                if crosses(*foot, *head, pair[0].position, pair[1].position) {
                    violations.push(Violation::RibCrossesLine {
                        belt: *reference,
                        line,
                    });
                    break;
                }
            }
        }
    }
    for (index, (reference, _, foot, head)) in ribs.iter().enumerate() {
        for (other, _, other_foot, other_head) in ribs.iter().skip(index + 1) {
            if crosses(*foot, *head, *other_foot, *other_head) {
                violations.push(Violation::RibCrossesRib {
                    first: *reference,
                    second: *other,
                });
                break;
            }
        }
    }
}

/// Checks that every sample of every line is inside the footprint.
fn check_inside(skeleton: &Skeleton, sampling: &Sampling, violations: &mut Vec<Violation>) {
    let outline: Vec<Vec2> = skeleton
        .footprint
        .iter()
        .map(|node| node.position)
        .collect();
    for line in skeleton.lines() {
        if line == LineRef::Footprint {
            continue;
        }
        for (node, sample) in sampling.run(line).samples.iter().enumerate() {
            if inside(&outline, sample.position) {
                continue;
            }
            violations.push(Violation::OutsideFootprint { line, node });
        }
    }
}

/// Returns whether a plan point is inside a simple closed polygon, by ray casting.
///
/// # Invariants
///
/// A point **on** the outline counts as inside. Every ridge tip and every valley mouth is a
/// footprint vertex, so a boundary point is the ordinary case here and not a degenerate one: a
/// test that called it outside would report every ridge in the massif, at the node where it
/// reaches the rim.
///
/// The polygon is the authored loop **without** the repeat [`crate::Skeleton::nodes`] adds, so the
/// closing edge is counted once. The ray is along `+x` and a vertex is counted by the half-open
/// rule `(a.z > p.z) != (b.z > p.z)`, which is what stops a vertex the ray passes exactly through
/// being counted twice.
pub fn inside(outline: &[Vec2], point: Vec2) -> bool {
    if outline.len() < 3 {
        return false;
    }
    for index in 0..outline.len() {
        let from = outline[index];
        let to = outline[(index + 1) % outline.len()];
        if distance_to_segment(point, from, to) <= WELD {
            return true;
        }
    }
    let mut inside = false;
    let mut previous = outline[outline.len() - 1];
    for &current in outline {
        if (current[1] > point[1]) != (previous[1] > point[1]) {
            let width = previous[1] - current[1];
            if width.abs() > f32::EPSILON {
                let crossing =
                    current[0] + (point[1] - current[1]) / width * (previous[0] - current[0]);
                if point[0] < crossing {
                    inside = !inside;
                }
            }
        }
        previous = current;
    }
    inside
}

/// Returns how far a plan point is from a plan segment.
fn distance_to_segment(point: Vec2, from: Vec2, to: Vec2) -> f32 {
    let edge = crate::math::difference(to, from);
    let length_squared = crate::math::dot(edge, edge);
    let amount = if length_squared > f32::EPSILON {
        crate::math::clamp(
            crate::math::dot(crate::math::difference(point, from), edge) / length_squared,
            0.0,
            1.0,
        )
    } else {
        0.0
    };
    distance(point, crate::math::lerp(from, to, amount))
}

/// Returns whether two plan segments cross in their interiors.
///
/// # Invariants
///
/// A shared endpoint is **not** a crossing, and neither is a proper touch when the two are
/// welded apart by less than [`WELD`]: a rib meets the line it climbs to by construction, and a
/// test that called that a crossing would report every rib in the skeleton. That is what the
/// tolerance is for, and it is the same tolerance the corner resolution uses.
fn crosses(a: Vec2, b: Vec2, c: Vec2, d: Vec2) -> bool {
    let first = crate::math::cross(crate::math::difference(b, a), crate::math::difference(c, a));
    let second = crate::math::cross(crate::math::difference(b, a), crate::math::difference(d, a));
    let third = crate::math::cross(crate::math::difference(d, c), crate::math::difference(a, c));
    let fourth = crate::math::cross(crate::math::difference(d, c), crate::math::difference(b, c));
    let straddles = |left: f32, right: f32, tolerance: f32| {
        (left > tolerance && right < -tolerance) || (left < -tolerance && right > tolerance)
    };
    let scale = (distance(a, b) * distance(c, d)).max(f32::EPSILON);
    let tolerance = WELD * scale;
    straddles(first, second, tolerance) && straddles(third, fourth, tolerance)
}
