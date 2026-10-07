//! Probes the properties of the simplex Bernstein basis that the layered patch representation
//! stands on.
//!
//! Run with `cargo run -p patch --example elevation_probe`. This is an example and not a gate,
//! following `facet_value_probe`: it prints what it measured, including the parts it could not
//! establish.
//!
//! 1. Level 0 is the mesh — a degree-1 net reproduces its own corners exactly.
//! 2. Elevation preserves the surface — the property the refinement ladder is built on.
//! 3. The basis is a partition of unity — what makes dragging an assignment, not a solve.
//! 4. Elevation is linear — what makes an all-zero layer stay all-zero forever.
//!
//! ## A measurement that used to be here, and why it is gone
//!
//! An earlier version carried a fifth measurement: that the soft-edge `mu` invariant produces a
//! continuous tangent plane across a shared triangle edge. It passed, and it was wrong.
//!
//! It passed because it measured one hand-built pair of degree-2 triangles at a single sample
//! configuration, and degree 2 is the one degree where the two sides' interior rows happen to line
//! up. `continuity_probe` measures the same claim on a real twenty-four-face mesh and finds it
//! false: **1.49e-8 rad where the edge has not been dragged, 6.92e-2 after a corner moves, and
//! 1.29 rad once every edge is soft.** A triangle's adjacent rows carry `degree` entries on one side
//! of an edge and `degree - 1` on the other, and no assignment makes them one to one, so the formula
//! has nothing to bind to away from the degrees where the counts happen to agree.
//!
//! That measurement was always `continuity_probe`'s to make, and `continuity_probe` reports it
//! honestly. It was never an elevation property.

use whiting_patch::{Point3, SimplexNet, count};

const FRACTION: f64 = 1.0 / 3.0;

/// A deterministic spread of interior points, so the numbers below are reproducible.
fn sample_points() -> Vec<[f64; 3]> {
    let mut points = Vec::new();
    for step in 1..12 {
        let a = step as f64 / 12.0;
        for inner in 1..12 - step {
            let b = inner as f64 / 12.0;
            points.push([a, b, 1.0 - a - b]);
        }
    }
    points
}

/// A net whose shape is not its own symmetry: three corners and six degree-2 points.
fn irregular_degree2() -> SimplexNet {
    SimplexNet::new(
        2,
        vec![
            Point3::new(0.10, -0.40, 0.90),
            Point3::new(0.30, 0.10, 0.35),
            Point3::new(0.55, -0.15, 0.65),
            Point3::new(0.80, 0.25, -0.20),
            Point3::new(0.65, 0.60, 0.45),
            Point3::new(0.20, 0.55, -0.50),
        ],
    )
    .expect("degree 2 takes six points")
}

fn report(name: &str, detail: &str) {
    println!("\n== {name} ==\n{detail}");
}

fn main() {
    let samples = sample_points();
    println!(
        "simplex elevation probe — {} interior samples, degrees 0 through 5",
        samples.len()
    );

    // 1. The base net is the mesh.
    let corners = [
        Point3::new(-2.0, 1.0, 0.5),
        Point3::new(3.5, 1.25, -1.0),
        Point3::new(0.75, -4.0, 2.5),
    ];
    let mesh_level = SimplexNet::from_corners(corners[0], corners[1], corners[2]);
    // A degree-0 net is one point and a constant surface; the ladder starts at degree 1.
    assert_eq!(count(0), 1, "a degree-0 simplex net is a single point");
    let mut corner_error = 0.0f64;
    for (index, corner) in corners.iter().enumerate() {
        let at = [
            if index == 0 { 1.0 } else { 0.0 },
            if index == 1 { 1.0 } else { 0.0 },
            if index == 2 { 1.0 } else { 0.0 },
        ];
        corner_error = corner_error.max(mesh_level.evaluate(at).distance(*corner));
    }
    // And the interior must be the flat triangle, not a curve: the centre of mass of the
    // corners is the centroid, and a linear net through the corners reproduces it.
    let centroid = (corners[0] + corners[1] + corners[2]) / 3.0;
    let centroid_error = mesh_level
        .evaluate([FRACTION, FRACTION, FRACTION])
        .distance(centroid);
    let mesh_area = 0.5 * ((corners[1] - corners[0]).cross(corners[2] - corners[0])).length();
    let base_degree = mesh_level.degree();
    report(
        "1  the base net is the mesh",
        &format!(
            "base net degree       {base_degree}\nlargest corner error  {corner_error:.3e}\n\
             centroid error       {centroid_error:.3e}\ntriangle area       {mesh_area:.6}\n\
             level 0 is a degree-1 net, because a degree-0 simplex net is one point and a constant\n\
             surface. a model built at level 0 is the mesh, not an approximation of it"
        ),
    );

    // 2. Elevation preserves the surface.
    let source = irregular_degree2();
    println!("\n   degree  points   max |S_elevated - S|   max |relative to triangle|");
    let mut worst_surface_error = 0.0f64;
    let mut worst_relative = 0.0f64;
    for steps in 0..=3 {
        let raised = source.elevate_by(steps);
        let mut largest = 0.0f64;
        for sample in &samples {
            let before = source.evaluate(*sample);
            let after = raised.evaluate(*sample);
            largest = largest.max(before.distance(after));
        }
        let relative = largest / mesh_area;
        worst_surface_error = worst_surface_error.max(largest);
        worst_relative = worst_relative.max(relative);
        println!(
            "   {:>5}   {:>6}   {:>22.3e}   {:>24.3e}",
            raised.degree(),
            raised.len(),
            largest,
            relative
        );
    }
    println!(
        "\n   worst across the ladder: {worst_surface_error:.3e} absolute, {worst_relative:.3e} of the \
         triangle size"
    );
    report(
        "2  elevation is shape preserving",
        "PASS if the absolute error sits near f64 rounding for a net whose points span order one.\n\
         Elevation is also the only way this crate adds a control point, so this number is the\n\
         whole cost of going from a mesh to an editable patch.",
    );

    // 3. Partition of unity. A net whose every point is (1,0,0) evaluates to the basis sum in x.
    let mut worst_basis_sum = 0.0f64;
    for degree in 0..=5 {
        let unit = SimplexNet::new(degree, vec![Point3::new(1.0, 0.0, 0.0); count(degree)])
            .expect("unit net matches its degree");
        for sample in &samples {
            let sum = unit.evaluate(*sample).x;
            worst_basis_sum = worst_basis_sum.max((sum - 1.0).abs());
        }
        // Outside the simplex the basis is still a partition, of (a+b+c)^n rather than 1.
        let outside = unit.evaluate([0.2, 0.2, 0.2]).x;
        let expected = 0.6f64.powi(degree as i32);
        worst_basis_sum = worst_basis_sum.max((outside - expected).abs());
    }
    report(
        "3  the basis is a partition of unity",
        &format!(
            "worst |sum of basis values - 1|  {worst_basis_sum:.3e}\n\
             (the off-simplex samples are checked against (a+b+c)^n instead)\n\
             this is what makes dragging an assignment: move a point's whole support and the surface\n\
             point moves by exactly that vector, with no solve"
        ),
    );

    // 4. Elevation is linear, which is what makes an empty layer an empty layer.
    let scale = -3.25;
    let scaled_source = SimplexNet::new(2, source.points().iter().map(|p| *p * scale).collect())
        .expect("scaled net keeps its shape");
    let lifted_then_scaled = source
        .elevate_by(3)
        .points()
        .iter()
        .map(|p| *p * scale)
        .collect::<Vec<_>>();
    let scaled_then_lifted = scaled_source.elevate_by(3).points().to_vec();
    let linearity_error = lifted_then_scaled
        .iter()
        .zip(&scaled_then_lifted)
        .map(|(a, b)| a.distance(*b))
        .fold(0.0, f64::max);

    // And the property the storage design actually rests on: a zero displacement field stays
    // exactly zero through any number of refinements.
    let zero = SimplexNet::from_corners(Point3::ZERO, Point3::ZERO, Point3::ZERO);
    let mut zero_drift = 0.0f64;
    for steps in 0..=5 {
        let lifted = zero.elevate_by(steps);
        for point in lifted.points() {
            zero_drift = zero_drift.max(point.length());
        }
    }
    report(
        "4  elevation is linear, so an empty layer is empty",
        &format!(
            "elevate(s*n) vs s*elevate(n)   {linearity_error:.3e}\n\
             worst coordinate of a zero net after five elevations   {zero_drift:.3e}\n\
             the second number is the storage rule from the design: a level holds a displacement\n\
             against the root, and an all-zero level contributes exactly nothing, at any depth"
        ),
    );

    // Verdict.
    let tolerance_absolute = 1.0e-12;
    let mut failures: Vec<&str> = Vec::new();
    if corner_error > tolerance_absolute {
        failures.push("level 0 does not reproduce its corners");
    }
    if worst_relative > tolerance_absolute {
        failures.push("elevation moved the surface");
    }
    if worst_basis_sum > tolerance_absolute {
        failures.push("the basis is not a partition of unity");
    }
    if linearity_error > tolerance_absolute {
        failures.push("elevation is not linear");
    }
    if zero_drift > tolerance_absolute {
        failures.push("a zero level does not stay zero");
    }

    println!();
    if failures.is_empty() {
        println!("all measurements passed");
    } else {
        println!("FAILED:");
        for failure in &failures {
            println!("  - {failure}");
        }
        std::process::exit(1);
    }
}
