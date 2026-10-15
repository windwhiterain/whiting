//! What the erosion lattice puts in the picture, measured.
//!
//! Run with `cargo run -p whiting-gallery --example texel_probe`. It reads the
//! landscape scene's own massif onto the scene's own lattice, solves it, and
//! measures the four things the texel-artifact investigation is about: the
//! cell-scale roughness the solve leaves, the grain the drawn normal stencil
//! paints, how far the shading normal stands from the mesh's own triangles, and
//! the one-sided rim reads. Every number is a statistic over the drawn surface,
//! so a change to `CELL`, to `Erosion::normal_at`'s stencil or to the solver's
//! routing moves them.
//!
//! The same surface is measured twice — at the spacing the working tree asks
//! for and at twelve — because the spacing is one of the knobs under test.

use terrain::{ErodeParams, HeightField, Hydraulic, erode_traced, rasterize};
use whiting_gallery::landscape::{Erosion, RANGES, eye_at, field_of_view, massif, stood_at};

fn main() {
    let authored = massif();
    let built = authored.build().expect("the authored massif expands");
    let (low, high) = built.bounds();
    let radius = (((high[0] - low[0]).powi(2) + (high[2] - low[2]).powi(2)) as f64).sqrt() * 0.5;
    let summit = high[1] as f64;

    // The frame, measured: how many pixels one cell is. The aim is near the
    // massif's own centre, so the distance to the centre is the distance that
    // decides the frame's scale.
    let eye = eye_at(radius);
    let target = [
        RANGES[0].centre[0] * radius,
        1450.0 + summit * 0.46,
        RANGES[0].centre[1] * radius,
    ];
    let distance = ((eye[0] - target[0]).powi(2)
        + (eye[1] - target[1]).powi(2)
        + (eye[2] - target[2]).powi(2))
    .sqrt();
    let frame_height = 2.0 * distance * (field_of_view() * 0.5).tan();
    println!(
        "frame at 1080px: height {frame_height:.0} units; one cell of 6 units = {:.2} px, of 12 = {:.2} px",
        1080.0 / frame_height * 6.0,
        1080.0 / frame_height * 12.0
    );

    let stood = stood_at(
        &built,
        [RANGES[0].centre[0] * radius, RANGES[0].centre[1] * radius],
        RANGES[0].scale,
        RANGES[0].turn,
    );

    // The scene's own erosion, at whatever spacing the working tree asks for.
    let scene = Erosion::of(&stood);
    report(&scene.surface, "scene (working tree)");

    // The same massif at twelve, for comparison.
    let read = rasterize(&stood, 12.0).expect("covered");
    let (eroded, _) =
        erode_traced(&ErodeParams::default(), &read.field, &Hydraulic).expect("solved");
    let margin = read.field.edge_distance();
    let fade: Vec<f32> = margin
        .iter()
        .map(|cells| ((*cells - 1.0) / 3.0).clamp(0.0, 1.0))
        .collect();
    let surface = read.field.with_surface(&eroded, &fade).expect("faded");
    report(&surface, "twelve units");
}

fn report(surface: &HeightField, name: &str) {
    println!(
        "\n=== {name}: {} x {} cells of {:.2} units, {} covered ===",
        surface.width(),
        surface.height(),
        surface.cell(),
        surface.covered()
    );
    roughness(surface, name);
    steep_edges(surface, name);
    anisotropy(surface, name);
    // The shading normal the working tree draws (two-cell span, with fallbacks).
    let drawn = |column: usize, row: usize| normal_span(surface, column, row, 2);
    // The one-cell span the committed code used.
    let narrow = |column: usize, row: usize| normal_span(surface, column, row, 1);
    // The exact normals of the mesh's own triangles.
    normal_mismatch(
        surface,
        &drawn,
        &format!("{name}: two-cell-span normal vs geometry"),
    );
    normal_mismatch(
        surface,
        &narrow,
        &format!("{name}: one-cell-span normal vs geometry"),
    );
    normal_grain(
        surface,
        &drawn,
        &format!("{name}: two-cell-span normal grain"),
    );
    normal_grain(
        surface,
        &narrow,
        &format!("{name}: one-cell-span normal grain"),
    );
    rim_normals(surface, name);
    interior_sinks(surface, name);
}

/// The second difference along both axes: how much cell-scale noise the field carries.
fn roughness(field: &HeightField, name: &str) {
    let mut second = Vec::new();
    let mut first = Vec::new();
    for row in 1..field.height() - 1 {
        for column in 1..field.width() - 1 {
            let Some(at) = field.index(column, row) else {
                continue;
            };
            if !field.is_inside(at) {
                continue;
            }
            let (Some(w), Some(e), Some(s), Some(n)) = (
                field.index(column - 1, row),
                field.index(column + 1, row),
                field.index(column, row - 1),
                field.index(column, row + 1),
            ) else {
                continue;
            };
            if ![w, e, s, n].iter().all(|next| field.is_inside(*next)) {
                continue;
            }
            let h = field.height_at(at);
            second.push((field.height_at(w) + field.height_at(e) - 2.0 * h).abs());
            second.push((field.height_at(s) + field.height_at(n) - 2.0 * h).abs());
            first.push((field.height_at(e) - h).abs());
            first.push((field.height_at(n) - h).abs());
        }
    }
    second.sort_by(|a, b| a.total_cmp(b));
    first.sort_by(|a, b| a.total_cmp(b));
    let at = |v: &[f32], q: f32| v[(v.len() as f32 * q) as usize];
    println!(
        "{name}: |first difference| median {:.2} p90 {:.2}; |second difference| median {:.3} p90 {:.2} max {:.2}",
        at(&first, 0.5),
        at(&first, 0.9),
        at(&second, 0.5),
        at(&second, 0.9),
        second.last().copied().unwrap_or(0.0),
    );
}

/// Whether the micro-relief runs along the lattice axes or across them: a D8
/// steepest-descent solver routes along the axes first, and its channels read as
/// grid-aligned rills. The second difference along an axis against along a
/// diagonal says which.
fn anisotropy(field: &HeightField, name: &str) {
    let mut axis = Vec::new();
    let mut diagonal = Vec::new();
    for row in 1..field.height() - 1 {
        for column in 1..field.width() - 1 {
            let Some(at) = field.index(column, row) else {
                continue;
            };
            if !field.is_inside(at) {
                continue;
            }
            let (Some(w), Some(e), Some(s), Some(n), Some(sw), Some(se), Some(nw), Some(ne)) = (
                field.index(column - 1, row),
                field.index(column + 1, row),
                field.index(column, row - 1),
                field.index(column, row + 1),
                field.index(column - 1, row - 1),
                field.index(column + 1, row - 1),
                field.index(column - 1, row + 1),
                field.index(column + 1, row + 1),
            ) else {
                continue;
            };
            if ![w, e, s, n, sw, se, nw, ne]
                .iter()
                .all(|next| field.is_inside(*next))
            {
                continue;
            }
            let h = field.height_at(at);
            axis.push((field.height_at(w) + field.height_at(e) - 2.0 * h).abs());
            axis.push((field.height_at(s) + field.height_at(n) - 2.0 * h).abs());
            diagonal.push((field.height_at(sw) + field.height_at(ne) - 2.0 * h).abs());
            diagonal.push((field.height_at(nw) + field.height_at(se) - 2.0 * h).abs());
        }
    }
    let mean = |v: &[f32]| v.iter().sum::<f32>() / v.len() as f32;
    println!(
        "{name}: |second difference| mean along the axes {:.3}, along the diagonals {:.3} (ratio {:.2})",
        mean(&axis),
        mean(&diagonal),
        mean(&axis) / mean(&diagonal),
    );
}

/// How steep the mesh's cell edges are, as the slope the triangles must draw.
fn steep_edges(field: &HeightField, name: &str) {
    let cell = field.cell();
    let mut worst = 0.0_f32;
    let mut steep = 0usize;
    let mut edges = 0usize;
    for at in 0..field.cells() {
        if !field.is_inside(at) {
            continue;
        }
        let here = field.height_at(at);
        let column = (at % field.width()) as isize;
        let row = (at / field.width()) as isize;
        for (dx, dy) in [(1, 0), (0, 1)] {
            let Some(next) = usize::try_from(column + dx)
                .ok()
                .zip(usize::try_from(row + dy).ok())
                .and_then(|(c, r)| field.index(c, r))
            else {
                continue;
            };
            if !field.is_inside(next) {
                continue;
            }
            edges += 1;
            let drop = (here - field.height_at(next)).abs() / cell;
            worst = worst.max(drop);
            if drop > 1.0 {
                steep += 1;
            }
        }
    }
    println!(
        "{name}: {steep} of {edges} cell edges steeper than 45 degrees ({:.0}%), steepest slope {worst:.2}",
        100.0 * steep as f32 / edges as f32
    );
}

/// The normal a vertex is shaded by, over a span of `span` cells either side,
/// with the fallbacks the scene's own `normal_at` takes.
fn normal_span(field: &HeightField, column: usize, row: usize, span: isize) -> [f64; 3] {
    let cell = field.cell() as f64;
    let read = |column: isize, row: isize| {
        usize::try_from(column)
            .ok()
            .zip(usize::try_from(row).ok())
            .and_then(|(column, row)| field.index(column, row))
            .filter(|at| field.is_inside(*at))
            .map(|at| field.height_at(at) as f64)
    };
    let column = column as isize;
    let row = row as isize;
    let here = read(column, row).unwrap_or(0.0);
    let slope = |dc: isize, dr: isize| match (
        read(column + span * dc, row + span * dr),
        read(column - span * dc, row - span * dr),
    ) {
        (Some(a), Some(b)) => (a - b) / (2.0 * span as f64 * cell),
        _ => match (read(column + dc, row + dr), read(column - dc, row - dr)) {
            (Some(a), Some(b)) => (a - b) / (2.0 * cell),
            (Some(a), None) => (here - a) / cell,
            (None, Some(b)) => (here - b) / cell,
            (None, None) => 0.0,
        },
    };
    let normal = [-slope(1, 0), 1.0, -slope(0, 1)];
    let length = (normal[0] * normal[0] + normal[1] * normal[1] + normal[2] * normal[2]).sqrt();
    if length > f64::EPSILON {
        [normal[0] / length, normal[1] / length, normal[2] / length]
    } else {
        [0.0, 1.0, 0.0]
    }
}

/// The angle between the shading normal and the geometric normal of each drawn triangle.
fn normal_mismatch(field: &HeightField, normal_at: &dyn Fn(usize, usize) -> [f64; 3], name: &str) {
    let cell = field.cell() as f64;
    let place = [(0.0, 0.0), (1.0, 0.0), (1.0, 1.0), (0.0, 1.0)];
    let mut angles = Vec::new();
    for row in 0..field.height().saturating_sub(1) {
        for column in 0..field.width().saturating_sub(1) {
            let corners = [
                (column, row),
                (column + 1, row),
                (column + 1, row + 1),
                (column, row + 1),
            ];
            if !corners
                .into_iter()
                .all(|(c, r)| field.index(c, r).is_some_and(|at| field.is_inside(at)))
            {
                continue;
            }
            let heights: Vec<f64> = corners
                .into_iter()
                .map(|(c, r)| field.height_at(field.index(c, r).unwrap()) as f64)
                .collect();
            for triangle in [[0, 3, 1], [1, 3, 2]] {
                let [a, b, c] = triangle.map(|corner| {
                    let (dc, dr) = place[corner];
                    [dc * cell, heights[corner], dr * cell]
                });
                let edge_one = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
                let edge_two = [c[0] - a[0], c[1] - a[1], c[2] - a[2]];
                let geometric = [
                    edge_one[1] * edge_two[2] - edge_one[2] * edge_two[1],
                    edge_one[2] * edge_two[0] - edge_one[0] * edge_two[2],
                    edge_one[0] * edge_two[1] - edge_one[1] * edge_two[0],
                ];
                let length = (geometric[0] * geometric[0]
                    + geometric[1] * geometric[1]
                    + geometric[2] * geometric[2])
                    .sqrt();
                if length < 1.0e-12 {
                    continue;
                }
                let geometric = [
                    geometric[0] / length,
                    geometric[1] / length,
                    geometric[2] / length,
                ];
                // The triangle's shading normal, interpolated at its centroid.
                let mut shading = [0.0; 3];
                for corner in triangle {
                    let (c, r) = corners[corner];
                    let n = normal_at(c, r);
                    for axis in 0..3 {
                        shading[axis] += n[axis] / 3.0;
                    }
                }
                let shading_length =
                    (shading[0] * shading[0] + shading[1] * shading[1] + shading[2] * shading[2])
                        .sqrt();
                if shading_length < 1.0e-12 {
                    continue;
                }
                let dot = (geometric[0] * shading[0] / shading_length
                    + geometric[1] * shading[1] / shading_length
                    + geometric[2] * shading[2] / shading_length)
                    .clamp(-1.0, 1.0);
                angles.push(dot.acos() * 180.0 / std::f64::consts::PI);
            }
        }
    }
    angles.sort_by(|a, b| a.total_cmp(b));
    let at = |q: f64| angles[(angles.len() as f64 * q) as usize];
    println!(
        "{name}: median {:.1} deg, p90 {:.1} deg, p99 {:.1} deg, max {:.1} deg over {} triangles",
        at(0.5),
        at(0.9),
        at(0.99),
        angles.last().copied().unwrap_or(0.0),
        angles.len(),
    );
}

/// How much the shading normal changes between neighbouring vertices: the
/// per-vertex grain the interpolated normal paints across the surface.
fn normal_grain(field: &HeightField, normal_at: &dyn Fn(usize, usize) -> [f64; 3], name: &str) {
    let mut angles = Vec::new();
    for row in 0..field.height() {
        for column in 0..field.width() {
            let Some(at) = field.index(column, row) else {
                continue;
            };
            if !field.is_inside(at) {
                continue;
            }
            let here = normal_at(column, row);
            for (dc, dr) in [(1, 0), (0, 1)] {
                let Some(next) = usize::try_from(column as isize + dc)
                    .ok()
                    .zip(usize::try_from(row as isize + dr).ok())
                    .and_then(|(c, r)| field.index(c, r))
                else {
                    continue;
                };
                if !field.is_inside(next) {
                    continue;
                }
                let there = normal_at(
                    (column as isize + dc) as usize,
                    (row as isize + dr) as usize,
                );
                let dot =
                    (here[0] * there[0] + here[1] * there[1] + here[2] * there[2]).clamp(-1.0, 1.0);
                angles.push(dot.acos() * 180.0 / std::f64::consts::PI);
            }
        }
    }
    angles.sort_by(|a, b| a.total_cmp(b));
    let at = |q: f64| angles[(angles.len() as f64 * q) as usize];
    println!(
        "{name}: median {:.2} deg, p90 {:.2} deg, p99 {:.2} deg between neighbouring vertices",
        at(0.5),
        at(0.9),
        at(0.99),
    );
}

/// How many vertices take the one-sided fallback, and what the drawn fallback
/// costs against the correct one-sided difference.
fn rim_normals(field: &HeightField, name: &str) {
    let cell = field.cell() as f64;
    let read = |column: isize, row: isize| {
        usize::try_from(column)
            .ok()
            .zip(usize::try_from(row).ok())
            .and_then(|(column, row)| field.index(column, row))
            .filter(|at| field.is_inside(*at))
            .map(|at| field.height_at(at) as f64)
    };
    let mut one_sided = 0usize;
    let mut wrong_sign = 0usize;
    let mut worst = 0.0_f64;
    for row in 0..field.height() {
        for column in 0..field.width() {
            let Some(at) = field.index(column, row) else {
                continue;
            };
            if !field.is_inside(at) {
                continue;
            }
            let (column, row) = (column as isize, row as isize);
            for (ahead, behind) in [
                ((column + 1, row), (column - 1, row)),
                ((column, row + 1), (column, row - 1)),
            ] {
                match (read(ahead.0, ahead.1), read(behind.0, behind.1)) {
                    (Some(_), None) => {
                        one_sided += 1;
                        wrong_sign += 1;
                        let here = read(column, row).unwrap();
                        let a = read(ahead.0, ahead.1).unwrap();
                        let drawn = (here - a) / cell;
                        let correct = (a - here) / cell;
                        worst = worst.max((drawn - correct).abs());
                    }
                    (None, Some(_)) => one_sided += 1,
                    _ => {}
                }
            }
        }
    }
    println!(
        "{name}: rim normals {one_sided} one-sided reads, {wrong_sign} of them the ahead-only branch; worst slope error {worst:.2} per unit"
    );
}

/// Interior pits: each is a dot of steep triangles the mesh must draw.
fn interior_sinks(field: &HeightField, name: &str) {
    let mut interior = 0usize;
    let mut rim = 0usize;
    for at in 0..field.cells() {
        if !field.is_inside(at) {
            continue;
        }
        let here = field.height_at(at);
        let column = (at % field.width()) as isize;
        let row = (at / field.width()) as isize;
        let mut lower = false;
        let mut outside = false;
        for (dx, dy) in [
            (-1, -1),
            (0, -1),
            (1, -1),
            (-1, 0),
            (1, 0),
            (-1, 1),
            (0, 1),
            (1, 1),
        ] {
            let (next_column, next_row) = (column + dx, row + dy);
            let neighbour = usize::try_from(next_column)
                .ok()
                .zip(usize::try_from(next_row).ok())
                .and_then(|(next_column, next_row)| field.index(next_column, next_row))
                .filter(|at| field.is_inside(*at));
            match neighbour {
                Some(next) => lower |= field.height_at(next) < here,
                None => outside = true,
            }
        }
        if !lower {
            if outside {
                rim += 1;
            } else {
                interior += 1;
            }
        }
    }
    println!("{name}: sinks {interior} interior, {rim} on the rim");
}
