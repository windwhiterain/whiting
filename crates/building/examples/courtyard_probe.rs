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
/// wrong one for "is this loop simple": a loop that touches itself at a point is not simple, and a
/// ring built on one has a face with no material behind it. The walk accepts a touch; this does
/// not, so it can catch a shape the walk let through.
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
fn offset_loop_from_mesh(built: &BuildingMesh, walked: &[Vec2]) -> Option<Vec<Vec2>> {
    let count = walked.len();
    let mut found: Vec<Option<Vec2>> = vec![None; count];
    for face in &built.faces {
        if face.element != Element::Mass(MassId(0), MassSurface::VoidBase { void: 0 }) {
            continue;
        }
        let corners = built.triangle(face);
        let mut authored = Vec::new();
        let mut other = None;
        for corner in corners {
            let plan = [corner[0], corner[2]];
            match walked.iter().position(|node| {
                (node[0] - plan[0]).abs() <= TOLERANCE && (node[1] - plan[1]).abs() <= TOLERANCE
            }) {
                Some(at) => authored.push(at),
                None => other = Some(plan),
            }
        }
        let [Some(from), Some(to)] = [authored.first().copied(), authored.get(1).copied()] else {
            continue;
        };
        let Some(edge) = edge_between(count, from, to) else {
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

/// Returns the 10 × 10 mass a void stands in: the same housing every courtyard probe below uses.
fn housing() -> Mass {
    Mass::new(
        outline(&[[-5.0, -5.0], [5.0, -5.0], [5.0, 5.0], [-5.0, 5.0]]),
        3.2,
    )
    .with_wall(0.3)
    .with_slab(0.4)
}

/// Prints one void loop and the loop its wall is built on, and every reading the walk has to pass.
fn measure_void(label: &str, corners: &[[f32; 2]], wall: f32) {
    let walked = wound_positive(corners);
    let mut skeleton = BuildingSkeleton::default();
    skeleton.masses = vec![housing().with_wall(wall).with_voids(vec![outline(corners)])];
    let document = Building::new(skeleton);
    let violations = document.validate();
    println!(
        "\n  {label}: {} node(s), {} reflex corner(s), void area {:.4}",
        walked.len(),
        reflex_corners(&walked),
        shoelace(&walked)
    );
    print_loop("    the void loop", &walked);
    if !violations.is_empty() {
        for fault in &violations {
            println!("    {fault:?}");
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
    println!(
        "    the ring: manifold {}, loops {}, euler {}, volume {volume:.4} against (outer - inner) x height \
         {:.4} — off {:.6}",
        built.solid_is_manifold(ring),
        built.solid_boundary_loops(ring),
        built.solid_euler_characteristic(ring),
        (measured - shoelace(&walked)) * height,
        (volume - (measured - shoelace(&walked)) * height).abs()
    );
    println!(
        "    every flat face stands on its own loop: {}",
        faces_sit_on_their_loops(&built, &walked, &offset)
    );
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
    let faults: Vec<String> = reported
        .iter()
        .map(|fault| {
            format!("{fault:?}")
                .split([' ', '{'])
                .next()
                .unwrap_or("a violation")
                .to_owned()
        })
        .collect();
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
            let names: Vec<String> = error
                .problems
                .iter()
                .map(|problem| {
                    format!("{problem:?}")
                        .split([' ', '{'])
                        .next()
                        .unwrap_or("a problem")
                        .to_owned()
                })
                .collect();
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
