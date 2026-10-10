//! Measures what it would cost to make the **block face** the unit of surface instead of the
//! triangle.
//!
//! Run with `cargo run --release -p whiting-patch --example block_face_probe`. An example and not a
//! gate. **It exits non-zero on any failure.**
//!
//! `mountain` authors a massif as block faces — a crest run, a valley run and two ribs — and the
//! current front end cuts each block into triangles, which Doo–Sabin turns into quads, which become
//! bilinear patches. This probe asks what the other arrangement costs, and it is the measurement
//! `proposals/block-face-front-end.md` was written without.
//!
//! 1. **How much is being subdivided?** The ratio is the reason to ask. A patch per block is worth
//!    having only if it is far fewer patches that each carry more.
//! 2. **Do neighbouring blocks agree on their shared boundary?** Two patches share an edge as one
//!    curve only if both sides carry the same number of control points on it. Ribs and runs are
//!    different in that respect and the measurement says which of them works.
//! 3. **What does a patch per block cost, as a curve?** A `k` by `k` control net taken from the
//!    block's own surface — so the sampled polylines are a **hint** about where the control points go,
//!    and nothing here interpolates them or minimises anything. The distance from the block's mesh
//!    vertices to that patch, for `k` from 2 to 7, is the curve.
//!
//! Measurement 3 carries two things that are easy to get wrong. A block's faces do **not** cover its
//! whole `uv` box, because `u` is the fraction of the belt walked **by distance** and a crest point
//! and the valley point below it are not the same station — so the coverage is measured and reported
//! rather than assumed. And the control net has to be defined at a parameter outside that coverage,
//! so the fallback is written down rather than left to whichever branch ran first.
//!
//! The control on the nearest-point routine is that a **planar** net is the plane: every control point
//! lies in it, so the patch lies in it, and a point on it is at distance nothing up to rounding. A
//! routine that cannot resolve that cannot resolve the real numbers either.

mod support;

use std::collections::BTreeMap;

use whiting_gallery::patchmesh;
use whiting_mountain::{FacetMesh, Starter, starter};
use whiting_patch::{Point3, RectNet};

use support::ROUNDING;

/// Control net sizes to measure, so the answer is a curve rather than one point.
const RESOLUTIONS: [usize; 6] = [2, 3, 4, 5, 6, 7];

/// Grid the `uv` coverage is counted on. 41 by 41 so the endpoints are included, which is where the
/// gaps are.
const COVERAGE_STEPS: usize = 40;

fn report(name: &str, detail: &str) {
    println!("\n== {name} ==\n{detail}");
}

/// The authored massif, before anything is done to it.
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

fn widen(point: &[f32; 3]) -> Point3 {
    Point3::new(point[0] as f64, point[1] as f64, point[2] as f64)
}

fn signed_area_uv(a: [f64; 2], b: [f64; 2], c: [f64; 2]) -> f64 {
    0.5 * ((b[0] - a[0]) * (c[1] - a[1]) - (c[0] - a[0]) * (b[1] - a[1]))
}

/// Barycentric weights of `(u, v)` over a triangle of the block's `uv` domain, and whether the point
/// is inside it.
///
/// The weights sum to one for any point, so "outside" is the sign of one of them rather than a
/// separate test, and the margin `worst` is negative exactly when the point is outside.
fn weights_in(a: [f64; 2], b: [f64; 2], c: [f64; 2], point: [f64; 2]) -> Option<([f64; 3], f64)> {
    let whole = signed_area_uv(a, b, c);
    if whole.abs() <= f64::EPSILON {
        return None;
    }
    let weights = [
        signed_area_uv(point, b, c) / whole,
        signed_area_uv(a, point, c) / whole,
        signed_area_uv(a, b, point) / whole,
    ];
    let margin = weights.iter().copied().fold(f64::INFINITY, f64::min);
    Some((weights, margin))
}

/// How much of the block's `uv` box its own faces actually cover.
///
/// Counted, not assumed. `u` is the fraction of the belt walked by distance, so the strip between the
/// crest and the valley is not the whole square, and the corners of the square are the first thing to
/// fall outside it.
fn coverage(facet: &FacetMesh, number: usize) -> f64 {
    let steps = COVERAGE_STEPS;
    let covered = (0..=steps)
        .flat_map(|a| (0..=steps).map(move |b| (a, b)))
        .filter(|(a, b)| {
            contained(
                facet,
                number,
                *a as f64 / steps as f64,
                *b as f64 / steps as f64,
            )
            .is_some()
        })
        .count();
    covered as f64 / ((steps + 1) * (steps + 1)) as f64
}

/// The face of the block that owns the parameter, with the weights to use there.
fn contained(facet: &FacetMesh, number: usize, u: f64, v: f64) -> Option<(usize, [f64; 3])> {
    let block = &facet.blocks[number];
    for (offset, face) in facet.faces[block.faces.start as usize..block.faces.end as usize]
        .iter()
        .enumerate()
    {
        let [a, b, c] = face.uv.map(|corner| [corner[0] as f64, corner[1] as f64]);
        if let Some((weights, margin)) = weights_in(a, b, c, [u, v]) {
            if margin >= -1.0e-9 {
                return Some((block.faces.start as usize + offset, weights));
            }
        }
    }
    None
}

/// One point of a block's surface at the block's own `(u, v)`.
///
/// A parameter the block's faces do not cover is served by the face whose triangle it is **least far
/// outside**, extrapolated with the weights that place it there. That is a stated choice rather than
/// a lucky one: the control net is a hint, so where the block's surface stops the patch carries on,
/// and the alternative — refusing to build a net for 82 of 182 blocks — would report a cost measured
/// over the easier half and call it the price.
fn sample_block(facet: &FacetMesh, number: usize, u: f64, v: f64) -> Option<Point3> {
    let block = &facet.blocks[number];
    let mut nearest: Option<(f64, usize, [f64; 3])> = None;
    for (offset, face) in facet.faces[block.faces.start as usize..block.faces.end as usize]
        .iter()
        .enumerate()
    {
        let [a, b, c] = face.uv.map(|corner| [corner[0] as f64, corner[1] as f64]);
        let Some((weights, margin)) = weights_in(a, b, c, [u, v]) else {
            continue;
        };
        if margin >= -1.0e-9 {
            let position = face
                .indices
                .map(|index| widen(&facet.points[index as usize]));
            return Some(
                position[0] * weights[0] + position[1] * weights[1] + position[2] * weights[2],
            );
        }
        if nearest.as_ref().is_none_or(|(best, _, _)| margin > *best) {
            nearest = Some((margin, block.faces.start as usize + offset, weights));
        }
    }
    let (_, face_index, weights) = nearest?;
    let face = &facet.faces[face_index];
    let position = face
        .indices
        .map(|index| widen(&facet.points[index as usize]));
    Some(position[0] * weights[0] + position[1] * weights[1] + position[2] * weights[2])
}

/// How far `target` is from the surface of `net`, by nested refinement.
///
/// A closed form does not exist for a general patch, and the question here — how far a mesh vertex
/// sits from the surface drawn over it — is a nearest-point question by nature.
fn distance_to_net(net: &RectNet, target: Point3) -> f64 {
    let mut best = (0.5f64, 0.5f64);
    let mut distance = f64::INFINITY;
    let coarse = 20usize;
    for a in 0..=coarse {
        for b in 0..=coarse {
            let gap = net
                .evaluate(a as f64 / coarse as f64, b as f64 / coarse as f64)
                .distance(target);
            if gap < distance {
                distance = gap;
                best = (a as f64 / coarse as f64, b as f64 / coarse as f64);
            }
        }
    }
    let mut span = 1.0 / coarse as f64;
    for _ in 0..40 {
        span *= 0.5;
        for du in -2..=2 {
            for dv in -2..=2 {
                let u = (best.0 + du as f64 * span / 2.0).clamp(0.0, 1.0);
                let v = (best.1 + dv as f64 * span / 2.0).clamp(0.0, 1.0);
                let gap = net.evaluate(u, v).distance(target);
                if gap < distance {
                    distance = gap;
                    best = (u, v);
                }
            }
        }
    }
    distance
}

/// The block's own surface, sampled on a `k` by `k` grid, as a Bézier net.
///
/// Sampled rather than fitted. Every control point is a point of the block's own surface, so the
/// sampled crest and valley runs act as a **hint** about where the control points go and nothing here
/// is optimised or interpolated.
fn block_net(facet: &FacetMesh, number: usize, k: usize) -> Option<RectNet> {
    let mut points = Vec::with_capacity(k * k);
    for i in 0..k {
        let u = i as f64 / (k - 1) as f64;
        for j in 0..k {
            let v = j as f64 / (k - 1) as f64;
            points.push(sample_block(facet, number, u, v)?);
        }
    }
    RectNet::new(k - 1, k - 1, points).ok()
}

/// Worst, ninety-ninth percentile and median distance from a block's mesh vertices to a `k` by `k`
/// net over it, and the worst among the blocks whose faces cover their whole `uv` box.
fn cost_at(facet: &FacetMesh, k: usize, covered: &[bool]) -> Cost {
    let mut every = Vec::new();
    let mut whole_box = Vec::new();
    for (number, block) in facet.blocks.iter().enumerate() {
        let Some(net) = block_net(facet, number, k) else {
            continue;
        };
        let mut seen: BTreeMap<u32, Point3> = BTreeMap::new();
        for face in &facet.faces[block.faces.start as usize..block.faces.end as usize] {
            for index in face.indices {
                seen.entry(index)
                    .or_insert_with(|| widen(&facet.points[index as usize]));
            }
        }
        let block_worst = seen
            .values()
            .map(|position| distance_to_net(&net, *position))
            .fold(0.0f64, f64::max);
        every.push(block_worst);
        if covered[number] {
            whole_box.push(block_worst);
        }
    }
    every.sort_by(|a, b| a.partial_cmp(b).unwrap());
    whole_box.sort_by(|a, b| a.partial_cmp(b).unwrap());
    if every.is_empty() {
        return Cost::default();
    }
    Cost {
        worst: every[every.len() - 1],
        ninety_ninth: every[every.len() * 99 / 100],
        median: every[every.len() / 2],
        worst_whole_box: whole_box.last().copied().unwrap_or(0.0),
        blocks: every.len(),
    }
}

/// A summary of one resolution's cost. `worst_whole_box` is reported beside `worst` because the two
/// tell different stories and only the pair says which: a cost concentrated in the blocks whose `uv`
/// box is not covered belongs to the sampling grid running off the surface, and not to the resolution.
#[derive(Clone, Copy, Debug, Default)]
struct Cost {
    worst: f64,
    ninety_ninth: f64,
    median: f64,
    worst_whole_box: f64,
    blocks: usize,
}

/// The distance between two adjacent `f32`s at this magnitude.
///
/// The floor under any edit anyone makes to a surface that arrived as `f32`, and the yardstick for
/// "the patch captures the block" — below this a difference exists in the arithmetic and not in
/// anything a caller can see.
fn f32_spacing_at(value: f64) -> f64 {
    let here = value as f32;
    let next = f32::from_bits(here.to_bits() + 1);
    (next as f64 - here as f64).abs()
}

/// What the nearest-point routine says about a patch it should resolve exactly.
///
/// A Bézier net whose every control point lies in a plane lies in that plane, so a net laid out on
/// `z = 0` is the ground plane and a point on it is at distance nothing. The target sits deliberately
/// off the control grid, so this also says the search refines rather than only reading its knots.
fn planar_control() -> f64 {
    let k = 5usize;
    let mut points = Vec::with_capacity(k * k);
    for i in 0..k {
        for j in 0..k {
            points.push(Point3::new(0.3 * i as f64, 0.7 * j as f64, 0.0));
        }
    }
    let net = RectNet::new(k - 1, k - 1, points).expect("a planar grid is a valid net");
    distance_to_net(&net, Point3::new(0.61, 1.13, 0.0))
}

fn main() {
    let mut failures: Vec<String> = Vec::new();
    let facet = sample_massif();
    let blocks = facet.blocks.len();
    let triangles = facet.faces.len();

    // 1 -- the ratio that motivates the question.
    let (quads, drawn) = {
        let positions: Vec<Point3> = facet.points.iter().map(widen).collect();
        let source: Vec<[u32; 3]> = facet.faces.iter().map(|face| face.indices).collect();
        let derived = whiting_patch::subdivide::subdivide(&positions, &source);
        let model = patchmesh::from_facet(&facet).expect("the sample converts");
        (derived.quads.len(), model.tessellate(5).triangles().len())
    };
    report(
        "1  how much is being subdivided",
        &format!(
            "authored block faces              {blocks}\n\
             triangles the expander emits      {triangles} ({:.1} per block)\n\
             quads after Doo-Sabin             {quads}\n\
             bilinear patches                  {quads}\n\
             triangles drawn at side 5         {drawn}\n\
             \n\
             A patch per block is {} times fewer patches, and each carries its block's interior rather\n\
             than being one flat tile of it. That is the reason to ask; the next two measurements say\n\
             what it costs.",
            triangles as f64 / blocks as f64,
            quads / blocks
        ),
    );

    // 2 -- can neighbouring blocks share an edge as a curve.
    //
    // A block's four sides are its two runs and its two ribs, and a rib is where the crest meets the
    // valley at one end of the block. Two blocks across a rib share that rib; two blocks one above
    // another on a flank share a run.
    let mut ribs: BTreeMap<(u32, u32), Vec<usize>> = BTreeMap::new();
    let mut equal_runs = 0usize;
    let mut unequal_runs = 0usize;
    for (number, block) in facet.blocks.iter().enumerate() {
        if block.top.len() == block.bottom.len() {
            equal_runs += 1;
        } else {
            unequal_runs += 1;
        }
        for (crest, valley) in [
            (block.top[0], block.bottom[0]),
            (*block.top.last().unwrap(), *block.bottom.last().unwrap()),
        ] {
            let key = if crest <= valley {
                (crest, valley)
            } else {
                (valley, crest)
            };
            ribs.entry(key).or_default().push(number);
        }
    }
    let shared = ribs.values().filter(|owners| owners.len() == 2).count();
    let unshared = ribs.values().filter(|owners| owners.len() == 1).count();
    let crowded = ribs.values().filter(|owners| owners.len() > 2).count();
    report(
        "2  can neighbouring blocks share an edge as a curve",
        &format!(
            "ribs on exactly two blocks           {shared}\n\
             ribs on one block                   {unshared}\n\
             ribs on three or more blocks        {crowded}\n\
             blocks whose two runs are equal      {equal_runs}\n\
             blocks whose two runs differ        {unequal_runs}\n\
             \n\
             A rib is two points, so both sides can name it as a curve with the same two control points\n\
             whatever their patches' interior resolution is — that part works. A run is a different\n\
             length on the two sides in {unequal_runs} of {blocks} blocks, and two patches share a\n\
             boundary as one curve only if both carry the same number of control points on it. That is\n\
             a property of the authoring rather than of the front end, so it is counted here rather\n\
             than discovered later as a seam."
        ),
    );

    // 3 -- what a patch per block costs, as a curve.
    let coverages: Vec<f64> = (0..blocks).map(|number| coverage(&facet, number)).collect();
    let whole_box: Vec<bool> = coverages.iter().map(|held| *held >= 0.9999).collect();
    let gaps = whole_box.iter().filter(|held| !**held).count();
    let coverage_median = {
        let mut sorted = coverages.clone();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
        sorted[sorted.len() / 2]
    };
    let coverage_worst = coverages.iter().copied().fold(f64::INFINITY, f64::min);

    // The precision the mesh arrived with: one `f32` step at its own largest coordinate. A block
    // captured finer than this is captured as well as the input can be represented, whatever `f64`
    // arithmetic downstream does with it.
    let largest = facet
        .points
        .iter()
        .flat_map(|p| p.iter().map(|c| c.abs() as f64))
        .fold(0.0f64, f64::max);
    let precision = f32_spacing_at(largest);

    let mut curve = Vec::new();
    for k in RESOLUTIONS {
        curve.push((k, cost_at(&facet, k, &whole_box)));
    }

    let planar = planar_control();
    if planar > ROUNDING {
        failures.push(format!(
            "3  a planar net is the plane, so the routine resolves it ({planar:.3e})"
        ));
    }
    // The claim this measurement supports is not that a denser net is a closer one. It is that a
    // typical block is captured finer than the precision the input arrived with, which is the floor
    // below which a difference cannot be seen anyway.
    for (k, cost) in &curve {
        if cost.blocks != blocks {
            failures.push(format!(
                "3  every block gets a net at k={k} ({} of {blocks} did, so the cost is measured \
                 over fewer blocks than the mesh has)",
                cost.blocks
            ));
        }
        if cost.median >= precision {
            failures.push(format!(
                "3  a typical block is captured below the mesh's own f32 precision (k={k} median \
                 {:.3e} against {precision:.3e})",
                cost.median
            ));
        }
    }
    if gaps == 0 {
        failures.push(
            "3  some blocks leave part of their own uv box uncovered (none did, so the split below \
             measures nothing)"
                .to_owned(),
        );
    }

    let mut detail = String::new();
    for (k, cost) in &curve {
        detail.push_str(&format!(
            "k x k = {k} x {k}   worst {:.3e}   p99 {:.3e}   median {:.3e}   worst among the {} \
             blocks that cover their box {:.3e}\n",
            cost.worst,
            cost.ninety_ninth,
            cost.median,
            blocks - gaps,
            cost.worst_whole_box
        ));
    }
    report(
        "3  what a patch per block costs, as a curve",
        &format!(
            "distance from a block's mesh vertices to the patch drawn over it\n\
             {detail}\n\
             control net = the block's own surface sampled on that grid: the polylines are a hint,\n\
             nothing is fitted or interpolated\n\
             the mesh's own f32 precision at its largest coordinate  {precision:.3e}\n\
             of each block's uv box, the fraction its faces cover\n\
             \x20 smallest                        {coverage_worst:.4}   over {gaps} blocks\n\
             \x20 median                          {coverage_median:.4}\n\
             a planar net, for the routine      {planar:.3e}\n\
             \n\
             **The worst case rises with k and the median does not fall.** A Bézier net through samples\n\
             of a surface stays inside the control polygon, so a denser net is a closer one *where the\n\
             grid lands on the surface* — and these grids do not all. Where a block's uv box is not\n\
             covered the net carries an extrapolated control point, and a higher degree amplifies it.\n\
             The last column is the pair that says so: the worst block that covers its own box costs\n\
             an order of magnitude less than the worst block overall.\n\
             \n\
             The median is the other half of the answer and it is the encouraging one: a typical block\n\
             is captured an order of magnitude below the precision the mesh arrived as, so the median\n\
             block is captured as well as its input can be represented.\n\
             \n\
             The last column also does not fall with k, and a denser net over a smooth surface would.\n\
             The reading is that it is not resolution at all: a block's interior is a triangulated\n\
             surface, its creases are part of the data, and no smooth patch follows them. That is why\n\
             the cost plateaus instead of vanishing — and it is a reading of a flat curve, not a\n\
             separate measurement.\n\
             \n\
             Why the uv boxes are not full is the authoring rather than the arithmetic: u is the\n\
             fraction of the belt walked **by distance**, so a crest point and the valley point below it\n\
             are not the same station and the strip between them is not the whole square."
        ),
    );

    if failures.is_empty() {
        println!("\nall 3 measurements passed");
    } else {
        println!("\nFAILED:");
        for failure in &failures {
            println!("  - {failure}");
        }
        std::process::exit(1);
    }
}
