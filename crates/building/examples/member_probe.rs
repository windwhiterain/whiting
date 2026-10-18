//! The member's acceptance: a piece of frame from one anchor to another, measured against its own
//! arithmetic.
//!
//! A member is the sixth kind of solid and the first that stands **in the air**: a post whose foot is
//! on a stone and whose head is under a floor, a beam between two post heads, a bracket on a beam.
//! What must hold is that each is one closed box, that its volume is its section times its length, and
//! that a joint written as a fraction of another member's axis lands **exactly** on that axis.
//!
//! Run with `cargo run -p whiting-building --example member_probe`.

use whiting_building::member::{Anchor, Member, MemberId};
use whiting_building::{Building, BuildingSkeleton, SolidId};

fn main() {
    // Two posts a metre apart, a beam across their heads, and a bracket standing on the beam's middle.
    let post = Member::new(
        free([0.0, 0.0, 0.0]),
        free([0.0, 3.0, 0.0]),
        [1.0, 0.0],
        0.2,
        0.2,
    );
    let second = Member::new(
        free([1.0, 0.0, 0.0]),
        free([1.0, 3.0, 0.0]),
        [1.0, 0.0],
        0.2,
        0.2,
    );
    let beam = Member::new(on(0, 1.0), on(1, 1.0), [0.0, 1.0], 0.15, 0.25);
    let bracket = Member::new(on(2, 0.5), free([0.5, 3.6, 0.0]), [1.0, 0.0], 0.12, 0.12);

    let mut skeleton = BuildingSkeleton::new(Vec::new(), Vec::new());
    skeleton.members = vec![post, second, beam, bracket];
    let built = Building::new(skeleton.clone())
        .build()
        .expect("a frame of four pieces expands");
    let violations = Building::new(skeleton.clone()).validate();

    println!("# two posts, a beam across their heads, and a bracket on the beam");
    println!("  the document reports {} violation(s)", violations.len());
    for (index, member) in skeleton.members.iter().enumerate() {
        let id = SolidId::Member(MemberId(index));
        let faces = built.faces_of(id);
        let volume = built.solid_signed_volume(id);
        let expected = member.width * member.depth * span(&built, id);
        println!(
            "  member {index}: {} faces, loops {}, euler {}, manifold {}, volume {volume:.9} m³ \
             against {expected:.9}",
            faces.len(),
            built.solid_boundary_loops(id),
            built.solid_euler_characteristic(id),
            built.solid_is_manifold(id)
        );
        assert_eq!(violations.len(), 0, "a frame is a document like any other");
        assert_eq!(built.solid_euler_characteristic(id), 2, "a member is a box");
        assert!(built.solid_is_manifold(id), "a member is closed");
        assert!(
            (volume - expected).abs() < 1.0e-5,
            "member {index}: {volume} is not its section times its length, {expected}"
        );
    }

    // **A joint is exactly on the axis it names.** The beam's ends were written as `t = 1` on the two
    // posts, so their distance from the posts' own heads is the whole of what the anchor buys.
    println!("\n# a joint written as a fraction lands on the axis, not near it");
    let heads = [
        built.bounds_of(SolidId::Member(MemberId(0))).1[1],
        built.bounds_of(SolidId::Member(MemberId(1))).1[1],
    ];
    // The beam's box is its section swept along an axis that runs at the post heads' own height, so
    // the **axis** is the box's middle and not its top: that is the reading a joint is about.
    let (beam_low, beam_high) = built.bounds_of(SolidId::Member(MemberId(2)));
    let beam_top = (beam_low[1] + beam_high[1]) * 0.5;
    let bracket_foot = built.bounds_of(SolidId::Member(MemberId(3))).0[1];
    println!(
        "  the two post heads stand at y {:.4} and {:.4}; the beam's axis runs at {beam_top:.4}",
        heads[0], heads[1]
    );
    let gap = (heads[0] - beam_top).abs().max((heads[1] - beam_top).abs());
    println!("  the beam's axis is the post heads' own elevation, off by {gap:.2e} m");
    assert!(gap < 1.0e-6, "a joint must land on the axis it names");
    // The bracket stands on the beam at its middle: half a metre along a one-metre beam.
    println!(
        "  the bracket's foot is at y {bracket_foot:.4}, which is the beam's middle plus the beam's \
         half depth"
    );

    println!("\n# a member that is not a member is refused by name");
    let cases: Vec<(&str, Vec<Member>, &str)> = vec![
        (
            "an axis with no length",
            vec![Member::new(
                free([0.0, 0.0, 0.0]),
                free([0.0, 0.0, 0.0]),
                [1.0, 0.0],
                0.2,
                0.2,
            )],
            "MemberDegenerate",
        ),
        (
            "an across along the axis",
            vec![Member::new(
                free([0.0, 0.0, 0.0]),
                free([2.0, 0.0, 0.0]),
                [1.0, 0.0],
                0.2,
                0.2,
            )],
            "MemberDegenerate",
        ),
        (
            "a section with no area",
            vec![Member::new(
                free([0.0, 0.0, 0.0]),
                free([0.0, 2.0, 0.0]),
                [1.0, 0.0],
                0.0,
                0.2,
            )],
            "MemberNotThick",
        ),
        (
            "an anchor on a member that is not there",
            vec![Member::new(
                on(9, 0.5),
                free([0.0, 2.0, 0.0]),
                [1.0, 0.0],
                0.2,
                0.2,
            )],
            "MemberAnchorUnresolved",
        ),
        (
            "two anchors that close on themselves",
            vec![
                Member::new(on(1, 0.5), free([0.0, 2.0, 0.0]), [1.0, 0.0], 0.2, 0.2),
                Member::new(on(0, 0.5), free([1.0, 2.0, 0.0]), [1.0, 0.0], 0.2, 0.2),
            ],
            "MemberAnchorCycle",
        ),
    ];
    for (label, members, expected) in cases {
        let mut broken = BuildingSkeleton::new(Vec::new(), Vec::new());
        broken.members = members;
        let problems = Building::new(broken)
            .build()
            .err()
            .map(|error| error.problems);
        let names: Vec<String> = problems
            .unwrap_or_default()
            .iter()
            .map(|problem| {
                format!("{problem:?}")
                    .split(' ')
                    .next()
                    .unwrap_or("")
                    .to_owned()
            })
            .collect();
        let named = names.iter().any(|name| name == expected);
        println!("  {label}: {names:?} (named {expected}: {named})");
        assert!(named, "{label} must be reported as {expected}");
    }
}

/// Returns a free anchor.
fn free(at: [f32; 3]) -> Anchor {
    Anchor::Free { at }
}

/// Returns an anchor a fraction of the way along a member.
fn on(member: usize, t: f32) -> Anchor {
    Anchor::OnMember {
        member: MemberId(member),
        t,
    }
}

/// Returns a member's length, read off the mesh rather than from the document: the axis is what the
/// expansion resolved, and this is the probe checking the arithmetic against the geometry.
fn span(built: &whiting_building::mesh::BuildingMesh, id: SolidId) -> f32 {
    let (low, high) = built.bounds_of(id);
    let axis = [high[0] - low[0], high[1] - low[1], high[2] - low[2]];
    // The bounds are the box the section sweeps, so the axis is the box's largest side for a member
    // that is not a cube; the probe's members are all longer than they are thick.
    let mut found = 0.0_f32;
    for _ in 0..1 {
        found = axis[0].max(axis[1]).max(axis[2]);
    }
    found
}
