//! Plan-polygon clipping: **the area two simple polygons share**, computed rather than guessed at.
//!
//! # Why this exists
//!
//! `validate` has to say whether two masses' plans share area. It used to ask four predicates — one polygon
//! contains a vertex of the other, or its edges properly cross — and that test missed the shape two rectangles
//! sharing a wall have: **collinear** shared edges and a corner touching an edge are neither containment nor a
//! proper crossing, and a 0.15 m strip of two buildings' walls went unreported.
//!
//! An edge-midpoint predicate closed the case the crate draws, and its own docs said it was a detector rather than
//! a proof. This module is the proof: two polygons are ear-clipped into triangles, every pair of triangles is
//! clipped against each other, and the areas are summed. A flush pair shares **exactly** zero, not "no vertex
//! inside".
//!
//! # Invariants
//!
//! - **Both polygons are simple** (non-self-intersecting), and **this was an assumption this module's
//!   callers could not all promise**: `validate` reports a reversed or too-short outline, and a loop
//!   that meets itself is named before anything here is asked of it — **a void loop's by
//!   [`Violation::VoidNotSimple`](crate::validate::Violation::VoidNotSimple) and a mass outline's by
//!   [`Violation::OutlineNotSimple`](crate::validate::Violation::OutlineNotSimple), which is the gate
//!   the overlap subtraction never had until the two were given one reading between them**.
//!   `courtyard_probe`'s section on a void loop that meets itself measured what happens when the
//!   assumption is broken, and the answer was that the numbers this module returned were whatever its
//!   ear clipper happened to produce, so a caller comparing two of them was reading the clipper and
//!   not the shape — the `+8` and `-8` of a symmetric bowtie gave `8.0` against `12.0` and the same
//!   bowtie as an hourglass gave `8.0` against `8.0`. **Nothing here is wrong for a simple polygon**;
//!   the reading simply stops being about the polygon the moment it stops being one, which is why
//!   [`Violation::VoidNotInside`](crate::validate::Violation::VoidNotInside) is asked only where
//!   [`Violation::VoidNotSimple`](crate::validate::Violation::VoidNotSimple) has nothing to say.
//!
//!   **An assumption is not a proof, so the walk now proves it of itself rather than relying on it**:
//!   [`shared_area`] answers `None` for a loop it could not cover, and every caller declines rather
//!   than measure a number it cannot stand behind. See [`triangles`].
//! - **The result is a sum of exact triangle intersections**, so it is exact for every loop this
//!   module answers for. **Measured rather than assumed:** a polygon with **collinear** corners is
//!   covered exactly — a `4 × 4` rectangle with one node inserted on an edge reads `16.0000` against
//!   its own `16.0000`, one with a node on every edge likewise, and a `0.0001 m` sliver four metres
//!   long reads `15.9996` against `15.9996`. **The one input measured that lost real area was an
//!   edge of no length**, and it is taken out of the loop before the walk rather than measured around.
//! - **An edge of no length is removed first**, by [`without_edges_of_no_length`]. Such an edge bounds no
//!   area and no part of the interior, so the loop it hands the clipper is the same region and the clipper
//!   is free to triangulate it. **This is not a nicety: [`ops::split_outline`](crate::ops::split_outline)
//!   writes one on purpose** — "a station that lands exactly on an existing node still inserts one" — and
//!   measured *before* the removal, the clipper lost half of a `16.0000 m²` loop, which made
//!   [`validate`](crate::validate) **miss two faults**: a void loop `4 cm` outside its own outline, and two
//!   masses standing `50.0000 m²` in one another, both reported as sharing nothing at all. The walk still
//!   names the degenerate edge itself, through
//!   [`Problem::OutlineEdgeDegenerate`](crate::expand::Problem::OutlineEdgeDegenerate) and
//!   [`Violation::DidNotExpand`](crate::validate::Violation::DidNotExpand), so measuring it here silences
//!   nothing — it gives the other checks an exact number to subtract.
//! - **A loop this module cannot cover is `None` and never a number.** Ear clipping **stops rather than
//!   loops** when no corner is an ear, and the triangles it has already cut are not then the loop, so
//!   their sum would be a measurement of the clipper. **`None` is decided by one test, [`covers`],
//!   and not by the walk stopping** — a stopped walk can still have cut triangles that add up to the
//!   loop, and measured one does: a `1 m` slit four metres long reads `12.0000` against a loop of
//!   `12.0000`. **The failures it does catch, measured:** a bowtie reads `16.0000` against a loop of
//!   `0.0000`, a spike that doubles back over its own nodes `38.3333` against `10.0000`, and a
//!   **needle that doubles back along its own line over the very node it repeats** — a loop
//!   [`Violation::VoidNotSimple`](crate::validate::Violation::VoidNotSimple) is deliberately silent
//!   about, because the meeting is at a point its author wrote down — reads `32.0000` against a loop
//!   of `64.0000`.

use crate::math::Vec2;

/// How far a set of ear-clipped triangles may fail to add up to the loop's own area before the walk
/// is treated as having covered nothing.
///
/// # Invariants
///
/// **It is a fraction of the loop's own area and not an absolute number in plan units**, because the
/// noise it absorbs is `f32` rounding accumulated over `O(nodes)` additions and therefore grows with
/// the building: an absolute threshold would reject a large exact answer and accept a small wrong one.
/// **The value sits between two measured things.** Above it, `f32` noise: every loop that reached
/// this test in `courtyard_probe` — the concave plans, the bite sweep, the pinch sweep, all nine
/// gallery scenes' own plans — printed its triangle sum and its own shoelace as the **same four
/// decimals**, so the noise this absorbs is under `1.0e-4` of the loop's area and the threshold is
/// safely above it. Below it, every failure this module has been measured on, and the **smallest**
/// of them carries a relative departure of `0.5`: a pinch reading `9.0000` against a loop of
/// `18.0000` and a needle reading `32.0000` against `64.0000`, against `0.667` for a dart and a
/// triangle and `2.83` for a spike that doubles back over its own nodes. **So the slack is two and a
/// half decades below the smallest fault and two and a half above the largest noise.**
const COVER_SLACK: f32 = 1.0e-3;

/// Returns the area two simple plan polygons share, in plan units squared, or `None` when either loop
/// has no triangulation to measure.
///
/// # Invariants
///
/// **A flush pair answers exactly zero**: two masses standing against each other share a boundary and no area, and
/// computing rather than testing is what makes that "no area" rather than "no predicate fired".
///
/// **It answers `None` rather than a partial sum, and that is the whole of what this function is for.**
/// The triangles of a loop the ear walk could not finish are not the loop, so their pairwise
/// intersections are a measurement of the clipper: measured, a spike that doubles back over its own
/// nodes reads `38.3333` against a loop of `10.0000` and a needle that doubles back along its own
/// line over the very node it repeats reads `32.0000` against a loop of `64.0000`. **A caller that
/// received either number would subtract it and report the difference as a fault**, so the number is
/// withheld until [`triangles`] has said the loop was covered. What a caller does with "cannot
/// measure" is its own decision, and every one of them in this crate declines rather than guess.
pub fn shared_area(first: &[Vec2], second: &[Vec2]) -> Option<f32> {
    let first = triangles(first)?;
    let second = triangles(second)?;
    let mut total = 0.0;
    for one in &first {
        for other in &second {
            total += triangle_overlap(*one, *other);
        }
    }
    Some(total.max(0.0))
}

/// Returns a simple polygon's triangles, by ear clipping, or `None` when the walk could not cover it.
///
/// # Invariants
///
/// The polygon is wound counter-clockwise first, so a corner is convex exactly when its cross product is
/// positive — the same sign convention every other plan test in this crate reads.
///
/// **An edge of no length is out of the loop before the walk starts**, by [`without_edges_of_no_length`],
/// because a corner between two nodes at one place is never strictly convex and blocks every ear it stands
/// inside, so leaving one in stops the walk with the loop unbroken.
///
/// **`None` is this function saying it cannot measure, and [`covers`] is how it says so — one test
/// and not two.** The ear walk stopping with the loop unbroken is a *symptom* of that, not the
/// reading: **a walk that stopped can still have cut triangles that add up to the loop**, and measured,
/// one does — a `1 m` slit cut four metres down from a square's north edge stops the walk, and the
/// triangles it stopped with read `12.0000` against a loop of `12.0000`, which is the right number for
/// that loop. Declining on the walk's word alone would have withheld a measurement this module can
/// support, so the test is the one that asks the only question that matters: **are the triangles the
/// loop?** That is what a triangulation *is*, it holds for a partial one exactly when the partial one
/// happens to tile the whole, and it costs `O(triangles)` on numbers already in hand.
///
/// **The triangles that fail it are the ones measured here, and one of them was in the tree the whole
/// time.** A bowtie reads `16.0000` against a loop of `0.0000`, a spike that doubles back over its own
/// nodes `38.3333` against `10.0000`, a needle that doubles back along its own line over the very node
/// it repeats `32.0000` against `64.0000` — and **an authored pinch, which [`Violation::VoidNotSimple`]
/// is deliberately silent about, reads `9.0000` against a loop of `18.0000`**: two `3 × 3` squares
/// sharing a corner, and the ear walk can only cut one lobe because the node the pinch names stands
/// inside every ear across it. **That one was cancelling rather than measuring** — the two terms
/// `VoidNotInside` subtracts were both `9.0000`, so the check stayed silent and the sentence in the
/// record that said the two areas "were measured equal" was true of the subtraction and false of the
/// clipper.
///
/// **A walk that reaches its last triangle is not the same as a walk that covered the loop**, and
/// treating them as the same was the defect this `None` removes.
fn triangles(polygon: &[Vec2]) -> Option<Vec<[Vec2; 3]>> {
    let mut remaining = without_edges_of_no_length(polygon);
    if remaining.len() < 3 {
        return None;
    }
    if signed_area(&remaining) < 0.0 {
        remaining.reverse();
    }
    // **The loop's own area is read once, here, because the walk consumes `remaining`**: by the time
    // the last triangle is left in it, the variable holds three nodes and its shoelace is that
    // triangle's area rather than the loop's. `covers` compares the triangles against the loop.
    let loop_area = signed_area(&remaining).abs();
    let mut ears = Vec::new();
    while remaining.len() > 3 {
        let mut clipped = false;
        for index in 0..remaining.len() {
            let count = remaining.len();
            let previous = remaining[(index + count - 1) % count];
            let corner = remaining[index];
            let next = remaining[(index + 1) % count];
            if cross(subtract2(corner, previous), subtract2(next, corner)) <= 0.0 {
                continue;
            }
            let holds_a_vertex = remaining.iter().enumerate().any(|(at, point)| {
                at != index
                    && at != (index + count - 1) % count
                    && at != (index + 1) % count
                    && inside_triangle(*point, previous, corner, next)
            });
            if holds_a_vertex {
                continue;
            }
            ears.push([previous, corner, next]);
            remaining.remove(index);
            clipped = true;
            break;
        }
        if !clipped {
            break;
        }
    }
    if remaining.len() == 3 {
        ears.push([remaining[0], remaining[1], remaining[2]]);
    }
    covers(loop_area, &ears).then_some(ears)
}

/// Returns whether a set of ear-clipped triangles adds up to the area of the loop they came from.
///
/// # Invariants
///
/// **A triangulation of a loop is a tiling of it**, so the areas of its triangles are the area of the
/// loop — that is the definition of the word rather than an extra property of this walk, and it is
/// what lets the sum stand in for the loop. **A partial triangulation is not a tiling of anything**,
/// so the sum is a measurement of the walk rather than of the polygon, and the two are told apart by
/// exactly this comparison.
///
/// **The slack is relative to the loop's own area and not an absolute area**, because the noise being
/// absorbed is `f32` rounding over `O(nodes)` additions: it grows with the polygon, so an absolute
/// threshold would call a large building's exact answer wrong and a small one's wrong answer right.
/// [`COVER_SLACK`] is two and a half decades above that noise and below the departure every measured
/// failure carries — `16.0000` against a loop of `0.0000`, `38.3333` against `10.0000`, `32.0000`
/// against `64.0000` and `9.0000` against `18.0000` — and the `1.0` floor is what makes a loop of
/// almost no area comparable with a large one rather than dividing by something near zero.
///
/// **An authored pinch is on the failing side of this, and that is measured rather than a limit of the
/// test.** Two squares meeting at a point the author wrote down are not simple, the node they meet at
/// stands inside every ear that spans it, and the walk can only cut one lobe: `9.0000` against
/// `18.0000`. **A loop that meets itself has no triangulation to check**, and this says so rather than
/// returning a half.
///
/// `O(triangles)`, one pass and one comparison per triangle.
fn covers(loop_area: f32, ears: &[[Vec2; 3]]) -> bool {
    let covered: f32 = ears
        .iter()
        .map(|held| signed_area(held.as_slice()).abs())
        .sum();
    (covered - loop_area).abs() <= COVER_SLACK * loop_area.max(1.0)
}

/// Returns a loop with every edge of no length taken out of it.
///
/// # Invariants
///
/// **It changes no area and no part of the interior, which is the whole permission to do it.** A segment
/// whose two ends stand at one place bounds nothing, so the region either loop encloses is the region this
/// one encloses, and every area computed from the returned loop is the area of the loop handed in.
///
/// **It runs before the ear clipper rather than inside it, and the clipper is the reason.** Ear clipping
/// needs a corner that is convex **strictly** — `cross > 0` — so a corner between two nodes at one place is
/// never an ear, and it also **blocks** every ear whose triangle it stands inside, boundary included. On a
/// loop carrying such a corner **no** corner is an ear, the walk stops with the loop unbroken, and the final
/// triangle is dropped with it: a `4 × 4` loop with one node written down twice read `8.0000` against its
/// own `16.0000`. **`Vec::dedup` is the whole removal for the open chain** and the closing edge is the one
/// case it cannot see — a loop whose last node stands at its first names that edge too, so it is taken off
/// the end here.
///
/// **The test is exact equality and not a tolerance**, because this is the same arithmetic the clipper itself
/// runs: `cross` and `inside_triangle` compare at `0.0`, so an edge that is very short rather than exactly
/// zero is left in and the loop is clipped as the author wrote it. **The walk is what names such an edge** —
/// [`Problem::OutlineEdgeDegenerate`](crate::expand::Problem::OutlineEdgeDegenerate), at its own
/// `LENGTH_EPSILON` — so a near-degenerate edge is still reported by name and is not silently dropped here.
///
/// `O(nodes)`, one pass and one comparison per node.
fn without_edges_of_no_length(polygon: &[Vec2]) -> Vec<Vec2> {
    let mut held = polygon.to_vec();
    held.dedup();
    if held.len() > 1 && held[held.len() - 1] == held[0] {
        held.pop();
    }
    held
}

/// Returns how much of two triangles is shared.
///
/// # Invariants
///
/// The first triangle is clipped by each of the second's three edges in turn, which is a convex clip because a
/// triangle is convex — so the result is the exact intersection polygon and its area is exact.
fn triangle_overlap(first: [Vec2; 3], second: [Vec2; 3]) -> f32 {
    let mut held: Vec<Vec2> = first.to_vec();
    for index in 0..3 {
        let from = second[index];
        let to = second[(index + 1) % 3];
        held = clip_to_left(held, from, to);
        if held.len() < 3 {
            return 0.0;
        }
    }
    signed_area(&held).abs()
}

/// Returns a polygon clipped to the left of the directed line `from → to`.
///
/// # Invariants
///
/// A segment that crosses the line is cut at the crossing; a segment wholly on the kept side is kept whole; a
/// segment wholly on the cut side is dropped. A point **on** the line is kept, so a shared edge survives the clip
/// and contributes zero area rather than dropping a face.
fn clip_to_left(polygon: Vec<Vec2>, from: Vec2, to: Vec2) -> Vec<Vec2> {
    if polygon.is_empty() {
        return polygon;
    }
    let direction = subtract2(to, from);
    let side = |point: Vec2| cross(direction, subtract2(point, from));
    let mut kept = Vec::new();
    for index in 0..polygon.len() {
        let current = polygon[index];
        let next = polygon[(index + 1) % polygon.len()];
        let current_side = side(current);
        let next_side = side(next);
        if current_side >= 0.0 {
            kept.push(current);
        }
        if (current_side > 0.0 && next_side < 0.0) || (current_side < 0.0 && next_side > 0.0) {
            let ratio = current_side / (current_side - next_side);
            kept.push([
                current[0] + (next[0] - current[0]) * ratio,
                current[1] + (next[1] - current[1]) * ratio,
            ]);
        }
    }
    kept
}

/// Returns a polygon's signed area, positive when wound counter-clockwise.
fn signed_area(polygon: &[Vec2]) -> f32 {
    let mut twice = 0.0;
    for index in 0..polygon.len() {
        let current = polygon[index];
        let next = polygon[(index + 1) % polygon.len()];
        twice += current[0] * next[1] - next[0] * current[1];
    }
    twice * 0.5
}

/// Returns whether a point stands inside a triangle, corners included.
fn inside_triangle(point: Vec2, a: Vec2, b: Vec2, c: Vec2) -> bool {
    let one = cross(subtract2(b, a), subtract2(point, a));
    let two = cross(subtract2(c, b), subtract2(point, b));
    let three = cross(subtract2(a, c), subtract2(point, c));
    (one >= 0.0 && two >= 0.0 && three >= 0.0) || (one <= 0.0 && two <= 0.0 && three <= 0.0)
}

/// Returns the two-dimensional cross product, which is the sign of a corner's turn.
fn cross(first: Vec2, second: Vec2) -> f32 {
    first[0] * second[1] - first[1] * second[0]
}

/// Returns the plan difference of two points.
fn subtract2(first: Vec2, second: Vec2) -> Vec2 {
    [first[0] - second[0], first[1] - second[1]]
}
