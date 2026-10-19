//! The offset slice's first step: measure the arithmetic an instanced id channel would rest on.
//!
//! # The claim under test
//!
//! One instanced mesh drawn five times carries **one** set of ids, so a pixel of the second shutter cannot be
//! told from a pixel of the first unless the shader adds something per instance. The cheapest "something" is an
//! **offset**, and an offset is only a mapping if the baked identity table lays each placement's surfaces down
//! as a **contiguous run, in the same order** the template's own table uses. Then
//!
//! ```text
//! baked_id(placement, surface) = template_local_id(surface) + base(placement) - 1
//! ```
//!
//! This probe measures that equation on every surface of every placement of a five-placement facade, and prints
//! the bases a shader would need. If it holds, the frame half is mechanical; if it does not, the offset design
//! is dead and the two-channel design is what is left.
//!
//! Run with `cargo run -p whiting-building --example instance_identity_probe`.

use std::collections::BTreeMap;

use whiting_building::derive::{RoofStyle, Starter, starter};
use whiting_building::grid::{FaceRef, Rhythm};
use whiting_building::identity::Identity;
use whiting_building::mesh::PartSurface;
use whiting_building::ops;
use whiting_building::parts::{Pin, Placement, Template};
use whiting_building::{Building, Element, Mass, MassId, PlanNode};

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
    let table = Identity::of(&built, 1);

    // The template's own table, which is what one instanced mesh could carry.
    let held = skeleton
        .parts
        .template(template)
        .expect("the shutter is in the document")
        .mesh()
        .expect("the shutter expands on its own");
    let local = Identity::of(&held, 1);

    println!("# one template, {placements} placements");
    println!(
        "  the baked table holds {} row(s); the template's own holds {} — so {placements} placements of one \
         surface are {placements} rows",
        table.len(),
        local.len()
    );

    // The baked rows, grouped by placement, in the table's own order.
    let mut by_placement: BTreeMap<usize, Vec<(u32, PartSurface)>> = BTreeMap::new();
    for row in table.rows() {
        if let Element::Part { placement, surface } = row.element {
            by_placement
                .entry(placement)
                .or_default()
                .push((row.id, surface));
        }
    }

    println!("\n# each placement's run in the baked table");
    let mut contiguous = true;
    let mut in_step = true;
    for (placement, rows) in &by_placement {
        let first = rows
            .first()
            .expect("a placement draws at least one surface")
            .0;
        let last = rows
            .last()
            .expect("a placement draws at least one surface")
            .0;
        let run = (last - first) as usize + 1;
        if run != rows.len() {
            contiguous = false;
        }
        println!(
            "  placement {placement}: ids {first}..={last} ({} row(s), run {run}), base {first}",
            rows.len()
        );
    }
    println!("  every placement's rows are one contiguous run: {contiguous}");

    // The equation, surface by surface: baked = local + base - 1.
    let mut checked = 0;
    for rows in by_placement.values() {
        let base = rows
            .first()
            .expect("a placement draws at least one surface")
            .0;
        for (id, surface) in rows {
            let local_element = match surface {
                PartSurface::Mass(id, held_surface) => Element::Mass(*id, *held_surface),
                PartSurface::Roof(id, held_surface) => Element::Roof(*id, *held_surface),
                PartSurface::Wall(id, held_surface) => Element::Wall(*id, *held_surface),
                PartSurface::Slab(id, held_surface) => Element::Slab(*id, *held_surface),
                PartSurface::Member(id, held_surface) => Element::Member(*id, *held_surface),
                PartSurface::Roll(id, held_surface) => Element::Roll(*id, *held_surface),
            };
            let Some(local_id) = local.id_of(local_element) else {
                in_step = false;
                continue;
            };
            let predicted = local_id + base - 1;
            if predicted != *id {
                in_step = false;
            }
            checked += 1;
        }
        // And the same count per placement, which is what makes one table serve all of them.
        if rows.len() != local.len() {
            in_step = false;
        }
    }
    println!("\n# baked = local + base - 1, over {checked} surface(s): {in_step}");

    println!("\n# what a shader would need, per placement");
    for (placement, rows) in &by_placement {
        let base = rows
            .first()
            .expect("a placement draws at least one surface")
            .0;
        println!(
            "  placement {placement}: instance offset {} (so local id 1 answers {})",
            base as i64 - 1,
            base
        );
    }

    println!(
        "\n# the offset design is {}",
        if contiguous && in_step {
            "alive: one mesh, one table, one offset per instance"
        } else {
            "dead: the two tables do not line up, so the two-channel design is what is left"
        }
    );
}
