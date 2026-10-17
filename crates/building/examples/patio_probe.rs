//! The courtyard, drawn: **the void is a ring of wall, and the crate proves it by measurement**.
//!
//! # What this measures
//!
//! `Mass::voids` lets a document author a courtyard — a loop inside the outline that the mass has a
//! hole into. In the volume model the drawing is a **second ring**, not a hole in the first: the void
//! loop and the same loop offset outward by `mass.wall`, closed at the top and at the base exactly as
//! the mass's own ring is closed, standing on the plate without cutting it. This probe is the record
//! of what that is and is not:
//!
//! 1. a mass with **no** void is unchanged — the control, and it is what every other slice's numbers
//!    were measured against;
//! 2. the same mass with a 4 × 4 void **builds**, and its courtyard solid reads **manifold, zero
//!    boundary loops and Euler characteristic 0** — a ring is a torus, and `0` is what a torus is
//!    (`2` is a box). The mass's own ring is still `0` beside it, which is the point: two solids, two
//!    answers, neither borrowed;
//! 3. a void wound the other way round is the same courtyard (the winding is normalised, as
//!    `fill::ring_fill` has always normalised the loop it bridges);
//! 4. the volume is **the annulus times the height**, computed here rather than compared with the
//!    mesh, because a ring that is wrong by area is the failure this crate has shipped twice;
//! 5. the four faults a courtyard can break are each reported, and a fault still refuses the build;
//! 6. **a hole cut in a courtyard's wall**, which is the same `Opening` a mass's wall takes: it is cut
//!    through both of the ring's faces at once, its four reveals name it, and the ring reads one more
//!    handle — `2 − 2(1 + 1) = −2` — which is measured here rather than assumed, against the mass's
//!    own ring reading `0` beside it in the very same document;
//! 7. **a doorway in that wall** is a notch in the ring's base and not a handle, and reads `0`;
//! 8. every way the cut cannot be made is refused by name, and each names the **loop** it is about so
//!    that "edge 1" is never two different walls.
//!
//! Run with `cargo run -p whiting-building --example patio_probe`.

use whiting_building::identity::Identity;
use whiting_building::mesh::{MassSurface, SolidId};
use whiting_building::openings::RevealSide;
use whiting_building::parts::{Pin, Placement, Template};
use whiting_building::{
    Building, BuildingSkeleton, Element, FaceRef, Inset, Mass, MassId, Opening, PlanNode, Rhythm,
    ops,
};

/// Returns a rectangular outline.
fn rectangle(x0: f32, z0: f32, x1: f32, z1: f32) -> Vec<PlanNode> {
    [[x0, z0], [x1, z0], [x1, z1], [x0, z1]]
        .into_iter()
        .map(|corner| PlanNode::new(corner, 0.0))
        .collect()
}

/// Returns a mass with one courtyard, and the solid that courtyard is.
fn courtyard(loop_nodes: Vec<PlanNode>) -> Building {
    let mut skeleton = BuildingSkeleton::default();
    skeleton.masses = vec![
        Mass::new(rectangle(-5.0, -5.0, 5.0, 5.0), 3.2)
            .with_wall(0.3)
            .with_slab(0.4)
            .with_voids(vec![loop_nodes]),
    ];
    Building::new(skeleton)
}

/// Returns one courtyard's solid.
fn courtyard_ring() -> SolidId {
    SolidId::Void {
        mass: MassId(0),
        void: 0,
    }
}

/// Returns the mass a 4 × 4 courtyard stands in: a 10 × 10 outline, a 0.3 m wall, a 0.4 m slab.
fn housed_patio() -> Mass {
    Mass::new(rectangle(-5.0, -5.0, 5.0, 5.0), 3.2)
        .with_wall(0.3)
        .with_slab(0.4)
        .with_voids(vec![rectangle(-2.0, -2.0, 2.0, 2.0)])
}

/// Returns that mass's skeleton with a rhythm laid on **one courtyard wall**: three bays across the
/// void loop's own edge and two storeys up the ring, which is the same two rhythms a mass's facade
/// takes and the same op that lays them.
///
/// The face is a courtyard's and not the mass's own, and that is the whole of the difference the
/// document has to express: a face names a loop, and a courtyard is one of a mass's loops.
fn court_face() -> FaceRef {
    FaceRef::court(MassId(0), 0, 0)
}

/// Returns the rectangle an opening cuts, as the document reads it, for the report below.
fn hole_of(skeleton: &BuildingSkeleton, index: usize) -> (f32, f32, f32, f32) {
    let (_, extents) = skeleton.aperture(index).expect("the slot is in the grid");
    (extents.from, extents.to, extents.low, extents.high)
}

/// Returns the volume of the 4 × 4 courtyard's ring with nothing cut into it, written down.
///
/// **The arithmetic, not a reading off the expander**: `(4 + 0.6)² − 4²` of annulus standing `3.2` m
/// high, because a ring fill that overlaps itself is the failure this crate has shipped twice and the
/// volume is where it shows. Every "and that is what a hole took away" number below is this one minus
/// the mesh's own.
fn uncut_ring_volume() -> f32 {
    ((4.0 + 0.6) * (4.0 + 0.6) - 4.0 * 4.0) * 3.2
}

/// Returns how many base strips an un-cut ring's annulus is drawn in: one per loop edge.
///
/// `O(1)`, and it is the mesh's own face count read off the document's loop length rather than a
/// number typed into a report.
fn base_strips(skeleton: &BuildingSkeleton) -> usize {
    skeleton
        .face_loop(court_face())
        .map_or(0, |loop_nodes| loop_nodes.len())
}

/// Returns a small template to stand in a courtyard's bay.
///
/// The leaf is a template like any other and its **slot** is a courtyard bay, so the placement itself
/// is perfectly ordinary: everything the document says resolves, and the refusal comes from the frame
/// the expander cannot read.
fn leaf() -> Template {
    let wall = 0.010;
    let slab = 0.020;
    let outline = vec![
        PlanNode::new([0.0, 0.0], slab),
        PlanNode::new([0.6, 0.0], slab),
        PlanNode::new([0.6, 0.05], slab),
        PlanNode::new([0.0, 0.05], slab),
    ];
    Template::new(
        "courtyard leaf",
        vec![Mass::new(outline, 1.8).with_wall(wall).with_slab(slab)],
        Vec::new(),
    )
}

fn main() {
    // 1. The control: no void, and the mass is what it always was.
    let mut plain = BuildingSkeleton::default();
    plain.masses = vec![
        Mass::new(rectangle(-5.0, -5.0, 5.0, 5.0), 3.2)
            .with_wall(0.3)
            .with_slab(0.4),
    ];
    let control = Building::new(plain);
    let control_violations = control.validate();
    let control_volume = match control.build() {
        Ok(built) => format!("{:.4}", built.solid_signed_volume(SolidId::Mass(MassId(0)))),
        Err(error) => format!("does not build: {:?}", error.problems.first()),
    };
    println!("# a 10 x 10 mass with no courtyard");
    println!(
        "  {} violation(s), its own ring's volume {control_volume}",
        control_violations.len()
    );

    // 2. The same mass with a courtyard authored: drawn, as a ring of its own.
    let mut patio = BuildingSkeleton::default();
    patio.masses = vec![
        Mass::new(rectangle(-5.0, -5.0, 5.0, 5.0), 3.2)
            .with_wall(0.3)
            .with_slab(0.4)
            .with_voids(vec![rectangle(-2.0, -2.0, 2.0, 2.0)]),
    ];
    let with_patio = Building::new(patio);
    let violations = with_patio.validate();
    println!("\n# the same mass with a 4 x 4 courtyard authored");
    println!("  the document reports {} violation(s)", violations.len());
    for violation in &violations {
        println!("    {violation:?}");
    }
    match with_patio.build() {
        Ok(built) => {
            let ring = courtyard_ring();
            let mass = SolidId::Mass(MassId(0));
            println!(
                "  it builds: the courtyard reads manifold {}, loops {}, euler {}",
                built.solid_is_manifold(ring),
                built.solid_boundary_loops(ring),
                built.solid_euler_characteristic(ring)
            );
            println!(
                "  the mass's own ring still reads manifold {}, loops {}, euler {}",
                built.solid_is_manifold(mass),
                built.solid_boundary_loops(mass),
                built.solid_euler_characteristic(mass)
            );
            // **The area check is arithmetic, not a comparison of two readings of the same code.** A
            // 4 x 4 courtyard in a 0.3 m wall is (4 + 0.6)^2 - 4^2 = 6.76 m^2 of annulus standing
            // 3.2 m high, and a ring fill that overlaps itself is the failure this crate has shipped
            // twice — the volume is where it shows.
            let expected = ((4.0 + 0.6) * (4.0 + 0.6) - 4.0 * 4.0) * 3.2;
            let volume = built.solid_signed_volume(ring);
            println!(
                "  its volume {volume:.4} against the annulus times the height {expected:.4} ({:.6} off)",
                (volume - expected).abs()
            );
            // **The base of the ring is its own element, and the paving is nobody's**: the plate is
            // not cut, so every face here belongs to the ring or to the mass, and the identity table
            // says which without a reader having to know the crate's internals.
            let table = Identity::of(&built, 1);
            let named: Vec<String> = table
                .rows()
                .iter()
                .filter(|row| {
                    matches!(
                        row.element,
                        Element::Mass(_, MassSurface::VoidWall { .. })
                            | Element::Mass(_, MassSurface::VoidInner { .. })
                            | Element::Mass(_, MassSurface::VoidTop { .. })
                            | Element::Mass(_, MassSurface::VoidBase { .. })
                    )
                })
                .map(|row| row.path.clone())
                .collect();
            println!(
                "  the table names {} courtyard face(s), from \"{}\" to \"{}\"",
                named.len(),
                named.first().cloned().unwrap_or_default(),
                named.last().cloned().unwrap_or_default()
            );
        }
        Err(error) => println!("  it does not build: {:?}", error.problems.first()),
    }

    // 3. The same courtyard wound the other way round: the same solid, not an inside-out one.
    let mut reversed: Vec<PlanNode> = rectangle(-2.0, -2.0, 2.0, 2.0);
    reversed.reverse();
    let flipped = courtyard(reversed);
    match flipped.build() {
        Ok(built) => println!(
            "\n# the courtyard authored the other way round\n  it builds: manifold {}, loops {}, euler {}, volume {:.4}",
            built.solid_is_manifold(courtyard_ring()),
            built.solid_boundary_loops(courtyard_ring()),
            built.solid_euler_characteristic(courtyard_ring()),
            built.solid_signed_volume(courtyard_ring())
        ),
        Err(error) => println!(
            "\n# the courtyard authored the other way round\n  it does not build: {:?}",
            error.problems.first()
        ),
    }
    println!(
        "  and the document calls it clean: {} violation(s)",
        flipped.validate().len()
    );

    // **Four courtyards that break the contract, each reported rather than repaired.** A void outside
    // the outline, a void with a notch, two voids that overlap, and — the fault the three above would
    // have missed — a void inside the outline with no room for its own wall.
    let outline = rectangle(-5.0, -5.0, 5.0, 5.0);
    let cases = [
        (
            "a void reaching outside the outline",
            vec![rectangle(3.0, -2.0, 7.0, 2.0)],
        ),
        (
            "a void with a notch (not convex)",
            vec![vec![
                PlanNode::new([-2.0, -2.0], 0.0),
                PlanNode::new([2.0, -2.0], 0.0),
                PlanNode::new([2.0, 2.0], 0.0),
                PlanNode::new([0.0, 0.0], 0.0),
                PlanNode::new([-2.0, 2.0], 0.0),
            ]],
        ),
        (
            "two voids sharing plan area",
            vec![
                rectangle(-2.0, -2.0, 1.0, 1.0),
                rectangle(0.0, 0.0, 2.0, 2.0),
            ],
        ),
        (
            "a void inside the outline with no room for its wall",
            vec![rectangle(-4.9, -4.9, 4.9, 4.9)],
        ),
    ];
    println!("\n# four courtyards that break the contract");
    for (label, voids) in cases {
        let mut skeleton = BuildingSkeleton::default();
        skeleton.masses = vec![
            Mass::new(outline.clone(), 3.2)
                .with_wall(0.3)
                .with_slab(0.4)
                .with_voids(voids),
        ];
        let building = Building::new(skeleton);
        let faults = building.validate();
        println!("  {label}: {} fault(s)", faults.len());
        for fault in &faults {
            println!("    {fault:?}");
        }
        if let Err(error) = building.build() {
            println!("    and it does not build: {:?}", error.problems.first());
        }
    }

    // **The case the two kinds exist for: a courtyard in a mass that also has an opening.** The mass's
    // own ring is cut, so it carries a handle for the window and reads `-2`; the courtyard's ring is cut
    // by nothing and reads `0`. Were both one solid — were a courtyard folded into the mass's face set —
    // the two would sum to `-2` and the check would ask for `-2` and be told `-2`, so a solid that is
    // not a solid would read as exactly the shape it should have.
    let mut opened = BuildingSkeleton::default();
    opened.masses = vec![
        Mass::new(rectangle(-5.0, -5.0, 5.0, 5.0), 3.2)
            .with_wall(0.3)
            .with_slab(0.4)
            .with_voids(vec![rectangle(-2.0, -2.0, 2.0, 2.0)]),
    ];
    let rhythm = Rhythm::Even { count: 3 };
    for face in opened.faces_of(MassId(0)) {
        ops::split_face(&mut opened, face, &rhythm, &rhythm);
    }
    let cut = ops::add_opening(
        &mut opened,
        Opening::new(
            FaceRef::new(MassId(0), 1),
            1,
            1,
            Inset::new(0.5, 0.5, 0.4, 0.3),
        ),
    );
    println!("\n# the same mass with a window as well, which is what a courtyard building has");
    let with_window = Building::new(opened);
    match (cut, with_window.build()) {
        (None, _) => println!("  the window was refused: the slot is not in the grid"),
        (Some(_), Err(error)) => println!("  it does not build: {:?}", error.problems.first()),
        (Some(_), Ok(built)) => {
            let mass = SolidId::Mass(MassId(0));
            let ring = courtyard_ring();
            println!(
                "  the mass reads euler {} (a ring, plus a handle for the window: 2 - 2(1 + 1))",
                built.solid_euler_characteristic(mass)
            );
            println!(
                "  the courtyard reads euler {}, cut by nothing",
                built.solid_euler_characteristic(ring)
            );
            println!(
                "  the document reports {} violation(s)",
                with_window.validate().len()
            );
        }
    }

    println!(
        "\n# the reading: a void is drawn as a ring of its own — manifold, closed, euler 0 — and every void it\n# cannot draw is refused by name rather than quietly left out of the mesh."
    );

    // **A hole in a courtyard's wall, cut by the same `Opening` a mass's wall takes.** The face here is
    // the void loop's own edge, so the grid, the bay and the insets are laid out and read exactly as
    // they are on a mass's facade: nothing about the cut knows it is in a courtyard except which loop
    // the face names, which is what makes this a capability rather than a second implementation.
    let mut opened_court = BuildingSkeleton::default();
    opened_court.masses = vec![housed_patio()];
    let face = court_face();
    let columns = Rhythm::Even { count: 3 };
    let rows = Rhythm::Even { count: 2 };
    let bays = ops::split_face(&mut opened_court, face, &columns, &rows);
    let length = opened_court
        .face_length(face)
        .expect("the courtyard's edge is there");
    let (low, high) = opened_court
        .face_extent(face)
        .expect("the courtyard's edge is there");
    println!(
        "\n# a courtyard's own wall, read through its void loop\n  the face is {face:?}, {length:.4} m long, ground {low:.4}, top {high:.4}, and it carries {bays:?} bay/ies",
    );

    let window = Opening::new(face, 1, 1, Inset::new(0.3, 0.3, 0.3, 0.3));
    let cut = ops::add_opening(&mut opened_court, window);
    let Some(window_index) = cut else {
        println!("  the window was refused: the slot is not in the grid");
        return;
    };
    let (from, to, sill, head) = hole_of(&opened_court, window_index);
    println!(
        "  a window in the top storey's middle bay: index {window_index}, hole [{from:.4}, {to:.4}] x \
         [{sill:.4}, {head:.4}], {:.4} m² of wall",
        (to - from) * (head - sill)
    );

    let with_window = Building::new(opened_court.clone());
    let violations = with_window.validate();
    println!("  the document reports {} violation(s)", violations.len());
    for violation in &violations {
        println!("    {violation:?}");
    }
    match with_window.build() {
        Ok(built) => {
            let ring = courtyard_ring();
            let mass = SolidId::Mass(MassId(0));
            println!(
                "  it builds: the courtyard reads manifold {}, loops {}, euler {}",
                built.solid_is_manifold(ring),
                built.solid_boundary_loops(ring),
                built.solid_euler_characteristic(ring)
            );
            println!(
                "    which is 2 - 2(1 + 1) = -2: a hole through a ring is what makes it not-a-ring",
            );
            println!(
                "  the mass's own ring is untouched: manifold {}, loops {}, euler {}",
                built.solid_is_manifold(mass),
                built.solid_boundary_loops(mass),
                built.solid_euler_characteristic(mass)
            );
            // **The material removed is the hole's own area times the ring's own thickness**, computed
            // here rather than read back off the expander: a ring that closed and counted its handle
            // correctly while removing the wrong amount of masonry would pass every predicate there is.
            let removed = (to - from) * (head - sill) * 0.3;
            let volume = built.solid_signed_volume(ring);
            println!(
                "  its volume {volume:.4} against the un-cut ring {:.4} less {removed:.4} of masonry \
                 ({:.6} off)",
                uncut_ring_volume(),
                (volume - (uncut_ring_volume() - removed)).abs()
            );
            // **The reveal names the courtyard, not the mass**: the element carries the loop a hole was
            // cut into, which is what keeps a ring's jambs in the ring's own face set and out of the
            // mass's — otherwise the two would sum their Euler characteristics and read `0` for the
            // wrong reason.
            let reveals: Vec<RevealSide> = built
                .faces
                .iter()
                .filter_map(|face| match face.element {
                    Element::Mass(_, MassSurface::Reveal { side, .. }) => Some(side),
                    _ => None,
                })
                .collect();
            let mut sides: Vec<RevealSide> = RevealSide::ALL.to_vec();
            sides.retain(|side| reveals.contains(side));
            let named: Vec<String> = Identity::of(&built, 1)
                .rows()
                .iter()
                .filter(|row| matches!(row.element, Element::Mass(_, MassSurface::Reveal { .. })))
                .map(|row| row.path.clone())
                .collect();
            println!(
                "  {} reveal face(s) for {} hole(s), covering {sides:?}, and the table names them:\n{}",
                reveals.len(),
                1,
                named
                    .iter()
                    .map(|path| format!("    {path}"))
                    .collect::<Vec<_>>()
                    .join("\n")
            );
        }
        Err(error) => println!("  it does not build: {:?}", error.problems.first()),
    }

    // **The same document without the hole reads what it always read.** This is the control for the
    // reading above: if the courtyard's own ring moved when a window was cut somewhere else in the
    // same document, the two numbers would be one measurement and not two.
    let plain_court = Building::new(BuildingSkeleton {
        masses: vec![housed_patio()],
        ..BuildingSkeleton::default()
    });
    match plain_court.build() {
        Ok(built) => println!(
            "\n# the same courtyard with nothing cut into it\n  the courtyard reads euler {}, manifold {}, loops {}",
            built.solid_euler_characteristic(courtyard_ring()),
            built.solid_is_manifold(courtyard_ring()),
            built.solid_boundary_loops(courtyard_ring()),
        ),
        Err(error) => println!("  it does not build: {:?}", error.problems.first()),
    }

    // **A doorway in a courtyard's wall.** Its sill stands on the ring's own base, which notches that
    // base across the door and leaves a lintel above the head — so the ring is still a ring and the
    // characteristic does not move. A courtyard and a mass are the same shape here too, and the two
    // numbers that say so are measured rather than argued: the ring reads `0` with a door in it and
    // `-2` with a window in it, in documents that differ by nothing else.
    let mut doored = BuildingSkeleton {
        masses: vec![housed_patio()],
        ..BuildingSkeleton::default()
    };
    let door_face = FaceRef::court(MassId(0), 0, 1);
    let _ = ops::split_face(&mut doored, door_face, &columns, &rows);
    let doorway = Opening::new(door_face, 0, 1, Inset::new(0.5, 0.5, 0.0, 0.2));
    let door = ops::add_opening(&mut doored, doorway);
    let Some(door_index) = door else {
        println!("\n# a door in the courtyard's wall\n  refused: the slot is not in the grid");
        return;
    };
    let (from, to, sill, head) = hole_of(&doored, door_index);
    let with_door = Building::new(doored.clone());
    println!(
        "\n# a door in the courtyard's wall, its sill on the ring's own base\n  hole [{from:.4}, {to:.4}] \
         x [{sill:.4}, {head:.4}], so the sill is the ring's ground line exactly"
    );
    println!(
        "  the document reports {} violation(s)",
        with_door.validate().len()
    );
    match with_door.build() {
        Ok(built) => {
            let ring = courtyard_ring();
            // **The base annulus is notched across the door rather than drawn under it**, which is the
            // whole of what makes a door a notch in the ring's section and not a handle through it. It
            // is counted in *triangles* because that is what the mesh holds: the door's edge gains a
            // station at each jamb, so its base is two strips with the door's own span removed between
            // them, and the three un-cut edges keep one each.
            let strips = built
                .faces
                .iter()
                .filter(|face| {
                    matches!(
                        face.element,
                        Element::Mass(_, MassSurface::VoidBase { void: 0 })
                    )
                })
                .count();
            let removed = (to - from) * (head - sill) * 0.3;
            let volume = built.solid_signed_volume(ring);
            println!(
                "  it builds: manifold {}, loops {}, euler {}, and the base annulus is {strips} triangles\n  \
                 where the un-cut ring draws {} — the door's edge is two strips with the door's own span\n  \
                 notched out between them\n  and the volume {volume:.4} is the un-cut {:.4} less {removed:.4} \
                 ({:.6} off)",
                built.solid_is_manifold(ring),
                built.solid_boundary_loops(ring),
                built.solid_euler_characteristic(ring),
                2 * base_strips(&doored) as u32,
                uncut_ring_volume(),
                (volume - (uncut_ring_volume() - removed)).abs()
            );
        }
        Err(error) => println!("  it does not build: {:?}", error.problems.first()),
    }

    // **The door and the window in one document**, which is the reading the two above are halves of.
    let mut both = doored.clone();
    let _ = ops::split_face(&mut both, court_face(), &columns, &rows);
    let _ = ops::add_opening(
        &mut both,
        Opening::new(court_face(), 1, 1, Inset::new(0.3, 0.3, 0.3, 0.3)),
    );
    let with_both = Building::new(both);
    println!(
        "\n# the same courtyard with the door and the window\n  the document reports {} violation(s)",
        with_both.validate().len()
    );
    match with_both.build() {
        Ok(built) => println!(
            "  it builds: euler {}, manifold {}, loops {} — one handle for the window and none for the door",
            built.solid_euler_characteristic(courtyard_ring()),
            built.solid_is_manifold(courtyard_ring()),
            built.solid_boundary_loops(courtyard_ring()),
        ),
        Err(error) => println!("  it does not build: {:?}", error.problems.first()),
    }

    // **Every way the cut cannot be made, each reached through the document and named.** Every one of
    // them names the **loop**, because a mass's wall and a courtyard's are both "edge 1" and reporting
    // one as the other would send an author to the wrong piece of masonry.
    println!("\n# every way a hole cannot be cut into a courtyard's wall");
    let refusals: [(&str, fn(&mut BuildingSkeleton) -> Option<usize>); 6] = [
        ("a slot the courtyard's grid no longer holds", |skeleton| {
            let index = skeleton.openings.push(Opening::new(
                court_face(),
                1,
                1,
                Inset::new(0.3, 0.3, 0.3, 0.3),
            ));
            // **The rhythm is taken away under the opening**, which is the whole reason a slot is an
            // index and not a place: an edit the author made elsewhere orphans this one.
            ops::clear_face(skeleton, court_face());
            Some(index)
        }),
        ("a head that stands at the ring's own top", |skeleton| {
            ops::add_opening(
                skeleton,
                Opening::new(court_face(), 1, 1, Inset::new(0.3, 0.3, 0.3, 0.0)),
            )
        }),
        ("a jamb at the ring's own end", |skeleton| {
            ops::add_opening(
                skeleton,
                Opening::new(court_face(), 1, 0, Inset::new(0.0, 0.3, 0.3, 0.3)),
            )
        }),
        (
            "two holes that share wall in one courtyard face",
            |skeleton| {
                let first = ops::add_opening(
                    skeleton,
                    Opening::new(court_face(), 1, 1, Inset::new(0.1, 0.9, 0.2, 0.2)),
                );
                let second = ops::add_opening(
                    skeleton,
                    Opening::new(court_face(), 1, 1, Inset::new(0.3, 0.1, 0.2, 0.2)),
                );
                first.and(second)
            },
        ),
        ("a part placed in a courtyard's bay", |skeleton| {
            let index = ops::add_template(skeleton, leaf());
            ops::place_part(
                skeleton,
                Placement::new(index, court_face(), 1, 1, Pin::new(0.2, 0.2)),
            )
        }),
        (
            "an opening naming a void the document does not hold",
            |skeleton| {
                skeleton
                    .openings
                    .push(Opening::new(
                        FaceRef::court(MassId(0), 7, 0),
                        0,
                        0,
                        Inset::new(0.1, 0.1, 0.1, 0.1),
                    ))
                    .into()
            },
        ),
    ];
    for (label, author) in refusals {
        let mut skeleton = BuildingSkeleton {
            masses: vec![housed_patio()],
            ..BuildingSkeleton::default()
        };
        for at in skeleton.court_faces_of(MassId(0), 0) {
            ops::split_face(&mut skeleton, at, &columns, &rows);
        }
        let authored = author(&mut skeleton);
        if authored.is_none() {
            println!("  {label}: refused at the verb, before the document holds it");
            continue;
        }
        let document = Building::new(skeleton.clone());
        let faults = document.validate();
        let built = document.build();
        println!("  {label}:");
        for fault in &faults {
            println!("    {fault:?}");
        }
        match built {
            Ok(_) => println!(
                "    and it builds: there is no ring for it to be cut into, so the document's own report is the name"
            ),
            Err(error) => {
                for problem in &error.problems {
                    println!("    and it does not build: {problem:?}");
                }
            }
        }
    }

    println!(
        "\n# the reading: a courtyard's wall takes an opening exactly as the building's own does — the\n# hole goes through both of the ring's faces, the reveals name the loop, and the ring's Euler\n# characteristic follows the document rather than staying at 0."
    );
}
