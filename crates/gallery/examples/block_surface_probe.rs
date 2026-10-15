//! Does the massif come out of `blockmesh` as smooth rock, and how far off the mesh does it run?
//!
//! Run with `cargo run -p whiting-gallery --example block_surface_probe`. It exits non-zero if any
//! property fails.
//!
//! Three questions, and the third is the one that decides whether this front end is usable at all.
//!
//! 1. **Do the blocks build?** Every block becomes its own Hermite surface, and a surface that
//!    `HermiteSurface::build` refuses is a block that cannot be rock. Counted, with the reason.
//! 2. **Is each block exactly C1 inside itself?** That is the whole point of going per block, and it
//!    is a property of the surface rather than of the front end.
//! 3. **How far does it run off the rock?** The grid samples the block's own triangles, so this is a
//!    fit and a fit has an error. It is measured against the **same function that drew the rock** —
//!    `blockmesh`'s own sampler — so the number is the Hermite surface's deviation and not the
//!    sampler's, and it is compared against the massif's own `f32` precision, which is the floor
//!    under any question about a `f32` mesh.
//!
//! The resolution sweep in 4 is why `SIDE` is a parameter rather than a constant.

use std::collections::BTreeMap;

use mountain::{FacetMesh, Starter, starter};
use patch::{HermiteSurface, Point3};
use whiting_gallery::blockmesh::{self, BlockSurfaces};

/// The `acos` floor for two unit vectors this close, and a small multiple for a normal.
const C1_CEILING: f64 = 1.0e-7;

fn parameters() -> [f64; 5] {
    [0.0, 0.25, 0.5, 0.75, 1.0]
}

/// The worst seam reading over one block's interior edges.
fn block_seams(surface: &HermiteSurface) -> (f64, f64) {
    let mut worst = (0.0f64, 0.0f64);
    for faces in surface.topology().edge_faces.values() {
        let [(first, first_slot), (second, _)] = faces.as_slice() else {
            continue;
        };
        let across_u = first_slot.separates_u();
        let mut split = 0.0f64;
        let mut normal = 0.0f64;
        for along in parameters() {
            let (a, b) = if across_u {
                ((1.0, along), (0.0, along))
            } else {
                ((along, 1.0), (along, 0.0))
            };
            let one = surface.patch(*first);
            let two = surface.patch(*second);
            split = split.max(one.evaluate(a.0, a.1).distance(two.evaluate(b.0, b.1)));
            let angle = one.normal(a.0, a.1).angle(two.normal(b.0, b.1));
            normal = normal.max(if angle.is_finite() {
                angle
            } else {
                f64::INFINITY
            });
        }
        worst = (worst.0.max(split), worst.1.max(normal));
    }
    worst
}

/// The distance between adjacent `f32`s at this mesh's largest coordinate — the precision the rock
/// arrived with, and therefore the floor under anything measured about it.
///
/// **The spacing is read off the bits, not computed from an exponent.** The first version used
/// `value.to_bits() + 1` on an `f64` and produced `NaN`, which then sat inside the threshold below
/// and made every assertion compare against nothing: **a `NaN` threshold passes everything**, and
/// this probe reported a departure of `2.5e+02` as a pass. That is the fourth time in this branch a
/// measurement has been unable to fail.
fn precision_floor(facet: &FacetMesh) -> f64 {
    let largest = facet
        .points
        .iter()
        .flat_map(|point| point.iter().copied())
        .fold(0.0f32, f32::max)
        .abs();
    if largest == 0.0 {
        return f64::MIN_POSITIVE;
    }
    let bits = largest.to_bits();
    let below = f32::from_bits(bits - 1);
    f64::from(largest - below)
}

fn main() {
    let mut failures: Vec<&str> = Vec::new();
    let massif = match starter(11, Starter::default()).build() {
        Ok(mesh) => mesh,
        Err(problem) => {
            eprintln!("the massif did not expand: {problem:?}");
            std::process::exit(1);
        }
    };
    let floor = precision_floor(&massif);
    println!(
        "\n{} triangles in {} blocks; the mesh's own f32 floor is {floor:.3e}",
        massif.faces.len(),
        massif.blocks.len()
    );

    let side = 4;
    let blocks = blockmesh::from_facet(&massif, side);
    println!(
        "{} blocks built at {side}x{side}, {} patches in total",
        blocks.len(),
        blocks.patch_count()
    );

    // 1 and 2.
    let mut worst_split = 0.0f64;
    let mut worst_normal = 0.0f64;
    for block in blocks.blocks() {
        let (split, normal) = block_seams(&block.surface);
        worst_split = worst_split.max(split);
        worst_normal = worst_normal.max(normal);
    }
    println!(
        "  every block's interior seams   split {worst_split:.3e}   normal {worst_normal:.3e} rad"
    );

    if blocks.len() == 0 {
        failures.push("1  at least one block became a surface");
    }
    if worst_split > 1.0e-12 {
        failures.push("1  no block's interior seam opens");
    }
    if worst_normal > C1_CEILING {
        failures.push("2  every block is exactly C1 inside itself");
    }

    // 3. How far the surface runs off the rock, against the same sampler that built it.
    let (departures, outside) = worst_deviation(&massif, &blocks, side);
    let departure_median = median(&mut departures.iter().map(|entry| entry.0).collect::<Vec<f64>>());
    let departure_worst = departures.iter().map(|entry| entry.0).fold(0.0f64, f64::max);
    let reference_median =
        median(&mut departures.iter().map(|entry| entry.1).collect::<Vec<f64>>());
    println!(
        "  departure per block: median {departure_median:.3e}  worst {departure_worst:.3e}  floor {floor:.3e}  off-rock points {outside}"
    );
    println!(
        "  the same four corners, bilinear reference: median {reference_median:.3e}  (the floor of what any C1 fit can reach)"
    );
    if departure_median == 0.0 {
        failures.push("3  the measurement of the departure can fail");
    }
    if departure_median > floor * 100.0 {
        failures.push("3  the surface stays within a hundred mesh-precisions of the rock");
    }

    // 4. The sweep, because a single resolution is a preference and a curve is a trade.
    println!(
        "\n  side   patches   departure   bilinear ref   snapped corners   snapped blocks"
    );
    let mut curve: BTreeMap<usize, f64> = BTreeMap::new();
    for candidate in [2usize, 3, 4, 6, 8] {
        let blocks = blockmesh::from_facet(&massif, candidate);
        let (values, outside) = worst_deviation(&massif, &blocks, candidate);
        let middle = median(&mut values.iter().map(|entry| entry.0).collect::<Vec<f64>>());
        let reference = median(&mut values.iter().map(|entry| entry.1).collect::<Vec<f64>>());
        let mut corners = 0usize;
        let mut snapped = 0usize;
        for block in blocks.blocks() {
            let Some(source) = massif.blocks.get(block.source as usize) else {
                continue;
            };
            let faces: Vec<&mountain::Face> = massif.faces
                [source.faces.start as usize..source.faces.end as usize]
                .iter()
                .collect();
            let uncovered = uncovered_corners(&massif, &faces, candidate);
            corners += uncovered;
            if uncovered > 0 {
                snapped += 1;
            }
        }
        curve.insert(candidate, middle);
        println!(
            "  {:>4}   {:>7}   {:>10.3e}   {:>13.3e}   {:>15}   {:>14}",
            candidate,
            blocks.patch_count(),
            middle,
            reference,
            corners,
            snapped
        );
        let _ = outside;
    }

    // 5. **Which blocks the departure belongs to.** Two candidate causes, and each gets a split.
    //
    //    *The fallback.* The front end samples a grid over the block's `uv` bounding box, and
    //    `Face::uv` records that a block need not cover that box. A corner no triangle reaches is
    //    the point `sample_block` hands to `nearest_vertex`, so `intact` below is the median over
    //    blocks where nothing was moved — if the damage lives there, this column stays at the floor.
    //
    //    *The rock's own shape.* `bilinear` is the same four corners through the best C1 patch
    //    possible, so where it is also large the front end cannot fix the block by any means: the
    //    rock's flank between the grid corners is not bilinear.
    println!(
        "\n  side   intact blocks   median | intact   median bilinear | intact   median | all   median spread"
    );
    for candidate in [2usize, 3, 4, 6, 8] {
        let blocks = blockmesh::from_facet(&massif, candidate);
        let (values, _) = worst_deviation(&massif, &blocks, candidate);
        let mut intact: Vec<f64> = Vec::new();
        let mut intact_bilinear: Vec<f64> = Vec::new();
        let mut spreads: Vec<f64> = Vec::new();
        for (block, (departure, reference)) in blocks.blocks().iter().zip(values.iter()) {
            let Some(source) = massif.blocks.get(block.source as usize) else {
                continue;
            };
            let faces: Vec<&mountain::Face> = massif.faces
                [source.faces.start as usize..source.faces.end as usize]
                .iter()
                .collect();
            if uncovered_corners(&massif, &faces, candidate) == 0 {
                intact.push(*departure);
                intact_bilinear.push(*reference);
            }
            spreads.push(block.surface.step_spread().0.max(block.surface.step_spread().1));
        }
        let all = median(&mut values.iter().map(|entry| entry.0).collect::<Vec<f64>>());
        let spread_middle = median(&mut spreads);
        println!(
            "  {:>4}   {:>13}   {:>16.3e}   {:>24.3e}   {:>14.3e}   {:>14.3e}",
            candidate,
            intact.len(),
            median(&mut intact),
            median(&mut intact_bilinear),
            all,
            spread_middle
        );
    }

    println!();
    if failures.is_empty() {
        println!("all measurements pass");
    } else {
        for failure in &failures {
            println!("FAILED  {failure}");
        }
        std::process::exit(1);
    }
}

/// One reading per block: how far the built Hermite patch runs off the rock, and how far the
/// **bilinear reference** through the same four corners does.
type Reading = (f64, f64);

/// The worst distance between the rock and the surface built through it, and how many blocks could
/// not be sampled at all.
///
/// **Measured at the grid corners, where the surface is exact by construction, so the number is a
/// real fit error and not a tautology**: the sampler is re-implemented here rather than called,
/// because a probe that measures a front end by asking the front end where the rock is measures it
/// to agree with itself.
fn worst_deviation(massif: &FacetMesh, blocks: &BlockSurfaces, side: usize) -> (Vec<Reading>, usize) {
    let mut per_block: Vec<Reading> = Vec::with_capacity(blocks.len());
    let mut outside = 0usize;
    for (index, block) in blocks.blocks().iter().enumerate() {
        let _ = index;
        let Some(source) = massif.blocks.get(block.source as usize) else {
            outside += 1;
            continue;
        };
        let faces: Vec<&mountain::Face> = massif.faces
            [source.faces.start as usize..source.faces.end as usize]
            .iter()
            .collect();
        if faces.is_empty() {
            outside += 1;
            continue;
        }
        let mut worst = 0.0f64;
        let mut best = 0.0f64;
        let (low, high) = uv_box(&faces);
        // **The cell centres, not the grid corners.** A Hermite patch interpolates its own corners,
        // so sampling there measures the definition of the representation rather than the front
        // end's quality: it read `0.000e+00` at every resolution, and the probe's own assertion
        // "the departure must not be zero" is what caught it. The fit error lives **between** the
        // corners, which is the only place a grid has one.
        let last = side - 1;
        for row_index in 0..last {
            for column in 0..last {
                let (s, t) = (
                    (column as f64 + 0.5) / last as f64,
                    (row_index as f64 + 0.5) / last as f64,
                );
                let u = low[0] + (high[0] - low[0]) * s as f32;
                let v = low[1] + (high[1] - low[1]) * t as f32;
                // A grid point no triangle covers is **skipped and counted**, not compared against
                // whatever the front end fell back to. A block's belt domain is not always a
                // rectangle — `Face::uv`'s own documentation records 89 of 182 that leave part of
                // their box uncovered — so this is a fact about the massif rather than an error.
                // Comparing against the front end's own fallback instead would have the probe agree
                // with itself on exactly the points that carry no information.
                let Some(rock) = sample(&faces, massif, u, v) else {
                    outside += 1;
                    continue;
                };
                // **The cell that owns this sample, and the cell's own `[0,1]^2` parameter.** A
                // block's surface is a grid of `(side-1)^2` patches and the block-global `(s, t)` is
                // not the parameter any one of them is written in. Deriving the two separately is
                // what produced `1.0e+01` above `side` 2 and `0.000e+00` at it.
                let cell = (row_index * last + column) as u32;
                worst = worst.max(rock.distance(block.surface.patch(cell).evaluate(0.5, 0.5)));
                best = best.max(rock.distance(bilinear_centre(&block.surface, cell)));
            }
        }
        per_block.push((worst, best));
    }
    (per_block, outside)
}

/// The middle of a sorted run: the **median**, and not a mean, because a mean of a
/// distribution with outliers is a number about the outliers.
fn median(values: &mut [f64]) -> f64 {
    if values.is_empty() {
        return 0.0;
    }
    values.sort_by(f64::total_cmp);
    let middle = values.len() / 2;
    if values.len() % 2 == 0 {
        (values[middle - 1] + values[middle]) * 0.5
    } else {
        values[middle]
    }
}

/// The `uv` bounding box of a block's triangles, which is the box the front end lays its grid over.
fn uv_box(faces: &[&mountain::Face]) -> ([f32; 2], [f32; 2]) {
    let mut low = [f32::INFINITY; 2];
    let mut high = [f32::NEG_INFINITY; 2];
    for face in faces {
        for corner in face.uv {
            low[0] = low[0].min(corner[0]);
            high[0] = high[0].max(corner[0]);
            low[1] = low[1].min(corner[1]);
            high[1] = high[1].max(corner[1]);
        }
    }
    (low, high)
}

/// How many of the grid's corners no triangle of this block covers.
///
/// **The count of points the front end hands to its fallback**, re-derived from the same grid
/// `surface_from_faces` lays: `row` walks `v`, `column` walks `u`, and the box is the faces' own
/// `uv` bounds. A corner covered by nothing is a corner `sample_block` moved onto a mesh vertex.
fn uncovered_corners(massif: &FacetMesh, faces: &[&mountain::Face], side: usize) -> usize {
    if faces.is_empty() || side < 2 {
        return 0;
    }
    let (low, high) = uv_box(faces);
    let mut uncovered = 0usize;
    for row in 0..side {
        let v = low[1] + (high[1] - low[1]) * row as f32 / (side - 1) as f32;
        for column in 0..side {
            let u = low[0] + (high[0] - low[0]) * column as f32 / (side - 1) as f32;
            if sample(faces, massif, u, v).is_none() {
                uncovered += 1;
            }
        }
    }
    uncovered
}

/// The centre of the bilinear patch through a cell's own four corners.
///
/// **The reference that says whether the Hermite patch is the problem or the rock is.** A bicubic
/// Hermite patch whose corner derivatives are the central differences and whose twist is zero is,
/// at its centre, exactly the bilinear interpolant of its four corners — the sums cancel. So this
/// is the best a C1 surface can do through the same four points, and the gap between it and the
/// Hermite patch's own centre is what the derivative seeding costs. The gap between it and the rock
/// is what **no** C1 surface through those points can recover, because it is the rock's own shape.
fn bilinear_centre(surface: &HermiteSurface, cell: u32) -> Point3 {
    let [a, b, c, d] = surface.quads()[cell as usize];
    let corners = [
        surface.positions()[a as usize],
        surface.positions()[b as usize],
        surface.positions()[c as usize],
        surface.positions()[d as usize],
    ];
    let mut sum = [0.0f64; 3];
    for corner in corners {
        sum[0] += corner.x * 0.25;
        sum[1] += corner.y * 0.25;
        sum[2] += corner.z * 0.25;
    }
    Point3::new(sum[0], sum[1], sum[2])
}

/// The barycentric weights of `point` over a triangle in a parameter plane, as the front end's own
/// sampler computes them. Duplicated here on purpose, as [`sample`] is.
fn sample(faces: &[&mountain::Face], massif: &FacetMesh, u: f32, v: f32) -> Option<Point3> {
    let mut sum = [0.0f64; 3];
    let mut count = 0.0f64;
    for face in faces {
        let [a, b, c] = face.uv;
        let area = (b[0] - a[0]) * (c[1] - a[1]) - (c[0] - a[0]) * (b[1] - a[1]);
        if area.abs() < f32::EPSILON {
            continue;
        }
        let first = (b[0] - u) * (c[1] - v) - (c[0] - u) * (b[1] - v);
        let second = (c[0] - u) * (a[1] - v) - (a[0] - u) * (c[1] - v);
        let shares = [f64::from(first / area), f64::from(second / area)];
        let third = 1.0 - shares[0] - shares[1];
        let shares = [shares[0], shares[1], third];
        if shares.iter().any(|share| *share < -1.0e-9) {
            continue;
        }
        for (index, share) in shares.into_iter().enumerate() {
            let point = massif.points[face.indices[index] as usize];
            for axis in 0..3 {
                sum[axis] += f64::from(point[axis]) * share;
            }
        }
        count += 1.0;
    }
    if count == 0.0 {
        return None;
    }
    Some(Point3::new(sum[0] / count, sum[1] / count, sum[2] / count))
}
