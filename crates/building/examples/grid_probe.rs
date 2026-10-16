//! The grid slice's acceptance: what a rhythm lays out, what a re-split replaces, and what is refused.
//!
//! Every number here is measured off the document. The claims being measured are the ones
//! `grid.rs` and `ops::split_face` make about themselves:
//!
//! - a family of `n` stops makes `n - 1` bays, and the bays **tile the face** — their widths add up
//!   to the edge's own length and their heights to the wall's own height, to within
//!   [`GRID_TOLERANCE`];
//! - dividing a face **replaces** its family rather than adding to it, and a slot still names the
//!   same cell after a re-split with the same rhythm;
//! - a rhythm the face cannot carry is **refused** rather than stretched, and so is a face the
//!   document does not hold.
//!
//! Run with `cargo run -p whiting-building --example grid_probe`.

use whiting_building::derive::{RoofStyle, Starter, starter};
use whiting_building::grid::{FaceRef, GRID_TOLERANCE, Rhythm};
use whiting_building::ops;
use whiting_building::{Building, MassId};

fn main() {
    let building = starter(&Starter {
        width: 7.0,
        depth: 11.0,
        roof: RoofStyle::Hip,
        ..Starter::default()
    });
    let mut skeleton = building.skeleton.clone();
    let faces = skeleton.faces_of(MassId(0));
    println!(
        "the starter house has {} faces; wall top {:.4}",
        faces.len(),
        skeleton.masses[0].top
    );

    println!("\n# one rhythm on every face: 3 bays across, 2 storeys up");
    let columns = Rhythm::Even { count: 3 };
    let rows = Rhythm::Even { count: 2 };
    for face in &faces {
        let bays = ops::split_face(&mut skeleton, *face, &columns, &rows);
        let length = skeleton.face_length(*face).expect("the face is there");
        let (base, top) = skeleton.face_extent(*face).expect("the face is there");
        let grid = skeleton.grid(*face).expect("a rhythm was just written");
        let widths: f32 = grid
            .slots()
            .iter()
            .filter(|(row, _)| *row == 0)
            .map(|(row, column)| grid.bay(*face, *row, *column).expect("in range").width())
            .sum();
        let heights: f32 = grid
            .slots()
            .iter()
            .filter(|(_, column)| *column == 0)
            .map(|(row, column)| grid.bay(*face, *row, *column).expect("in range").height())
            .sum();
        println!(
            "  edge {} of mass {}: bays {:?}, length {:.4}, widths {:.4} (off by {:.2e}), height {:.4}, \
             heights {:.4} (off by {:.2e})",
            face.edge,
            face.mass.0,
            bays,
            length,
            widths,
            (widths - length).abs(),
            top - base,
            heights,
            (heights - (top - base)).abs(),
        );
    }

    println!("\n# replace semantics: dividing a face a second time replaces its family");
    let face = faces[0];
    let before = skeleton.grid(face).expect("a rhythm").bays();
    let replaced = ops::split_face(&mut skeleton, face, &Rhythm::Even { count: 5 }, &rows);
    let after = skeleton.grid(face).expect("a rhythm").bays();
    println!(
        "  bays {before} -> {after} (asked for {:?} of 5 by 2 = 10), so the family was replaced and not \
         added: {}",
        replaced,
        after == 10
    );

    println!("\n# slot stability: the same slot names the same cell across a re-split");
    let column = 2;
    let row = 0;
    let first = skeleton.bay(face, row, column).expect("in range");
    ops::split_face(&mut skeleton, face, &Rhythm::Even { count: 5 }, &rows);
    let second = skeleton.bay(face, row, column).expect("in range");
    println!(
        "  slot ({row}, {column}): [{:.4}, {:.4}] x [{:.4}, {:.4}] before, [{:.4}, {:.4}] x [{:.4}, {:.4}] after",
        first.from,
        first.to,
        first.low,
        first.high,
        second.from,
        second.to,
        second.low,
        second.high
    );

    println!("\n# rhythms the face carries, and one it does not");
    let length = skeleton.face_length(face).expect("the face is there");
    // A module chosen so the modules do **not** fill the face: with bookends of 0.5 on a 7 m edge
    // there are 6 m between them, and 1.3 m modules leave 0.8 m over. That remainder is the whole
    // question this pair of rhythms exists to answer, and a module that divided evenly would show
    // nothing.
    let bookend = Rhythm::Rhythmic {
        bookend: 0.5,
        module: 1.3,
        flexible: false,
    };
    let flexible = Rhythm::Rhythmic {
        bookend: 0.5,
        module: 1.3,
        flexible: true,
    };
    for (name, rhythm) in [
        ("bookends and modules", &bookend),
        ("the same, flexible", &flexible),
    ] {
        match rhythm.stops(length) {
            Some(stops) => println!(
                "  {name}: {} stops, widths {:?}",
                stops.len(),
                stops
                    .windows(2)
                    .map(|pair| pair[1] - pair[0])
                    .collect::<Vec<_>>()
            ),
            None => println!("  {name}: refused on a face of length {length:.4}"),
        }
    }

    println!("\n# refusals");
    let cases: [(&str, FaceRef, Rhythm, Rhythm); 6] = [
        (
            "stops that do not span the edge",
            face,
            Rhythm::Absolute {
                stops: vec![0.0, 1.0, 2.0],
            },
            rows.clone(),
        ),
        (
            "no bays across",
            face,
            Rhythm::Even { count: 0 },
            rows.clone(),
        ),
        (
            "a module wider than the face",
            face,
            Rhythm::Rhythmic {
                bookend: 0.0,
                module: length * 2.0,
                flexible: true,
            },
            rows.clone(),
        ),
        (
            "a module so small it asks for millions of stops",
            face,
            Rhythm::Rhythmic {
                bookend: 0.0,
                module: 1.0e-6,
                flexible: true,
            },
            rows.clone(),
        ),
        (
            "a hundred thousand bays",
            face,
            Rhythm::Even { count: 100_000 },
            rows.clone(),
        ),
        (
            "a face the document does not hold",
            FaceRef::new(MassId(9), 0),
            columns.clone(),
            rows.clone(),
        ),
    ];
    for (name, at, across, up) in cases {
        let answer = ops::split_face(&mut skeleton, at, &across, &up);
        println!("  {name}: {answer:?}");
    }

    println!("\n# the document is still clean, and the tolerance is the one the sums used");
    let authored = Building::new(skeleton.clone());
    let violations = authored.validate();
    println!("  violations: {}", violations.len());
    for violation in &violations {
        println!("    {violation:?}");
    }
    println!("  GRID_TOLERANCE {GRID_TOLERANCE:e} m against the worst tiling error printed above");
}
