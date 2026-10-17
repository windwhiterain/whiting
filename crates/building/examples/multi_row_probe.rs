//! A reduction probe for the openings' cutting: which configurations of holes on one wall close it?
//!
//! # What this is for
//!
//! The workshop scene found a mass that expands **open** — `loops 4`, `euler -14`, non-manifold — with two
//! rows of openings on one wall, and this probe is how the failing configuration was reduced to four lines
//! of a table. It is a **diagnostic**, not an acceptance: the acceptance for the fix is that the last row of
//! the table reads `loops 0` and `manifold true`.
//!
//! The configuration that fails is **two holes whose `u` spans overlap and whose jambs are not aligned**: a
//! doorway with one inset and a window above it with another. Equal insets per column, or holes side by side
//! in one row, both close.
//!
//! Run with `cargo run -p whiting-building --example multi_row_probe`.

use std::collections::BTreeMap;

use whiting_building::derive::{RoofStyle, Starter, starter};
use whiting_building::grid::{FaceRef, Rhythm};
use whiting_building::openings::{Inset, Opening};
use whiting_building::ops;
use whiting_building::{Building, MassId, SolidId};

/// The wall's own edge, where every opening in this probe is cut.
fn front() -> FaceRef {
    FaceRef::new(MassId(0), 2)
}

/// Returns a shed with a grid of `bays × rows` on every face.
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

/// Adds one opening and returns the document, refusing to continue if the slot is not in the grid.
fn with_opening(mut building: Building, row: usize, column: usize, inset: Inset) -> Building {
    ops::add_opening(
        &mut building.skeleton,
        Opening::new(front(), row, column, inset),
    )
    .expect("the slot is in the grid");
    building
}

/// Prints what the mass closes like.
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

/// Returns the insets this probe uses: one per hole, all clear of the miter's own offset.
fn window() -> Inset {
    Inset::new(0.35, 0.35, 1.0, 0.6)
}

fn narrow_window() -> Inset {
    Inset::new(0.40, 0.40, 0.6, 0.8)
}

fn door() -> Inset {
    Inset::new(0.28, 0.28, 0.0, 0.75)
}

fn main() {
    println!("# one storey, one row");
    for count in 1..=3 {
        let mut building = shed(1);
        for column in 0..count {
            building = with_opening(building, 0, column, window());
        }
        report(&format!("{count} hole(s) in row 0"), building);
    }

    println!("\n# two storeys, holes in one row");
    for count in 1..=3 {
        let mut building = shed(2);
        for column in 0..count {
            building = with_opening(building, 0, column, window());
        }
        report(&format!("{count} hole(s) in row 0 of 2"), building);
    }

    println!("\n# two storeys, one hole in each row");
    let building = with_opening(with_opening(shed(2), 0, 1, door()), 1, 1, window());
    report(
        "a door below and a window above, jambs not aligned",
        building,
    );
    let building = with_opening(
        with_opening(shed(2), 0, 0, Inset::new(0.35, 0.35, 0.0, 0.75)),
        1,
        2,
        window(),
    );
    report("a door in one column and a window in another", building);
    let building = with_opening(
        with_opening(shed(2), 0, 1, Inset::new(0.35, 0.35, 0.0, 0.75)),
        1,
        1,
        window(),
    );
    report("a door below and a window above, jambs aligned", building);

    println!("\n# two storeys, three holes in each row");
    let mut aligned = shed(2);
    for row in 0..2 {
        for column in 0..3 {
            aligned = with_opening(aligned, row, column, window());
        }
    }
    report("six holes, the same inset in every column", aligned);

    let mut staggered = shed(2);
    for column in 0..3 {
        staggered = with_opening(staggered, 0, column, window());
        staggered = with_opening(staggered, 1, column, narrow_window());
    }
    report("six holes, a different inset per row", staggered);

    println!("\n# the failing case, in detail: which elements the mass has");
    let failing = with_opening(with_opening(shed(2), 0, 1, door()), 1, 1, window());
    match failing.build() {
        Ok(built) => {
            let solid = SolidId::Mass(MassId(0));
            let faces = built.faces_of(solid);
            let edges = built.boundary_edges_of(&faces);
            println!("  {} boundary edges", edges.len());
            for (index, (from, to)) in edges.iter().take(8).enumerate() {
                let a = built.points[*from as usize];
                let b = built.points[*to as usize];
                println!(
                    "    {index}: ({:.3}, {:.3}, {:.3}) -> ({:.3}, {:.3}, {:.3})",
                    a[0], a[1], a[2], b[0], b[1], b[2]
                );
            }
            let mut by_element: BTreeMap<String, usize> = BTreeMap::new();
            for face in &built.faces {
                if face.element.solid() == solid {
                    *by_element.entry(format!("{:?}", face.element)).or_insert(0) += 1;
                }
            }
            for (element, count) in &by_element {
                if element.contains("Reveal") {
                    println!("    {count:3} x {element}");
                }
            }
        }
        Err(error) => println!("  refused: {:?}", error.problems),
    }

    println!("\n# every face of the failing mass near the door, with its corners");
    let failing = with_opening(with_opening(shed(2), 0, 1, door()), 1, 1, window());
    if let Ok(built) = failing.build() {
        for (index, face) in built.faces.iter().enumerate() {
            let corners: Vec<[f32; 3]> = face
                .indices
                .iter()
                .map(|corner| built.points[*corner as usize])
                .collect();
            let near = corners.iter().all(|at| at[0].abs() < 0.7 && at[2] > 3.5);
            if !near {
                continue;
            }
            let shape: Vec<String> = corners
                .iter()
                .map(|at| format!("({:.3},{:.3},{:.3})", at[0], at[1], at[2]))
                .collect();
            println!("  {index}: {:?} {}", face.element, shape.join(" "));
        }
    }

    println!("\n# who owns each boundary edge of the failing mass");
    let failing = with_opening(with_opening(shed(2), 0, 1, door()), 1, 1, window());
    if let Ok(built) = failing.build() {
        let solid = SolidId::Mass(MassId(0));
        let faces = built.faces_of(solid);
        let edges = built.boundary_edges_of(&faces);
        for (from, to) in edges.iter().take(6) {
            let a = built.points[*from as usize];
            let b = built.points[*to as usize];
            let mut owners = Vec::new();
            for index in &faces {
                let corners = built.faces[*index].indices;
                let forward = corners.windows(2).any(|pair| pair[0] == *from && pair[1] == *to)
                    || corners.contains(from) && corners.contains(to);
                if forward {
                    owners.push(format!("{:?}", built.faces[*index].element));
                }
            }
            println!(
                "  ({:.3},{:.3},{:.3})->({:.3},{:.3},{:.3}): {} owner(s) {:?}",
                a[0], a[1], a[2], b[0], b[1], b[2], owners.len(), owners
            );
        }
    }
}