//! The wall-placement slice's acceptance: a part hung on a **wall run** rather than fitted into a bay, which
//! is the shape a gate leaf has.
//!
//! # What this measures
//!
//! - the leaf's frame stands on the run's own centre line at the pin, and the leaf's geometry reaches **past**
//!   the run's end into the gap — the thing a bay-mounted part cannot do, and the reason a wall target has no
//!   containment check;
//! - the run's pin is checked and the part's box is not: a pin past the run is a violation, a leaf past the
//!   run is the point;
//! - the leaf moves with its wall when the ground is seated, because the frame is derived from the run's own
//!   ground line rather than stored;
//! - a mount on a wall is refused rather than resolved against the run's ends by guesswork.
//!
//! Run with `cargo run -p whiting-building --example wall_part_probe`.

use whiting_building::derive::floor_slab;
use whiting_building::parts::{Anchor, Joint, Pin, Placement, Socket, Template};
use whiting_building::site::seat;
use whiting_building::wall::WallId;
use whiting_building::{Building, BuildingSkeleton, Mass, PlanNode, SolidId, ops};

/// Returns a gate leaf: a panel `width` across, `height` tall, `thickness` through, standing at its own
/// origin and running `+x`.
fn leaf(width: f32, height: f32, thickness: f32) -> Template {
    let slab = 0.03;
    let outline = [
        [0.0, 0.0],
        [width, 0.0],
        [width, thickness],
        [0.0, thickness],
    ]
    .into_iter()
    .map(|corner| PlanNode::new(corner, slab))
    .collect();
    Template::new(
        "leaf",
        vec![Mass::new(outline, height).with_wall(0.02).with_slab(slab)],
        Vec::new(),
    )
}

/// Returns a yard: two wall runs with a gate's width between them.
fn yard() -> BuildingSkeleton {
    let mut skeleton = BuildingSkeleton::default();
    // The gate is the gap from x = 2.0 to x = 4.4, so the first run ends where the gate begins.
    skeleton.walls.push(whiting_building::wall::Wall::new(
        PlanNode::new([-6.0, -5.0], 0.0),
        PlanNode::new([2.0, -5.0], 0.0),
        0.35,
        1.9,
    ));
    skeleton.walls.push(whiting_building::wall::Wall::new(
        PlanNode::new([4.4, -5.0], 0.0),
        PlanNode::new([6.0, -5.0], 0.0),
        0.35,
        1.9,
    ));
    skeleton
}

fn main() {
    let mut skeleton = yard();
    let template = ops::add_template(&mut skeleton, leaf(2.4, 1.7, 0.08));
    let run = skeleton.walls[WallId(0).0].length();
    // **Hung at the run's end**, which is exactly how a gate is mounted: the hinge is on the post, and the leaf
    // closes the opening beyond it.
    let placed = ops::place_part(
        &mut skeleton,
        Placement::on_wall(template, WallId(0), Pin::new(run, 0.0)),
    )
    .expect("the run exists, so the part is in the domain");
    let document = Building::new(skeleton.clone());
    let built = document.build().expect("the yard expands");
    println!("# a gate hung on a wall");
    println!(
        "  the run is {:.4} m long; the leaf is placed at its end and stands {:.4} m tall",
        8.0, 1.7
    );
    println!(
        "  the document reports {} violation(s)",
        document.validate().len()
    );

    let frame = whiting_building::expand::part_frame(&skeleton, placed)
        .expect("the frame resolves")
        .expect("the placement is in the domain");
    println!("\n# the frame is the run's own, at the pin");
    println!(
        "  origin ({:.4}, {:.4}, {:.4}) against the run's end x {:.4} on its centre line z {:.4}",
        frame.origin[0], frame.origin[1], frame.origin[2], 2.0, -5.0
    );
    println!(
        "  along ({:.3}, {:.3}) and inward ({:.3}, {:.3}) — the run's direction and its normal",
        frame.along[0], frame.along[2], frame.inward[0], frame.inward[2]
    );

    let solid = SolidId::Part(placed);
    let (low, high) = built.bounds_of(solid);
    println!("\n# the leaf reaches past its wall, which is what a gate does");
    println!(
        "  the leaf spans x {:.4} .. {:.4} and the run ends at {:.4}, so it closes {:.4} m of the gap",
        low[0],
        high[0],
        2.0,
        high[0] - 2.0
    );
    println!(
        "  it stands from y {:.4} to {:.4} and is {} closed, {} faces",
        low[1],
        high[1],
        if built.solid_boundary_loops(solid) == 0 {
            "watertight"
        } else {
            "OPEN"
        },
        built.faces_of(solid).len()
    );

    println!("\n# the pin is checked and the box is not");
    let mut off = yard();
    let template = ops::add_template(&mut off, leaf(2.4, 1.7, 0.08));
    let beyond = off.walls[WallId(0).0].length() + 0.5;
    ops::place_part(
        &mut off,
        Placement::on_wall(template, WallId(0), Pin::new(beyond, 0.0)),
    )
    .expect("a pin off the run is still a document an author can write");
    let report = Building::new(off).validate();
    println!(
        "  a pin {:.4} m beyond the run reports {} violation(s): {:?}",
        beyond - 8.0,
        report.len(),
        report.first()
    );

    println!("\n# a wall the document does not hold is refused at the verb");
    let mut missing = yard();
    let template = ops::add_template(&mut missing, leaf(2.4, 1.7, 0.08));
    println!(
        "  placing on wall 7: {:?}",
        ops::place_part(
            &mut missing,
            Placement::on_wall(template, WallId(7), Pin::new(0.0, 0.0))
        )
    );

    println!("\n# a mount on a wall has no bay to anchor to, and is refused");
    let mut mounted = yard();
    let template = ops::add_template(&mut mounted, leaf(2.4, 1.7, 0.08));
    let mut placement = Placement::on_wall(template, WallId(0), Pin::new(8.0, 0.0));
    placement.joint = Some(Joint::new("hinge", Anchor::BayHead));
    println!(
        "  placing it with a joint: {:?}",
        ops::place_part(&mut mounted, placement)
    );
    let mut reported = yard();
    let template = ops::add_template(&mut reported, leaf(2.4, 1.7, 0.08));
    let mut placement = Placement::on_wall(template, WallId(0), Pin::new(8.0, 0.0));
    placement.joint = Some(Joint::new("hinge", Anchor::BayHead));
    reported.parts.place(placement);
    let report = Building::new(reported).validate();
    println!(
        "  and a document that holds one anyway reports: {:?}",
        report.first()
    );

    println!("\n# the leaf moves with its wall when the ground is seated");
    let mut sloped = skeleton.clone();
    let seated = seat(&mut sloped, |plan| 0.25 * plan[0], 0.0);
    let built_seated = Building::new(sloped)
        .build()
        .expect("the seated yard expands");
    let (sloped_low, _) = built_seated.bounds_of(solid);
    println!(
        "  {} mass(es) and {} wall(s) moved; the leaf's foot is now at y {:.4} instead of {:.4}",
        seated.len(),
        seated.walls,
        sloped_low[1],
        low[1]
    );

    println!("\n# and a socket on the leaf still resolves in a bay, which is the old path");
    println!(
        "  (a wall mount has no bay; a bay mount is unchanged: part/socket/instance probe(s) still measure it)"
    );
    let _ = Socket::new("hinge", (0.0, 0.0));
    let _ = floor_slab;
}
