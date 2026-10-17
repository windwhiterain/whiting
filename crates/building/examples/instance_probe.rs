//! The instancing slice's acceptance: one template mesh, *n* frames, and the baked geometry they equal.
//!
//! The claim the slice makes is not "a scene can draw fewer faces" but **"it draws the same faces"**: a
//! template expanded once and mapped by a [`PartFrame`] must reproduce, point for point, the geometry
//! `build` baked. If those two ever disagreed, instancing would be a second renderer with its own opinion
//! about where a part stands — which is exactly the drift the crate has paid for twice.
//!
//! Run with `cargo run -p whiting-building --example instance_probe`.

use std::collections::BTreeSet;

use whiting_building::derive::{RoofStyle, Starter, starter};
use whiting_building::expand::part_frame;
use whiting_building::grid::{FaceRef, Rhythm};
use whiting_building::ops;
use whiting_building::parts::{Anchor, Joint, Pin, Placement, Socket, Template};
use whiting_building::{Building, Mass, MassId, PlanNode, SolidId};

/// Returns a shutter: a panel `width` across, `height` tall, `thickness` through.
fn shutter(width: f32, height: f32, thickness: f32) -> Template {
    let slab = 0.02;
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
        "shutter",
        vec![Mass::new(outline, height).with_wall(0.01).with_slab(slab)],
        Vec::new(),
    )
}

/// Returns a canopy with a `"back"` socket at the middle of its top-back edge.
fn canopy(width: f32, thickness: f32, reach: f32) -> Template {
    let slab = thickness * 0.5;
    let outline = [[0.0, -reach], [width, -reach], [width, 0.0], [0.0, 0.0]]
        .into_iter()
        .map(|corner| PlanNode::new(corner, slab))
        .collect();
    Template::new(
        "canopy",
        vec![
            Mass::new(outline, thickness)
                .with_wall(0.02)
                .with_slab(slab),
        ],
        Vec::new(),
    )
    .with_socket(Socket::new("back", (width * 0.5, thickness)))
}

/// Rounds a point so two points that stand in the same place compare equal.
fn key(point: [f32; 3]) -> [i64; 3] {
    point.map(|value| (value * 10_000.0).round() as i64)
}

fn main() {
    // A shed with six bays on its front face: three across, two storeys. Every bay carries the **same**
    // template, which is the situation D4 exists for.
    let plan = Starter {
        width: 5.0,
        depth: 8.0,
        height: 2.8,
        roof: RoofStyle::Gable,
        ..Starter::default()
    };
    let mut skeleton = starter(&plan).skeleton.clone();
    let mass = MassId(0);
    let front = FaceRef::new(mass, 2);
    for face in skeleton.faces_of(mass) {
        ops::split_face(
            &mut skeleton,
            face,
            &Rhythm::Even { count: 3 },
            &Rhythm::Even { count: 2 },
        );
    }
    let shutter_template = ops::add_template(&mut skeleton, shutter(0.8, 1.0, 0.05));
    let mut placed = 0;
    for row in 0..2 {
        for column in 0..3 {
            ops::place_part(
                &mut skeleton,
                Placement::new(shutter_template, front, row, column, Pin::new(0.18, 0.15)),
            )
            .expect("every bay of the grid is there");
            placed += 1;
        }
    }
    // And a part hung on a feature, to check that the frame is the joint's and not a pin's.
    let shelter = ops::add_template(&mut skeleton, canopy(1.2, 0.10, 0.6));
    ops::place_part(
        &mut skeleton,
        Placement::new(shelter, front, 0, 1, Pin::new(0.0, 0.0))
            .with_joint(Joint::new("back", Anchor::BayHead)),
    )
    .expect("the middle bay of the upper storey is in the grid");

    let built = Building::new(skeleton.clone())
        .build()
        .expect("the shed expands");
    let template = skeleton
        .parts
        .template(shutter_template)
        .expect("the shutter is in the document");
    let once = template.mesh().expect("the template expands on its own");
    let baked: usize = (0..placed)
        .map(|index| built.faces_of(SolidId::Part(index)).len())
        .sum();
    println!("# one template, {placed} placements");
    println!(
        "  the baked mesh holds {baked} part faces; the template's own mesh holds {} — {:.0} times \
         fewer, and the same geometry",
        once.faces.len(),
        baked as f32 / once.faces.len() as f32
    );

    println!("\n# the frame reproduces the baked geometry, point for point");
    let mut mismatched = 0;
    for index in 0..placed {
        let baked_points: BTreeSet<[i64; 3]> = built
            .faces_of(SolidId::Part(index))
            .iter()
            .flat_map(|face| {
                built.faces[*face]
                    .indices
                    .iter()
                    .map(|corner| key(built.points[*corner as usize]))
            })
            .collect();
        let frame = part_frame(&skeleton, index)
            .expect("the wall offsets")
            .expect("the placement resolves");
        let instanced: BTreeSet<[i64; 3]> = once
            .points
            .iter()
            .map(|point| key(frame.apply(*point)))
            .collect();
        if baked_points != instanced {
            mismatched += 1;
        }
    }
    println!("  placements whose instanced points differ from the baked ones: {mismatched}");

    println!("\n# and the same for the box each one stands in");
    let mut bounds_wrong = 0;
    for index in 0..placed {
        let (low, high) = built.bounds_of(SolidId::Part(index));
        let frame = part_frame(&skeleton, index)
            .expect("the wall offsets")
            .expect("the placement resolves");
        let mapped: Vec<[f32; 3]> = once.points.iter().map(|at| frame.apply(*at)).collect();
        let mut mapped_low = [f32::INFINITY; 3];
        let mut mapped_high = [f32::NEG_INFINITY; 3];
        for at in &mapped {
            for axis in 0..3 {
                mapped_low[axis] = mapped_low[axis].min(at[axis]);
                mapped_high[axis] = mapped_high[axis].max(at[axis]);
            }
        }
        if (0..3).any(|axis| (mapped_low[axis] - low[axis]).abs() > 1.0e-5)
            || (0..3).any(|axis| (mapped_high[axis] - high[axis]).abs() > 1.0e-5)
        {
            bounds_wrong += 1;
        }
    }
    println!("  placements whose instanced box differs: {bounds_wrong}");

    println!("\n# a mounted part's frame is its joint's, not a pin's");
    let mounted = placed;
    let frame = part_frame(&skeleton, mounted)
        .expect("the wall offsets")
        .expect("the placement resolves");
    let bay = skeleton
        .placed(mounted)
        .expect("it resolves")
        .bay
        .expect("the canopy hangs in a bay rather than on a wall");
    println!(
        "  the canopy's origin stands at y {:.4}; the bay's head is at {:.4} and the socket is {:.4} \
         below its top, so the frame is {:.4} above the bay's own sill",
        frame.origin[1],
        bay.high,
        frame.origin[1] - bay.high,
        frame.origin[1] - bay.low
    );
    let baked_canopy = built.bounds_of(SolidId::Part(mounted));
    println!(
        "  and the baked canopy runs y {:.4} .. {:.4}, which is the origin plus the template's own box",
        baked_canopy.0[1], baked_canopy.1[1]
    );

    println!("\n# a placement that does not resolve has no frame");
    let mut orphaned = skeleton.clone();
    ops::clear_face(&mut orphaned, front);
    println!(
        "  after clearing the face's rhythm: part 0's frame is {:?}",
        part_frame(&orphaned, 0)
    );
}
