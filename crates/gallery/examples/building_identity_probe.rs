//! The identity slice's acceptance on the CPU half: every corner of a building carries the id of the
//! element it belongs to, and the id is a row of the table.
//!
//! The frame half — a pixel carrying that id through the shader and back — is the renderer's; what this
//! probe can prove without one is the half the picture will *read*: the numbers are per face, the three
//! corners of a face agree, and the set of numbers is exactly the set of authored elements. That is the
//! join an agent needs, and it is checkable here rather than argued.
//!
//! Run with `cargo run -p whiting-gallery --example building_identity_probe`.

use std::collections::BTreeSet;

use building::derive::{RoofStyle, Starter, starter};
use building::grid::{FaceRef, Rhythm};
use building::identity::{Identity, NOT_AUTHORED};
use building::ops;
use building::parts::{Pin, Placement, Template};
use building::wall::Wall;
use building::{Building, Mass, MassId, PlanNode};

use whiting_gallery::building_identity::{building_extras, ids, outputs_field, packed};

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
    // A document that holds one of everything: a two-storey shell with a plate, a courtyard wall, and a part
    // standing in a bay. The identity is over all of it.
    let plan = Starter {
        width: 5.0,
        depth: 8.0,
        height: 5.6,
        roof: RoofStyle::Gable,
        ..Starter::default()
    };
    let mut skeleton = starter(&plan).skeleton.clone();
    let mass = MassId(0);
    for face in skeleton.faces_of(mass) {
        ops::split_face(
            &mut skeleton,
            face,
            &Rhythm::Even { count: 3 },
            &Rhythm::Even { count: 2 },
        );
    }
    let floor = building::derive::floor_slab(&skeleton.masses[0].clone(), mass, 2.8, 0.24)
        .expect("the plate is inside the building");
    skeleton.slabs.push(floor);
    skeleton.walls.push(Wall::new(
        PlanNode::new([-6.0, -6.0], 0.0),
        PlanNode::new([6.0, -6.0], 0.0),
        0.3,
        1.8,
    ));
    let front = FaceRef::new(mass, 2);
    let template = ops::add_template(&mut skeleton, shutter(0.8, 1.0, 0.05));
    ops::place_part(
        &mut skeleton,
        Placement::new(template, front, 0, 1, Pin::new(0.3, 0.3)),
    )
    .expect("the bay is in the grid");

    let building = Building::new(skeleton);
    let built = building.build().expect("the document expands");
    let identity = Identity::of(&built, 1);

    println!("# the table");
    println!(
        "  {} faces, {} elements numbered {}..={}",
        built.faces.len(),
        identity.len(),
        identity.first_id(),
        identity.first_id() + identity.len() as u32 - 1
    );
    println!("  zero is reserved for what is not the building: {NOT_AUTHORED}");

    println!("\n# one id per corner, and the three corners of a face agree");
    let per_point = ids(&built, &identity);
    println!(
        "  {} points for {} faces (three each): {}",
        per_point.len(),
        built.faces.len(),
        per_point.len() == built.faces.len() * 3
    );
    let mut disagreeing = 0;
    for face in per_point.chunks(3) {
        if face[0] != face[1] || face[1] != face[2] {
            disagreeing += 1;
        }
    }
    println!("  faces whose corners disagree: {disagreeing}");

    println!("\n# the numbers are exactly the elements, and no others");
    let written: BTreeSet<u32> = per_point.iter().copied().collect();
    let expected: BTreeSet<u32> = identity.rows().iter().map(|row| row.id).collect();
    println!(
        "  {} distinct ids written against {} rows; equal: {}",
        written.len(),
        expected.len(),
        written == expected
    );
    println!(
        "  zero written anywhere: {}",
        written.contains(&NOT_AUTHORED as &u32)
    );

    println!("\n# what a pixel would answer with, for a few of them");
    for wanted in [
        "outer wall over edge",
        "reveal of opening",
        "its top",
        "wall 0",
    ] {
        if let Some(row) = identity.rows().iter().find(|row| row.path.contains(wanted)) {
            let faced = per_point[3 * built
                .faces
                .iter()
                .position(|face| face.element == row.element)
                .expect("the element has a face")];
            println!("  id {faced} → {}", row.path);
        }
    }

    println!("\n# the buffer the frame reads, and the scene's own outputs field");
    let buffer = packed(&built, &identity);
    println!(
        "  {} points written; the outputs field declares {:?}",
        buffer.len(),
        outputs_field()
    );
    println!(
        "  the extras declare {} output(s), the first at location {:?}",
        building_extras().outputs().len(),
        building_extras()
            .outputs()
            .first()
            .map(|output| output.name.clone())
    );

    println!("\n# a document with nothing in it writes nothing");
    let empty = Building::new(building::skeleton::BuildingSkeleton::default())
        .build()
        .expect("an empty skeleton builds an empty mesh");
    let empty_identity = Identity::of(&empty, 1);
    println!(
        "  {} faces, {} elements, {} ids",
        empty.faces.len(),
        empty_identity.len(),
        ids(&empty, &empty_identity).len()
    );

    println!("\n# the join closes: an id from a frame names the element it shows");
    for id in [3, 4, 16, 20, 0, 9999] {
        println!(
            "  id {id:4} → {:?}",
            whiting_gallery::building_identity::locate(id)
        );
    }
}
