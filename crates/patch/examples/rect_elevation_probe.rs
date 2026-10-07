//! Probes the rectangular net: partition of unity, and elevation that is linear and shape
//! preserving.
//!
//! Run with `cargo run -p whiting-patch --example rect_elevation_probe`. An example and not a gate,
//! following `facet_value_probe`. It exits non-zero if any property fails.
//!
//! 1. An all-zero net stays all zero through any number of elevations — this is what makes an empty
//!    refinement layer contribute nothing.
//! 2. Elevation does not move the surface.
//! 3. Elevation is linear in the control points.
//! 4. Partition of unity: translating a point's whole support translates the point, with no solve.
//! 5. A patch built on a subdivided quad and elevated still *is* that quad — which closes the claim
//!    `subdivision_probe` makes about the mesh.
//!
//! The Bernstein weights in this file are written out rather than imported, so that "the net is a
//! partition of unity" is checked against an independently computed basis and not against the
//! crate's own basis function.

mod support;
use support::{EXACT, bilinear, grid};

use whiting_patch::Point3;
use whiting_patch::rect::{RectNet, count};
use whiting_patch::subdivide::subdivide;

/// `C(degree, k) t^k (1-t)^(degree-k)`, straight from the definition.
fn bernstein_weight(degree: usize, k: usize, t: f64) -> f64 {
    let mut binomial = 1.0f64;
    for step in 1..=k {
        binomial *= (degree - k + step) as f64 / step as f64;
    }
    binomial * t.powi(k as i32) * (1.0 - t).powi((degree - k) as i32)
}

/// A degree-1 net through four corners spread apart, so nothing about it is degenerate.
fn sample_net() -> RectNet {
    RectNet::from_corners(
        Point3::new(0.0, 0.0, 0.0),
        Point3::new(2.0, 0.3, -0.4),
        Point3::new(-0.5, 1.7, 0.6),
        Point3::new(1.4, 2.1, 1.3),
    )
}

/// The parameters probed by measurements 2 and 4, chosen to include corners, edge interiors and the
/// middle of the domain.
fn probes() -> Vec<(f64, f64)> {
    let mut parameters = Vec::new();
    for step in 0..=8 {
        let t = step as f64 / 8.0;
        parameters.push((t, 0.0));
        parameters.push((t, 1.0));
        parameters.push((0.0, t));
        parameters.push((1.0, t));
        parameters.push((t, 1.0 - t));
    }
    parameters
}

fn report(name: &str, detail: &str) {
    println!("\n== {name} ==\n{detail}");
}

fn main() {
    let mut failures: Vec<&str> = Vec::new();

    // 1. An all-zero net stays all zero.
    let zero = RectNet::new(3, 3, vec![Point3::ZERO; count(3, 3)]).expect("3x3 needs 16 points");
    let lifted = zero.elevate_by(3);
    let mut worst_zero = 0.0f64;
    let (degree_u, degree_v) = lifted.degrees();
    for i in 0..=degree_u {
        for j in 0..=degree_v {
            let point = lifted[(i, j)];
            worst_zero = worst_zero
                .max(point.x.abs())
                .max(point.y.abs())
                .max(point.z.abs());
        }
    }
    if worst_zero > EXACT {
        failures.push("1  an all-zero net stays all zero through elevation");
    }
    report(
        "1  an all-zero net stays all zero",
        &format!(
            "largest coordinate after three elevations   {worst_zero:.3e}\n\
             starting from a 3x3 net and ending at {}x{}\n\
             this is the foundation the layered design stands on. a layer stores a displacement, and\n\
             a displacement that is zero has to survive refinement as exactly zero -- otherwise an\n\
             empty layer would quietly start bending the surface",
            lifted.degrees().0,
            lifted.degrees().1,
        ),
    );

    // 2. Elevation does not move the surface.
    let flat = sample_net();
    let mut worst_drift = 0.0f64;
    for steps in 0..=5 {
        let raised = flat.elevate_by(steps);
        for (u, v) in probes() {
            worst_drift = worst_drift.max(flat.evaluate(u, v).distance(raised.evaluate(u, v)));
        }
    }
    if worst_drift > EXACT {
        failures.push("2  elevation does not move the surface");
    }
    report(
        "2  elevation does not move the surface",
        &format!(
            "worst departure over degrees 1 to 6   {worst_drift:.3e}\n\
             at {} parameters including all four corners and both edge interiors\n\
             a patch can be raised to whatever degree a continuity condition needs without the\n\
             surface shifting underneath it, so raising a degree is never an edit",
            probes().len(),
        ),
    );

    // 3. Elevation is linear in the control points.
    let left = sample_net();
    let right = RectNet::from_corners(
        Point3::new(-1.0, 2.0, 0.5),
        Point3::new(0.7, -1.3, 2.2),
        Point3::new(1.1, 0.4, -0.9),
        Point3::new(-0.2, 0.6, 1.8),
    );
    let sum = RectNet::new(
        1,
        1,
        (0..count(1, 1))
            .map(|index| left.points()[index] + right.points()[index])
            .collect(),
    )
    .expect("1x1 needs 4 points");
    let mut worst_linearity = 0.0f64;
    for steps in 0..=4 {
        let elevated_sum = sum.elevate_by(steps);
        let elevated_left = left.elevate_by(steps);
        let elevated_right = right.elevate_by(steps);
        for (u, v) in probes() {
            let separate = elevated_left.evaluate(u, v) + elevated_right.evaluate(u, v);
            worst_linearity = worst_linearity.max(elevated_sum.evaluate(u, v).distance(separate));
        }
    }
    if worst_linearity > EXACT {
        failures.push("3  elevation is linear in the control points");
    }
    report(
        "3  elevation is linear in the control points",
        &format!(
            "elevating a sum against summing the elevations   {worst_linearity:.3e}\n\
             a displacement level that satisfies a continuity condition therefore keeps satisfying it\n\
             after the patch is refined. without this, a layer would stop meaning anything the moment\n\
             the degree changed"
        ),
    );

    // 4. Partition of unity.
    let net = sample_net().elevate_by(2);
    let (degree_u, degree_v) = net.degrees();
    let offset = Point3::new(0.3, -0.7, 1.1);
    let mut worst_support = 0.0f64;
    let mut worst_support_count = 0usize;
    for (u, v) in probes() {
        let mut shifted = net.clone();
        let mut touched = 0usize;
        for i in 0..=degree_u {
            let weight_u = bernstein_weight(degree_u, i, u);
            for j in 0..=degree_v {
                let weight_v = bernstein_weight(degree_v, j, v);
                if weight_u * weight_v > 0.0 {
                    touched += 1;
                    let point = shifted[(i, j)];
                    shifted.set_point(i, j, point + offset);
                }
            }
        }
        worst_support_count = worst_support_count.max(touched);
        worst_support =
            worst_support.max(net.evaluate(u, v).distance(shifted.evaluate(u, v) - offset));
    }
    if worst_support > EXACT {
        failures.push("4  translating a support translates the point");
    }
    report(
        "4  partition of unity: a drag is an assignment, not a solve",
        &format!(
            "worst error in moving a whole support   {worst_support:.3e}\n\
             largest support size at any probed parameter   {worst_support_count} of {}\n\
             the basis weights sum to one, so a point moves by exactly the displacement applied to\n\
             every control point that can reach it. that is why dragging needs no inverse solve and\n\
             why the same drag gives the same answer on every face",
            count(degree_u, degree_v),
        ),
    );

    // 5. A patch built on a subdivided quad still is that quad.
    let (positions, triangles) = grid();
    let derived = subdivide(&positions, &triangles);
    let target = 3usize;
    let mut worst_patch = 0.0f64;
    let mut worst_corner = 0.0f64;
    for quad in &derived.quads {
        let corners = quad.map(|index| derived.positions[index as usize]);
        let patch = RectNet::from_corners(corners[0], corners[1], corners[3], corners[2])
            .elevate_by(target - 1);
        for step_u in 0..=8 {
            for step_v in 0..=8 {
                let (u, v) = (step_u as f64 / 8.0, step_v as f64 / 8.0);
                let raised = patch.evaluate(u, v);
                worst_patch = worst_patch.max(raised.distance(bilinear(corners, u, v)));
            }
        }
        for (index, (u, v)) in [(0.0, 0.0), (1.0, 0.0), (1.0, 1.0), (0.0, 1.0)]
            .into_iter()
            .enumerate()
        {
            worst_corner = worst_corner.max(patch.evaluate(u, v).distance(corners[index]));
        }
    }
    if worst_patch > EXACT {
        failures.push("5  an elevated patch still is its quad");
    }
    if worst_corner > EXACT {
        failures.push("5  an elevated patch still passes through its corners");
    }
    report(
        "5  an elevated patch still is the quad it was built on",
        &format!(
            "worst departure from the reference bilinear map   {worst_patch:.3e}\n\
             worst corner error at degree {target}   {worst_corner:.3e}\n\
             {} quads from a {} triangle mesh, each raised from degree 1 to {target} and compared\n\
             against `support::bilinear` at a 9x9 parameter grid. this is the promise\n\
             `subdivision_probe` makes about the mesh, cashed out in the new representation: level 0\n\
             of a quad patch is the face, so there is no construction error to absorb later",
            derived.face_count(),
            triangles.len(),
        ),
    );

    if failures.is_empty() {
        println!("\nall 5 measurements passed");
    } else {
        println!("\nFAILED:");
        for failure in &failures {
            println!("  - {failure}");
        }
        std::process::exit(1);
    }
}
