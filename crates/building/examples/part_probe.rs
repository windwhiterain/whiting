//! The parts slice's acceptance: what a template occupies, where a placement puts it, and what is
//! refused.
//!
//! The interesting measurement is not that a part exists but that it lands **in the hole it was made
//! for**: a door leaf is authored in its own frame, placed by a bay slot and a pin, and the numbers
//! here put its box against the opening's own extents — in one frame, because a part that fits its bay
//! and misses its hole is a door in the wall beside the doorway.
//!
//! **The fourth face.** A mass has two faces of its own ring — the outline it was drawn on and the
//! loop a wall's thickness inside it — and `patio_probe` measures the two faces of a *courtyard's*
//! ring. `mass_inner_face_part` measures this pair, and it is reached by name on both sides of the
//! pair: [`FaceRef::new`] for the outer face, [`FaceRef::inner`] for the inner one, and
//! `BuildingSkeleton::faces_of` / `inner_faces_of` for the two lists they are the edge of.
//!
//! Run with `cargo run -p whiting-building --example part_probe`.

use whiting_building::derive::{RoofStyle, Starter, starter};
use whiting_building::expand::part_frame;
use whiting_building::grid::{FaceRef, Rhythm};
use whiting_building::math::{Vec3, cross3, dot3, subtract};
use whiting_building::mesh::SolidId;
use whiting_building::openings::{Inset, Opening};
use whiting_building::ops;
use whiting_building::parts::{PartFrame, Pin, Placement, Template};
use whiting_building::{Building, Element, Mass, MassId, PlanNode};

/// Returns whether a frame is a rotation, as the sign of `(along × up) · inward`.
///
/// **A frame with `+1` can be reproduced by a yaw about `up`, which is what a scene's instance
/// transform carries; `−1` is a mirror**, and `PartFrame::apply` on a mirror hands back every face of
/// the template wound the wrong way round. A part's own placement here is a mass's bay, so this reads
/// `+1` and always has — the same reading the courtyard's bay in `patio_probe` gives, and the reason
/// this slice did not seat a courtyard part inside its ring's thickness.
fn handedness(frame: &whiting_building::parts::PartFrame) -> f32 {
    dot3(cross3(frame.along, frame.up), frame.inward)
}

/// Returns a door leaf: a hollow panel `width` across, `height` tall, `thickness` through.
///
/// # Invariants
///
/// It is authored the way the frame a placement maps into says to author one: `x` across the leaf,
/// `y` up it, `z` through it. Its own frame's origin is its bottom outer corner, so a pin puts that
/// corner where the author wants the leaf to stand.
fn door_leaf(width: f32, height: f32, thickness: f32) -> Template {
    let wall = 0.010;
    let slab = 0.020;
    // The outline stands at the elevation of its own slab, so the template's local origin is its
    // **floor** and its bounds start at `0`: that is what makes `Pin::new(along, 0.0)` stand the leaf
    // on the bay's own sill rather than a slab below it.
    let outline = vec![
        PlanNode::new([0.0, 0.0], slab),
        PlanNode::new([width, 0.0], slab),
        PlanNode::new([width, thickness], slab),
        PlanNode::new([0.0, thickness], slab),
    ];
    Template::new(
        "door leaf",
        vec![Mass::new(outline, height).with_wall(wall).with_slab(slab)],
        Vec::new(),
    )
}

/// Measures a part standing on **a mass's own inner wall face**: the fourth of the four faces of a
/// ring this crate can name, and the one no scene measured.
///
/// # Invariants
///
/// **Two documents differing in one field.** [`patio_probe`] measures the two faces of a
/// *courtyard's* ring; this measures the two faces of a *mass's* ring, with the same starter, the
/// same rhythm, the same template and the same pin, so every difference printed below is
/// `loop_side` and nothing else.
///
/// **The determinant cannot tell these two faces apart, and the distance can.** `inward` is the
/// walker's left on every face of a ring, so `(along × up) · inward` reads `+1` on all four and the
/// handedness test says nothing about which loop a face named. What does say it is the sign of the
/// way the part's own thickness runs: out of the outer face and into the masonry, and out of the
/// inner face **into the mass's own rooms**. The plate is seated on that inner loop, so the plate's
/// own plan extent *is* the rooms, and the part's box is read against it — the inner loop, the wall
/// between the two loops, and the building's own outline are three numbers a reader can check.
///
/// **The control is the same part on the same face with `inward` reversed, and it is asked of
/// [`whiting_building::validate::mesh`] rather than of a second build.** A `-1` frame is a mirror,
/// so every face of the template comes back wound the other way and the signed volume goes
/// negative. Putting it beside this part — rather than beside a document that cannot express a
/// mirror at all — is what makes "the volume matches the template's" mean something.
fn mass_inner_face_part(columns: &Rhythm, rows: &Rhythm) {
    let plan = Starter {
        width: 7.0,
        depth: 11.0,
        roof: RoofStyle::Hip,
        ..Starter::default()
    };
    let mut skeleton = starter(&plan).skeleton.clone();
    let mass = MassId(0);
    let outside = FaceRef::new(mass, 1);
    let inside = FaceRef::inner(mass, 1);

    println!("\n# a part on a mass's own inner wall face");
    println!(
        "  the outer face is {outside:?} and the inner one is {inside:?} — the same edge of the same \
         mass, read off the loop a wall's thickness in"
    );

    // **The rhythm is laid on both halves of the ring, through the two lists that name them.**
    // `faces_of` walks the outline and `inner_faces_of` the loop a wall's thickness inside it, so
    // chaining them is what makes "every face of this mass" reach the inside — which is the whole
    // of what the second constructor is for. The two loops are walked in step, so edge 1 of each is
    // the pair measured below, and a rhythm alone emits no geometry.
    for face in skeleton
        .faces_of(mass)
        .into_iter()
        .chain(skeleton.inner_faces_of(mass))
    {
        ops::split_face(&mut skeleton, face, columns, rows);
    }
    let outside_length = skeleton
        .face_length(outside)
        .expect("the outline's own edge");
    let inside_length = skeleton
        .face_length(inside)
        .expect("the offset loop's edge");
    let outside_bay = skeleton.bay(outside, 0, 0).expect("the outer grid");
    let inside_bay = skeleton.bay(inside, 0, 0).expect("the inner grid");
    println!(
        "  its edge is {:.4} m against the outer face's {:.4} m — {:.4} m shorter at **each** end, \
         because this mitre runs inwards, so a mass's inner loop is the shorter of its two loops and \
         a courtyard's is the longer",
        inside_length,
        outside_length,
        (outside_length - inside_length) * 0.5
    );
    println!(
        "  bay (0, 0) is [{:.4}, {:.4}] on the outer face and [{:.4}, {:.4}] on the inner one: the \
         same rhythm laid by the same op over two different edges",
        outside_bay.from, outside_bay.to, inside_bay.from, inside_bay.to
    );

    let outside_template = ops::add_template(&mut skeleton, door_leaf(0.9, 1.35, 0.06));
    let inside_template = ops::add_template(&mut skeleton, door_leaf(0.9, 1.35, 0.06));
    let outside_part = ops::place_part(
        &mut skeleton,
        Placement::new(outside_template, outside, 0, 0, Pin::new(0.5, 0.0)),
    )
    .expect("the outer bay is there");
    let inside_part = ops::place_part(
        &mut skeleton,
        Placement::new(inside_template, inside, 0, 0, Pin::new(0.5, 0.0)),
    )
    .expect("the inner bay is there");

    let outside_frame = part_frame(&skeleton, outside_part)
        .expect("the outer frame resolves")
        .expect("the outer placement resolves");
    let inside_frame = part_frame(&skeleton, inside_part)
        .expect("the inner frame resolves")
        .expect("the inner placement resolves");
    println!(
        "  its frame: origin ({:.4}, {:.4}, {:.4}), along ({:.1}, {:.1}, {:.1}), up ({:.1}, {:.1}, \
         {:.1}), inward ({:.1}, {:.1}, {:.1})",
        inside_frame.origin[0],
        inside_frame.origin[1],
        inside_frame.origin[2],
        inside_frame.along[0],
        inside_frame.along[1],
        inside_frame.along[2],
        inside_frame.up[0],
        inside_frame.up[1],
        inside_frame.up[2],
        inside_frame.inward[0],
        inside_frame.inward[1],
        inside_frame.inward[2],
    );
    println!(
        "  and (along x up) . inward = {:+.1}, which is what the outer face reads too ({:+.1}) — the \
         handedness test cannot tell these two faces apart, and the distance below is what does",
        handedness(&inside_frame),
        handedness(&outside_frame)
    );

    let document = Building::new(skeleton.clone());
    let faults = document.validate();
    println!("  the document reports {} violation(s)", faults.len());
    for fault in &faults {
        println!("    {fault:?}");
    }
    let Ok(built) = document.build() else {
        println!("  it does not build, which is the fault this section was written to look for");
        return;
    };

    let outside_solid = SolidId::Part(outside_part);
    let inside_solid = SolidId::Part(inside_part);
    let (mass_low, mass_high) = built.bounds_of(SolidId::Mass(mass));
    let outside_volume = built.solid_signed_volume(outside_solid);
    let inside_volume = built.solid_signed_volume(inside_solid);
    for (label, solid, volume) in [
        ("outer", outside_solid, outside_volume),
        ("inner", inside_solid, inside_volume),
    ] {
        println!(
            "  {label}: {solid:?} reads {} faces, loops {}, euler {}, manifold {}, volume {volume:.6}",
            built.faces_of(solid).len(),
            built.solid_boundary_loops(solid),
            built.solid_euler_characteristic(solid),
            built.solid_is_manifold(solid)
        );
    }
    let held = door_leaf(0.9, 1.35, 0.06)
        .mesh()
        .expect("the template stands in its own frame");
    let template_volume = held.solid_signed_volume(SolidId::Mass(MassId(0)));
    println!(
        "  the template's own solid reads volume {template_volume:.6} in its own frame, and the two \
         parts stand {:.6} and {:.6} from it: the sixth digit is f32 cancellation in the divergence \
         sum over world coordinates near {:.1} m, which is why the crate's own predicate is the \
         **sign** of a volume and never an equality with the template's",
        (outside_volume - template_volume).abs(),
        (inside_volume - template_volume).abs(),
        mass_high[0]
    );

    let (low, high) = built.bounds_of(inside_solid);
    let (outer_low, outer_high) = built.bounds_of(outside_solid);
    println!(
        "  where the outer one stands: plan x [{:.4}, {:.4}], z [{:.4}, {:.4}], from y {:.4} to {:.4}",
        outer_low[0], outer_high[0], outer_low[2], outer_high[2], outer_low[1], outer_high[1]
    );
    let plate = built
        .faces
        .iter()
        .find_map(|face| match face.element {
            Element::Slab(id, _) => Some(SolidId::Slab(id)),
            _ => None,
        })
        .expect("the starter seats a plate on the mass's own inner loop");
    let (rooms_low, rooms_high) = built.bounds_of(plate);
    println!(
        "  and where the inner one stands: plan x [{:.4}, {:.4}], z [{:.4}, {:.4}], from y {:.4} \
         to {:.4}",
        low[0], high[0], low[2], high[2], low[1], high[1]
    );
    println!(
        "  against the three numbers a reader can check: the mass's own outline is x [{:.4}, \
         {:.4}], its inner loop — the plate's extent, because the plate is seated on that loop — is \
         x [{:.4}, {:.4}], and the wall between the two is one {:.4} m thickness",
        mass_low[0],
        mass_high[0],
        rooms_low[0],
        rooms_high[0],
        mass_high[0] - rooms_high[0]
    );
    let in_the_rooms = low[0] >= rooms_low[0] - 1.0e-3
        && high[0] <= rooms_high[0] + 1.0e-3
        && low[2] >= rooms_low[2] - 1.0e-3
        && high[2] <= rooms_high[2] + 1.0e-3;
    let clear_of_the_wall = high[0] <= rooms_high[0] + 1.0e-3;
    let inside_the_building = low[0] >= mass_low[0] - 1.0e-3 && high[0] <= mass_high[0] + 1.0e-3;
    println!(
        "  so it stands **in the mass's own rooms**: {in_the_rooms}, out of the wall's thickness: \
         {clear_of_the_wall}, and inside the building: {inside_the_building} — its depth runs from \
         the inner loop's own line at x {:.4} towards x {:.4}, which is away from the masonry",
        inside_frame.origin[0],
        low[0].min(high[0])
    );

    println!("\n# the control: the same part on the same face, with `inward` reversed");
    let mirrored = PartFrame::new(
        inside_frame.origin,
        inside_frame.along,
        inside_frame.up,
        [
            -inside_frame.inward[0],
            -inside_frame.inward[1],
            -inside_frame.inward[2],
        ],
    );
    println!(
        "  its determinant is {:+.1} against the {:+.1} above, so it is a mirror rather than a \
         rotation — the frame the part-in-a-court slice measured as unavailable",
        handedness(&mirrored),
        handedness(&inside_frame)
    );
    let mut control = built.clone();
    let mut owned: Vec<usize> = Vec::new();
    for face in &control.faces {
        match face.element {
            Element::Part { placement, .. } if placement == inside_part => {
                owned.extend(face.indices.iter().map(|index| *index as usize));
            }
            _ => {}
        }
    }
    owned.sort_unstable();
    owned.dedup();
    for index in &owned {
        let here: Vec3 = control.points[*index];
        let moved = subtract(here, inside_frame.origin);
        let local = [
            dot3(moved, inside_frame.along),
            dot3(moved, inside_frame.up),
            dot3(moved, inside_frame.inward),
        ];
        control.points[*index] = mirrored.apply(local);
    }
    println!(
        "  so the same solid reads volume {:.6} where the part reads {:.6} and the template reads \
         {template_volume:.6}",
        control.solid_signed_volume(inside_solid),
        built.solid_signed_volume(inside_solid)
    );
    let reported = whiting_building::validate::mesh(&skeleton, &control);
    let inside_out = reported
        .iter()
        .filter(|fault| matches!(fault, whiting_building::Violation::SolidInsideOut { .. }))
        .count();
    for fault in &reported {
        println!("    {fault:?}");
    }
    println!(
        "  the document's own mesh check reports {} violation(s) on the control, {inside_out} of \
         them a solid inside out — which is the reading that would catch a frame like this one",
        reported.len()
    );

    // **What the inner face will not take, measured here rather than carried over from the
    // courtyard.** A hole is cut through both of a ring's faces from the loop the walk reads, so
    // one authored on the far loop is a second rectangle for one hole with no second wall to put it
    // in. A mass's inner loop is the far loop of its own ring, so the same refusal is the right one
    // — and it is the whole of what this face gives up, since it takes a rhythm, bays and parts.
    let mut cut = skeleton.clone();
    let _ = ops::add_opening(
        &mut cut,
        Opening::new(inside, 1, 0, Inset::new(0.3, 0.3, 0.3, 0.3)),
    );
    let faults = Building::new(cut.clone()).validate();
    println!("\n# and what that face will not take");
    println!(
        "  a window on the inner loop, where the same document on the outer face cuts, is {} \
         violation(s):",
        faults.len()
    );
    for fault in &faults {
        println!("    {fault:?}");
    }
    match Building::new(cut).build() {
        Ok(_) => println!("  and it builds, which is the fault"),
        Err(error) => println!("  and it does not build: {:?}", error.problems.first()),
    }
}

fn main() {
    let plan = Starter {
        width: 7.0,
        depth: 11.0,
        roof: RoofStyle::Hip,
        ..Starter::default()
    };
    let mut skeleton = starter(&plan).skeleton.clone();
    let mass = MassId(0);
    let columns = Rhythm::Even { count: 3 };
    let rows = Rhythm::Even { count: 2 };
    for face in skeleton.faces_of(mass) {
        ops::split_face(&mut skeleton, face, &columns, &rows);
    }
    let face = FaceRef::new(mass, 1);

    // The doorway this leaf is made for: bay 0 of the ground storey, cut from ground level.
    let doorway = Opening::new(face, 0, 0, Inset::new(0.5, 0.3, 0.0, 0.2));
    let opening = ops::add_opening(&mut skeleton, doorway).expect("the slot is in the grid");
    let (_, hole) = skeleton.aperture(opening).expect("just authored");
    println!(
        "# the doorway: [{:.4}, {:.4}] along the edge, [{:.4}, {:.4}] up it",
        hole.from, hole.to, hole.low, hole.high
    );

    let leaf = door_leaf(0.9, 1.35, 0.06);
    let bounds = leaf.bounds().expect("the leaf holds a mass");
    println!(
        "# the leaf in its own frame: x [{:.4}, {:.4}], y [{:.4}, {:.4}], z [{:.4}, {:.4}]",
        bounds.x.0, bounds.x.1, bounds.y.0, bounds.y.1, bounds.z.0, bounds.z.1
    );
    let template = ops::add_template(&mut skeleton, leaf);

    // The pin stands the leaf's own bottom outer corner on the bay's own sill and jamb, and a pin is
    // measured **from the bay's corner**: bay 0 starts at the edge's start, so here the leaf's `x` and
    // the hole's `from` are the same number and the general rule is easy to mistake for it.
    let doorway_bay = skeleton.bay(face, 0, 0).expect("the grid is there");
    let at = Pin::new(hole.from - doorway_bay.from, 0.0);
    let placed = ops::place_part(&mut skeleton, Placement::new(template, face, 0, 0, at))
        .expect("the template and the slot are both there");
    let standing = skeleton.placed(placed).expect("just placed");
    let bay = standing
        .bay
        .expect("the leaf stands in a bay rather than on a wall");
    let reach = standing.reach;
    println!(
        "# the leaf placed in bay [{:.4}, {:.4}] x [{:.4}, {:.4}]: x [{:.4}, {:.4}], y [{:.4}, {:.4}]",
        bay.from,
        bay.to,
        bay.low,
        bay.high,
        reach.x.0 + bay.from,
        reach.x.1 + bay.from,
        reach.y.0 + bay.low,
        reach.y.1 + bay.low
    );
    let inside_hole = reach.x.0 + bay.from >= hole.from - 1.0e-3
        && reach.x.1 + bay.from <= hole.to + 1.0e-3
        && reach.y.0 + bay.low >= hole.low - 1.0e-3
        && reach.y.1 + bay.low <= hole.high + 1.0e-3;
    println!("  the leaf fits its hole: {inside_hole}");

    println!("\n# a part that does not fit its bay is placed and reported, not refused");
    let tall = ops::add_template(&mut skeleton, door_leaf(0.9, 2.40, 0.06));
    let too_tall = ops::place_part(
        &mut skeleton,
        Placement::new(tall, face, 0, 1, Pin::new(0.6, 0.0)),
    )
    .expect("the bay is there");
    let big = skeleton.placed(too_tall).expect("just placed").reach;
    println!(
        "  a 2.40 m leaf in a {:.4} m storey: y [{:.4}, {:.4}], so it stands {:.4} m past the head",
        bay.height(),
        big.y.0,
        big.y.1,
        big.y.1 - bay.height()
    );

    println!("\n# refusals at the verb");
    let cases = [
        (
            "a template the document does not hold",
            Placement::new(99, face, 0, 0, Pin::new(0.0, 0.0)),
        ),
        (
            "a bay the grid does not hold",
            Placement::new(template, face, 7, 9, Pin::new(0.0, 0.0)),
        ),
    ];
    for (name, placement) in cases {
        println!("  {name}: {:?}", ops::place_part(&mut skeleton, placement));
    }

    println!("\n# the document");
    let violations = Building::new(skeleton.clone()).validate();
    println!("  violations: {}", violations.len());
    for violation in &violations {
        println!("    {violation:?}");
    }

    println!("\n# the mesh: a part is a solid of its own");
    // **The frame itself, which is the thing this slice changed.** The numbers below are the ones a
    // mass-wall placement has to keep reading whatever a courtyard's bay does, and they are printed
    // from [`part_frame`] rather than re-derived here, so the reading below is the same object
    // `build` applies.
    let frame = part_frame(&skeleton, placed)
        .expect("the frame resolves")
        .expect("the placement resolves");
    println!(
        "# the frame it stands in: origin ({:.4}, {:.4}, {:.4}), along ({:.1}, {:.1}, {:.1}), \
         up ({:.1}, {:.1}, {:.1}), inward ({:.1}, {:.1}, {:.1})",
        frame.origin[0],
        frame.origin[1],
        frame.origin[2],
        frame.along[0],
        frame.along[1],
        frame.along[2],
        frame.up[0],
        frame.up[1],
        frame.up[2],
        frame.inward[0],
        frame.inward[1],
        frame.inward[2],
    );
    println!(
        "  and (along x up) . inward = {:+.1}: a mass's own bay is unchanged by anything a courtyard's is",
        handedness(&frame)
    );
    match Building::new(skeleton.clone()).build() {
        Ok(mesh) => {
            let mut solids: std::collections::BTreeSet<whiting_building::SolidId> =
                std::collections::BTreeSet::new();
            for face in &mesh.faces {
                solids.insert(face.element.solid());
            }
            for solid in solids {
                match solid {
                    whiting_building::SolidId::Mass(_)
                    | whiting_building::SolidId::Roof(_)
                    | whiting_building::SolidId::Wall(_)
                    | whiting_building::SolidId::Slab(_)
                    | whiting_building::SolidId::Void { .. } => {}
                    whiting_building::SolidId::Part(_) => {
                        let (low, high) = mesh.bounds_of(solid);
                        println!(
                            "  {solid:?}: {} faces, loops {}, euler {}, manifold {}, volume {:.6}",
                            mesh.faces_of(solid).len(),
                            mesh.solid_boundary_loops(solid),
                            mesh.solid_euler_characteristic(solid),
                            mesh.solid_is_manifold(solid),
                            mesh.solid_signed_volume(solid)
                        );
                        println!(
                            "    it stands in plan x [{:.4}, {:.4}], z [{:.4}, {:.4}], and from \
                             y {:.4} to {:.4}",
                            low[0], high[0], low[2], high[2], low[1], high[1]
                        );
                    }
                }
            }
            let (mass_low, mass_high) = mesh.bounds_of(whiting_building::SolidId::Mass(mass));
            println!(
                "  the mass it stands in: plan x [{:.4}, {:.4}], z [{:.4}, {:.4}], y [{:.4}, {:.4}]",
                mass_low[0], mass_high[0], mass_low[2], mass_high[2], mass_low[1], mass_high[1]
            );
        }
        Err(error) => println!("  refused: {:?}", error.problems),
    }

    println!("\n# a placement whose slot has gone is named, not silently dropped");
    // Clearing the face's rhythm orphans the doorway **and** both leaves, and the crate's shape is
    // all-or-nothing: the problems come back as a list and no mesh is handed over. What the probe
    // measures is that the leaves are *in* that list rather than missing from it — a lookup that
    // answers `None` filters, and the companion that names what it dropped is the whole point.
    let mut orphaned = skeleton.clone();
    ops::clear_face(&mut orphaned, face);
    match Building::new(orphaned.clone()).build() {
        Ok(_) => {
            println!("  built with two orphaned placements and said nothing, which is the fault")
        }
        Err(error) => {
            let leaves = error
                .problems
                .iter()
                .filter(|problem| {
                    matches!(
                        problem,
                        whiting_building::Problem::PlacementUnresolved { .. }
                    )
                })
                .count();
            println!(
                "  {} problems, {leaves} of them leaves: {:?}",
                error.problems.len(),
                error.problems
            );
        }
    }
    let violations = Building::new(orphaned).validate();
    let named = violations
        .iter()
        .filter(|violation| {
            matches!(
                violation,
                whiting_building::Violation::PlacementBayMissing { .. }
            )
        })
        .count();
    println!("  and the document's own verdict names {named} placements, {violations:?}");

    mass_inner_face_part(&columns, &rows);
}
