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
//! 5. the four faults a courtyard can break are each reported, and a fault still refuses the build.
//!
//! Run with `cargo run -p whiting-building --example patio_probe`.

use whiting_building::identity::Identity;
use whiting_building::mesh::MassSurface;
use whiting_building::{
    Building, BuildingSkeleton, Element, FaceRef, Inset, Mass, MassId, Opening, PlanNode, Rhythm,
    SolidId, ops,
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
}
