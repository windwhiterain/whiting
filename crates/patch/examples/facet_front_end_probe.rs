//! Runs a **real** `FacetMesh` through the front-end and reports what happens to it.
//!
//! Run with `cargo run -p whiting-patch --example facet_front_end_probe`. An example and not a gate.
//! **It exits non-zero on any failure.**
//!
//! Every other probe in this crate runs on one regular f64 grid. That grid was chosen because it is
//! the *easy* case: manifold, consistently wound, no slivers, no boundary until you look, and twice
//! the precision of what terrain actually arrives as. This one runs `mountain`'s own authored massif
//! instead, because the questions worth asking are the ones a regular grid cannot raise:
//!
//! 1. **What is the mesh's shape?** Boundary edges, interior edges, non-manifold edges, degenerate
//!    triangles, slivers. The counts decide whether anything downstream is even meaningful.
//! 2. **Does Doo–Sabin keep it manifold?** The rule needs no boundary case, but an edge shared by
//!    three triangles gets one shared midpoint and a non-manifold quad mesh, and `enforce` shares an
//!    edge only when exactly two faces meet on it. A non-manifold edge is therefore a silent crack,
//!    not an error, and that is the kind of thing worth knowing before it happens.
//! 3. **Does level 0 reproduce the mesh**, corner for corner? Every point arrived as `f32`, and the
//!    widening to `f64` is exact for every `f32`, so this is a promise about representation rather
//!    than about arithmetic — and worth checking rather than assuming.
//! 4. **What precision is there to edit with?** The surface can never be more accurate than its
//!    input, so the `f32` spacing at this mesh's scale bounds every edit anyone makes to it. This is
//!    the number that decides how finely the model can be shaped at all.
//! 5. **Do hard edges share, and does elevation hold the surface** on a real mesh rather than the
//!    sample one?
//!
//! Measurement 2 is the load-bearing one. The rest are numbers; a non-manifold edge is a hole in the
//! representation that nothing in the crate would report.

mod support;

use whiting_mountain::{FacetMesh, Starter, starter};
use whiting_patch::subdivide::{QuadMesh, subdivide};
use whiting_patch::{PatchMesh, Point3, VertexId};

use support::{Incidence, quad_edges, split_over_shared_edges, triangle_edges};

/// The largest patch that may deviate from the mesh it reproduces, and the only one this crate
/// promises is exact.
const EXACT_REPRODUCTION: f64 = 0.0;

/// Elevation on an all-hard mesh does not move the surface; that is rounding, not a budget.
const ELEVATION_ROUNDING: f64 = 1.0e-12;

fn report(name: &str, detail: &str) {
    println!("\n== {name} ==\n{detail}");
}

/// The massif the front-end has to cope with: seven ridges, thirteen nodes each, authored rather
/// than sampled.
fn sample_massif() -> FacetMesh {
    let massif = starter(11, Starter::default());
    match massif.build() {
        Ok(mesh) => mesh,
        Err(problem) => {
            eprintln!("the sample massif did not expand: {problem:?}");
            std::process::exit(1);
        }
    }
}

/// The distance between two adjacent `f32`s at this magnitude.
///
/// This is the floor under any edit anyone makes to a surface that arrived as `f32`, and no `f64`
/// arithmetic downstream moves it. Note what it is *not*: the distance from a widened point back to
/// its own `f32` rounding, which is zero by construction, because the widened point already **is**
/// an `f32`.
fn f32_spacing_at(value: f64) -> f64 {
    let here = value as f32;
    let next = f32::from_bits(here.to_bits() + 1);
    (next as f64 - here as f64).abs()
}

fn main() {
    let mut failures: Vec<String> = Vec::new();
    let facet = sample_massif();

    // 1 -- the input, widened. f32 to f64 is exact for every value, which is what makes
    // measurement 3 an exactness question rather than an error-budget one.
    let positions: Vec<Point3> = facet
        .points
        .iter()
        .map(|p| Point3::new(p[0] as f64, p[1] as f64, p[2] as f64))
        .collect();
    let triangles: Vec<[VertexId; 3]> = facet.faces.iter().map(|face| face.indices).collect();

    let before = Incidence::of(&triangle_edges(&triangles));
    let degenerate = count_degenerate(&positions, &triangles);
    let shortest = shortest_edge(&positions, &triangles);
    let high = bounds(&positions)[1];
    let largest = largest_component(high);

    report(
        "1  what the front-end is actually handed",
        &format!(
            "points                     {}\n\
             triangles                  {}\n\
             edges                      {} distinct\n\
             \x20  on the boundary        {}\n\
             \x20  interior               {}\n\
             \x20  non-manifold           {}\n\
             worst edge incidence       {}\n\
             degenerate triangles       {degenerate}\n\
             shortest edge              {shortest:.3e}\n\
             largest coordinate         {largest:.3e}",
            positions.len(),
            triangles.len(),
            before.boundary + before.interior + before.non_manifold,
            before.boundary,
            before.interior,
            before.non_manifold,
            before.worst,
        ),
    );

    if before.non_manifold != 0 {
        failures.push(format!(
            "1  the sample has {} non-manifold edges, so it is not the easy case",
            before.non_manifold
        ));
    }
    if before.boundary == 0 {
        failures
            .push("1  the sample has no boundary, so the boundary case is untested".to_string());
    }

    // 2 -- subdivision.
    let derived: QuadMesh = subdivide(&positions, &triangles);
    let after = Incidence::of(&quad_edges(&derived.quads));

    if after.non_manifold != 0 {
        failures.push(format!(
            "2  Doo-Sabin turned {} non-manifold edges into a non-manifold quad mesh",
            after.non_manifold
        ));
    }
    report(
        "2  Doo-Sabin on it",
        &format!(
            "points                     {} from {}\n\
             quads                      {} from {} triangles\n\
             edges                      {} distinct\n\
             \x20  on the boundary        {}\n\
             \x20  interior               {}\n\
             \x20  non-manifold           {}\n\
             worst edge incidence       {}",
            derived.positions.len(),
            positions.len(),
            derived.quads.len(),
            triangles.len(),
            after.boundary + after.interior + after.non_manifold,
            after.boundary,
            after.interior,
            after.non_manifold,
            after.worst,
        ),
    );

    // 3 -- the model, and the precision it has.
    let mut mesh = PatchMesh::build(derived.positions.clone(), derived.quads.clone());
    let corner_error = worst_corner_error(&mesh);

    if corner_error != EXACT_REPRODUCTION {
        failures.push(format!(
            "3  level 0 reproduces the mesh corner for corner (worst {corner_error:.3e})"
        ));
    }

    // The spacing of an f32 at the largest coordinate this mesh reaches. Every later edit is
    // quantised by it, so it is printed next to the corner error to say what "exact" is exact to.
    let spacing = f32_spacing_at(high.x);
    report(
        "3  level 0 reproduces the mesh, exactly",
        &format!(
            "worst corner error          {corner_error:.3e}, against a promise of {EXACT_REPRODUCTION:.1}\n\
             \n\
             4  what that exactness is worth:\n\
             \x20  the input was f32, so every coordinate on this surface\n\
             \x20  sits on an f32 grid, and adjacent ones are\n\
             \x20  {spacing:.3e} apart at this mesh's largest coordinate ({largest:.3e})\n\
             \x20  No f64 arithmetic downstream moves that floor. An edit smaller than it is not an\n\
             \x20  edit anyone can see, and it is a thousand times coarser than the 1e-16 the rest\n\
             \x20  of this crate measures in."
        ),
    );

    // 5 -- hard sharing, then elevation.
    let shared = split_over_shared_edges(&mesh, mesh.level()).worst;
    if shared != EXACT_REPRODUCTION {
        failures.push(format!(
            "5  every shared hard edge is one curve on a real mesh (worst {shared:.3e})"
        ));
    }

    let samples = surface_samples(&mesh);
    let before_elevation: Vec<Point3> = samples
        .iter()
        .map(|&(face, u, v)| mesh.evaluate(face, u, v))
        .collect();
    mesh.elevate();
    let elevation_moved = before_elevation
        .iter()
        .zip(
            samples
                .iter()
                .map(|&(face, u, v)| mesh.evaluate(face, u, v)),
        )
        .map(|(was, now)| was.distance(now))
        .fold(0.0f64, f64::max);

    if elevation_moved > ELEVATION_ROUNDING {
        failures.push(format!(
            "5  elevation holds the surface on an all-hard real mesh (moved {elevation_moved:.3e})"
        ));
    }
    report(
        "5  hard sharing, and elevation holding still",
        &format!(
            "worst split on a shared edge  {shared:.3e}\n\
             degree after elevation       {}\n\
             surface moved                {elevation_moved:.3e} over {} samples",
            mesh.degree(),
            samples.len()
        ),
    );

    if failures.is_empty() {
        println!("\nall 5 measurements passed");
    } else {
        println!("\nFAILED:");
        for failure in &failures {
            println!("  - {failure}");
        }
        println!(
            "\nA non-manifold edge is the one failure here that would not be an error: enforce shares\n\
             an edge when exactly two faces meet on it, and a third face quietly keeps its own copy."
        );
        std::process::exit(1);
    }
}

/// Triangles with no area, which a subdivision turns into three degenerate quads.
fn count_degenerate(positions: &[Point3], triangles: &[[VertexId; 3]]) -> usize {
    triangles
        .iter()
        .filter(|&&[a, b, c]| {
            let [p, q, r] = [
                positions[a as usize],
                positions[b as usize],
                positions[c as usize],
            ];
            let u = q - p;
            let v = r - p;
            (u.cross(v)).length() == 0.0
        })
        .count()
}

fn shortest_edge(positions: &[Point3], triangles: &[[VertexId; 3]]) -> f64 {
    triangles
        .iter()
        .flat_map(|t| [[t[0], t[1]], [t[1], t[2]], [t[2], t[0]]])
        .map(|[a, b]| positions[a as usize].distance(positions[b as usize]))
        .fold(f64::INFINITY, f64::min)
}

fn bounds(positions: &[Point3]) -> [Point3; 2] {
    let mut low = Point3::new(f64::INFINITY, f64::INFINITY, f64::INFINITY);
    let mut high = Point3::new(f64::NEG_INFINITY, f64::NEG_INFINITY, f64::NEG_INFINITY);
    for point in positions {
        low = Point3::new(low.x.min(point.x), low.y.min(point.y), low.z.min(point.z));
        high = Point3::new(
            high.x.max(point.x),
            high.y.max(point.y),
            high.z.max(point.z),
        );
    }
    [low, high]
}

fn largest_component(point: Point3) -> f64 {
    point.x.abs().max(point.y.abs()).max(point.z.abs())
}

/// How far a face's patch is from the corners the quad mesh named for it.
fn worst_corner_error(mesh: &PatchMesh) -> f64 {
    let mut worst = 0.0f64;
    for (index, quad) in mesh.quads().iter().enumerate() {
        let face = mesh.base_net(index as u32);
        let named = [
            face.point(0, 0),
            face.point(face.degrees().0, 0),
            face.point(face.degrees().0, face.degrees().1),
            face.point(0, face.degrees().1),
        ];
        for corner in 0..4 {
            let held = named[corner].expect("a degree-1 corner is in range");
            let wanted = mesh.positions()[quad[corner] as usize];
            worst = worst.max(held.distance(wanted));
        }
    }
    worst
}

/// `(face, u, v)` triples spread over the mesh, for comparing a surface across a degree change.
fn surface_samples(mesh: &PatchMesh) -> Vec<(u32, f64, f64)> {
    let mut samples = Vec::new();
    for face in 0..mesh.quads().len() as u32 {
        for (u, v) in [(0.25f64, 0.25f64), (0.5, 0.75), (0.75, 0.125)] {
            samples.push((face, u, v));
        }
    }
    samples
}
