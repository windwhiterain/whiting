//! The instancing slice's acceptance: a scene's instance transform stands a template's mesh exactly where
//! the crate's own frame does, and where the baked geometry already is.
//!
//! Three routes to the same points, and the probe compares them as **sets**:
//!
//! 1. the template's own mesh mapped by [`PartFrame::apply`] — the crate's definition;
//! 2. the same mesh mapped by the scene's instance transform
//!    ([`transform_matrix`](whiting_scene::standard::math::transform_matrix) of [`standing`]) — what a scene
//!    actually draws;
//! 3. the geometry `build` baked for that placement.
//!
//! If (1) and (2) disagreed, a scene would draw a part somewhere the crate says it is not; if (1) and (3)
//! disagreed, the two paths would be two renderings rather than one geometry. The point of the slice is that
//! no scene uploads twelve copies of one window, and the point of *this* is that saving that upload changes
//! nothing about where the geometry is.
//!
//! Run with `cargo run -p whiting-gallery --example building_instancing_probe`.

use std::collections::BTreeSet;

use building::derive::{RoofStyle, Starter, starter};
use building::expand::part_frame;
use building::grid::{FaceRef, Rhythm};
use building::ops;
use building::parts::{Pin, Placement, Template};
use building::{Building, Mass, MassId, PlanNode, SolidId};
use scene::standard::math::{apply, transform_matrix};

use whiting_gallery::instancing::{standing, template_points};

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

/// Rounds a point so two points that stand in the same place compare equal.
fn key(point: [f32; 3]) -> [i64; 3] {
    point.map(|value| (value * 10_000.0).round() as i64)
}

fn main() {
    let plan = Starter {
        width: 5.0,
        depth: 8.0,
        height: 5.6,
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
    let template = ops::add_template(&mut skeleton, shutter(0.8, 1.0, 0.05));
    let mut placements = 0;
    for row in 0..2 {
        for column in 0..3 {
            ops::place_part(
                &mut skeleton,
                Placement::new(template, front, row, column, Pin::new(0.25, 0.25)),
            )
            .expect("every bay of the grid is in the grid");
            placements += 1;
        }
    }
    let document = Building::new(skeleton.clone());
    let built = document.build().expect("the document expands");
    let held = skeleton
        .parts
        .template(template)
        .expect("the template is in it");
    let own = held.mesh().expect("the template expands on its own");

    println!("# one template, {placements} placements");
    let baked: usize = (0..placements)
        .map(|index| built.faces_of(SolidId::Part(index)).len())
        .sum();
    println!(
        "  the baked mesh holds {baked} part faces; the template's own holds {} — {} copies, one upload",
        own.faces.len(),
        baked / own.faces.len()
    );
    println!(
        "  the template's points are {} and the instanced draw uploads them once",
        template_points(held).expect("the template packs").len()
    );

    println!("\n# the three routes agree, placement by placement");
    let mut frame_against_scene = 0;
    let mut scene_against_baked = 0;
    for index in 0..placements {
        let frame = part_frame(&skeleton, index)
            .expect("the wall offsets")
            .expect("the placement resolves");
        let (translation, rotation, scale) = standing(&frame);
        let matrix = transform_matrix(translation, rotation, scale);
        let by_frame: BTreeSet<[i64; 3]> =
            own.points.iter().map(|at| key(frame.apply(*at))).collect();
        let by_scene: BTreeSet<[i64; 3]> = own
            .points
            .iter()
            .map(|at| {
                let mapped = apply(
                    &matrix,
                    [f64::from(at[0]), f64::from(at[1]), f64::from(at[2])],
                );
                key([mapped[0] as f32, mapped[1] as f32, mapped[2] as f32])
            })
            .collect();
        let baked: BTreeSet<[i64; 3]> = built
            .faces_of(SolidId::Part(index))
            .iter()
            .flat_map(|face| {
                built.faces[*face]
                    .indices
                    .iter()
                    .map(|corner| key(built.points[*corner as usize]))
            })
            .collect();
        if by_frame != by_scene {
            frame_against_scene += 1;
        }
        if by_frame != baked {
            scene_against_baked += 1;
        }
    }
    println!(
        "  placements where the frame and the scene's instance transform disagree: {frame_against_scene}"
    );
    println!("  placements where the frame and the baked geometry disagree: {scene_against_baked}");

    println!("\n# the rotation is a yaw, and it is the frame's own");
    for index in [0, 5] {
        let frame = part_frame(&skeleton, index)
            .expect("the wall offsets")
            .expect("the placement resolves");
        let (translation, rotation, _) = standing(&frame);
        println!(
            "  part {index}: at ({:.3}, {:.3}, {:.3}), turned {:.4} rad; its along is ({:.3}, {:.3})",
            translation[0],
            translation[1],
            translation[2],
            rotation[1],
            frame.along[0],
            frame.along[2]
        );
    }
}
