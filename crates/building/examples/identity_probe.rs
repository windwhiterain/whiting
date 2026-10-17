//! The identity slice's acceptance: every face has a name, the names survive an edit, and the numbers
//! are a run with no gaps.
//!
//! Run with `cargo run -p whiting-building --example identity_probe`.

use whiting_building::derive::{RoofStyle, Starter, starter};
use whiting_building::grid::{FaceRef, Rhythm};
use whiting_building::identity::{Identity, NOT_AUTHORED};
use whiting_building::openings::{Inset, Opening};
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

fn main() {
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
    let columns = Rhythm::Even { count: 3 };
    let rows = Rhythm::Even { count: 1 };
    for face in skeleton.faces_of(mass) {
        ops::split_face(&mut skeleton, face, &columns, &rows);
    }
    ops::add_opening(
        &mut skeleton,
        Opening::new(front, 0, 1, Inset::new(0.25, 0.25, 0.0, 0.45)),
    )
    .expect("the middle bay is in the grid");
    let door = ops::add_template(&mut skeleton, shutter(1.10, 2.10, 0.05));
    ops::place_part(
        &mut skeleton,
        Placement::new(door, front, 0, 1, Pin::new(0.28, 0.0)),
    )
    .expect("the doorway is in a bay of the grid");

    let built = Building::new(skeleton.clone())
        .build()
        .expect("the shed expands");
    let identity = Identity::of(&built, 1);

    println!(
        "# {} faces name {} elements, ids {}..={}",
        built.faces.len(),
        identity.len(),
        identity.first_id(),
        identity.first_id() + identity.len() as u32 - 1
    );
    println!(
        "  zero is reserved and unallocated: NOT_AUTHORED is {NOT_AUTHORED}, and the table's \
         smallest id is {}",
        identity.rows().first().map(|row| row.id).unwrap_or(0)
    );

    println!("\n# every face resolves, and every id is a face");
    let mut unresolved = 0;
    for (index, _) in built.faces.iter().enumerate() {
        if identity.of_face(&built, index).is_none() {
            unresolved += 1;
        }
    }
    let mut unseen = 0;
    for row in identity.rows() {
        if !built.faces.iter().any(|face| face.element == row.element) {
            unseen += 1;
        }
    }
    println!("  faces without an id: {unresolved}; rows without a face: {unseen}");

    println!("\n# the round trip: an element's id names that element back");
    let mut wrong = 0;
    for row in identity.rows() {
        match identity.row(row.id) {
            Some(back) if back.element == row.element => {}
            _ => wrong += 1,
        }
    }
    println!("  rows whose id does not come back: {wrong}");

    println!("\n# what a reader is given, for one element of each kind");
    let samples: Vec<&str> = identity
        .rows()
        .iter()
        .map(|row| row.path.as_str())
        .collect();
    for wanted in [
        "outer wall over edge",
        "reveal of opening",
        "slab's soffit",
        "the skin of plane",
        "the fascia",
        "part 0",
    ] {
        if let Some(found) = samples.iter().find(|path| path.contains(wanted)) {
            println!("  {found}");
        }
    }

    println!("\n# the numbers survive an edit elsewhere: a second shutter on another face");
    let before: Vec<(u32, String)> = identity
        .rows()
        .iter()
        .map(|row| (row.id, row.path.clone()))
        .collect();
    let mut edited = skeleton.clone();
    let extra = ops::add_template(&mut edited, shutter(0.60, 1.20, 0.04));
    let side = FaceRef::new(mass, 1);
    ops::place_part(
        &mut edited,
        Placement::new(extra, side, 0, 0, Pin::new(0.40, 0.90)),
    )
    .expect("the side face has bays too");
    let edited_built = Building::new(edited)
        .build()
        .expect("the edited shed expands");
    let after = Identity::of(&edited_built, 1);
    let mut moved = 0;
    for (id, path) in &before {
        match after.rows().iter().find(|row| row.path == *path) {
            Some(row) if row.id == *id => {}
            _ => moved += 1,
        }
    }
    println!(
        "  {} rows before, {} after; rows of the old ones that changed id: {moved}",
        before.len(),
        after.len()
    );
    println!(
        "  and the new rows are the second shutter's: {}",
        after
            .rows()
            .iter()
            .filter(|row| row.path.contains("part 1"))
            .count()
    );

    println!("\n# two builds of one document number the elements the same");
    let again = Building::new(skeleton).build().expect("the shed expands");
    let twice = Identity::of(&again, 1);
    println!("  identical tables: {}", twice.rows() == identity.rows());

    println!("\n# a caller drawing two buildings numbers the second from where the first ended");
    let second = Identity::of(&built, identity.first_id() + identity.len() as u32);
    println!(
        "  first run {}..={}, second run {}..={}",
        identity.first_id(),
        identity.first_id() + identity.len() as u32 - 1,
        second.first_id(),
        second.first_id() + second.len() as u32 - 1
    );
    let door_faces = built.faces_of(SolidId::Part(0)).len();
    println!(
        "  the placed shutter is {} faces, all of them part 0's",
        door_faces
    );
}
