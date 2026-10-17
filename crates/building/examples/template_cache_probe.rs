//! The cache slice's acceptance: a document expands each template **once**, and the geometry is the same one
//! the uncached path produced.
//!
//! The measurement is structural rather than timed. `TemplateCache::of` answers how many templates it expanded,
//! so a document with twelve placements of one shutter must answer **one** — and the twelve placements' baked
//! points must still equal the template's own mesh mapped by each placement's frame, which is the equality
//! `instance_probe` established and this probe re-reads through the cached path.
//!
//! Run with `cargo run -p whiting-building --example template_cache_probe`.

use std::collections::BTreeSet;

use whiting_building::derive::{RoofStyle, Starter, starter};
use whiting_building::expand::{TemplateCache, part_frame};
use whiting_building::grid::{FaceRef, Rhythm};
use whiting_building::ops;
use whiting_building::parts::{Pin, Placement, Template};
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

/// Returns a broken template: a plan of two nodes, which has no area and cannot be a room.
fn broken() -> Template {
    Template::new(
        "broken",
        vec![Mass::new(
            vec![
                PlanNode::new([0.0, 0.0], 0.0),
                PlanNode::new([1.0, 0.0], 0.0),
            ],
            1.0,
        )],
        Vec::new(),
    )
}

/// Rounds a point so two points that stand in the same place compare equal.
fn key(point: [f32; 3]) -> [i64; 3] {
    point.map(|value| (value * 10_000.0).round() as i64)
}

fn main() {
    // Twelve placements: four bays across, three storeys.
    let plan = Starter {
        width: 8.0,
        depth: 8.0,
        height: 8.0,
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
            &Rhythm::Even { count: 4 },
            &Rhythm::Even { count: 3 },
        );
    }
    let template = ops::add_template(&mut skeleton, shutter(0.7, 0.9, 0.05));
    let mut placements = 0;
    for row in 0..3 {
        for column in 0..4 {
            ops::place_part(
                &mut skeleton,
                Placement::new(template, front, row, column, Pin::new(0.2, 0.2)),
            )
            .expect("every bay of the grid is in the grid");
            placements += 1;
        }
    }

    println!("# one template, {placements} placements");
    let cache = TemplateCache::of(&skeleton);
    println!(
        "  the cache expanded {} template(s) for {placements} placement(s): {} expansion(s) instead of {}",
        cache.expanded(),
        cache.expanded(),
        placements
    );

    println!("\n# and the geometry is the one the placements asked for");
    let built = Building::new(skeleton.clone())
        .build()
        .expect("the document expands");
    let held = cache
        .mesh(template)
        .expect("the shutter is in the document")
        .as_ref()
        .expect("the shutter expands");
    let mut wrong = 0;
    for index in 0..placements {
        let frame = part_frame(&skeleton, index)
            .expect("the wall offsets")
            .expect("the placement resolves");
        let by_frame: BTreeSet<[i64; 3]> = held
            .points
            .iter()
            .map(|point| key(frame.apply(*point)))
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
        if by_frame != baked {
            wrong += 1;
        }
    }
    println!("  placements whose baked points differ from the cached template's: {wrong}");
    println!(
        "  the mesh holds {} part faces, which is {} x the template's {}",
        (0..placements)
            .map(|index| built.faces_of(SolidId::Part(index)).len())
            .sum::<usize>(),
        placements,
        held.faces.len()
    );

    println!("\n# two templates are two expansions, and the count does not grow with placements");
    let second = ops::add_template(&mut skeleton, shutter(0.4, 0.9, 0.05));
    ops::place_part(
        &mut skeleton,
        Placement::new(second, front, 0, 0, Pin::new(0.2, 0.2)),
    )
    .expect("the bay is in the grid");
    let cache = TemplateCache::of(&skeleton);
    println!(
        "  {} placement(s), {} template(s), {} expansion(s)",
        skeleton.parts.placements().len(),
        skeleton.parts.template_count(),
        cache.expanded()
    );

    println!("\n# a template that cannot expand is held as its problems, once");
    let mut failing = skeleton.clone();
    let bad = ops::add_template(&mut failing, broken());
    ops::place_part(
        &mut failing,
        Placement::new(bad, front, 1, 1, Pin::new(0.0, 0.0)),
    )
    .expect("the bay is in the grid");
    let cache = TemplateCache::of(&failing);
    println!(
        "  the cache expanded {} of {} template(s), so the broken one is held as a fault",
        cache.expanded(),
        failing.parts.template_count()
    );
    match Building::new(failing).build() {
        Ok(_) => println!("  the document built anyway, which is wrong"),
        Err(error) => println!(
            "  and the document reports {} problem(s), the first {:?}",
            error.problems.len(),
            error.problems.first()
        ),
    }
}
