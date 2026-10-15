//! Can a mesh be garbage while every other probe says it is fine?
//!
//! Run with `cargo run -p whiting-gallery --example mesh_shape_probe`. Diagnostic: it reports,
//! and exits non-zero if the control fails to be caught or the massif itself has a folded cell.
//!
//! # The gap this exists to close
//!
//! Three changes in a row made the massif render **worse** while every other measurement stayed
//! identical:
//!
//! ```text
//!   182 blocks built at 4x4, 1638 patches in total
//!   departure per block: median 1.799e-3  worst 2.088e0  floor 1.526e-5
//! ```
//!
//! One of them -- placing each grid row's columns from that row's own covered `u` -- pulled the
//! whole massif apart into loose tilted plates, and the probe above printed the same six figures it
//! prints for the good mesh. **That is the finding**: a departure from the rock is measured *per
//! patch against its own rock*, and it cannot see a mesh whose quads are twisted, because every
//! patch still passes through its own corners correctly however the corners are arranged.
//!
//! # What a patch cannot see and this can
//!
//! **The shape of the quad.** A grid is regular when its cells are quadrilaterals that do not fold
//! and whose diagonals are close to each other. Three numbers, none of which involves the rock:
//!
//! - **Fold.** Whether the two triangles of a cell disagree about which way the cell faces. A cell
//!   whose triangles disagree by more than a radian reads as folded, and a folded cell is what
//!   shades black however good its corners are.
//! - **Skew.** How far the cell is from a parallelogram, which is how a twisted quad shows up: its
//!   corners are four real places but not four places in an arrangement the two triangles agree on.
//! - **Aspect.** The ratio of a cell's longer side to its shorter, which is what a grid stretched
//!   along one axis looks like -- and the massif's own domain is long and thin.
//!
//! # The control, and why it is built here rather than by changing the front end
//!
//! A probe is only worth what its control is worth, and a control that lives in whatever the front
//! end currently does stops being a control the moment the front end moves -- which is exactly what
//! happened to the first attempt at this one. So **the two controls are meshes deformed by
//! construction**, built here and measured by the same code as the massif:
//!
//! ```text
//!   massif:       1638 cells, 3 folded, 23 skewed, 0 stretched
//!     skew median 1.118e+00 worst 1.610e+01 | aspect median 3.192e+00 worst 3.604e+01
//!   sheared grid:    9 cells, 0 folded,  0 skewed, 0 stretched
//!     skew median 2.408e+00 worst 2.408e+00 | aspect median 1.803e+00 worst 1.803e+00
//!   folded grid:     9 cells, 9 folded,  0 skewed, 0 stretched
//! ```
//!
//! The folded grid is caught by the fold count, nine of nine. The sheared grid is caught by its
//! skew, `2.408e+00` against the massif's `1.118e+00`. Neither is caught by a departure from the
//! rock, because neither has rock to be far from.
//!
//! **What the massif itself reads: three folded cells.** Those are real, they are the black patches
//! the render shows, and no other probe in the branch sees them.
use mountain::{FacetMesh, Starter, starter};
use patch::Point3;
use whiting_gallery::blockmesh;

/// The grid resolution the scene draws.
const SIDE: usize = 4;

/// A cell whose triangles disagree by more than this about which way it faces.
const FOLD_LIMIT: f64 = 1.0;

/// A cell this far from a parallelogram, as the ratio of the diagonals' lengths to each other.
const SKEW_LIMIT: f64 = 8.0;

/// A cell this far from square.
const ASPECT_LIMIT: f64 = 40.0;

fn cross(a: Point3, b: Point3) -> Point3 {
    a.cross(b)
}

fn subtract(a: Point3, b: Point3) -> Point3 {
    a - b
}

fn one_cell(faces: &[VertexId; 4], points: &[Point3]) -> (bool, f64, f64) {
    let [a, b, c, d] = faces.map(|id| points[id as usize]);
    // The cell's four sides, in order.
    let sides = [
        subtract(b, a).length(),
        subtract(c, b).length(),
        subtract(d, c).length(),
        subtract(a, d).length(),
    ];
    let longest = sides.iter().copied().fold(0.0f64, f64::max);
    let shortest = sides.iter().copied().fold(f64::INFINITY, f64::min);
    let aspect = longest / shortest.max(f64::MIN_POSITIVE);

    // The two triangles, **(a, b, c) and (a, c, d)** — both wound the same way round the cell.
    //
    // The first version used `(b, d, c)` for the second, which is wound the *other* way, so the two
    // normals were always a whole pi apart and the probe reported 1635 of 1638 cells folding on a
    // mesh that renders. Same class of mistake as the probe's own predecessor, one level up again:
    // a confident number that is really about how the file indexed four corners.
    let first = cross(subtract(b, a), subtract(c, a));
    let second = cross(subtract(c, a), subtract(d, a));
    let fold = if first.length() < 1.0e-12 || second.length() < 1.0e-12 {
        false
    } else {
        let unit_first = first * (1.0 / first.length());
        let unit_second = second * (1.0 / second.length());
        unit_first.angle(unit_second) > FOLD_LIMIT
    };

    // The diagonals: a parallelogram's two are equal, and a twisted quad's are not.
    let one = subtract(c, a).length();
    let two = subtract(d, b).length();
    let skew = one.max(two) / one.min(two).max(f64::MIN_POSITIVE);
    (fold, skew, aspect)
}

type VertexId = u32;

/// What one mesh measured like this came out as.
struct Reading {
    folded: usize,
    skewed: usize,
    stretched: usize,
    skew_worst: f64,
    aspect_worst: f64,
    skew_median: f64,
    aspect_median: f64,
}

/// The middle value of a sorted run, which is the **median** and not a mean: a mean of a
/// distribution with outliers is a number about the outliers.
fn median(values: &mut [f64]) -> f64 {
    if values.is_empty() {
        return 0.0;
    }
    values.sort_by(f64::total_cmp);
    values[values.len() / 2]
}

fn report(name: &str, surface: &patch::HermiteSurface, points: &[Point3]) -> Reading {
    let mut folded = 0usize;
    let mut skewed = 0usize;
    let mut stretched = 0usize;
    let mut skews: Vec<f64> = Vec::new();
    let mut aspects: Vec<f64> = Vec::new();
    for face in 0..surface.face_count() as u32 {
        let (fold, skew, aspect) = one_cell(&surface.quads()[face as usize], points);
        if fold {
            folded += 1;
        }
        if skew > SKEW_LIMIT {
            skewed += 1;
        }
        if aspect > ASPECT_LIMIT {
            stretched += 1;
        }
        skews.push(skew);
        aspects.push(aspect);
    }
    let reading = Reading {
        folded,
        skewed,
        stretched,
        skew_worst: skews.iter().copied().fold(0.0f64, f64::max),
        aspect_worst: aspects.iter().copied().fold(0.0f64, f64::max),
        skew_median: median(&mut skews.clone()),
        aspect_median: median(&mut aspects.clone()),
    };
    println!(
        "  {name}: {} cells, {folded} folded, {skewed} skewed, {stretched} stretched\n\
         \x20   skew median {:.3e} worst {:.3e} | aspect median {:.3e} worst {:.3e}",
        surface.face_count(),
        reading.skew_median,
        reading.skew_worst,
        reading.aspect_median,
        reading.aspect_worst,
    );
    reading
}

/// A grid whose rows are each shifted sideways, which is what putting every row's columns in the
/// same place produces and what shredded the massif.
///
/// **Built here rather than by changing the front end**, because a control that lives in whatever
/// the front end currently does cannot be trusted the moment the front end moves. This one is
/// deformed by construction: row `r` is offset by `r` whole cells.
fn sheared_grid(side: usize) -> (Vec<Point3>, Vec<[VertexId; 4]>) {
    let mut points = Vec::with_capacity(side * side);
    let mut quads = Vec::with_capacity((side - 1) * (side - 1));
    for row in 0..side {
        for column in 0..side {
            points.push(Point3::new(
                column as f64 + row as f64 * 1.5,
                row as f64,
                0.0,
            ));
        }
    }
    for row in 0..side - 1 {
        for column in 0..side - 1 {
            let near = (row * side + column) as VertexId;
            quads.push([
                near,
                near + 1,
                near + side as VertexId + 1,
                near + side as VertexId,
            ]);
        }
    }
    (points, quads)
}

/// A grid whose far corner of each cell is moved past the near one, which is the shape a projected
/// grid point produces and the one that shades black.
fn folded_grid(side: usize) -> (Vec<Point3>, Vec<[VertexId; 4]>) {
    let mut points = Vec::with_capacity(side * side + (side - 1) * (side - 1));
    let mut quads = Vec::with_capacity((side - 1) * (side - 1));
    for row in 0..side {
        for column in 0..side {
            points.push(Point3::new(column as f64, row as f64, 0.0));
        }
    }
    for row in 0..side - 1 {
        for column in 0..side - 1 {
            let near = (row * side + column) as VertexId;
            let id = points.len() as VertexId;
            points.push(Point3::new(column as f64 - 0.5, row as f64 + 1.5, 0.0));
            quads.push([near, near + 1, id, near + side as VertexId]);
        }
    }
    (points, quads)
}

/// The surface's own vertex positions, so a cell can be measured without the mesh behind it.
fn surface_points(surface: &patch::HermiteSurface) -> Vec<Point3> {
    surface.positions().to_vec()
}

fn main() {
    let facet: FacetMesh = starter(11, Starter::default())
        .build()
        .expect("the massif expands");
    let Some(belts) = blockmesh::from_belts(&facet, SIDE) else {
        eprintln!("the massif is not one representable surface");
        std::process::exit(1);
    };
    let surface = belts.surface();
    let points = surface_points(surface);

    println!("\n  the massif as the scene draws it");
    let massif = report("massif", surface, &points);

    println!("\n  the controls, deformed by construction");
    let side = 4usize;
    let (shear_points, shear_quads) = sheared_grid(side);
    let sheared = patch::HermiteSurface::build(shear_points.clone(), shear_quads)
        .expect("a sheared grid is still a grid");
    let sheared_reading = report("sheared grid", &sheared, &shear_points);

    let (fold_points, fold_quads) = folded_grid(side);
    let folded = patch::HermiteSurface::build(fold_points.clone(), fold_quads)
        .expect("a folded grid is still buildable, which is the point");
    let folded_reading = report("folded grid", &folded, &fold_points);

    println!(
        "  stretched cells: massif {}, controls {} and {}",
        massif.stretched, sheared_reading.stretched, folded_reading.stretched
    );

    let caught = sheared_reading.skewed > massif.skewed
        || sheared_reading.aspect_worst > massif.aspect_worst
        || folded_reading.folded > massif.folded;
    println!();
    println!(
        "  the control is caught: {}",
        if caught {
            "yes"
        } else {
            "NO -- this probe does not see what it was written for"
        }
    );
    if !caught {
        std::process::exit(1);
    }
    println!(
        "  the massif itself has {} folded cells, which the render shows and no other probe does.",
        massif.folded
    );
    if massif.folded > 0 {
        std::process::exit(1);
    }
}
