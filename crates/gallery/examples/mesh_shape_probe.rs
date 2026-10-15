//! # NOT YET VALIDATED, and that is the first thing to say about this file
//!
//! The control it needs was attempted and **failed**. Placing each grid row's columns from that
//! row's own covered `u` shredded the massif into loose plates while every departure figure stayed
//! identical -- which is what this file exists to catch. But the attempt to reproduce that here
//! changed only the row's `v`, not its columns, so it did not reproduce the defect, and the probe
//! reported the same numbers on it as on the good mesh. **That agreement is not evidence: the
//! control was not the control.**
//!
//! So: the three folded cells below are a real measurement of the mesh as it stands, and the
//! claim that this probe would catch a mesh that every other probe passes is **not yet shown**.
//! The control has to be a change that really deforms the mesh, and building one is the next
//! thing this file needs.
//! Can a mesh be garbage while every other probe says it is fine?
//!
//! Run with `cargo run -p whiting-gallery --example mesh_shape_probe`. Diagnostic: it reports, and
//! exits non-zero if it finds a mesh that is deformed.
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
//! One of them — placing each grid row's columns from that row's own covered `u` — pulled the whole
//! massif apart into loose tilted plates, and the probe above printed the same six figures it
//! prints for the good mesh. **That is the finding**: a departure from the rock is measured *per
//! patch against its own rock*, and it cannot see a mesh whose quads are twisted, because every
//! patch still passes through its own corners correctly however the corners are arranged.
//!
//! # What a patch cannot see and this can
//!
//! **The shape of the quad.** A grid is regular when its cells are quadrilaterals that do not fold
//! and whose diagonals are close to each other. Three numbers, all independent of the rock:
//!
//! - **Fold.** Whether the two triangles of a cell disagree about which way the cell faces. A cell
//!   whose `du` and `dv` run the other way round reads as folded, and a folded cell is what shades
//!   black however good its corners are.
//! - **Skew.** How far the cell's two triangles are from agreeing, which is how a twisted quad shows
//!   up: its corners are four real places but not four places in a plane-ish arrangement.
//! - **Aspect.** The ratio of a cell's longer side to its shorter, which is what a grid that has
//!   been stretched along one axis looks like — and the massif's own domain is long and thin.
//!
//! **None of the three involves the rock**, which is the point: a probe that compares against the
//! rock cannot fail when the rock is nowhere near the defect.

use mountain::{FacetMesh, Starter, starter};
use patch::Point3;
use whiting_gallery::blockmesh::{self, BeltSurfaces};

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

/// Every cell of the surface, as the four vertex ids the topology holds.
///
/// **Read from the surface's own quads** rather than from a grid's arithmetic, because that is the
/// whole of the point: a position in a grid is not a face id once any cell has been dropped, which
/// is how `block_surface_probe` came to read a wrong patch and report a good mesh.
fn cells(surface: &patch::HermiteSurface) -> Vec<[VertexId; 4]> {
    (0..surface.face_count() as u32)
        .map(|face| surface.quads()[face as usize])
        .collect()
}

fn median(values: &mut [f64]) -> f64 {
    if values.is_empty() {
        return 0.0;
    }
    values.sort_by(f64::total_cmp);
    values[values.len() / 2]
}

fn report(name: &str, belts: &BeltSurfaces, points: &[Point3]) {
    let surface = belts.surface();
    let all = cells(surface);
    let mut folded = 0usize;
    let mut skewed = 0usize;
    let mut stretched = 0usize;
    let mut skews: Vec<f64> = Vec::with_capacity(all.len());
    let mut aspects: Vec<f64> = Vec::with_capacity(all.len());
    for cell in &all {
        let (fold, skew, aspect) = one_cell(cell, points);
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
    println!(
        "  {name}: {} cells, {folded} folded, {skewed} skewed, {stretched} stretched\n\
         \x20   skew median {:.3e} worst {:.3e} | aspect median {:.3e} worst {:.3e}",
        all.len(),
        median(&mut skews),
        skews.iter().copied().fold(0.0f64, f64::max),
        median(&mut aspects),
        aspects.iter().copied().fold(0.0f64, f64::max),
    );
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
    let points = surface_points(belts.surface());

    println!("\n  the massif as the scene draws it");
    report("massif", &belts, &points);

    let surface = belts.surface();
    let folded = (0..surface.face_count() as u32)
        .filter(|face| {
            let cell = surface.quads()[*face as usize];
            let [a, b, c, d] = cell.map(|id| points[id as usize]);
            let first = cross(subtract(b, a), subtract(c, a));
            let second = cross(subtract(c, a), subtract(d, a));
            if first.length() < 1.0e-12 || second.length() < 1.0e-12 {
                return false;
            }
            (first * (1.0 / first.length())).angle(second * (1.0 / second.length())) > FOLD_LIMIT
        })
        .count();

    println!();
    if folded > 0 {
        println!("FOUND  {folded} cells fold, and no departure-from-the-rock measurement sees it");
        std::process::exit(1);
    }
    println!("  no cell folds. A mesh that folds shades black and still passes every other probe,");
    println!("  which is what this file is here to catch.");
}
