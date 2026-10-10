//! Probes the rasteriser's half: turning sampled points and indices into triangles a pipeline draws.
//!
//! Run with `cargo run --release -p whiting-patch --example raster_probe`. An example and not a
//! gate. **It exits non-zero on any failure.**
//!
//! `--release` because the finest resolution here puts about two million triangles on the massif and
//! debug arithmetic on that is measured in minutes. `f64` operations are IEEE in both profiles, so
//! the numbers are the numbers a debug run would print, only sooner.
//!
//! `tessellation_probe` produced points and indices and stopped there. This one asks the four
//! questions a draw call needs answered before it can be made, each of them a number rather than an
//! opinion.
//!
//! 1. **Does every triangle face the same way?** Winding is meant to be derived from the grid rather
//!    than chosen, so every triangle's geometric normal is compared against the patch's own analytic
//!    normal at the point that triangle covers. The same triangles are then measured again with their
//!    winding reversed, because a count of flips is worth nothing until the count is shown to be able
//!    to come out non-zero. The result is also broken down by face, because "some triangles point the
//!    wrong way" and "some faces are wound the wrong way" are different faults with different fixes.
//! 2. **Is the triangle soup actually a mesh?** Sampling a shared boundary once was for storage and
//!    for not asking the rasteriser to reconcile two copies. It is only worth anything if the result
//!    has no crack in it: every edge must be used by one triangle, which is the surface's own rim, or
//!    two, which is interior — never more. The rim's size is checked against what the quad mesh
//!    implies, which is an identity in integers rather than a tolerance.
//! 3. **Is any triangle degenerate?** A zero-area triangle draws as nothing at all, and a mesh that
//!    came out of a subdivision rule has no reason in advance to be free of them.
//! 4. **What does flat shading cost?** A rasteriser may take its normal off the triangle or ask the
//!    patch, and the patch can answer analytically while the triangle cannot. The angle between the
//!    two, as a function of sampling density, is the number that decides it — so it is measured
//!    across resolutions rather than at one.
//!
//! Measurements 1, 2 and 3 are null results, so each is paired with a known-bad input it must
//! actually reject. Measurement 4 is the one that pays for itself: the landscape scene deliberately
//! shades its facets flat, because averaging across shared vertices would hide exactly what that
//! scene exists to show. A patch surface is the other case, and the two want different answers.

mod support;

use whiting_gallery::patchmesh;
use whiting_mountain::{FacetMesh, Starter, starter};
use whiting_patch::{Level, PatchMesh, Point3, RectNet};

use support::{Incidence, quad_edges, triangle_edges};

/// Samples per edge for the structural measurements, so each face becomes four by four quads.
///
/// The same value `tessellation_probe` uses, so the two probes' numbers are about the same mesh.
const SIDE: usize = 5;

/// Resolutions to measure faceting error at, so the answer is a curve rather than one point.
///
/// Coarse enough to afford, fine enough to show the fall: a faceting error falls with the grid, and
/// four decades of density is what it takes to see the rate rather than just the direction.
const RESOLUTIONS: [usize; 5] = [2, 3, 5, 9, 17];

/// How many measurements have been reported, so the closing line cannot drift out of step with them.
///
/// Counted rather than written out: a hardcoded count went stale once already, and printed a
/// confident "all 4" over five measurements.
fn report(name: &str, detail: &str) {
    MEASUREMENTS.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    println!("\n== {name} ==\n{detail}");
}

static MEASUREMENTS: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

/// The authored massif, at the degree it arrives in.
///
/// No elevation here. Measurement 4 asks how far a flat triangle sits from the surface under it, and
/// a bilinear patch has all the curvature that question needs; raising the degree would move the
/// answer without making it more true.
fn real_massif() -> PatchMesh {
    let massif = starter(11, Starter::default());
    let facet: FacetMesh = match massif.build() {
        Ok(mesh) => mesh,
        Err(problem) => {
            eprintln!("the sample massif did not expand: {problem:?}");
            std::process::exit(1);
        }
    };
    match patchmesh::from_facet(&facet) {
        Ok(model) => model,
        Err(problem) => {
            eprintln!("the sample massif is not convertible: {problem}");
            std::process::exit(1);
        }
    }
}

/// The geometric normal of a triangle, by the right-hand rule over its own winding.
///
/// Unnormalised on purpose: only its direction is read, and a positive scale factor cannot change a
/// sign.
fn triangle_normal(points: &[Point3], triangle: [usize; 3]) -> Point3 {
    let [a, b, c] = triangle.map(|index| points[index]);
    (b - a).cross(c - a)
}

/// Which face and parameter the `n`th triangle covers.
///
/// `triangles()` emits two per cell in `(face, i, j)` order, so both the cell and which half of it
/// this is fall out of the index by division — no walk kept in step with it, and no state that can
/// fall out of step.
///
/// The parameter is the triangle's own centroid rather than the centre of its cell. The two halves of
/// a cell have centroids a sixth of a cell from the cell's centre, which is the same order as the
/// deviation being measured, so a cell-centre reference would answer a slightly different question at
/// roughly half the magnitude.
fn sample_of(index: usize, last: usize) -> (usize, f64, f64) {
    let per_face = last * last * 2;
    let face = index / per_face;
    let within = index % per_face;
    let cell = within / 2;
    let (di, dj) = if within % 2 == 0 {
        (2.0 / 3.0, 1.0 / 3.0)
    } else {
        (1.0 / 3.0, 2.0 / 3.0)
    };
    let i = cell / last;
    let j = cell % last;
    (
        face,
        (i as f64 + di) / last as f64,
        (j as f64 + dj) / last as f64,
    )
}

/// The patch's own normal at `(u, v)`, read from the **composed** surface.
///
/// Never per level. A normal is not a displacement, and the normal of `base + offset` is neither the
/// sum of theirs nor any blend of them, so a hierarchy that added them would be reporting a direction
/// that belongs to no surface.
fn patch_normal(level: &Level, face: usize, u: f64, v: f64) -> Point3 {
    level
        .face(face)
        .expect("a triangle below the face count names a face that exists")
        .normal(u, v)
}

/// The angle a rasteriser taking its normal off the triangle would shade off, worst and average.
///
/// Both, because they answer different questions: the peak is the worst single pixel the surface will
/// ever show, and the average is what most of it looks like. One without the other describes no
/// particular triangle.
fn shading_error(mesh: &PatchMesh, composed: &Level, side: usize) -> (f64, f64, usize) {
    let sampled = mesh.tessellate(side);
    let points = sampled.points();
    let triangles = sampled.triangles();
    let last = side - 1;
    let mut peak = 0.0f64;
    let mut total = 0.0f64;
    for (index, triangle) in triangles.iter().enumerate() {
        let (face, u, v) = sample_of(index, last);
        let angle = triangle_normal(points, *triangle).angle(patch_normal(composed, face, u, v));
        peak = peak.max(angle);
        total += angle;
    }
    (peak, total / triangles.len() as f64, triangles.len())
}

fn main() {
    let mut failures: Vec<String> = Vec::new();
    let mesh = real_massif();
    let composed = mesh.composed();

    // 0 -- is the reference itself well defined?
    //
    // Every triangle below is compared with "the patch's own analytic normal", and a comparison
    // against a number that is not there is not a comparison. A flat patch's normal is its plane's
    // normal *everywhere*, and **the boundary is everywhere too**: it is the edge a crease runs
    // along, which is the one place a caller asks. This is the control on that.
    let flat = RectNet::from_corners(
        Point3::new(0.0, 0.0, 0.0),
        Point3::new(1.0, 0.0, 0.0),
        Point3::new(0.0, 0.0, -1.0),
        Point3::new(1.0, 0.0, -1.0),
    );
    let mut missing: Vec<String> = Vec::new();
    let mut worst = 0.0f64;
    for i in 0..=6 {
        for j in 0..=6 {
            let (u, v) = (i as f64 / 6.0, j as f64 / 6.0);
            let normal = flat.normal(u, v);
            if normal.length() == 0.0 {
                missing.push(format!("({i},{j})"));
            } else {
                worst = worst.max(normal.angle(Point3::new(0.0, 1.0, 0.0)));
            }
        }
    }
    let zero_normals = missing.len();
    if zero_normals != 0 {
        failures.push(format!(
            "0  a flat patch's normal is its plane's normal everywhere, boundary included ({zero_normals} \
             of 49 samples had no normal at all)"
        ));
    }
    if worst > support::ANGLE_FLOOR {
        failures.push(format!(
            "0  that normal is the plane's own (worst {worst:.3e} rad from straight up)"
        ));
    }
    report(
        "0  the reference the other measurements are taken against",
        &format!(
            "samples on a flat patch          49\n\
             samples with no normal at all    {zero_normals} at {missing:?}\n\
             worst angle from the plane's own normal  {worst:.3e} rad\n\
             \n\
             A comparison against a normal that is not there is not a comparison, and `f64::max` and\n\
             `f64::min` both return their other operand when one is NaN — so a normal that came back\n\
             zero would leave no trace in a worst-case accumulator and this whole probe would report\n\
             clean. The boundary is where it matters most: it is the line a crease runs along, and it\n\
             is where a caller asks."
        ),
    );

    let tessellation = mesh.tessellate(SIDE);
    let triangles = tessellation.triangles();
    let points = tessellation.points();
    let last = SIDE - 1;

    // Every measurement below is a count, and a count over nothing is zero however wrong the code
    // under it is. So the emptiness is checked first rather than being reported as four passes.
    if triangles.is_empty() {
        eprintln!("the tessellation produced no triangles, so none of this can be measured");
        std::process::exit(1);
    }

    // 1 -- winding. Every triangle's geometric normal must point the same way as the patch's.
    //
    // The second count is the control: the same triangles with their winding reversed must *all* come
    // out facing away. A detector that reports zero for a reversed mesh is not measuring winding.
    //
    // The breakdown is the localisation. A face that is entirely one way is a mesh whose faces are
    // wound inconsistently; a face with some of each is a patch whose surface folds through itself,
    // which is a different fault in a different place and would need a different fix.
    let mut facing_away = 0usize;
    let mut reversed_facing_away = 0usize;
    let mut away_per_face = vec![0usize; tessellation.face_count()];
    let mut all_per_face = vec![0usize; tessellation.face_count()];
    for (index, triangle) in triangles.iter().enumerate() {
        let (face, u, v) = sample_of(index, last);
        let reference = patch_normal(&composed, face, u, v);
        let geometric = triangle_normal(points, *triangle);
        all_per_face[face] += 1;
        if geometric.dot(reference) <= 0.0 {
            facing_away += 1;
            away_per_face[face] += 1;
        }
        if (-geometric).dot(reference) <= 0.0 {
            reversed_facing_away += 1;
        }
    }
    let (mut wholly_away, mut wholly_toward, mut folded) = (0usize, 0usize, 0usize);
    for (away, all) in away_per_face.iter().zip(&all_per_face) {
        if *away == *all {
            wholly_away += 1;
        } else if *away == 0 {
            wholly_toward += 1;
        } else {
            folded += 1;
        }
    }
    if facing_away != 0 {
        failures.push(format!(
            "1  every triangle is wound to agree with its patch ({facing_away} of {} face away, \
             {wholly_away} of {} faces wholly so)",
            triangles.len(),
            all_per_face.len()
        ));
    }
    if reversed_facing_away != triangles.len() {
        failures.push(format!(
            "1  the same check rejects a reversed mesh (it caught {reversed_facing_away} of {}, \
             expected {})",
            triangles.len(),
            triangles.len()
        ));
    }
    if folded != 0 {
        failures.push(format!(
            "1  a face is uniformly wound, so disagreement means inconsistent faces and not a \
             folding patch ({folded} faces have some triangles each way)"
        ));
    }
    report(
        "1  winding comes from the grid, and the check has teeth",
        &format!(
            "triangles                          {}\n\
             facing away from their patch       {facing_away}\n\
             facing away once winding reversed  {reversed_facing_away}\n\
             faces wholly facing away           {wholly_away}\n\
             faces wholly facing toward         {wholly_toward}\n\
             faces with some of each            {folded}\n\
             \n\
             Each triangle is compared with the patch's own analytic normal at the point that triangle\n\
             covers. The second line is the control: the same triangles with every winding reversed, on\n\
             which the count has to be all of them. A check that cannot come out non-zero is not a\n\
             check, and a flipped triangle is the fault here that does not announce itself — the mesh\n\
             draws, and one face of it is lit from the wrong side.\n\
             \n\
             The last three lines are the localisation. A face is either wholly one way or wholly the\n\
             other, so the disagreement is in how the *faces* are wound rather than in the surface\n\
             inside a face.",
            triangles.len()
        ),
    );

    // 2 -- is the soup a mesh. An edge used twice is interior, once is the rim, three or more is a
    // fold. A crack would show up here as an interior edge used once, so the rim's size is not just
    // reported but held to what the quad mesh implies.
    let source = Incidence::of(&quad_edges(mesh.quads()));
    if source.non_manifold != 0 {
        failures.push(format!(
            "2  the quad mesh is manifold, so the rim identity below is well defined ({} edges \
             are not)",
            source.non_manifold
        ));
    }
    let soup = Incidence::of(&triangle_edges(&triangles));
    if soup.non_manifold != 0 {
        failures.push(format!(
            "2  no edge of the soup carries three or more triangles ({} do)",
            soup.non_manifold
        ));
    }
    // Each quad rim edge carries `side` samples, hence `side - 1` segments, and two faces meeting on
    // it share those indices rather than producing two rims.
    let expected_rim = source.boundary * last;
    let soup_rim = soup.boundary;
    if soup_rim != expected_rim {
        failures.push(format!(
            "2  the rim is the quad rim subdivided ({soup_rim} edges, expected {expected_rim} = {} \
             quad edges x {last})",
            source.boundary
        ));
    }
    let unused = tessellation
        .uses_per_point()
        .iter()
        .filter(|uses| **uses == 0)
        .count();
    if unused != 0 {
        failures.push(format!(
            "2  every sampled point is used by some triangle ({unused} are used by none)"
        ));
    }
    // A surface with no holes and no handles: V - E + F is 1 for the massif's heightfield disc.
    let euler = points.len() as isize - soup.total() as isize + triangles.len() as isize;
    report(
        "2  the soup is a mesh, with no crack in it",
        &format!(
            "points                          {}\n\
             triangles                       {}\n\
             interior edges (two triangles)  {}\n\
             rim edges (one triangle)        {soup_rim}, expected {expected_rim}\n\
             edges over two triangles       {}\n\
             points no triangle uses        {unused}\n\
             Euler characteristic            {euler}\n\
             \n\
             Sharing a boundary once is worth nothing if the result has a hole in it, and a crack is\n\
             an interior edge used once. The rim is therefore held against an identity rather than a\n\
             tolerance: the {} quad rim edges carry {} samples each, and two faces meeting on one edge\n\
             share those indices instead of producing two rims.",
            points.len(),
            triangles.len(),
            soup.interior,
            soup.non_manifold,
            source.boundary,
            SIDE,
        ),
    );

    // 3 -- degeneracy. A zero-area triangle draws as nothing, and the control is a triangle the check
    // must reject: three positions that are one position.
    let mut degenerate = 0usize;
    let mut shortest_edge = f64::INFINITY;
    for triangle in &triangles {
        let [a, b, c] = triangle.map(|index| points[index]);
        if (b - a).cross(c - a).length() == 0.0 {
            degenerate += 1;
        }
        shortest_edge = shortest_edge
            .min(a.distance(b))
            .min(b.distance(c))
            .min(c.distance(a));
    }
    let collapsed = [0usize, 0, 0];
    let collapsed_caught = triangle_normal(points, collapsed).length() == 0.0;
    if degenerate != 0 {
        failures.push(format!(
            "3  no triangle has zero area ({degenerate} of {} do)",
            triangles.len()
        ));
    }
    if !collapsed_caught {
        failures
            .push("3  the degeneracy check rejects a collapsed triangle (it did not)".to_owned());
    }
    report(
        "3  no triangle is degenerate, and the check catches one",
        &format!(
            "degenerate                     {degenerate} of {}\n\
             shortest edge in any triangle  {shortest_edge:.3e}\n\
             a collapsed triangle is caught {collapsed_caught}\n\
             \n\
             The control is three corners that are one corner. Without it, a zero count would only say\n\
             the check never fired, which is indistinguishable from a check that cannot fire.",
            triangles.len()
        ),
    );

    // 4 -- what flat shading costs, as a curve.
    let mut curve = Vec::new();
    for resolution in RESOLUTIONS {
        curve.push((resolution, shading_error(&mesh, &composed, resolution)));
    }

    // The **mean** answers the question: a rasteriser taking its normal off the triangle is shading
    // the surface it sampled, or it is not. The peak is reported beside it and is not asserted on,
    // because it is not one thing — see the report.
    let coarsest_mean = curve[0].1.1;
    let finest_mean = curve[curve.len() - 1].1.1;
    let worst_mean = curve.iter().map(|entry| entry.1.1).fold(0.0f64, f64::max);
    if worst_mean > support::ANGLE_FLOOR {
        failures.push(format!(
            "4  the patch's own normal is the surface, so the typical deviation between it and a \
             triangle's sits on the floor ({worst_mean:.3e} rad)"
        ));
    }
    if coarsest_mean > finest_mean {
        failures.push(format!(
            "4  the typical deviation does not grow with resolution (coarse {coarsest_mean:.3e}, \
             fine {finest_mean:.3e})"
        ));
    }
    let mut detail = String::new();
    for (resolution, (peak, mean, count)) in &curve {
        detail.push_str(&format!(
            "side {resolution:>3}  {peak:.3e} rad peak  {mean:.3e} mean  over {count} triangles\n"
        ));
    }
    report(
        "4  what flat shading costs, as a curve",
        &format!(
            "angle between a triangle's own normal and its patch's\n\
             {detail}\n\
             The **mean** is the number that answers the question, and it sits on the floor at every\n\
             resolution: a rasteriser taking its normal off the triangle is shading the surface it\n\
             sampled, to the precision the arithmetic has. One that can ask the patch pays nothing,\n\
             and `Face::normal` is how it asks. The landscape scene wants the flat end regardless —\n\
             averaging across its shared vertices would hide the facets it exists to produce.\n\
             \n\
             The **peak** rises with resolution, and that is not the surface getting worse. A cell's own\n\
             normal is a cross product of two shorter and shorter vectors, and past a point its\n\
             direction is set by how the positions round rather than by what they describe: the\n\
             mesh's `f32` spacing at this scale is `1.526e-5`, and a cell edge near side 17 is three\n\
             orders of magnitude below it. So the peak is a statement about the input's precision at\n\
             the finest resolutions, which is why it is printed beside the mean and not asserted on."
        ),
    );
    if failures.is_empty() {
        println!(
            "\nall {} measurements passed",
            MEASUREMENTS.load(std::sync::atomic::Ordering::Relaxed)
        );
    } else {
        println!("\nFAILED:");
        for failure in &failures {
            println!("  - {failure}");
        }
        std::process::exit(1);
    }
}
