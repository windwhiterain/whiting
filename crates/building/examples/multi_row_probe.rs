//! A reduction probe: which configuration of openings on one face stops the wall closing?
//!
//! Run with `cargo run -p whiting-building --example multi_row_probe`.

use whiting_building::derive::{RoofStyle, Starter, starter};
use whiting_building::grid::{FaceRef, Rhythm};
use whiting_building::openings::{Inset, Opening};
use whiting_building::ops;
use whiting_building::{Building, MassId, SolidId};

fn shed(rows: usize) -> Building {
    let plan = Starter {
        width: 5.0,
        depth: 8.0,
        height: if rows == 1 { 2.8 } else { 5.6 },
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
            &Rhythm::Even { count: rows },
        );
    }
    Building::new(skeleton)
}

fn report(label: &str, building: Building) {
    match building.build() {
        Ok(built) => {
            let solid = SolidId::Mass(MassId(0));
            println!(
                "  {label}: loops {}, euler {}, manifold {}",
                built.solid_boundary_loops(solid),
                built.solid_euler_characteristic(solid),
                built.solid_is_manifold(solid)
            );
        }
        Err(error) => println!("  {label}: refused, {:?}", error.problems),
    }
}

fn main() {
    let front = FaceRef::new(MassId(0), 2);

    println!("# one storey, openings in one row");
    for count in 1..=3 {
        let mut with = shed(1);
        for column in 0..count {
            ops::add_opening(
                &mut with.skeleton,
                Opening::new(front, 0, column, Inset::new(0.35, 0.35, 1.0, 0.6)),
            )
            .expect("the bay is in the grid");
        }
        report(&format!("{count} opening(s) in row 0"), with);
    }

    println!("\n# two storeys, openings in one row");
    for count in 1..=3 {
        let mut with = shed(2);
        for column in 0..count {
            ops::add_opening(
                &mut with.skeleton,
                Opening::new(front, 0, column, Inset::new(0.35, 0.35, 1.0, 0.6)),
            )
            .expect("the bay is in the grid");
        }
        report(&format!("{count} opening(s) in row 0"), with);
    }

    println!("\n# two storeys, one opening in each row");
    let mut two = shed(2);
    ops::add_opening(
        &mut two.skeleton,
        Opening::new(front, 0, 1, Inset::new(0.35, 0.35, 0.0, 0.7)),
    )
    .expect("the bay is in the grid");
    ops::add_opening(
        &mut two.skeleton,
        Opening::new(front, 1, 1, Inset::new(0.35, 0.35, 0.9, 0.8)),
    )
    .expect("the bay is in the grid");
    report("one in each row, same column", two);

    let mut two = shed(2);
    ops::add_opening(
        &mut two.skeleton,
        Opening::new(front, 0, 0, Inset::new(0.35, 0.35, 0.0, 0.7)),
    )
    .expect("the bay is in the grid");
    ops::add_opening(
        &mut two.skeleton,
        Opening::new(front, 1, 2, Inset::new(0.35, 0.35, 0.9, 0.8)),
    )
    .expect("the bay is in the grid");
    report("one in each row, different columns", two);

    println!("\n# two storeys, three in each row (the workshop's own gable end)");
    let mut all = shed(2);
    for row in 0..2 {
        for column in 0..3 {
            ops::add_opening(
                &mut all.skeleton,
                Opening::new(front, row, column, Inset::new(0.35, 0.35, 0.8, 0.7)),
            )
            .expect("the bay is in the grid");
        }
    }
    report("six openings, two rows", all);
    println!("\n# the workshop's own gable end: a door and two windows below, three windows above");
    let mut exact = shed(2);
    ops::add_opening(
        &mut exact.skeleton,
        Opening::new(front, 0, 1, Inset::new(0.28, 0.28, 0.0, 0.75)),
    )
    .expect("the bay is in the grid");
    for column in [0, 2] {
        ops::add_opening(
            &mut exact.skeleton,
            Opening::new(front, 0, column, Inset::new(0.34, 0.34, 1.15, 0.6)),
        )
        .expect("the bay is in the grid");
    }
    for column in 0..3 {
        ops::add_opening(
            &mut exact.skeleton,
            Opening::new(front, 1, column, Inset::new(0.34, 0.34, 0.95, 0.85)),
        )
        .expect("the bay is in the grid");
    }
    report("the workshop's six openings", exact);
    println!("\n# a door whose sill is the ground, alone and with neighbours");
    let mut door_alone = shed(1);
    ops::add_opening(
        &mut door_alone.skeleton,
        Opening::new(front, 0, 1, Inset::new(0.28, 0.28, 0.0, 0.75)),
    )
    .expect("the bay is in the grid");
    report("one door to the ground in one row", door_alone);

    let mut door_and_window = shed(1);
    ops::add_opening(
        &mut door_and_window.skeleton,
        Opening::new(front, 0, 1, Inset::new(0.28, 0.28, 0.0, 0.75)),
    )
    .expect("the bay is in the grid");
    ops::add_opening(
        &mut door_and_window.skeleton,
        Opening::new(front, 0, 0, Inset::new(0.34, 0.34, 1.15, 0.6)),
    )
    .expect("the bay is in the grid");
    report(
        "a door to the ground and a window beside it",
        door_and_window,
    );

    let mut door_two_rows = shed(2);
    ops::add_opening(
        &mut door_two_rows.skeleton,
        Opening::new(front, 0, 1, Inset::new(0.28, 0.28, 0.0, 0.75)),
    )
    .expect("the bay is in the grid");
    ops::add_opening(
        &mut door_two_rows.skeleton,
        Opening::new(front, 1, 1, Inset::new(0.34, 0.34, 0.95, 0.85)),
    )
    .expect("the bay is in the grid");
    report("a door to the ground and a window above it", door_two_rows);
    println!(
        "\n# where the failing wall is open: the boundary edges of the door-plus-window-above case"
    );
    let mut broken = shed(2);
    ops::add_opening(
        &mut broken.skeleton,
        Opening::new(front, 0, 1, Inset::new(0.28, 0.28, 0.0, 0.75)),
    )
    .expect("the bay is in the grid");
    ops::add_opening(
        &mut broken.skeleton,
        Opening::new(front, 1, 1, Inset::new(0.34, 0.34, 0.95, 0.85)),
    )
    .expect("the bay is in the grid");
    if let Ok(built) = broken.build() {
        let solid = SolidId::Mass(MassId(0));
        let faces = built.faces_of(solid);
        let edges = built.boundary_edges_of(&faces);
        println!("  {} boundary edges of the mass", edges.len());
        for (index, (from, to)) in edges.iter().take(16).enumerate() {
            let a = built.points[*from as usize];
            let b = built.points[*to as usize];
            println!(
                "    {index}: ({:.3}, {:.3}, {:.3}) -> ({:.3}, {:.3}, {:.3})",
                a[0], a[1], a[2], b[0], b[1], b[2]
            );
        }
    }
}
