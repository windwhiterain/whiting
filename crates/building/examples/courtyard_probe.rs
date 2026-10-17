//! The concave shapes, measured as a matrix rather than one case.
//!
//! # What this pins
//!
//! `courtyard_probe` measured one U-shaped mass and found it refused; `courtyard_widths` proved the refusal was a
//! defect and not a boundary, and the strict crossing test fixed it. This probe is the record of what the fix
//! bought: **three concave plans the crate could not draw before**, each with the wall the crate would use for it.
//!
//! - **U** — a courtyard open to the north: two reflex corners at the mouth.
//! - **L** — a wing off a square: one reflex corner.
//! - **porch** — a shallow notch two metres deep: the case a cottage's front door wants.
//!
//! Each must report **no violations, a manifold solid and zero boundary loops**. A volume is printed for the
//! record rather than asserted: what makes a solid a solid is the first three readings, and the volume is what a
//! later change would move if it moved the geometry.
//!
//! # The same matrix on the outward walk
//!
//! The three plans above are **outlines**, and an outline is offset **inward**. A courtyard's void loop is offset
//! **outward** — the ring stands outside it — so the corner that fails in one is not the corner that fails in the
//! other, and a void with a reflex corner was refused by name rather than walked. The second half of this probe
//! measures that walk: **the offset loop is read back out of the drawn mesh**, not recomputed, so the instrument
//! can disagree with the crate, and it is compared against **plan geometry's own answer**
//! (`area + wall · perimeter + wall² · Σ tan(turn / 2)`), which is arithmetic on the authored loop and shares no
//! code with the walk.
//!
//! # The one class neither half reached: a loop that meets itself
//!
//! The concave half asks which *reflex* corners a ring can have, and the sweep beside it asks how narrow a bite
//! can get. Neither asks what happens when the loop's edges cross or touch, which is what the proposal's "where
//! the system stands" left open. Seven shapes answer it here, and they are seven different questions rather than
//! seven sizes of one: a **bowtie** and an **hourglass** both cross with a signed area of exactly zero, so the
//! expander's winding normalisation has nothing to normalise; an **asymmetric bowtie** crosses with a signed area
//! that is not zero, so the expander *reverses* it, and a reversal of a self-intersecting loop turns each edge's
//! outward side the other way round; a **T-junction** touches at a point the author never wrote down, which the
//! walk's crossing sweep cannot see at all because `segments_cross_properly` needs each segment to have the
//! other's endpoints *strictly* on opposite sides; a **crossing the author wrote down** puts the crossing point in
//! the node list, which is a third reading again and the one that says whether the check names a contact or a
//! count of points on it.
//!
//! **The two shapes that must stay silent are the other half of the finding.** **Two squares touching at a
//! point** and a **dart and a triangle sharing a point the author wrote down twice** are both not simple, and both
//! build: `validate` reports nothing, the ring is edge-manifold with no boundary loop and Euler `0`, and its volume
//! is `(offset − loop) × height` to `4 × 10⁻⁶`. The second is there so the exclusion is measured on a lobe that is
//! reflex rather than on the one arrangement that happens to work; an argument would not have settled it.
//!
//! **Each is read three ways and none may answer for another**: what `validate` reports about the document, what
//! `build` does with the loop, and — only where a solid came out — its volume against plan geometry's own answer
//! and its manifold, closure and Euler readings. A refusal from one and a solid from the other is the finding.
//!
//! Run with `cargo run -p whiting-building --example courtyard_probe`.

use whiting_building::math::{Vec2, cross, difference, dot, length};
use whiting_building::mesh::{Element, MassSurface};
use whiting_building::{Building, BuildingMesh, BuildingSkeleton, Mass, MassId, PlanNode, SolidId};

/// Returns an outline from plan corners at one elevation.
fn outline(corners: &[[f32; 2]]) -> Vec<PlanNode> {
    corners
        .iter()
        .map(|corner| PlanNode::new(*corner, 0.0))
        .collect()
}

/// Answers whether one concave plan expands into one watertight solid.
fn measure(label: &str, corners: &[[f32; 2]], wall: f32) {
    let mut skeleton = BuildingSkeleton::default();
    skeleton.masses = vec![
        Mass::new(outline(corners), 3.2)
            .with_wall(wall)
            .with_slab(0.4),
    ];
    let building = Building::new(skeleton);
    let violations = building.validate();
    match building.build() {
        Ok(built) => {
            let solid = SolidId::Mass(MassId(0));
            println!(
                "  {label:<7} {} node(s), wall {wall:.2}: {} violation(s), manifold {}, loops {}, volume {:.4}",
                corners.len(),
                violations.len(),
                built.solid_is_manifold(solid),
                built.solid_boundary_loops(solid),
                built.solid_signed_volume(solid)
            );
        }
        Err(error) => println!(
            "  {label:<7} {} node(s), wall {wall:.2}: {} violation(s), does not build: {:?}",
            corners.len(),
            violations.len(),
            error.problems.first()
        ),
    }
}

// ── the outward walk ────────────────────────────────────────────────────────────────
// Everything below measures `expand::offset_loop` with `OffsetSide::Outward`, which is what
// builds a courtyard's ring, and it reads the walk back out of the geometry rather than
// recomputing it: a probe that ran the same code twice could not have disagreed with it.

/// The tolerance every reading below is compared at, which is `f32` noise on a plan of this size.
const TOLERANCE: f32 = 1.0e-5;

/// Returns the shape's plan positions wound so the loop's own area is positive.
///
/// The expander normalises a void loop the same way before it walks it, and which way a loop is
/// wound is what says which side the material is on, so the probe reads the loop the walk saw.
fn wound_positive(corners: &[[f32; 2]]) -> Vec<Vec2> {
    let mut points: Vec<Vec2> = corners.to_vec();
    if shoelace(&points) < 0.0 {
        points.reverse();
    }
    points
}

/// Returns the area a loop encloses, by the shoelace formula.
fn shoelace(loop_points: &[Vec2]) -> f32 {
    let count = loop_points.len();
    (0..count)
        .map(|index| cross(loop_points[index], loop_points[(index + 1) % count]))
        .sum::<f32>()
        * 0.5
}

/// Returns the length of a loop's boundary.
fn perimeter(loop_points: &[Vec2]) -> f32 {
    let count = loop_points.len();
    (0..count)
        .map(|index| {
            length(difference(
                loop_points[(index + 1) % count],
                loop_points[index],
            ))
        })
        .sum()
}

/// Returns the signed turn at one node, in radians: positive where the loop turns to its left.
fn turn_at(loop_points: &[Vec2], node: usize) -> f32 {
    let count = loop_points.len();
    let coming = difference(loop_points[node], loop_points[(node + count - 1) % count]);
    let going = difference(loop_points[(node + 1) % count], loop_points[node]);
    cross(coming, going).atan2(dot(coming, going))
}

/// Returns the area a **mitred** offset of the loop by `wall` encloses, from plan geometry alone.
///
/// # Invariants
///
/// **The answer is three numbers read off the authored loop and no reading of the walk.** Every edge's
/// strip adds `wall × length`, and every corner adds `wall² × tan(turn / 2)`: positive where the
/// corner turns the loop's own way and **negative where it turns back**, which is the whole content
/// of the reflex case. A corner that the walk mitred to the wrong side, or a loop that came back
/// folded or touching, cannot match this number, so the comparison is one that can disagree.
fn offset_area_answer(loop_points: &[Vec2], wall: f32) -> f32 {
    let count = loop_points.len();
    let corners: f32 = (0..count)
        .map(|node| (turn_at(loop_points, node) * 0.5).tan())
        .sum();
    shoelace(loop_points) + wall * perimeter(loop_points) + wall * wall * corners
}

/// Returns the largest distance by which an offset corner misses the wall's thickness on either of
/// the two edges it joins.
///
/// **The mitre is right when both numbers read `−wall`.** The signed distance of a point from the
/// line an edge's unit direction `d` runs along is `cross(d, point − node)`, which is positive on the
/// loop's own interior side; a courtyard's wall stands on the **outward** side of the void loop, so
/// every corner of the offset loop must read `−wall` against both of its edges. A mitre that crossed
/// a corner to the far side reads `+wall`, and this is what says so.
fn mitre_error(walked: &[Vec2], offset: &[Vec2], wall: f32) -> f32 {
    let count = walked.len();
    let mut worst = 0.0f32;
    for node in 0..count {
        for edge in [node, (node + count - 1) % count] {
            let along = difference(walked[(edge + 1) % count], walked[edge]);
            let length = length(along);
            if length <= f32::EPSILON {
                continue;
            }
            let unit = [along[0] / length, along[1] / length];
            let gap = difference(offset[node], walked[node]);
            worst = worst.max((cross(unit, gap) + wall).abs());
        }
    }
    worst
}

/// Returns whether a loop is simple: no two of its edges share a point.
///
/// **Stricter than the walk's own test, on purpose.** `segments_cross_properly` answers false for a
/// touch and for a shared line, which is the right question for "did these edges cross" and the
/// wrong one for "is this loop simple": a loop that touches itself at a point is not simple. The
/// walk accepts a touch, so this can catch a shape the walk let through — and on this crate's own
/// shapes it did: see the two-loops-touching case below, whose offset loop this answers **true**
/// for and whose ring is a sound closed genus-one solid. **The second half of the sentence this
/// function used to carry is false and was measured false**: "a ring built on one has a face with no
/// material behind it" is not what came out. Every face stands on one of the ring's own loops, the
/// solid is edge-manifold with no boundary loop, its Euler characteristic is the `0` a torus is, and
/// its volume is `(offset − loop) × height` to `2 × 10⁻⁶ m³`. A touch at a point the author did
/// not author as a node — a T-junction — is caught by the walk's own `MiterTooLong` instead, so
/// every path a self-touching loop can take through the outward walk ends in a named refusal or in
/// a solid that measures right.
fn is_simple(loop_points: &[Vec2]) -> bool {
    let count = loop_points.len();
    for left in 0..count {
        for right in left + 2..count {
            if left == 0 && right == count - 1 {
                continue;
            }
            if segments_meet(
                loop_points[left],
                loop_points[(left + 1) % count],
                loop_points[right],
                loop_points[(right + 1) % count],
            ) {
                return false;
            }
        }
    }
    true
}

/// Returns the signed area of a triangle of plan points: positive where its interior is to the left.
fn orientation(a: Vec2, b: Vec2, c: Vec2) -> f32 {
    cross(difference(b, a), difference(c, a))
}

/// Returns whether a plan point lies on a segment, endpoints included.
fn on_segment(a: Vec2, b: Vec2, point: Vec2) -> bool {
    orientation(a, b, point).abs() <= TOLERANCE
        && point[0] >= a[0].min(b[0]) - TOLERANCE
        && point[0] <= a[0].max(b[0]) + TOLERANCE
        && point[1] >= a[1].min(b[1]) - TOLERANCE
        && point[1] <= a[1].max(b[1]) + TOLERANCE
}

/// Returns whether two segments share any point at all: they cross, they touch, or they overlap.
fn segments_meet(from_a: Vec2, to_a: Vec2, from_b: Vec2, to_b: Vec2) -> bool {
    let straddles = |one: f32, two: f32| {
        (one > 0.0) != (two > 0.0) && one.abs() > TOLERANCE && two.abs() > TOLERANCE
    };
    if straddles(
        orientation(from_a, to_a, from_b),
        orientation(from_a, to_a, to_b),
    ) && straddles(
        orientation(from_b, to_b, from_a),
        orientation(from_b, to_b, to_a),
    ) {
        return true;
    }
    on_segment(from_a, to_a, from_b)
        || on_segment(from_a, to_a, to_b)
        || on_segment(from_b, to_b, from_a)
        || on_segment(from_b, to_b, to_a)
}

/// Returns the edge of `walked` whose two ends are the two nodes given, and the loop's node order.
fn edge_between(count: usize, from: usize, to: usize) -> Option<usize> {
    if to == (from + 1) % count {
        Some(from)
    } else if from == (to + 1) % count {
        Some(to)
    } else {
        None
    }
}

/// Returns the loop a courtyard's ring was **actually built on**, read back out of the drawn mesh.
///
/// # Invariants
///
/// **The instrument asks the geometry rather than the walk, because a probe that ran the walk twice
/// could only agree with it.** The ring's base annulus is one quad per void-loop edge wound from the
/// offset loop's corner across to the authored one, so every quad carries exactly one corner that is
/// *not* an authored node — and that corner is `offset[edge]`, because it is the third corner of the
/// triangle that also holds both of the quad's authored corners. Every edge answers, so the whole
/// loop comes back in node order with no nearest-neighbour guess anywhere in it, and a walk that
/// put a corner in the wrong place shows up as a loop the area does not match.
///
/// **The one guess it does make is which of a quad's two authored corners is which, and that is
/// settled by the loop's own order rather than by the first node that matches.** A loop that
/// touches itself names one plan position from two of its nodes, and a nearest-match read answers
/// the earlier index for both and pairs a node with a non-neighbour — the probe would then report
/// that it cannot read a ring it is looking straight at.
fn offset_loop_from_mesh(built: &BuildingMesh, walked: &[Vec2]) -> Option<Vec<Vec2>> {
    let count = walked.len();
    let mut found: Vec<Option<Vec2>> = vec![None; count];
    for face in &built.faces {
        if face.element != Element::Mass(MassId(0), MassSurface::VoidBase { void: 0 }) {
            continue;
        }
        let corners = built.triangle(face);
        let mut authored: Vec<Vec<usize>> = Vec::new();
        let mut other = None;
        for corner in corners {
            let plan = [corner[0], corner[2]];
            let at: Vec<usize> = walked
                .iter()
                .enumerate()
                .filter(|(_, node)| {
                    (node[0] - plan[0]).abs() <= TOLERANCE && (node[1] - plan[1]).abs() <= TOLERANCE
                })
                .map(|(index, _)| index)
                .collect();
            if at.is_empty() {
                other = Some(plan);
            } else {
                authored.push(at);
            }
        }
        let [Some(first), Some(second)] = [authored.first(), authored.get(1)] else {
            continue;
        };
        let mut edge = None;
        for from in first {
            for to in second {
                if let Some(found_edge) = edge_between(count, *from, *to) {
                    edge = Some(found_edge);
                }
            }
        }
        let Some(edge) = edge else {
            continue;
        };
        if let Some(plan) = other {
            found[edge] = Some(plan);
        }
    }
    found.into_iter().collect()
}

/// Returns whether every flat face of the courtyard's ring stands on one of the ring's two loops.
///
/// **The vertical faces are the check that the loops are the faces' own edges and not something
/// else.** `VoidWall` over edge `e` must span the authored edge, `VoidInner` the offset edge, and the
/// wall top between them — so a ring that closed and was manifold with the right volume while
/// standing somewhere else would be caught here.
fn faces_sit_on_their_loops(built: &BuildingMesh, walked: &[Vec2], offset: &[Vec2]) -> bool {
    let count = walked.len();
    let plan_of = |face: usize| {
        built
            .triangle(&built.faces[face])
            .map(|corner| [corner[0], corner[2]])
    };
    for face in 0..built.faces.len() {
        let Element::Mass(_, surface) = built.faces[face].element else {
            continue;
        };
        let want: &[Vec2] = match surface {
            MassSurface::VoidWall { void: 0, edge } => &[walked[edge], walked[(edge + 1) % count]],
            MassSurface::VoidInner { void: 0, edge } => &[offset[edge], offset[(edge + 1) % count]],
            MassSurface::VoidTop { void: 0, edge } => &[
                walked[edge],
                walked[(edge + 1) % count],
                offset[edge],
                offset[(edge + 1) % count],
            ],
            _ => continue,
        };
        for corner in 0..3 {
            let plan = plan_of(face)[corner];
            if !want.iter().any(|node| {
                (node[0] - plan[0]).abs() <= TOLERANCE && (node[1] - plan[1]).abs() <= TOLERANCE
            }) {
                return false;
            }
        }
    }
    true
}

/// Returns the solid a courtyard's ring is.
fn court_ring() -> SolidId {
    SolidId::Void {
        mass: MassId(0),
        void: 0,
    }
}

/// Returns the square mass a void stands in, `half` metres from the origin on each side.
///
/// **The size is a parameter so that a loop's own fault is not measured against the courtyard's
/// fit.** `VoidReachesTheWall` asks whether the ring has room inside the mass, and a shape that
/// meets itself would confound two refusals if the housing were the one every other reading used.
fn housing_sized(half: f32) -> Mass {
    Mass::new(outline(&square_plan(half)), 3.2)
        .with_wall(0.3)
        .with_slab(0.4)
}

/// Returns the plan corners of that mass, which is what the area check below is measured against.
fn square_plan(half: f32) -> Vec<[f32; 2]> {
    vec![[-half, -half], [half, -half], [half, half], [-half, half]]
}

/// Returns the 10 × 10 mass a void stands in: the same housing every courtyard probe below uses.
fn housing() -> Mass {
    housing_sized(5.0)
}

/// Prints one void loop and the loop its wall is built on, and every reading the walk has to pass.
fn measure_void(label: &str, corners: &[[f32; 2]], wall: f32) {
    measure_void_in(5.0, label, corners, wall)
}

/// Prints one void loop against a mass of the given half-width.
fn measure_void_in(half: f32, label: &str, corners: &[[f32; 2]], wall: f32) {
    let walked = wound_positive(corners);
    let authored: Vec<Vec2> = corners.to_vec();
    let enclosing = square_plan(half);
    let mut skeleton = BuildingSkeleton::default();
    skeleton.masses = vec![
        housing_sized(half)
            .with_wall(wall)
            .with_voids(vec![outline(corners)]),
    ];
    let document = Building::new(skeleton);
    let violations = document.validate();
    let faults = names_of(&violations);
    println!(
        "\n  {label}: {} node(s), {} reflex corner(s), void area {:.4}",
        walked.len(),
        reflex_corners(&walked),
        shoelace(&walked)
    );
    print_loop("    the void loop", &walked);
    // **The loop's own simplicity is printed for every shape above and below, and it is this probe's
    // canary.** `is_simple` has never answered anything but true on an *offset* loop, because the
    // walk refuses a crossing before the probe is asked — so the reading that proves the test can
    // fire is the one taken on the **authored** loop, where the concave shapes read true and the
    // self-intersecting ones read false. An instrument shown to fire on one shape and not another
    // is an instrument; one shown only ever to agree is not.
    println!("    the loop itself is simple: {}", is_simple(&walked));
    // **The two numbers `VoidNotInside` compares are printed rather than inferred from whether the
    // violation fired**, because "the loop is not inside the outline" is a reading about a shape and
    // the check is a subtraction of two computed areas. A loop that meets itself has no
    // triangulation, so what `shared_area` returns for it is whatever its ear clipper happened to
    // produce — and the difference between the two numbers is the whole of what that violation says.
    println!(
        "    the area check reads: {:.4} shared with the outline against {:.4} shared with itself, so \
         `VoidNotInside` fires on a difference of {:.4}",
        whiting_building::clip::shared_area(&enclosing, &authored),
        whiting_building::clip::shared_area(&authored, &authored),
        (whiting_building::clip::shared_area(&enclosing, &authored)
            - whiting_building::clip::shared_area(&authored, &authored))
        .abs()
    );
    println!(
        "    validate says: {}",
        if faults.is_empty() {
            "nothing".to_owned()
        } else {
            faults.join(" and ")
        }
    );
    if !violations.is_empty() {
        for fault in &violations {
            println!("      {fault:?}");
        }
    }
    let built = match document.build() {
        Ok(built) => built,
        Err(error) => {
            println!(
                "    refused by the document: {} violation(s), and the walk says",
                violations.len()
            );
            for problem in &error.problems {
                println!("      {problem:?}");
            }
            return;
        }
    };
    let Some(offset) = offset_loop_from_mesh(&built, &walked) else {
        println!(
            "    the base annulus did not answer for every edge: the probe cannot read this ring"
        );
        return;
    };
    print_loop("    the offset loop", &offset);
    let measured = shoelace(&offset);
    let answered = offset_area_answer(&walked, wall);
    let ring = court_ring();
    let volume = built.solid_signed_volume(ring);
    let height = 3.2;
    println!(
        "    offset loop: simple {}, every corner exactly {wall:.4} m out on the correct side (worst miss {:.7})",
        is_simple(&offset),
        mitre_error(&walked, &offset, wall)
    );
    println!(
        "    offset area {measured:.4} against plan geometry's {answered:.4} — off {:.6}",
        (measured - answered).abs()
    );
    // **The number a person can do in their head is printed beside the one the walk produced.** A
    // loop's signed area is `area + wall · perimeter + wall² · Σ tan(turn / 2)`'s own first term, and
    // for a loop that meets itself it is *two triangles, one positive and one negative* — the sum is
    // a difference, not an extent, and it can even be zero. A ring whose volume is not that number
    // times the height has not built what the author drew, and a reader has to be able to see which
    // of the two happened rather than be told.
    let expected = (measured - shoelace(&walked)) * height;
    println!(
        "    the loop's own signed area {:.4} x {height:.1} m is {:.4} m3, and the ring's volume is \
         {volume:.4} — the ring's own (offset - loop) x height is {expected:.4}",
        shoelace(&walked),
        shoelace(&walked) * height
    );
    println!(
        "    the ring: manifold {}, loops {}, euler {}, volume {volume:.4} against (outer - inner) x height \
         {:.4} — off {:.6}",
        built.solid_is_manifold(ring),
        built.solid_boundary_loops(ring),
        built.solid_euler_characteristic(ring),
        expected,
        (volume - expected).abs()
    );
    println!(
        "    every flat face stands on its own loop: {}",
        faces_sit_on_their_loops(&built, &walked, &offset)
    );
}

/// Returns the leading name of every fault in a list, which is what a probe line reads.
///
/// **The names come from the crate's own `Debug`,** so a probe cannot go on asking a question the
/// crate has stopped asking: a renamed variant changes these lines and nothing else in this file.
fn names_of<T: std::fmt::Debug>(faults: &[T]) -> Vec<String> {
    faults
        .iter()
        .map(|fault| {
            format!("{fault:?}")
                .split([' ', '{'])
                .next()
                .unwrap_or("a fault")
                .to_owned()
        })
        .collect()
}

/// Returns how many of a loop's corners turn against the loop's own winding.
fn reflex_corners(loop_points: &[Vec2]) -> usize {
    let count = loop_points.len();
    (0..count)
        .filter(|node| turn_at(loop_points, *node) < 0.0)
        .count()
}

/// Prints one loop, node by node.
fn print_loop(label: &str, loop_points: &[Vec2]) {
    println!("{label}:");
    for (index, node) in loop_points.iter().enumerate() {
        println!("      {index:>2}: ({:>8.4}, {:>8.4})", node[0], node[1]);
    }
}

fn main() {
    println!("# three concave plans, each with the wall a cottage would use");
    // A courtyard open to the north: 10 x 10 outside, a 4 x 6 notch.
    measure(
        "U",
        &[
            [-5.0, -5.0],
            [5.0, -5.0],
            [5.0, 5.0],
            [2.0, 5.0],
            [2.0, -1.0],
            [-2.0, -1.0],
            [-2.0, 5.0],
            [-5.0, 5.0],
        ],
        0.3,
    );
    // A wing off a square: 10 x 10 with a 4 x 4 corner taken out.
    measure(
        "L",
        &[
            [-5.0, -5.0],
            [5.0, -5.0],
            [5.0, -1.0],
            [1.0, -1.0],
            [1.0, 5.0],
            [-5.0, 5.0],
        ],
        0.3,
    );
    // A shallow porch two metres deep: the notch a front door wants.
    measure(
        "porch",
        &[
            [-5.0, -5.0],
            [5.0, -5.0],
            [5.0, 5.0],
            [2.0, 5.0],
            [2.0, 3.0],
            [-2.0, 3.0],
            [-2.0, 5.0],
            [-5.0, 5.0],
        ],
        0.3,
    );

    // **The three shapes above are outlines, not courtyards**, and the distinction is what the volume
    // decision turned on: a mass's outline is still one simple polygon, so a U-shaped *building* is
    // drawn as a U. A courtyard is a **void** inside that outline, and the wall round it is a second
    // ring standing on the plate — which is why a courtyard can be closed, where the shell could not
    // draw one that was. `patio_probe` measures that ring.
    println!(
        "\n# these three are outlines and not courtyards: a mass's outline is one simple polygon, so a U-shaped\n# building is drawn as a U. A courtyard is a void inside the outline and its wall is a second ring on the\n# plate, which is why a courtyard can be closed — `patio_probe` measures that ring."
    );

    // **The identity table of a concave mass**, which is what a scene would draw through: every face of the
    // mass must name itself, and the two sides of the mouth must be two different elements. (The word
    // "courtyard" is not used for this shape anywhere in the crate: it names a void, which is a hole in
    // a mass's plan rather than a notch in its outline.)
    let mut skeleton = BuildingSkeleton::default();
    skeleton.masses = vec![
        Mass::new(
            outline(&[
                [-5.0, -5.0],
                [5.0, -5.0],
                [5.0, 5.0],
                [2.0, 5.0],
                [2.0, -1.0],
                [-2.0, -1.0],
                [-2.0, 5.0],
                [-5.0, 5.0],
            ]),
            3.2,
        )
        .with_wall(0.3)
        .with_slab(0.4),
    ];
    let built = Building::new(skeleton).build().expect("the U expands");
    let table = whiting_building::identity::Identity::of(&built, 1);
    println!();
    println!("# the concave mass identity table");
    println!(
        "  it holds {} row(s): one per face an element names",
        table.len()
    );
    for row in table.rows().iter().take(4) {
        println!("    id {:>3} to {}", row.id, row.path);
    }
    let last = table.rows().last().expect("a row");
    println!("    id {:>3} to {} (last)", last.id, last.path);
    println!(
        "  every row names something: {}",
        table.rows().iter().all(|row| !row.path.is_empty())
    );

    outward_walk();
}

/// Measures the **outward** walk, which is what a courtyard's ring is built from and what a mass's
/// own outline never is.
///
/// # Invariants
///
/// **The four shapes are the four ways a plan can fail to be convex, and the sweep is what says
/// where the boundary is rather than where it was guessed to be.** An L, a T and a bite each put
/// one or two corners back the other way; the U opens on one side. A variable sweep separates a limit
/// from a bug: a limit refuses thin and accepts thick in one direction, and a bug does whatever the
/// arithmetic happens to do.
fn outward_walk() {
    let wall = 0.3;
    println!(
        "\n# the same matrix on the outward walk: a void loop and the loop a wall's thickness away"
    );
    println!(
        "  a 10 x 10 mass, a 0.3 m wall, a 3.2 m ring; the housing is the same one every reading above used"
    );

    // An L: one reflex corner, at the inner angle where a wing meets the square.
    measure_void(
        "L, one reflex corner",
        &[
            [-3.0, -3.0],
            [3.0, -3.0],
            [3.0, 0.0],
            [0.0, 0.0],
            [0.0, 3.0],
            [-3.0, 3.0],
        ],
        wall,
    );
    // A T: two reflex corners, where the stem meets the bar.
    measure_void(
        "T, two reflex corners",
        &[
            [-4.0, -3.0],
            [4.0, -3.0],
            [4.0, 0.0],
            [1.0, 0.0],
            [1.0, 3.0],
            [-1.0, 3.0],
            [-1.0, 0.0],
            [-4.0, 0.0],
        ],
        wall,
    );
    // A bite: the shape `patio_probe` reaches its courtyard restriction through, and the one that has
    // a mouth the wall has to grow into rather than out of.
    measure_void(
        "a rectangle with a bite",
        &[
            [-3.0, -3.0],
            [3.0, -3.0],
            [3.0, 3.0],
            [1.5, 3.0],
            [1.5, 1.5],
            [-3.0, 1.5],
        ],
        wall,
    );
    // A courtyard open on one side: two reflex corners whose bite runs off the edge of the plan.
    measure_void(
        "U, open on the north",
        &[
            [-3.0, -3.0],
            [3.0, -3.0],
            [3.0, 3.0],
            [1.5, 3.0],
            [1.5, -1.0],
            [-1.5, -1.0],
            [-1.5, 3.0],
            [-3.0, 3.0],
        ],
        wall,
    );

    sweep_bite(wall);
    self_intersecting_voids();
}

/// Measures the one loop class the concave sweep never reached: **a void loop that meets itself**.
///
/// # Invariants
///
/// - **Three questions are asked separately and none of them may answer for another.** `validate` is
///   asked what it reports about the document, `build` is asked separately whether the walk takes
///   the loop, and only a solid that came out is measured at all. A refusal from one and a solid
///   from the other **is** the finding, and a probe that printed them as one sentence would have
///   hidden it.
/// - **The hand-computed number is printed beside the measurement and not asserted against it.** A
///   loop's own signed area is two triangles, one positive and one negative, so it is a difference
///   rather than an extent and can be exactly zero. The reading a person can do in their head is
///   what says whether a ring that came out is the shape the author drew, and it has to be visible
///   rather than baked into a comparison.
/// - **The four shapes are four different questions, not four sizes of one.** Two cross with a
///   signed area of zero, so the winding normalisation has nothing to normalise and every edge's
///   outward side is decided by the authored order alone; the third crosses with a signed area that
///   is not zero, so the expander **reverses** the loop — and reversing a self-intersecting loop
///   swaps which of its two triangles is the positive one, which is the only thing that decides the
///   side an edge offsets to; the fourth touches rather than crosses.
/// - **The housing is one metre a side larger than the largest loop's own offset needs to be.** A
///   courtyard that does not fit reports `VoidReachesTheWall` beside whatever else is wrong, and a
///   refusal measured alongside a refusal the shape did not ask for says nothing about the shape.
fn self_intersecting_voids() {
    let wall = 0.3;
    let half = 6.0;
    println!("\n# a void loop that meets itself: the one class the sweep above never reached");
    println!(
        "  a 12 x 12 mass, a {wall:.1} m wall, a 3.2 m ring; the loop's own fault is measured alone, so the\n\
         housing is larger than any offset below needs and `VoidReachesTheWall` cannot join in"
    );

    // A bowtie — `(0,0)(4,4)(4,0)(0,4)` — two right triangles of `8 m²` each, meeting at `(2,2)`.
    // Their signed areas are `+8` and `-8`, so the loop's own area is exactly **zero** and there is
    // no right way round it: the expander's normalisation has nothing to normalise, and the side
    // every edge offsets to is decided by the order the author happened to write.
    measure_void_in(
        half,
        "a bowtie, signed area zero",
        &[[0.0, 0.0], [4.0, 4.0], [4.0, 0.0], [0.0, 4.0]],
        wall,
    );

    // The same shape stood on its side: an hourglass, whose two triangles cross on edges 1 and 5
    // rather than on edges 0 and 2, so the walk's crossing sweep reaches a different pair of the
    // loop's own edges and a fault that named one pair would have to name this one too.
    measure_void_in(
        half,
        "an hourglass",
        &[
            [-3.0, 0.0],
            [-1.0, -2.0],
            [1.0, 2.0],
            [3.0, 0.0],
            [1.0, -2.0],
            [-1.0, 2.0],
        ],
        wall,
    );

    // The bowtie with its two triangles **different sizes** — `(0,0)(4,6)(4,0)(0,2)`, areas `12` and
    // `4`, a signed area of `-8`. This is the case the two above cannot reach, because the expander
    // reverses a negatively wound loop and a reversal of a self-intersecting loop turns each edge's
    // outward side the other way round.
    measure_void_in(
        half,
        "an asymmetric bowtie, signed area -8",
        &[[0.0, 0.0], [4.0, 6.0], [4.0, 0.0], [0.0, 2.0]],
        wall,
    );

    // A loop that touches itself at a point rather than crossing: two 3 x 3 squares sharing exactly
    // one corner, so the loop names that corner twice and reaches it from two directions at once.
    measure_void_in(
        half,
        "two squares touching at a point",
        &[
            [-3.0, -3.0],
            [0.0, -3.0],
            [0.0, 0.0],
            [3.0, 0.0],
            [3.0, 3.0],
            [0.0, 3.0],
            [0.0, 0.0],
            [-3.0, 0.0],
        ],
        wall,
    );

    // **The same touch, at a point the author never wrote down.** Here `(0,-3)` is not a node of the
    // loop: it is the interior of the loop's own bottom edge, and the loop's third and fourth edges
    // meet there. This is the case the crossing sweep cannot see at all, because
    // `segments_cross_properly` needs each segment to have the other's endpoints *strictly* on
    // opposite sides, and an endpoint standing on the other segment is never strictly on either
    // side. A touch named as a node and a touch found under one are the same defect wearing two
    // coats, so both are measured here rather than the easier one being taken as the whole.
    measure_void_in(
        half,
        "a T-junction: the loop touches its own bottom edge at (0,-3)",
        &[
            [-3.0, -3.0],
            [3.0, -3.0],
            [3.0, 3.0],
            [0.0, -3.0],
            [-3.0, 3.0],
        ],
        wall,
    );

    // **The same crossing with the crossing point written down as a node**, which is the case that
    // says whether the reading is about the *contact* or about the count of points on it. The bowtie
    // above crosses at `(2,2)` in the interior of two of its edges; here `(2,2)` is node 1 and edge 3
    // runs straight through it, so the contact is at a place the author did write down — and yet no
    // two of the loop's nodes stand at one place, so it is not the authored pinch. The question is
    // whether the check says "these two edges cross" or "this node stands in this edge", and only a
    // measurement says which.
    measure_void_in(
        half,
        "a crossing the author wrote down: edge 3 runs through node 1 at (2,2)",
        &[[0.0, 0.0], [2.0, 2.0], [4.0, 4.0], [4.0, 0.0], [0.0, 4.0]],
        wall,
    );

    // **A pinch the author wrote down twice, with the lobes in the other arrangement.** The two
    // squares above meet corner to corner with both lobes turning the same way; this one has a lobe
    // that is a **dart**, so the corner the two lobes share is reflex. The question the probe has to
    // answer is whether the ring still closes, because the check's silence rests on the measurement
    // that this shape builds rather than on the argument that a shared node is not a fault.
    measure_void_in(
        half,
        "a dart and a triangle sharing a point the author wrote down twice",
        &[
            [-3.0, 0.0],
            [0.0, -3.0],
            [0.0, 0.0],
            [3.0, 0.0],
            [0.0, 3.0],
            [0.0, 0.0],
            [-3.0, 3.0],
        ],
        wall,
    );

    sweep_bowtie(half, wall);
    sweep_pinch(half);
}

/// Sweeps a bowtie in the two variables that turned a bite into a limit rather than a defect.
///
/// # Invariants
///
/// **The sweep's value is negative evidence and is written down as such.** A crossing is a
/// property of the *order* a loop's nodes are in and holds at every size, so a walk that refused at
/// one scale and took it at another would be a boundary and would have to be monotone in the
/// variable causing it — which is the same question `sweep_bite` asked of `2 × wall`. The same
/// refusal, by the same name, at every reading is what a topological fault looks like.
fn sweep_bowtie(half: f32, wall: f32) {
    println!(
        "\n# a bowtie swept in half-width at a 0.3 m wall, and then in the wall at a 2.0 m bowtie"
    );
    let mut tenth = 10;
    while tenth <= 35 {
        let size = tenth as f32 / 10.0;
        report_loop(
            "bowtie",
            &[[-size, -size], [size, size], [size, -size], [-size, size]],
            wall,
            half,
        );
        tenth += 5;
    }
    let mut hundredth = 5;
    while hundredth <= 40 {
        report_loop(
            "bowtie",
            &[[-2.0, -2.0], [2.0, 2.0], [2.0, -2.0], [-2.0, 2.0]],
            hundredth as f32 / 100.0,
            half,
        );
        hundredth += 5;
    }
}

/// Sweeps the one case the check is silent for, in the variable that decides whether the walk carries it.
///
/// # Invariants
///
/// **The exclusion `VoidNotSimple` rests on is "a pinch at an authored point builds", and that is a
/// claim about a wall, not only about a loop.** A mitre's reach is `wall / sin(θ / 2)`, so whether an
/// authored pinch is carried depends on `mass.wall` and on how the two lobes meet — which is the one
/// question about a self-touching loop that **cannot** be answered from the authored nodes alone. The
/// sweep is what says whether the claim holds where it was measured, and the silence holds either way:
/// the check declines every authored pinch at every wall, so the reading it gives is never a
/// wall-dependent one.
fn sweep_pinch(half: f32) {
    println!(
        "\n# the two authored pinches swept in the wall: the check is silent for both at every reading,\n\
         # and this says whether the walk carries them at every reading too"
    );
    let squares: [[f32; 2]; 8] = [
        [-3.0, -3.0],
        [0.0, -3.0],
        [0.0, 0.0],
        [3.0, 0.0],
        [3.0, 3.0],
        [0.0, 3.0],
        [0.0, 0.0],
        [-3.0, 0.0],
    ];
    let dart: [[f32; 2]; 7] = [
        [-3.0, 0.0],
        [0.0, -3.0],
        [0.0, 0.0],
        [3.0, 0.0],
        [0.0, 3.0],
        [0.0, 0.0],
        [-3.0, 3.0],
    ];
    let mut hundredth = 5;
    while hundredth <= 45 {
        let wall = hundredth as f32 / 100.0;
        report_loop("two squares pinched", &squares, wall, half);
        report_loop("a dart and a triangle pinched", &dart, wall, half);
        hundredth += 5;
    }
}

/// Reports one loop in one line: what `validate` names and what `build` did with it.
///
/// **A shape that builds is measured and a shape that does not is not**, because the readings below
/// need a mesh to read and a missing mesh is the answer rather than a gap in it.
fn report_loop(label: &str, corners: &[[f32; 2]], wall: f32, half: f32) {
    let walked = wound_positive(corners);
    let mut skeleton = BuildingSkeleton::default();
    skeleton.masses = vec![
        housing_sized(half)
            .with_wall(wall)
            .with_voids(vec![outline(corners)]),
    ];
    let document = Building::new(skeleton);
    let faults = names_of(&document.validate());
    let said = if faults.is_empty() {
        "nothing".to_owned()
    } else {
        faults.join(" and ")
    };
    match document.build() {
        Ok(built) => {
            let Some(offset) = offset_loop_from_mesh(&built, &walked) else {
                println!(
                    "  {label} half {:.2} at wall {wall:.2}: builds, validate says {said}, and the probe cannot read the ring",
                    corners[1][0] - corners[0][0]
                );
                return;
            };
            let ring = court_ring();
            println!(
                "  {label} half {:.2} at wall {wall:.2}: builds, validate says {said}, simple {}, volume {:.4} against \
                 {:.4}, manifold {}, loops {}, euler {}",
                corners[1][0] - corners[0][0],
                is_simple(&offset),
                built.solid_signed_volume(ring),
                (shoelace(&offset) - shoelace(&walked)) * 3.2,
                built.solid_is_manifold(ring),
                built.solid_boundary_loops(ring),
                built.solid_euler_characteristic(ring)
            );
        }
        Err(error) => println!(
            "  {label} half {:.2} at wall {wall:.2}: refused, validate says {said}, and the walk says {}",
            corners[1][0] - corners[0][0],
            names_of(&error.problems).join(" and ")
        ),
    }
}

/// Sweeps the one variable that separates a limit from a bug: how wide a **closed** bite is.
///
/// # Invariants
///
/// **The shape is a bite the wall has to grow into on three sides, not a mouth that runs off the
/// edge of the plan.** That is the whole case: a mouth that leaves the loop is always open, whereas a
/// bite closed on three sides is a corridor whose width the wall eats from both sides, and the
/// boundary is therefore arithmetic — a ring standing `wall` out of the loop puts `wall` of itself
/// into each side of the bite, so a bite of `b` has `b − 2 × wall` left for the courtyard to show
/// and nothing at all once `b` reaches `2 × wall`.
///
/// **Sweeping the bite at a fixed wall, then the wall at a fixed bite, is what tells a limit from a
/// bug.** The first walk finds where the boundary sits; the second asks whether it sits at `2 ×
/// wall` wherever the wall is, which is what a geometric limit does and what a defect in the mitre
/// does not.
fn sweep_bite(wall: f32) {
    println!(
        "\n# a closed bite swept at a 0.3 m wall: the boundary should be at 2 x wall = 0.6000 m"
    );
    let mut tenth = 10;
    while tenth >= 1 {
        let bite = tenth as f32 / 10.0;
        report_bite(bite, wall);
        tenth -= 1;
    }
    println!("\n# a 0.6000 m bite swept in the wall: the boundary should stay at 2 x wall");
    let mut hundredth = 10;
    while hundredth <= 40 {
        report_bite(0.6, hundredth as f32 / 100.0);
        hundredth += 10;
    }
}

/// Returns a 6 × 6 void loop with a bite of `bite` metres cut out of the middle of its north edge.
fn bitten(bite: f32) -> Vec<PlanNode> {
    outline(&[
        [-3.0, -3.0],
        [3.0, -3.0],
        [3.0, 3.0],
        [bite, 3.0],
        [bite, 1.0],
        [0.0, 1.0],
        [0.0, 3.0],
        [-3.0, 3.0],
    ])
}

/// Reports one sweep reading: the bite and the wall, and what the walk did with them.
fn report_bite(bite: f32, wall: f32) {
    let corners = bitten(bite);
    let walked: Vec<Vec2> = corners.iter().map(|node| node.position).collect();
    let mut skeleton = BuildingSkeleton::default();
    skeleton.masses = vec![housing().with_wall(wall).with_voids(vec![corners])];
    let document = Building::new(skeleton);
    let reported = document.validate();
    let faults = names_of(&reported);
    match document.build() {
        Ok(built) => {
            let Some(offset) = offset_loop_from_mesh(&built, &walked) else {
                println!(
                    "  bite {bite:.2} at wall {wall:.2}: builds, but the probe cannot read the ring back"
                );
                return;
            };
            let answered = offset_area_answer(&walked, wall);
            let measured = shoelace(&offset);
            let ring = court_ring();
            let volume = built.solid_signed_volume(ring);
            let expect = (answered - shoelace(&walked)) * 3.2;
            println!(
                "  bite {bite:.2} at wall {wall:.2}: builds, {} violation(s), simple {}, \
                 area off {:.6}, volume {volume:.4} against {expect:.4} off {:.6}",
                if faults.is_empty() {
                    "0".to_owned()
                } else {
                    faults.join(" and ")
                },
                is_simple(&offset),
                (measured - answered).abs(),
                (volume - expect).abs()
            );
        }
        Err(error) => {
            let names = names_of(&error.problems);
            println!(
                "  bite {bite:.2} at wall {wall:.2}: refused, validate says {}, and the walk says {}",
                if faults.is_empty() {
                    "nothing".to_owned()
                } else {
                    faults.join(" and ")
                },
                names.join(" and ")
            );
        }
    }
}
