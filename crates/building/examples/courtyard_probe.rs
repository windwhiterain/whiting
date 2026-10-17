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
//! # The control for the area check: a simple loop that reaches outside the outline
//!
//! `VoidNotInside` subtracts two areas, and the subtraction is exact only for a simple loop, so every
//! shape below it is measured against is one that meets itself. **A probe made only of those shapes
//! cannot tell a check that reads "the loop is not inside the outline" from one that reads "the ear
//! clipper disagreed with itself"**, because on every one of them the two readings give the same
//! answer. The control is a loop with no meeting at all that leaves the outline anyway, and the section
//! for it stands before the shapes that meet themselves: two of them, one off an edge and one off a
//! corner, so the overlap is a rectangle in the first and a triangle in the second.
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
    // triangulation, so what `shared_area` returns for it is `None` — and a subtraction of two
    // `None`s is not a difference, which is what the line below says when it happens.
    //
    // **The line says the difference and not the verdict**, because the check is asked only of a loop
    // that does not meet itself: on a loop that does, the subtraction is not run at all, so a line
    // claiming it "fires" would be a reading the instrument did not take. The `validate says:` line
    // below is the verdict, and it is asked of the crate rather than predicted here.
    let enclosed = whiting_building::clip::shared_area(&enclosing, &authored);
    let own = whiting_building::clip::shared_area(&authored, &authored);
    let terms = match (enclosed, own) {
        (Some(enclosed), Some(own)) => format!(
            "the area check reads: {enclosed:.4} shared with the outline against {own:.4} shared with \
             itself, so the terms differ by {:.4}",
            (enclosed - own).abs()
        ),
        _ => "the area check reads: it cannot measure, so it was not asked".to_owned(),
    };
    println!("    {terms}");
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
    a_simple_loop_that_leaves_the_outline();
    a_loop_the_clipper_cannot_close();
    self_intersecting_voids();
}

/// Measures the loop [`Violation::VoidNotInside`](whiting_building::validate::Violation::VoidNotInside)
/// exists for: **a loop that is simple and still reaches outside the outline**.
///
/// # Invariants
///
/// - **Both shapes here are simple, and that is the whole content of the section.** The area check is
///   asked only where [`Violation::VoidNotSimple`](whiting_building::validate::Violation::VoidNotSimple)
///   has nothing to report, so the question a control has to answer is what it *does* report: a loop
///   with no meeting at all that the subtraction still fires on. A control that met itself would answer
///   for a shape the check now declines and would say nothing about the one it does not.
/// - **Both terms of the subtraction are numbers a person can do in their head** — a rectangle and a
///   right triangle — so the reading can be checked rather than believed.
/// - **`VoidReachesTheWall` joins in on both, and it is the same fact rather than a second one.** A
///   courtyard's ring stands `wall` outside its loop, so a loop that leaves the outline puts its ring
///   outside the rooms as well. Two names for one fault is the honest reading of one fault, and the
///   section says which is which rather than dropping one to make the other look alone.
fn a_simple_loop_that_leaves_the_outline() {
    let wall = 0.3;
    let half = 6.0;
    println!(
        "\n# a simple void loop that reaches outside the outline: the fault `VoidNotInside` exists for"
    );
    println!(
        "  the same 12 x 12 mass and {wall:.1} m wall as the section below; both loops here are simple, so\n\
         neither is a shape `VoidNotSimple` reports and the area check is the only reading left to speak"
    );

    // A `5 x 4` rectangle whose east side stands `2 m` outside the `12 x 12` outline: `20.0000 m²` of
    // loop, of which the `2 x 4` the plan reaches is `8.0000 m²`, so the two terms are `12.0000` against
    // `20.0000` and the check fires on a difference of `8.0000`.
    measure_void_in(
        half,
        "a rectangle whose east side stands 2 m outside the outline",
        &[[3.0, -2.0], [8.0, -2.0], [8.0, 2.0], [3.0, 2.0]],
        wall,
    );

    // The same fault off a **corner** rather than off an edge, so the overlap is a triangle rather than
    // a rectangle and no edge of the loop is square against the outline's. A `7 x 7` right triangle is
    // `24.5000 m²`, the plan reaches all but the two `4.5 m²` corners of it, and the two terms are
    // `15.5000` against `24.5000`.
    measure_void_in(
        half,
        "a right triangle over the north-east corner",
        &[[2.0, 2.0], [9.0, 2.0], [2.0, 9.0]],
        wall,
    );
}

/// Measures the case [`Violation::VoidNotInside`](whiting_building::validate::Violation::VoidNotInside)'s
/// own gate leaves open: **a void loop that does not meet itself and the ear clipper cannot fully
/// triangulate**.
///
/// # Invariants
///
/// - **The gate is `self_meeting`, and `self_meeting` is silent for every degeneracy that is not a
///   meeting.** A corner standing on the line through its two neighbours, a corner the author wrote
///   down twice, a finger a hundredth of a millimetre wide: none of them is a crossing and none is a
///   touch at a point the author did not write down, so every one of them reaches the subtraction —
///   and the subtraction is exact only where the clipper's own triangles cover the loop. **This is
///   the section that says whether that gate is enough**, and it says it with a measurement.
/// - **Three readings are printed and none of them may answer for another.** The loop's own signed
///   area is the shoelace of the authored nodes; `clip::shared_area(loop, loop)` is the sum of the
///   areas of the triangles the ear clipper made; the verdict is asked of `validate`. A loop whose
///   two areas differ has a **number**, and only the `validate says:` line says whether the check
///   fired on it.
/// - **Every shape is measured twice, because one placement cannot tell the two failures apart.** A
///   loop that leaves the outline must fire; a loop that stays inside must not. A clipper that
///   under-reports makes `own` too small and **silences** the first, and a clipper that leaves a
///   residue makes the second fire for a loop that never left. Same instrument, opposite faults.
/// - **The collinear rows are the crate's own shapes and not the probe's.** `ops::split_outline`
///   inserts a node on the edge a junction lands on, so a mass that has had a wing attached carries
///   corners standing on the line through their neighbours; `clip`'s own module docs claim that "a
///   polygon with collinear corners loses area-free triangles, never area", and that is a claim
///   about exactly these rows, so it is measured here rather than believed.
/// - **The first row is the canary.** A section whose every difference reads `0.0000` is an
///   instrument that cannot disagree with the crate; the bowtie is here so that a column of zeroes is
///   visibly the *right* zeroes and not a line that never reads its argument.
fn a_loop_the_clipper_cannot_close() {
    let wall = 0.3;
    let half = 8.0;
    let across = half * 2.0;
    let shapes: [(&str, &[[f32; 2]]); 9] = [
        (
            "the canary: a bowtie, which is NOT a simple loop and is here only so that a column of \
             zero differences can be told from an instrument that cannot disagree",
            &[[-2.0, -2.0], [2.0, 2.0], [2.0, -2.0], [-2.0, 2.0]],
        ),
        (
            "a plain 4 x 4 rectangle: the reference every other row is read against",
            &[[-2.0, -2.0], [2.0, -2.0], [2.0, 2.0], [-2.0, 2.0]],
        ),
        (
            "one collinear node, on the south edge: what `split_outline` inserts at a junction",
            &[
                [-2.0, -2.0],
                [0.0, -2.0],
                [2.0, -2.0],
                [2.0, 2.0],
                [-2.0, 2.0],
            ],
        ),
        (
            "a collinear node on every edge: four junctions on one loop",
            &[
                [-2.0, -2.0],
                [0.0, -2.0],
                [2.0, -2.0],
                [2.0, 0.0],
                [2.0, 2.0],
                [0.0, 2.0],
                [-2.0, 2.0],
                [-2.0, 0.0],
            ],
        ),
        (
            "a spike: a one metre slit cut four metres down from the north edge",
            &[
                [-2.0, -2.0],
                [2.0, -2.0],
                [2.0, 2.0],
                [0.5, 2.0],
                [0.5, -2.0],
                [-0.5, -2.0],
                [-0.5, 2.0],
                [-2.0, 2.0],
            ],
        ),
        (
            "a sliver: a 0.0001 m finger four metres long, standing off the south edge",
            &[
                [-2.0, -2.0],
                [2.0, -2.0],
                [2.0, 2.0],
                [0.0, 2.0],
                [0.0, -1.9999],
                [-0.0001, -1.9999],
                [-0.0001, 2.0],
                [-2.0, 2.0],
            ],
        ),
        (
            "a zero-area lobe: a node the author wrote down twice, leaving an edge of no length",
            &[
                [-2.0, -2.0],
                [2.0, -2.0],
                [2.0, 0.0],
                [2.0, 0.0],
                [2.0, 2.0],
                [-2.0, 2.0],
            ],
        ),
        (
            "a spike that doubles back along its own line and so runs over two of its own nodes",
            &[
                [-2.0, -2.0],
                [2.0, -2.0],
                [2.0, -1.0],
                [4.0, -1.0],
                [4.0, -4.0],
                [2.0, -4.0],
                [2.0, 2.0],
                [-2.0, 2.0],
            ],
        ),
        (
            "the control: a needle that doubles back along its own line over the very node it repeats,\n\
             so the meeting is one an author wrote down and `VoidNotSimple` is silent about it",
            &[
                [-4.0, -4.0],
                [4.0, -4.0],
                [4.0, 0.0],
                [2.0, 0.0],
                [4.0, 0.0],
                [4.0, 4.0],
                [-4.0, 4.0],
            ],
        ),
    ];
    println!("\n# a loop that does not meet itself and the ear clipper cannot fully triangulate");
    println!(
        "  every shape is measured twice against a {across:.0} x {across:.0} outline and a {wall:.1} m wall:\n  \
         once inside it, where `VoidNotInside` must stay silent, and once with its east side 2 m outside\n  \
         it, where it must fire. A clipper that under-reports silences the second and a clipper that\n  \
         leaves a residue fires the first, so one placement answers neither question."
    );
    for (label, corners) in shapes {
        let authored: Vec<Vec2> = corners.iter().map(|node| *node).collect();
        let signed = shoelace(&authored);
        let clipped = whiting_building::clip::shared_area(&authored, &authored);
        println!(
            "\n  {label}\n    the loop's own signed area {signed:+.4} ({:.4} absolute), and clip {}",
            signed.abs(),
            match clipped {
                Some(area) => format!(
                    "reads the loop's triangles as {area:.4}, so the two differ by {:.4}",
                    (area - signed.abs()).abs()
                ),
                None => "cannot measure it".to_owned(),
            }
        );
        measure_void_in(half, "inside the outline", corners, wall);
        measure_void_in(
            half,
            "the same loop with its east side 2 m outside the outline",
            &pushed_outside(corners, half, 2.0),
            wall,
        );
    }
    the_degenerate_edge_is_the_crates_own();
    the_loop_no_fault_names();
}

/// Asks the question every other section of this probe is arranged around: **is every loop
/// [`clip`](whiting_building::clip) cannot triangulate named by something else?**
///
/// # Invariants
///
/// - **The needle is the shape the void gate cannot see, and it is not a shape from nowhere.** Every
///   loop `clip` mis-measures in the section above is a bowtie or a spike that runs *over* two of its
///   own nodes, and both are [`Violation::VoidNotSimple`](whiting_building::validate::Violation::VoidNotSimple)'s
///   to report, so nothing reaches the subtraction with them. The needle's meeting is **at a node the
///   author wrote down twice**, which is the one coincidence that check is deliberately silent for,
///   and [`without_edges_of_no_length`](whiting_building::clip)'s own removal cannot reach it either,
///   because the two repeats are not neighbours in the node list. **It is `8 m` tall and encloses
///   `64.0000 m²`, both of which a person can check, so every step below carries the number the
///   subtraction *should* be asked about.**
/// - **The walk's own verdict is on the same line as the numbers, because it is what decides whether
///   a declining check loses anything.** A check that skips a loop the expander refuses by name has
///   lost nothing: there is no building for the fault to have opened in. A check that skips a loop the
///   expander *accepts* has thrown away a reading, and the two are told apart by asking `validate`
///   rather than by predicting it here.
/// - **The last reading is the other caller, and it is the one that had no gate at all until the mass
///   outline was asked the same question a void loop is.** The overlap test asks `clip` about **mass
///   outlines**, and nothing in [`document`](whiting_building::validate::document) used to ask whether
///   one is simple, so this loop reached the subtraction with nothing behind it. It now reads
///   [`Violation::OutlineNotSimple`](whiting_building::validate::Violation::OutlineNotSimple) from the
///   document check alone, and the pair is not asked at all — which is what makes the row below a
///   measurement of the gate rather than of the two masses.
fn the_loop_no_fault_names() {
    let wall = 0.3;
    let half = 8.0;
    let needle: [[f32; 2]; 7] = [
        [-4.0, -4.0],
        [4.0, -4.0],
        [4.0, 0.0],
        [2.0, 0.0],
        [4.0, 0.0],
        [4.0, 4.0],
        [-4.0, 4.0],
    ];
    println!(
        "\n# the loop above that no void fault names: a needle that doubles back along its own line over\n  \
         the very node it repeats. `VoidNotSimple` skips a meeting at a point the author wrote down, and\n  \
         an edge of no length is not what this carries, so the subtraction is asked of it"
    );
    println!(
        "  it is 8 m tall and encloses 64.0000 m2, so at every step below the check should be asked\n  \
         about eight times the distance its east side stands outside the outline"
    );
    for over in [0.01f32, 0.02, 0.05, 0.1, 0.25, 0.5, 1.0, 2.0] {
        report_the_subtraction(
            &format!(
                "{over:.2} m outside, so the check should be asked about {:.4}",
                8.0 * over
            ),
            half,
            &pushed_outside(&needle, half, over),
            wall,
        );
    }

    // The other caller: the same needle cut into a mass's own outline, which is now named by the same
    // reading a void loop is.
    let plain: [[f32; 2]; 4] = [[0.0, 0.0], [10.0, 0.0], [10.0, 10.0], [0.0, 10.0]];
    let needled: [[f32; 2]; 6] = [
        [0.0, 0.0],
        [10.0, 0.0],
        [10.0, 10.0],
        [6.0, 10.0],
        [8.0, 10.0],
        [0.0, 10.0],
    ];
    let other: [[f32; 2]; 4] = [[5.0, 0.0], [15.0, 0.0], [15.0, 10.0], [5.0, 10.0]];
    let two_masses = |first: &[[f32; 2]]| {
        let mut skeleton = BuildingSkeleton::default();
        skeleton.masses = vec![
            Mass::new(outline(first), 3.2)
                .with_wall(wall)
                .with_slab(0.4),
            Mass::new(outline(&other), 3.2)
                .with_wall(wall)
                .with_slab(0.4),
        ];
        skeleton
    };
    for (label, first) in [
        ("without the needle", &plain[..]),
        ("with the needle", &needled[..]),
    ] {
        let skeleton = two_masses(first);
        let shared = whiting_building::clip::shared_area(
            &plan_nodes_of(&skeleton.masses[0]),
            &plan_nodes_of(&skeleton.masses[1]),
        );
        let named = names_of(&Building::new(skeleton).validate());
        println!(
            "  the same needle on a mass's own outline, {label}: the two plans are {} against a true\n    \
             50.0000, and validate says {}",
            area_reading(shared),
            if named.is_empty() {
                "nothing".to_owned()
            } else {
                named.join(" and ")
            }
        );
    }
}

/// Returns a measurement `clip` gave, or the sentence it gives when it could not measure.
///
/// **The two spellings are one reading and this probe needs both**, because a check that reports
/// nothing because the loop measured `0.0000` and a check that reports nothing because the loop
/// could not be measured at all print the same line otherwise — which is precisely the silence this
/// slice is about.
fn area_reading(area: Option<f32>) -> String {
    match area {
        Some(area) => format!("{area:.4}"),
        None => "cannot measure it".to_owned(),
    }
}

/// Returns a loop's own area as `clip` reads it, or the sentence it answers with when it cannot.
///
/// **It is [`reading`] asked of one loop against itself**, which is the loop's own area read the only
/// way `shared_area` reads one: a loop's own area is the area of the intersection of the loop with
/// itself, so a triangulation that does not tile the loop shows up here as a wrong number.
fn clip_reads(loop_points: &[Vec2]) -> String {
    area_reading(whiting_building::clip::shared_area(
        loop_points,
        loop_points,
    ))
}

/// Prints the two areas [`Violation::VoidNotInside`](whiting_building::validate::Violation::VoidNotInside)
/// subtracts, what it is therefore asked about, and what `validate` says — and nothing else.
///
/// **It asks the document and never builds.** The sweep this prints is a question about the
/// subtraction, and a walk that refuses a loop the subtraction is still asked of would put a second
/// fault in the same line and make the column unreadable.
fn report_the_subtraction(label: &str, half: f32, corners: &[[f32; 2]], wall: f32) {
    let enclosing = square_plan(half);
    let authored: Vec<Vec2> = corners.iter().map(|node| *node).collect();
    let enclosed = whiting_building::clip::shared_area(&enclosing, &authored);
    let own = whiting_building::clip::shared_area(&authored, &authored);
    let mut skeleton = BuildingSkeleton::default();
    skeleton.masses = vec![
        housing_sized(half)
            .with_wall(wall)
            .with_voids(vec![outline(corners)]),
    ];
    let faults = names_of(&Building::new(skeleton).validate());
    let terms = match (enclosed, own) {
        (Some(enclosed), Some(own)) => format!(
            "the terms are {enclosed:.4} and {own:.4}, so the check is asked about {:.4}",
            (enclosed - own).abs()
        ),
        _ => "clip cannot measure, so the check is not asked at all".to_owned(),
    };
    println!(
        "  {label}: {terms}, while the loop itself encloses {:.4} — validate says {}",
        shoelace(&authored).abs(),
        if faults.is_empty() {
            "nothing".to_owned()
        } else {
            faults.join(" and ")
        }
    );
}

/// Shows that the degenerate edge of the section above is **the crate's own product**, and sweeps a loop
/// carrying one out past the outline half a metre at a time.
///
/// # Invariants
///
/// - **The first reading is `ops::split_outline` asked to do the thing its own docs say it always
///   does.** "A station that lands exactly on an existing node still inserts one", so a junction at a
///   corner leaves an edge of no length behind — **and the walk names that edge by name**
///   ([`DidNotExpand`](whiting_building::validate::Violation::DidNotExpand) carrying
///   `OutlineEdgeDegenerate`), which is the fault that owns it and the one this section must not
///   swallow. A fix that made the subtraction exact while also silencing this would be a fix that
///   traded one fault for another, and the two lines are printed together so that trade cannot happen
///   unnoticed.
/// - **The sweep is the control the fix needs, and it is built before the fix.** A clipper that
///   under-reports does not merely mis-size a difference it already fires on: **it silences the fault
///   entirely** wherever the part it loses is the part that left the outline. The sweep walks a
///   degenerate-edged loop from flush with the outline to three metres out, and every row carries the
///   number a person can do in their head — a `4 x 4` rectangle `over` metres out of a square shares
///   `4 * (4 - over)` and encloses `16`, so the check *should* be asked about `4 * over` at every row.
/// - **The last reading is the other caller.** `clip::shared_area` is asked about **mass outlines** as
///   well as void loops, by the overlap test, so the same degenerate edge hides a **whole missing
///   overlap** and not only a void's. Two unattached masses standing a half in one another are the
///   control: `50.0000 m²` of shared plan area is either reported or it is not, and a clipper that
///   answers `0` for it has reported nothing.
fn the_degenerate_edge_is_the_crates_own() {
    let wall = 0.3;
    let half = 8.0;
    println!(
        "\n# the degenerate edge of the row above is the crate's own product, and the faults it hides"
    );

    // The mass `attach_mass` leaves behind when the junction lands exactly on a corner.
    let mut host = Mass::new(outline(&square_plan(5.0)), 3.2)
        .with_wall(wall)
        .with_slab(0.4);
    let at =
        whiting_building::ops::split_outline(&mut host, 0.0).expect("a square carries a corner");
    println!(
        "  `ops::split_outline(&mut mass, 0.0)` inserted its node at index {at} and the outline now reads"
    );
    print_loop("    the host's own outline", &plan_nodes_of(&host));
    println!(
        "    the edge it leaves runs ({:.4}, {:.4}) to ({:.4}, {:.4}), which is {} long",
        host.outline[at - 1].position[0],
        host.outline[at - 1].position[1],
        host.outline[at].position[0],
        host.outline[at].position[1],
        length(difference(
            host.outline[at].position,
            host.outline[at - 1].position
        ))
    );
    let mut skeleton = BuildingSkeleton::default();
    skeleton.masses = vec![host];
    let faults = names_of(&Building::new(skeleton).validate());
    println!(
        "    validate says: {} — the walk names that edge by name, so it is never this check's to report",
        faults.join(" and ")
    );

    // The sweep: the same degenerate loop carried further and further out of the outline.
    let doubled = [
        [-2.0, -2.0],
        [2.0, -2.0],
        [2.0, 0.0],
        [2.0, 0.0],
        [2.0, 2.0],
        [-2.0, 2.0],
    ];
    println!(
        "\n  the same loop swept out past the outline. It encloses 16.0000, so the check should be asked\n  \
         about four times its distance past the edge, and it is asked about that only while the\n  \
         difference it reads stays above validate's own 0.0010 m2:"
    );
    for over in [
        0.01f32, 0.02, 0.04, 0.05, 0.1, 0.25, 0.5, 1.0, 1.5, 2.0, 3.0,
    ] {
        report_the_subtraction(
            &format!(
                "{over:.2} m outside, so the check should be asked about {:.4}",
                4.0 * over
            ),
            half,
            &pushed_outside(&doubled, half, over),
            wall,
        );
    }

    // The other caller: `clip::shared_area` is asked about mass outlines too, by the overlap test.
    let first = [[0.0, 0.0], [10.0, 0.0], [10.0, 10.0], [0.0, 10.0]];
    let second = [[5.0, 0.0], [15.0, 0.0], [15.0, 10.0], [5.0, 10.0]];
    let none = [None, Some(0usize), Some(1), Some(2), Some(3)];
    println!(
        "\n  two unattached masses standing 50.0000 m2 in one another, with the degenerate edge at none\n  \
         of the first mass's corners or at one of them, and the same for the second. The top grid is\n  \
         clip's own reading of the shared area and the lower one is whether `UnattachedOverlap` fired:"
    );
    for reading in [true, false] {
        if reading {
            print!("              second:");
        } else {
            print!("                        ");
        }
        for at_second in none {
            print!(
                "{:>11}",
                if at_second.is_none() {
                    "none".to_owned()
                } else {
                    format!("at {}", at_second.unwrap())
                }
            );
        }
        println!();
        for at_first in none {
            let one = with_degenerate_edge(&first, at_first);
            print!(
                "    first {:>6}:",
                if at_first.is_none() {
                    "none".to_owned()
                } else {
                    format!("at {}", at_first.unwrap())
                }
            );
            for at_second in none {
                let two = with_degenerate_edge(&second, at_second);
                let mut skeleton = BuildingSkeleton::default();
                skeleton.masses = vec![
                    Mass::new(outline(&one), 3.2).with_wall(wall).with_slab(0.4),
                    Mass::new(outline(&two), 3.2).with_wall(wall).with_slab(0.4),
                ];
                if reading {
                    let shared = whiting_building::clip::shared_area(
                        &plan_nodes_of(&skeleton.masses[0]),
                        &plan_nodes_of(&skeleton.masses[1]),
                    );
                    print!("{:>11}", area_reading(shared));
                } else {
                    let named = names_of(&Building::new(skeleton).validate())
                        .iter()
                        .any(|fault| fault == "UnattachedOverlap");
                    print!("{:>11}", if named { "fired" } else { "SILENT" });
                }
            }
            println!();
        }
    }
    println!(
        "    the true answer is 50.0000 at every cell, so a cell under it is the overlap check reading the\n  \
         clipper rather than the two masses, and a cell reading SILENT is a fault the tree cannot see"
    );
}

/// Returns a loop with an edge of no length inserted at `at`, or the loop untouched for `None`.
///
/// **It inserts at `at + 1` a copy of node `at`, which is exactly what `ops::split_outline` does**, so
/// every loop carrying the degenerate edge here is one the crate's own op would have written.
fn with_degenerate_edge(corners: &[[f32; 2]], at: Option<usize>) -> Vec<[f32; 2]> {
    let mut held = corners.to_vec();
    if let Some(at) = at {
        held.insert(at + 1, held[at]);
    }
    held
}

/// Returns a mass's own plan positions in node order, which is what every loop reading in this probe
/// takes.
fn plan_nodes_of(mass: &Mass) -> Vec<Vec2> {
    mass.outline.iter().map(|node| node.position).collect()
}

/// Returns the loop walked east until its own east side stands `over` metres outside an outline whose
/// own east side is `half` metres from the origin.
///
/// **This is the placement [`Violation::VoidNotInside`](whiting_building::validate::Violation::VoidNotInside)
/// exists for, and it is computed from the loop rather than written down per shape**, so every row of the
/// section above leaks by the same `over` and the column of differences is comparable down it.
fn pushed_outside(corners: &[[f32; 2]], half: f32, over: f32) -> Vec<[f32; 2]> {
    let east = corners
        .iter()
        .fold(f32::NEG_INFINITY, |farthest, node| farthest.max(node[0]));
    let shift = half + over - east;
    corners
        .iter()
        .map(|node| [node[0] + shift, node[1]])
        .collect()
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
///
/// **What the sweep also shows, and what it did not show before:** every row carries the reading
/// [`clip`](whiting_building::clip) gives of the loop, and it is `cannot measure it` at every wall and
/// for both pinches. **That is not a change in what these shapes build — the wall sweep and the ring's
/// own manifold, closure and Euler readings are untouched — it is a change in what the check can say
/// about them**, and the reason is in the line above.
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

/// Reports one loop in one line: what `validate` names, what `build` did with it, and what `clip`
/// reads of it.
///
/// # Invariants
///
/// - **The clip reading is on this line because "validate says nothing" cannot tell a silent check
///   from a check that is not running.** It reads `cannot measure it` on the two authored pinches,
///   and **that is the finding the sweep above was measuring without knowing it**: the loop `clip`
///   used to read for two `3 × 3` squares sharing a corner was `9.0000` against a loop that encloses
///   `18.0000`, so the two terms [`Violation::VoidNotInside`](whiting_building::validate::Violation::VoidNotInside)
///   subtracts were both `9.0000` and the silence was cancellation rather than measurement.
/// - **A shape that builds is measured and a shape that does not is not**, because the readings below
///   need a mesh to read and a missing mesh is the answer rather than a gap in it.
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
            let measured = clip_reads(&walked);
            let enclosed = shoelace(&walked).abs();
            println!(
                "  {label} half {:.2} at wall {wall:.2}: builds, validate says {said}, clip reads {measured} \
                 of a loop enclosing {enclosed:.4}, simple {}, volume {:.4} against \
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
