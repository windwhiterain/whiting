//! Measures the roof planes' planarity and coverage, on the document and on the expanded mesh.
//!
//! `authoring_probe` reads the **document**: two rows' crest points and eave stations, and the quad
//! they span. This probe reads what `validate` reads — the welded points of one authored element
//! **after** expansion — and prints the corners themselves, so a departure can be traced to the row
//! that caused it instead of guessed at.
//!
//! For every case it prints:
//!
//! 1. each plane's coplanarity departure in metres, twice: the fourth corner off the plane of the
//!    first three (exact, no fitting) and the farthest corner off the group's own least squares fit;
//! 2. the skin's plan area against the eave loop's plan area, which are equal only when the planes
//!    tile the loop;
//! 3. the rows adjacent planes share, and the crest edge a gable's two planes share;
//! 4. every crest elevation against `top - sink + rise`, and every row's crest index and station;
//! 5. the refusals and the seed discipline, as `authoring_probe` does;
//! 6. positive controls, so a run of zeros means the checks fire rather than that they do nothing.
//!
//! ```text
//! cargo run -p whiting-building --example roof_planarity_probe
//! ```

use std::array;
use std::collections::{BTreeSet, HashMap};

use whiting_building::derive::{self, RIDGE_INSETS, RoofStyle, Starter};
use whiting_building::math::{Vec3, cross3, dot3, length3, subtract};
use whiting_building::mesh::{Element, RoofSurface};
use whiting_building::{Mass, MassId, PlanNode, Roof, RoofId, RoofRow, SolidId};

/// The departures that count as planar for every claim in this file, in metres.
const TOLERANCE: f32 = 1.0e-4;

fn main() {
    for (style, width, depth) in starters() {
        measure(&Starter {
            width,
            depth,
            roof: style,
            ..Starter::default()
        });
    }

    println!("\n== every published ridge inset, over every starter ==");
    for (index, _wanted) in RIDGE_INSETS.iter().enumerate() {
        for (style, width, depth) in starters() {
            let seed = seed_of(index, width, depth);
            compact(&Starter {
                width,
                depth,
                roof: style,
                seed,
                ..Starter::default()
            });
        }
    }

    println!("\n== positive controls: the measurements above can fire ==");
    controls();

    println!("\n== refusals ==");
    refusals();

    println!("\n== the seed discipline ==");
    seed_discipline();
}

/// The three starters the crate's own census reports on.
fn starters() -> [(RoofStyle, f32, f32); 3] {
    [
        (RoofStyle::Hip, 7.0, 11.0),
        (RoofStyle::Gable, 7.0, 11.0),
        (RoofStyle::Hip, 6.0, 6.0),
    ]
}

/// Prints every measurement of one starter building.
fn measure(starter: &Starter) {
    let building = derive::starter(starter);
    let mass = &building.skeleton.masses[0];
    let roof = &building.skeleton.roofs[0];
    println!(
        "\n== {:?} {}x{} (seed {}): eave loop {} nodes, plan area {:.4}, mass plan area {:.4} ==",
        starter.roof,
        starter.width,
        starter.depth,
        starter.seed,
        roof.eaves.len(),
        roof.plan_area(),
        mass.plan_area()
    );
    println!(
        "   crest elevations {:?} against top - sink + rise = {:.4}; every row: {}",
        roof.crest
            .iter()
            .map(|point| point.elevation)
            .collect::<Vec<_>>(),
        mass.highest_top() - starter.sink + starter.rise,
        if rows_sound(roof) {
            "every crest index in range, every station in [0, 1)"
        } else {
            "A ROW NAMES SOMETHING OUT OF RANGE"
        }
    );
    println!(
        "   every row's station stands on a corner of the loop: {}",
        every_station_is_a_corner(roof)
    );

    // The document's own reading of each plane, which is what `derive` claims.
    for (index, plane) in roof.planes.iter().enumerate() {
        let corners: Vec<[Vec3; 4]> = plane
            .rows
            .windows(2)
            .map(|pair| quad(roof, &pair[0], &pair[1]))
            .collect();
        let departures: Vec<String> = corners
            .iter()
            .map(|corners| format!("{:.7}", exact_departure(corners)))
            .collect();
        println!(
            "   document plane {index}: {} row(s) -> {} facet(s), departure off the first three: {}",
            plane.rows.len(),
            corners.len(),
            departures.join(", ")
        );
    }

    let mesh = match building.build() {
        Ok(mesh) => mesh,
        Err(error) => {
            println!("   did not expand: {:?}", error.problems);
            return;
        }
    };
    match building.validate().first() {
        None => println!("   validate: 0 violations"),
        Some(violation) => println!("   validate: first violation {violation:?}"),
    }

    // Per element group: the vertices validate measures, and the departure it computes.
    let mut groups: HashMap<Element, Vec<usize>> = HashMap::new();
    for (index, face) in mesh.faces.iter().enumerate() {
        groups.entry(face.element).or_default().push(index);
    }
    let order = ordered(&groups);
    let mut worst = 0.0f32;
    for element in &order {
        if !matches!(element, Element::Roof(_, RoofSurface::Skin { .. })) {
            continue;
        }
        let faces = &groups[element];
        let vertices: BTreeSet<u32> = faces
            .iter()
            .flat_map(|index| mesh.faces[*index].indices)
            .collect();
        let points: Vec<Vec3> = vertices
            .iter()
            .map(|index| mesh.points[*index as usize])
            .collect();
        let departure = fitted_departure(&points);
        worst = worst.max(departure);
        println!(
            "   mesh {element:?}: {} face(s), {} distinct corner(s), fitted departure {departure:.7} m {}",
            faces.len(),
            points.len(),
            if departure < TOLERANCE {
                "planar"
            } else {
                "NOT PLANAR"
            }
        );
        for corner in &points {
            println!(
                "        corner ({:.4}, {:.4}, {:.4})",
                corner[0], corner[1], corner[2]
            );
        }
    }

    // The skin covers the eave loop exactly: the two plan areas are equal only then.
    let skin: Vec<usize> = order
        .iter()
        .filter(|element| matches!(element, Element::Roof(_, RoofSurface::Skin { .. })))
        .flat_map(|element| groups[element].iter().copied())
        .collect();
    let skin_area = mesh.plan_area_of(&skin).abs();
    let eave_area = roof.plan_area().abs();
    println!(
        "   skin plan area {skin_area:.4} against the eave loop's {eave_area:.4}: difference {:.7} m2 {}",
        (skin_area - eave_area).abs(),
        if (skin_area - eave_area).abs() < 1.0e-3 {
            "— the planes tile the loop"
        } else {
            "— THE PLANES DO NOT TILE THE LOOP"
        }
    );

    // Watertightness, read off the document the way `roof.rs` defines it.
    let mut shared = 0;
    let mut crest_edges = 0;
    for first in 0..roof.planes.len() {
        for second in first + 1..roof.planes.len() {
            let common = roof.planes[first]
                .rows
                .iter()
                .filter(|row| roof.planes[second].rows.contains(row))
                .count();
            if common > 0 {
                shared += common;
                continue;
            }
            let first_crest: BTreeSet<usize> = roof.planes[first]
                .rows
                .iter()
                .map(|row| row.crest)
                .collect();
            let second_crest: BTreeSet<usize> = roof.planes[second]
                .rows
                .iter()
                .map(|row| row.crest)
                .collect();
            let both: Vec<usize> = first_crest.intersection(&second_crest).copied().collect();
            if both.len() == 2 {
                crest_edges += 1;
                println!(
                    "   planes {first}/{second}: no shared row, one shared crest edge {both:?}"
                );
            }
        }
    }
    println!("   {shared} shared row(s), {crest_edges} shared crest edge(s)");

    let solid = SolidId::Roof(RoofId(0));
    let degenerate = degenerate_facets(&mesh, &groups, &order);
    println!(
        "   roof solid: faces {} loops {} euler {} manifold {} volume {:.4}, {} zero-area facet(s)",
        mesh.faces_of(solid).len(),
        mesh.solid_boundary_loops(solid),
        mesh.solid_euler_characteristic(solid),
        mesh.solid_is_manifold(solid),
        mesh.solid_signed_volume(solid),
        degenerate.len(),
    );
    for (element, face, area) in &degenerate {
        println!("        face {face} of {element:?} has plan area {area:.7}");
    }
}

/// Prints the measurements that decide whether a published inset may be authored at all.
fn compact(starter: &Starter) {
    let building = derive::starter(starter);
    let roof = &building.skeleton.roofs[0];
    let mesh = match building.build() {
        Ok(mesh) => mesh,
        Err(error) => {
            println!(
                "  {}x{} {:?}: did not expand: {:?}",
                starter.width, starter.depth, starter.roof, error.problems
            );
            return;
        }
    };
    let mut groups: HashMap<Element, Vec<usize>> = HashMap::new();
    for (index, face) in mesh.faces.iter().enumerate() {
        groups.entry(face.element).or_default().push(index);
    }
    let order = ordered(&groups);
    let worst = {
        let mut worst: Option<(String, f32)> = None;
        // **The walk is the ordered one and not the map's own**, because a tie is decided by which
        // element is read first: two planes of a starter are as planar as one another, so the name
        // printed beside the worst departure was whichever the map happened to yield first, and the
        // map yields a different one in every process.
        for element in &order {
            let faces = &groups[element];
            if !matches!(element, Element::Roof(_, RoofSurface::Skin { .. })) {
                continue;
            }
            let vertices: BTreeSet<u32> = faces
                .iter()
                .flat_map(|index| mesh.faces[*index].indices)
                .collect();
            let points: Vec<Vec3> = vertices
                .iter()
                .map(|index| mesh.points[*index as usize])
                .collect();
            let departure = fitted_departure(&points);
            if worst.as_ref().is_none_or(|(_, held)| departure > *held) {
                worst = Some((format!("{element:?}"), departure));
            }
        }
        worst.unwrap_or_else(|| (String::from("none"), 0.0))
    };
    // **The skin's faces are gathered in that same order, and the reason is arithmetic rather than
    // tidiness.** `plan_area_of` adds one term per face, and floating point addition is not
    // associative, so the sum a map-ordered walk produces differs from the ordered one's in the last
    // bits — which is a number the report prints, and a report that changes its own coverage reading
    // between two runs of one binary cannot be a gate on anything.
    let skin: Vec<usize> = order
        .iter()
        .filter(|element| matches!(element, Element::Roof(_, RoofSurface::Skin { .. })))
        .flat_map(|element| groups[element].iter().copied())
        .collect();
    let coverage = (mesh.plan_area_of(&skin).abs() - roof.plan_area().abs()).abs();
    let solid = SolidId::Roof(RoofId(0));
    println!(
        "  {}x{} {:?} seed {}: crest {} at {:?}, worst departure {:.7} m ({}), coverage {:.7} m2, volume {:.4}, {} zero-area facet(s), feet {}",
        starter.width,
        starter.depth,
        starter.roof,
        starter.seed,
        roof.crest.len(),
        roof.crest
            .iter()
            .map(|point| [point.position[0], point.position[1]])
            .collect::<Vec<_>>(),
        worst.1,
        worst.0,
        coverage,
        mesh.solid_signed_volume(solid),
        degenerate_facets(&mesh, &groups, &order).len(),
        feet(roof),
    );
}

/// Returns which corners the two crest ends pair with, read back from the document's rows.
fn feet(roof: &Roof) -> String {
    let (spans, total) = roof.eave_spans();
    let corner = |station: f32| -> usize {
        let target = station.rem_euclid(1.0) * total;
        spans
            .iter()
            .position(|span| (span - target).abs() < 1.0e-4)
            .unwrap_or(usize::MAX)
    };
    let per_plane: Vec<String> = roof
        .planes
        .iter()
        .map(|plane| {
            let first = corner(plane.rows[0].eave.t);
            let last = corner(plane.rows[plane.rows.len() - 1].eave.t);
            format!(
                "crest {}@corner {first}, crest {}@corner {last}",
                plane.rows[0].crest,
                plane.rows[plane.rows.len() - 1].crest
            )
        })
        .collect();
    per_plane.join(" | ")
}

/// Reports that the planarity and coverage measurements can both fail, on meshes this probe builds
/// by hand rather than from a starter.
fn controls() {
    // A quad whose fourth corner is lifted is not planar, and both readings say so.
    let square = [
        [0.0, 0.0, 0.0],
        [2.0, 0.0, 0.0],
        [2.0, 0.0, 2.0],
        [0.0, 0.0, 2.0],
    ];
    let lifted = [
        [0.0, 0.0, 0.0],
        [2.0, 0.0, 0.0],
        [2.0, 0.0, 2.0],
        [0.0, 0.25, 2.0],
    ];
    println!(
        "  a 2 m quad: exact {:.7} m, fitted {:.7} m; one corner lifted 0.25 m: exact {:.7} m, fitted {:.7} m",
        exact_departure(&square),
        fitted_departure(&square),
        exact_departure(&lifted),
        fitted_departure(&lifted),
    );

    // A hip whose planes are built from stations that are not corners: the twisted quad the brief
    // describes, measured rather than described.
    let twisted = twisted_quad();
    println!(
        "  a hip plane whose eave run is a diagonal rather than an edge: exact {:.7} m, fitted {:.7} m, so the checks do fire on a twist",
        exact_departure(&twisted),
        fitted_departure(&twisted)
    );

    // A reversed outline is offset inward, which is what "outward" is measured by.
    let starter = Starter::default();
    let wound = Mass::new(rectangle(starter.width, starter.depth), starter.height);
    let reversed = Mass::new(
        {
            let mut nodes = rectangle(starter.width, starter.depth);
            nodes.reverse();
            nodes
        },
        starter.height,
    );
    let roof_of = |mass: &Mass| {
        derive::hip_roof(
            MassId(0),
            mass,
            starter.overhang,
            starter.rise,
            starter.sink,
            0.25,
        )
        .map(|roof| (roof.plan_area(), roof.eave_length()))
    };
    println!(
        "  a positively wound outline (plan area {:.3}) offsets to an eave of plan area {:.3} and length {:.3}",
        wound.plan_area(),
        roof_of(&wound).expect("a quadrilateral").0,
        roof_of(&wound).expect("a quadrilateral").1
    );
    println!(
        "  the same outline wound the other way (plan area {:.3}) offsets to plan area {:.3} and length {:.3}, so the offset follows the winding and not a guess",
        reversed.plan_area(),
        roof_of(&reversed).expect("a quadrilateral").0,
        roof_of(&reversed).expect("a quadrilateral").1
    );

    validator_step();
}

/// Reports what `validate::smallest_direction` does to the matrix it accumulates its eigenvectors
/// in, reproduced here because the function is private and this crate does not own the file.
///
/// # Invariants
///
/// `turn` is `R X Rᵀ` for the rotation `R` of one sweep. Applied to the identity — which is what the
/// accumulator starts as — that is `R Rᵀ = I`, so the accumulator never moves and the "normal" the
/// planarity check measures with is whichever coordinate axis holds the smallest diagonal entry.
/// The printed identity and the printed departure are both measurements, not readings of the source.
fn validator_step() {
    let mut identity = [[0.0f32; 3]; 3];
    for (index, entry) in identity.iter_mut().enumerate() {
        entry[index] = 1.0;
    }
    let value = [[16.0f32, -6.4, 0.0], [-6.4, 2.56, 0.0], [0.0, 0.0, 80.0]];
    let (cosine, sine) = axis_rotation_of(value, 0, 1);
    let mut accumulator = identity;
    axis_turn(&mut accumulator, 0, 1, cosine, sine);
    let mut corrected = identity;
    turn_columns(&mut corrected, 0, 1, cosine, sine);
    println!(
        "  one of validate's own rotations on the identity accumulator: {accumulator:?}, and on one half of it: {corrected:?}"
    );

    // The covariance of the four corners of a roof plane that is exactly planar: the x-y block is
    // rank one, so its null direction is the plane's normal and its departure is zero.
    let corners = [
        [0.0, 4.7, -2.0],
        [0.0, 4.7, 2.0],
        [4.0, 3.1, -6.0],
        [4.0, 3.1, 6.0],
    ];
    let covariance = covariance_of(&corners);
    let (unit, eigenvalues) = smallest_direction(covariance);
    let departure = corners
        .iter()
        .map(|corner| {
            dot3(
                subtract(
                    *corner,
                    [
                        corners.iter().map(|point| point[0]).sum::<f32>() / 4.0,
                        corners.iter().map(|point| point[1]).sum::<f32>() / 4.0,
                        corners.iter().map(|point| point[2]).sum::<f32>() / 4.0,
                    ],
                ),
                unit,
            )
            .abs()
        })
        .fold(0.0f32, f32::max);
    println!(
        "  the same four corners, exactly planar: fitted normal {unit:?}, eigenvalues {:?}, departure {departure:.7} m",
        eigenvalues
    );
    let (cosine, sine) = axis_rotation_of(covariance, 0, 1);
    let mut value = covariance;
    axis_turn(&mut value, 0, 1, cosine, sine);
    let mut smallest = 0;
    for index in 1..3 {
        if value[index][index] < value[smallest][smallest] {
            smallest = index;
        }
    }
    let axis = match smallest {
        0 => [1.0, 0.0, 0.0],
        1 => [0.0, 1.0, 0.0],
        _ => [0.0, 0.0, 1.0],
    };
    let mean = [
        corners.iter().map(|point| point[0]).sum::<f32>() / 4.0,
        corners.iter().map(|point| point[1]).sum::<f32>() / 4.0,
        corners.iter().map(|point| point[2]).sum::<f32>() / 4.0,
    ];
    let spread = corners
        .iter()
        .map(|corner| dot3(subtract(*corner, mean), axis).abs())
        .fold(0.0f32, f32::max);
    println!(
        "  the diagonal entry that is smallest after that step is {smallest}, so a coordinate axis {axis:?} stands in for the normal and the departure it measures is the spread along it: {spread:.7} m"
    );
}

/// Returns the covariance of a set of points about their mean.
fn covariance_of(points: &[Vec3]) -> [[f32; 3]; 3] {
    let count = points.len() as f32;
    let mean = [
        points.iter().map(|point| point[0]).sum::<f32>() / count,
        points.iter().map(|point| point[1]).sum::<f32>() / count,
        points.iter().map(|point| point[2]).sum::<f32>() / count,
    ];
    let mut covariance = [[0.0f32; 3]; 3];
    for point in points {
        let offset = subtract(*point, mean);
        for row in 0..3 {
            for column in 0..3 {
                covariance[row][column] += offset[row] * offset[column];
            }
        }
    }
    covariance
}

/// Returns the rotation `validate::rotation_of` returns, copied because that module is not this
/// crate's to change.
fn axis_rotation_of(value: [[f32; 3]; 3], row: usize, column: usize) -> (f32, f32) {
    let shifted = (value[row][row] - value[column][column]) / (2.0 * value[row][column]);
    let sign = if shifted >= 0.0 { 1.0 } else { -1.0 };
    let tangent = shifted - sign * (shifted * shifted + 1.0).sqrt();
    (
        1.0 / (tangent * tangent + 1.0).sqrt(),
        tangent / (tangent * tangent + 1.0).sqrt(),
    )
}

/// Rotates the columns `row` and `column` of a matrix together, which is what `validate::turn`
/// computes: `J X Jᵀ`. Copied because that module is not this crate's to change.
fn axis_turn(value: &mut [[f32; 3]; 3], row: usize, column: usize, cosine: f32, sine: f32) {
    for entry in value.iter_mut() {
        let along_row = entry[row];
        let along_column = entry[column];
        entry[row] = cosine * along_row - sine * along_column;
        entry[column] = sine * along_row + cosine * along_column;
    }
    // The row half reads both rows before it writes either, which is the same value the per-index
    // walk produced and is only equal to it because the two rows are never the same one — the caller
    // reads its row and column from `largest_off_diagonal`, which returns row < column.
    let along_row = value[row];
    let along_column = value[column];
    value[row] = array::from_fn(|index| cosine * along_row[index] - sine * along_column[index]);
    value[column] = array::from_fn(|index| sine * along_row[index] + cosine * along_column[index]);
}

/// Returns the quad a plane makes when its eave run joins two corners that are not neighbours: the
/// crest run along the axis, the eave run along the loop's diagonal, so the two are not parallel.
fn twisted_quad() -> [Vec3; 4] {
    [
        [0.0, 2.0, -1.0],
        [0.0, 2.0, 1.0],
        [3.0, 0.0, 6.0],
        [-3.0, 0.0, -6.0],
    ]
}

/// Returns a rectangle centred on the origin, wound with the interior on the walker's right.
fn rectangle(width: f32, depth: f32) -> Vec<PlanNode> {
    let (half_width, half_depth) = (width * 0.5, depth * 0.5);
    [
        [-half_width, -half_depth],
        [half_width, -half_depth],
        [half_width, half_depth],
        [-half_width, half_depth],
    ]
    .into_iter()
    .map(|position| PlanNode::new(position, 0.0))
    .collect()
}

/// Returns the four corners two rows span, in the winding `derive` documents. A plane whose two rows
/// name one crest point is a triangle, and the duplicate is dropped by the departure readings.
fn quad(roof: &Roof, first: &RoofRow, second: &RoofRow) -> [Vec3; 4] {
    [
        roof.crest_point(first.crest)
            .expect("a crest index in range"),
        roof.crest_point(second.crest)
            .expect("a crest index in range"),
        roof.eave_point(second.eave)
            .expect("a station inside the loop"),
        roof.eave_point(first.eave)
            .expect("a station inside the loop"),
    ]
}

/// Returns how far the fourth corner stands off the plane of the first three, or `0` for a triangle
/// (three points are one plane) and `NaN` for a facet with no plane at all.
fn exact_departure(corners: &[Vec3; 4]) -> f32 {
    let distinct = distinct_points(corners);
    if distinct.len() < 3 {
        return f32::NAN;
    }
    if distinct.len() == 3 {
        return 0.0;
    }
    let normal = cross3(
        subtract(distinct[1], distinct[0]),
        subtract(distinct[2], distinct[0]),
    );
    let reach = length3(normal);
    if reach <= f32::EPSILON {
        return f32::NAN;
    }
    (dot3(subtract(distinct[3], distinct[0]), normal) / reach).abs()
}

/// Returns how far the farthest point stands off the group's own least squares plane.
fn fitted_departure(points: &[Vec3]) -> f32 {
    let distinct = distinct_points(points);
    if distinct.len() < 3 {
        return f32::NAN;
    }
    if distinct.len() == 3 {
        return 0.0;
    }
    let count = distinct.len() as f32;
    let mean = [
        distinct.iter().map(|point| point[0]).sum::<f32>() / count,
        distinct.iter().map(|point| point[1]).sum::<f32>() / count,
        distinct.iter().map(|point| point[2]).sum::<f32>() / count,
    ];
    let mut covariance = [[0.0f32; 3]; 3];
    for point in &distinct {
        let offset = subtract(*point, mean);
        for row in 0..3 {
            for column in 0..3 {
                covariance[row][column] += offset[row] * offset[column];
            }
        }
    }
    let (unit, eigenvalues) = smallest_direction(covariance);
    // A group that is a line rather than a plane has nothing for a point to stand off from.
    if eigenvalues[1] <= eigenvalues[2] * 1.0e-6 {
        return f32::NAN;
    }
    distinct
        .iter()
        .map(|point| dot3(subtract(*point, mean), unit).abs())
        .fold(0.0f32, f32::max)
}

/// Returns the points of a set with the coincident ones dropped.
fn distinct_points(points: &[Vec3]) -> Vec<Vec3> {
    let mut distinct: Vec<Vec3> = Vec::new();
    for point in points {
        if !distinct.iter().any(|kept| {
            (kept[0] - point[0]).abs() < 1.0e-6
                && (kept[1] - point[1]).abs() < 1.0e-6
                && (kept[2] - point[2]).abs() < 1.0e-6
        }) {
            distinct.push(*point);
        }
    }
    distinct
}

/// Returns the unit direction of a symmetric matrix's smallest eigenvalue, and its three eigenvalues
/// ascending. Cyclic Jacobi, accumulating the rotations into the columns of an identity.
///
/// The accumulator takes **one** half of each rotation — the column half — because the step is
/// `J A Jᵀ`, whose eigenvector matrix is `Jᵀ`: applying both halves to the accumulator is
/// `J I Jᵀ = I`, which is the reading [`validator_step`] measures.
fn smallest_direction(matrix: [[f32; 3]; 3]) -> ([f32; 3], [f32; 3]) {
    let mut value = matrix;
    let mut vectors = [[0.0f32; 3]; 3];
    for (index, column) in vectors.iter_mut().enumerate() {
        column[index] = 1.0;
    }
    for _ in 0..32 {
        let (row, column) = largest_off_diagonal(value);
        if value[row][column].abs() <= 1.0e-12 * scale_of(value) {
            break;
        }
        let (cosine, sine) = rotation_of(value, row, column);
        turn_columns(&mut value, row, column, cosine, sine);
        turn_rows(&mut value, row, column, cosine, sine);
        turn_columns(&mut vectors, row, column, cosine, sine);
    }
    let eigenvalues = [value[0][0], value[1][1], value[2][2]];
    let mut order = [0usize, 1, 2];
    order.sort_by(|first, second| eigenvalues[*first].total_cmp(&eigenvalues[*second]));
    let smallest = order[0];
    let column = [
        vectors[0][smallest],
        vectors[1][smallest],
        vectors[2][smallest],
    ];
    let reach = length3(column).max(f32::EPSILON);
    (
        [column[0] / reach, column[1] / reach, column[2] / reach],
        order.map(|index| eigenvalues[index]),
    )
}

/// Rotates the columns `row` and `column` of a matrix towards each other.
fn turn_columns(value: &mut [[f32; 3]; 3], row: usize, column: usize, cosine: f32, sine: f32) {
    for entry in value.iter_mut() {
        let along_row = entry[row];
        let along_column = entry[column];
        entry[row] = cosine * along_row - sine * along_column;
        entry[column] = sine * along_row + cosine * along_column;
    }
}

/// Rotates the rows `row` and `column` of a matrix towards each other.
fn turn_rows(value: &mut [[f32; 3]; 3], row: usize, column: usize, cosine: f32, sine: f32) {
    // Both rows are read before either is written, which is the same arithmetic the per-index walk
    // produced, and equal to it because the two rows are never the same one.
    let along_row = value[row];
    let along_column = value[column];
    value[row] = array::from_fn(|index| cosine * along_row[index] - sine * along_column[index]);
    value[column] = array::from_fn(|index| sine * along_row[index] + cosine * along_column[index]);
}

/// Returns the Jacobi rotation that zeroes one off-diagonal entry, as its cosine and sine.
fn rotation_of(value: [[f32; 3]; 3], row: usize, column: usize) -> (f32, f32) {
    let shifted = (value[row][row] - value[column][column]) / (2.0 * value[row][column]);
    let sign = if shifted >= 0.0 { 1.0 } else { -1.0 };
    let tangent = shifted - sign * (shifted * shifted + 1.0).sqrt();
    (
        1.0 / (tangent * tangent + 1.0).sqrt(),
        tangent / (tangent * tangent + 1.0).sqrt(),
    )
}

/// Returns the position of the largest off-diagonal entry of a symmetric matrix.
fn largest_off_diagonal(value: [[f32; 3]; 3]) -> (usize, usize) {
    let mut found = (0usize, 1usize);
    let mut largest = 0.0f32;
    for (row, entries) in value.iter().enumerate() {
        for (column, entry) in entries.iter().enumerate().skip(row + 1) {
            if entry.abs() > largest {
                largest = entry.abs();
                found = (row, column);
            }
        }
    }
    found
}

/// Returns the largest entry of a matrix by magnitude, which is the scale a sweep stops at.
fn scale_of(value: [[f32; 3]; 3]) -> f32 {
    let mut largest = 0.0f32;
    for row in value {
        for entry in row {
            largest = largest.max(entry.abs());
        }
    }
    largest
}

/// Returns a mesh's grouped elements in one fixed order, so that two runs of one binary print the
/// same lines.
///
/// # Invariants
///
/// **The key is the element's own spelling and never the map's own walk.** A `HashMap` yields its
/// keys in an order that differs between two processes running the same binary, so every reading
/// derived from that walk moves: which of two equally non-planar planes is named as the worst, and
/// the order a floating point sum adds its terms in. This is what makes the probe usable as a
/// regression gate at all — a gate whose own text moves between two runs of the tree it gates.
///
/// **Two distinct elements spell differently**, so the order is total and the sort is not left to
/// decide anything by the order it happened to be handed.
fn ordered(groups: &HashMap<Element, Vec<usize>>) -> Vec<Element> {
    let mut order: Vec<Element> = groups.keys().copied().collect();
    order.sort_by_key(|element| format!("{element:?}"));
    order
}

/// Returns every face of an element whose facet has no area, as the element, the face and its plan
/// area. A plane whose two rows name one crest point that stands at the same place as the other
/// leaves such a facet behind, and no closure check sees it.
fn degenerate_facets(
    mesh: &whiting_building::BuildingMesh,
    groups: &HashMap<Element, Vec<usize>>,
    order: &[Element],
) -> Vec<(Element, usize, f32)> {
    order
        .iter()
        .filter(|element| matches!(element, Element::Roof(_, RoofSurface::Skin { .. })))
        .flat_map(|element| {
            groups[element]
                .iter()
                .map(move |face| (*element, *face, mesh.plan_area_of(&[*face]).abs()))
                .filter(|(_, _, area)| *area <= 1.0e-7)
                .collect::<Vec<_>>()
        })
        .collect()
}

/// Returns whether every row names a crest index that exists and a station inside `[0, 1)`.
fn rows_sound(roof: &Roof) -> bool {
    roof.planes
        .iter()
        .flat_map(|plane| plane.rows.iter())
        .all(|row| row.crest < roof.crest.len() && (0.0..1.0).contains(&row.eave.t))
}

/// Returns whether every row's station stands on one of the loop's own nodes, which is what makes a
/// plane's eave run an edge of the loop rather than a diagonal of it.
fn every_station_is_a_corner(roof: &Roof) -> bool {
    let (spans, total) = roof.eave_spans();
    roof.planes
        .iter()
        .flat_map(|plane| plane.rows.iter())
        .all(|row| {
            let target = row.eave.t.rem_euclid(1.0) * total;
            spans.iter().any(|span| (span - target).abs() < 1.0e-4)
        })
}

/// Returns a seed whose starter draws the published inset at `index` for one shape, or the first
/// seed that does when the shape admits none of the others.
fn seed_of(index: usize, width: f32, depth: f32) -> u64 {
    (1u64..256)
        .find(|seed| {
            let roof = derive::starter(&Starter {
                width,
                depth,
                seed: *seed,
                ..Starter::default()
            })
            .skeleton
            .roofs[0]
                .clone();
            inset_read_back(&roof) == Some(index)
        })
        .unwrap_or(1)
}

/// Returns which published inset a roof's ridge reads back as, as an index into [`RIDGE_INSETS`].
///
/// The eave loop's own extent is what is measured: the span is the distance between its long sides
/// and the run the distance between its ends, and both already stand `overhang` outside the mass.
fn inset_read_back(roof: &Roof) -> Option<usize> {
    let mut low = [f32::INFINITY; 2];
    let mut high = [f32::NEG_INFINITY; 2];
    for node in &roof.eaves {
        for axis in 0..2 {
            low[axis] = low[axis].min(node.position[axis]);
            high[axis] = high[axis].max(node.position[axis]);
        }
    }
    let span = (high[0] - low[0]).min(high[1] - low[1]);
    let run = (high[0] - low[0]).max(high[1] - low[1]);
    let first = roof.crest.first()?;
    let last = roof.crest.last()?;
    let spacing = (first.position[0] - last.position[0])
        .abs()
        .max((first.position[1] - last.position[1]).abs());
    let ratio = (run - spacing) / (2.0 * span);
    RIDGE_INSETS
        .iter()
        .position(|value| (value - ratio).abs() < 1.0e-3)
}

/// Prints what the roofs refuse, and what they do instead.
fn refusals() {
    let starter = Starter::default();
    let triangle = Mass::new(
        vec![
            PlanNode::new([-3.0, -4.0], 0.0),
            PlanNode::new([3.0, -4.0], 0.0),
            PlanNode::new([0.0, 5.0], 0.0),
        ],
        starter.height,
    );
    let skewed = Mass::new(
        vec![
            PlanNode::new([-3.0, -4.0], 0.0),
            PlanNode::new([3.0, -3.0], 0.0),
            PlanNode::new([4.0, 5.0], 0.0),
            PlanNode::new([-1.0, 4.0], 0.0),
        ],
        starter.height,
    );
    let some_or_none = |found: bool| if found { "Some" } else { "None" };
    println!(
        "a three-node outline: hip {}, gable {}",
        some_or_none(
            derive::hip_roof(
                MassId(0),
                &triangle,
                starter.overhang,
                starter.rise,
                starter.sink,
                0.25
            )
            .is_some()
        ),
        some_or_none(
            derive::gable_roof(
                MassId(0),
                &triangle,
                starter.overhang,
                starter.rise,
                starter.sink,
                0.25
            )
            .is_some()
        )
    );
    println!(
        "a four-node loop whose long sides are not parallel: hip {}, gable {}",
        some_or_none(
            derive::hip_roof(
                MassId(0),
                &skewed,
                starter.overhang,
                starter.rise,
                starter.sink,
                0.25
            )
            .is_some()
        ),
        some_or_none(
            derive::gable_roof(
                MassId(0),
                &skewed,
                starter.overhang,
                starter.rise,
                starter.sink,
                0.25
            )
            .is_some()
        )
    );
}

/// Prints what the seed decides, and what it is guaranteed to decide.
fn seed_discipline() {
    let starter = Starter::default();
    let first = derive::starter(&starter);
    let second = derive::starter(&starter);
    println!(
        "the same seed twice: buildings identical {}, crest {:?}",
        first == second,
        first.skeleton.roofs[0]
            .crest
            .iter()
            .map(|point| [point.position[0], point.position[1]])
            .collect::<Vec<_>>()
    );
    for other in 2u64..6 {
        let other_building = derive::starter(&Starter {
            seed: other,
            ..starter.clone()
        });
        println!(
            "  seed {other}: crest spacing {:.4}, identical to seed 1's document {}",
            crest_spacing(&other_building.skeleton.roofs[0]),
            other_building == first
        );
    }
}

/// Returns the plan distance between a roof's two crest points.
fn crest_spacing(roof: &Roof) -> f32 {
    let (Some(first), Some(second)) = (roof.crest.first(), roof.crest.last()) else {
        return f32::NAN;
    };
    whiting_building::math::distance(first.position, second.position)
}
