//! The face anchor's acceptance: a bracket pinned to a wall, measured against the wall's own face.
//!
//! The claim is one claim with two readings — **the point is on the wall's face** — and it is read twice:
//! a bracket at the edge's own start lands on the mass's own corner, and one part way along and part way
//! up lands on the wall's plane at that station. Everything else here is the three refusals.
//!
//! Run with `cargo run -p whiting-building --example face_anchor_probe`.

use whiting_building::grid::FaceRef;
use whiting_building::member::{Anchor, Member, MemberId};
use whiting_building::{Building, BuildingSkeleton, Mass, MassId, PlanNode, SolidId};

/// The wall: ten by six, three point two high, with a 0.3 m section.
const WIDTH: f32 = 10.0;
const DEPTH: f32 = 6.0;
const HEIGHT: f32 = 3.2;
const WALL: f32 = 0.3;

/// The bracket's section, small enough that its own thickness does not hide the point it hangs from.
const SECTION: f32 = 0.02;

fn main() {
    println!("# a mass and two brackets, one at a corner and one part way along a wall");
    let mut skeleton = BuildingSkeleton::new(Vec::new(), Vec::new());
    let mut mass = Mass::new(
        vec![
            PlanNode::new([-WIDTH * 0.5, -DEPTH * 0.5], 0.0),
            PlanNode::new([WIDTH * 0.5, -DEPTH * 0.5], 0.0),
            PlanNode::new([WIDTH * 0.5, DEPTH * 0.5], 0.0),
            PlanNode::new([-WIDTH * 0.5, DEPTH * 0.5], 0.0),
        ],
        HEIGHT,
    );
    mass.wall = WALL;
    mass.slab = 0.3;
    skeleton.masses.push(mass);

    // **The corner**: edge 0 runs from the first outline node along `x`, so `along = 0` is that node.
    skeleton.members.push(Member::new(
        Anchor::OnFace {
            face: FaceRef::new(MassId(0), 0),
            along: 0.0,
            up: 0.0,
        },
        Anchor::Free {
            at: [-WIDTH * 0.5, 2.4, -DEPTH * 0.5 - 1.5],
        },
        [1.0, 0.0],
        SECTION,
        SECTION,
    ));
    // **Part way along and part way up**: two and a half metres in, one point eight above the base.
    skeleton.members.push(Member::new(
        Anchor::OnFace {
            face: FaceRef::new(MassId(0), 0),
            along: 2.5,
            up: 1.8,
        },
        Anchor::Free {
            at: [-2.5, 1.8, -DEPTH * 0.5 - 2.0],
        },
        [1.0, 0.0],
        SECTION,
        SECTION,
    ));

    let built = Building::new(skeleton.clone())
        .build()
        .expect("two brackets pinned to a wall expand");
    let violations = Building::new(skeleton.clone()).validate();
    println!("  the document reports {} violation(s)", violations.len());
    for violation in &violations {
        println!("    {violation:?}");
    }
    assert_eq!(
        violations.len(),
        0,
        "a pinned bracket is a document like any other"
    );

    let corner = bounds(&built, MemberId(0));
    println!(
        "  the corner bracket's box is x {:.4}..{:.4}, y {:.4}..{:.4}, z {:.4}..{:.4}",
        corner.0[0], corner.1[0], corner.0[1], corner.1[1], corner.0[2], corner.1[2]
    );
    // The anchor is the mass's own corner `(-5, 0, -3)`, and the bracket's box must reach it: the finest
    // that can be said of a *box* is that its nearest corner is the anchor's own corner, within the
    // section's half.
    println!("\n# the corner bracket stands on the mass's own corner");
    assert!(
        (corner.0[0] + WIDTH * 0.5).abs() < SECTION,
        "x is the wall's own plane"
    );
    assert!((corner.0[1]).abs() < SECTION, "its foot is the wall's base");
    assert!(
        (corner.1[2] + DEPTH * 0.5).abs() < 1.0e-2,
        "its anchor end is the edge's own start node's z: {}",
        corner.1[2]
    );

    let along = bounds(&built, MemberId(1));
    println!("\n# the second bracket is two and a half metres in and one point eight up");
    println!(
        "  its box is x {:.4}..{:.4}, y {:.4}..{:.4}, z {:.4}..{:.4}",
        along.0[0], along.1[0], along.0[1], along.1[1], along.0[2], along.1[2]
    );
    assert!(
        (along.0[0] - -2.5).abs() < SECTION,
        "x is the station along the edge"
    );
    assert!(
        (along.0[1] - (1.8 - SECTION * 0.5)).abs() < 1.0e-3,
        "its foot is 1.8 above the base: {}",
        along.0[1]
    );

    println!("\n# a face that does not have the point is refused by name");
    for (label, anchor) in [
        (
            "along past the edge",
            Anchor::OnFace {
                face: FaceRef::new(MassId(0), 0),
                along: WIDTH + 1.0,
                up: 0.5,
            },
        ),
        (
            "up past the wall's top",
            Anchor::OnFace {
                face: FaceRef::new(MassId(0), 0),
                along: 2.0,
                up: HEIGHT + 0.5,
            },
        ),
        (
            "a face on a mass that is not there",
            Anchor::OnFace {
                face: FaceRef::new(MassId(7), 0),
                along: 1.0,
                up: 0.5,
            },
        ),
    ] {
        let mut broken = skeleton.clone();
        broken.members[1].from = anchor;
        let problems = Building::new(broken)
            .build()
            .err()
            .map(|error| error.problems)
            .unwrap_or_default();
        let names: Vec<String> = problems
            .iter()
            .map(|problem| {
                format!("{problem:?}")
                    .split(' ')
                    .next()
                    .unwrap_or("")
                    .to_owned()
            })
            .collect();
        println!("  {label}: {names:?}");
        assert!(
            names.iter().any(|name| name == "MemberAnchorUnresolved"),
            "{label} must be refused as an unresolved anchor"
        );
    }

    // **A face anchor needs no other member**, which is the difference from `OnMember`: one pass resolves
    // it, and a cycle is impossible by construction.
    println!("\n# a face anchor is resolved in the first pass");
    let mut alone = BuildingSkeleton::new(Vec::new(), Vec::new());
    alone.masses.push(skeleton.masses[0].clone());
    alone.members.push(skeleton.members[1]);
    let one = Building::new(alone)
        .build()
        .expect("one bracket on one wall expands");
    let single = bounds(&one, MemberId(0));
    println!(
        "  one bracket, one mass: its box starts at x {:.4}",
        single.0[0]
    );
    assert!((single.0[0] - -2.5).abs() < SECTION);
}

/// Returns the box of one member's solid.
fn bounds(built: &whiting_building::mesh::BuildingMesh, member: MemberId) -> ([f32; 3], [f32; 3]) {
    built.bounds_of(SolidId::Member(member))
}
