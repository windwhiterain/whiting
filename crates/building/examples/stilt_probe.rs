//! The stilt slice's acceptance: a house whose floor is levelled to the uphill grade and whose
//! downhill side stands on posts, measured against an analytic slope.
//!
//! The ground is `0.35 · x` — a 35 % bank, which is the middle of the 30–70 % range the Miao stilt
//! house is built on (see `.agents/investigate/guizhou-dwellings.md`, §3.4) — so every claim here has a
//! closed form. A post's foot is the slope under that post's own corner; the deepest drop is the slope
//! under the house's downhill edge; a house on flat ground gets no posts at all.
//!
//! What the probe is really for is one sentence: **the post's length is the terrain query.** So it
//! measures the error between every post's foot and the analytic ground under it, prints the range of
//! post lengths, and fails the run when any of its claims stops holding.
//!
//! Run with `cargo run -p whiting-building --example stilt_probe`.

use whiting_building::derive::{RoofStyle, Starter, starter};
use whiting_building::math::Vec2;
use whiting_building::member::{Anchor, MemberId};
use whiting_building::site::{posts, seat};
use whiting_building::{Building, SolidId};

/// The house: three bays across a 10 m front, 6 m deep, one storey under a gable.
const PLAN: Starter = Starter {
    width: 10.0,
    depth: 6.0,
    height: 3.2,
    wall: 0.24,
    slab: 0.30,
    overhang: 0.60,
    sink: 0.10,
    rise: 2.0,
    roof_thickness: 0.22,
    roof: RoofStyle::Gable,
    seed: 5,
};

/// The bank: 35 % of a metre of rise per metre of `x`.
const SLOPE: f32 = 0.35;

/// How far apart the posts stand, and how wide each one is.
const SPACING: f32 = 1.5;
const SECTION: f32 = 0.2;
/// How far a post's top is sunk into the plate, so the two solids never present coincident faces.
const EMBED: f32 = 0.02;

fn ground(at: Vec2) -> f32 {
    SLOPE * at[0]
}

fn main() {
    let mut building = starter(&PLAN);
    let flat = Building::new(building.skeleton.clone())
        .build()
        .expect("the house expands on a plane");

    println!(
        "# a {:.0} x {:.0} body on a {:.0} % bank, posts every {:.2} m of {:.2} m section",
        PLAN.width,
        PLAN.depth,
        SLOPE * 100.0,
        SPACING,
        SECTION
    );

    let seated = seat(&mut building.skeleton, ground, 0.0);
    let stilt = posts(&mut building.skeleton, ground, SPACING, SECTION, EMBED);
    let one = stilt.masses.first().copied().expect("the house got posts");

    println!("\n# seating and posts");
    println!(
        "  the floor was levelled to {:.4} m — the highest ground under the outline plus 0 — and the \
         terrace fall under it is {:.4} m",
        seated.masses[0].level,
        seated.masses[0].fall()
    );
    println!(
        "  {} post(s) under mass {}, {} refused, {} member(s) in the document",
        stilt.posts,
        one.mass.0,
        stilt.refused,
        building.skeleton.members.len()
    );
    println!(
        "  the posts reach the underside at {:.4} m and stand between {:.4} and {:.4} m of ground: \
         a drop of {:.4} m and a relief of {:.4} m",
        one.underside,
        one.lowest,
        one.highest,
        one.drop(),
        one.relief()
    );

    // The one claim: a post's foot is the ground under it, exactly.
    let mut worst_foot = 0.0_f32;
    let mut shortest = f32::INFINITY;
    let mut longest = f32::NEG_INFINITY;
    for member in &building.skeleton.members {
        let Anchor::Free { at: foot } = member.from else {
            panic!("a post is a free member: its foot is a point, not an anchor");
        };
        let Anchor::Free { at: head } = member.to else {
            panic!("a post's head is a point too");
        };
        worst_foot = worst_foot.max((foot[1] - ground([foot[0], foot[2]])).abs());
        let length = head[1] - foot[1];
        shortest = shortest.min(length);
        longest = longest.max(length);
    }
    println!(
        "  the post lengths run {:.4} .. {:.4} m, and the largest gap between a foot and the analytic \
         ground under it is {:.2e} m",
        shortest, longest, worst_foot
    );
    assert!(
        worst_foot < 1.0e-6,
        "a post's foot must be the ground under it, not near it: {worst_foot}"
    );
    assert!(stilt.posts > 0, "a banked house must stand on posts");
    assert!(
        stilt.posts == building.skeleton.members.len(),
        "nothing else in this document is a member"
    );

    // The floor stays level, which is what seating bought and posts must not spend.
    let levels: Vec<f32> = building.skeleton.masses[0]
        .outline()
        .iter()
        .map(|node| node.elevation)
        .collect();
    let spread = levels.iter().cloned().fold(f32::NEG_INFINITY, f32::max)
        - levels.iter().cloned().fold(f32::INFINITY, f32::min);
    println!(
        "  the floor's spread over its own nodes is {:.2e} m",
        spread
    );
    assert!(spread.abs() < 1.0e-6, "posts must not tilt the floor");

    println!("\n# the uphill side stands on the hill, not on posts");
    let uphill = building
        .skeleton
        .members
        .iter()
        .filter(|member| foot_of(member)[0] > one.underside / SLOPE)
        .count();
    let reach = building
        .skeleton
        .members
        .iter()
        .map(|member| foot_of(member)[0])
        .fold(f32::NEG_INFINITY, f32::max);
    println!(
        "  {uphill} post(s) stand uphill of x = {:.4} (where the ground reaches the underside), and \
         the last post is at x = {:.4} of an outline that ends at {:.4}",
        one.underside / SLOPE,
        reach,
        PLAN.width * 0.5
    );
    assert_eq!(uphill, 0, "a post whose ground reaches the floor is buried");

    println!("\n# a flat site needs no posts");
    let mut level = starter(&PLAN);
    let _ = seat(&mut level.skeleton, |_| 0.0, 0.0);
    let none = posts(&mut level.skeleton, |_| 0.0, SPACING, SECTION, EMBED);
    println!(
        "  ground at 0 everywhere: {} post(s), {} mass(es) given any, {} member(s)",
        none.posts,
        none.len(),
        level.skeleton.members.len()
    );
    assert_eq!(none.posts, 0, "a house on its ground is not a stilt house");

    println!("\n# a ground that does not answer is refused, not guessed");
    let mut unknown = starter(&PLAN);
    let _ = seat(&mut unknown.skeleton, ground, 0.0);
    let blind = posts(
        &mut unknown.skeleton,
        |at| if at[0] > 0.0 { f32::NAN } else { ground(at) },
        SPACING,
        SECTION,
        EMBED,
    );
    println!(
        "  a NaN downhill of x = 0: {} post(s) placed, {} refused",
        blind.posts, blind.refused
    );
    assert!(
        blind.refused > 0 && blind.posts > 0,
        "half a ground is not a ground"
    );

    println!("\n# the mesh stays closed with them");
    let built = Building::new(building.skeleton.clone())
        .build()
        .expect("the stilt house expands");
    let violations = Building::new(building.skeleton.clone()).validate().len();
    println!(
        "  {} faces and {} points, against {} and {} without posts; {violations} violation(s)",
        built.faces.len(),
        built.points.len(),
        flat.faces.len(),
        flat.points.len()
    );
    let post = SolidId::Member(MemberId(0));
    println!(
        "  post 0: {} faces, loops {}, euler {}, manifold {}, volume {:.6} m³",
        built.faces_of(post).len(),
        built.solid_boundary_loops(post),
        built.solid_euler_characteristic(post),
        built.solid_is_manifold(post),
        built.solid_signed_volume(post)
    );
    assert_eq!(violations, 0, "a stilt house is a document like any other");
    assert!(built.faces.len() > flat.faces.len(), "the posts are drawn");
}

/// Returns the plan point a member''s foot stands at, which for a post is its `from` anchor.
fn foot_of(member: &whiting_building::Member) -> [f32; 2] {
    match member.from {
        Anchor::Free { at } => [at[0], at[2]],
        Anchor::OnMember { .. } | Anchor::OnFace { .. } => [f32::NAN, f32::NAN],
    }
}
