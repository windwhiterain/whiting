//! One Hermite surface per **belt** rather than per block: does it build, and how far off the rock?
//!
//! Run with `cargo run -p whiting-gallery --example belt_surface_probe`. It exits non-zero if any
//! property fails.
//!
//! The claim under test is narrow and it is about the *shape of the domain*, not about Hermite:
//!
//! > A block's faces carry `(u, v)` local to that block, so the block's covered region is a ragged
//! > subset of its own box. **A belt's faces carry `(u, v)` over the whole belt**, so its region is
//! > a rectangle and a regular grid lands on the rock instead of beside it.
//!
//! If that is true the belt construction should show three things the block one does not: every belt
//! builds, far fewer grid points fall off the rock, and the departure from the rock is smaller.
//!
//! 1. **Every belt builds**, and the count against the massif's own.
//! 2. **Every belt is exactly C1 inside itself** — the property the whole route is for.
//! 3. **The departure from the rock, per belt**, at the **cell centres**. Not the corners: a Hermite
//!    patch interpolates its own corners, so sampling there measures the definition of the
//!    representation and reads `0.000e+00` at every resolution.
//! 4. **The resolution sweep**, because a single resolution is a preference and a curve is a trade.

use std::collections::BTreeMap;

use mountain::tessellation::BeltRef;
use mountain::{FacetMesh, Starter, starter};
use patch::HermiteSurface;
use whiting_gallery::blockmesh::{self, BeltSurfaces};

/// The `acos` floor for two unit vectors this close, and a small multiple for a normal.
const C1_CEILING: f64 = 1.0e-7;

fn parameters() -> [f64; 5] {
    [0.0, 0.25, 0.5, 0.75, 1.0]
}

/// The worst seam reading over one belt's interior edges.
fn belt_seams(surface: &HermiteSurface) -> (f64, f64) {
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

/// The rock at `(u, v)`, averaged over every face of the massif that covers it.
///
/// **The massif's own triangles, not the front end's sampler**, so the number is the front end's
/// quality and not a comparison of it with itself.
fn sample(facet: &FacetMesh, faces: &[&mountain::Face], u: f32, v: f32) -> Option<patch::Point3> {
    let mut sum = [0.0f64; 3];
    let mut count = 0.0f64;
    for face in faces {
        let [a, b, c] = face.uv;
        let area = (b[0] - a[0]) * (c[1] - a[1]) - (c[0] - a[0]) * (b[1] - a[1]);
        if area.abs() < f32::EPSILON {
            continue;
        }
        let raw = [
            (b[0] - u) * (c[1] - v) - (c[0] - u) * (b[1] - v),
            (c[0] - u) * (a[1] - v) - (a[0] - u) * (c[1] - v),
        ];
        let shares = [f64::from(raw[0] / area), f64::from(raw[1] / area)];
        let shares = [shares[0], shares[1], 1.0 - shares[0] - shares[1]];
        if shares.iter().any(|share| *share < -1.0e-9) {
            continue;
        }
        for (index, share) in shares.into_iter().enumerate() {
            let point = facet.points[face.indices[index] as usize];
            for axis in 0..3 {
                sum[axis] += f64::from(point[axis]) * share;
            }
        }
        count += 1.0;
    }
    if count == 0.0 {
        return None;
    }
    Some(patch::Point3::new(
        sum[0] / count,
        sum[1] / count,
        sum[2] / count,
    ))
}

/// Each belt's worst departure from the rock, sampled at cell centres, and how many grid points the
/// sampler could not place.
fn belt_departures(facet: &FacetMesh, belts: &BeltSurfaces, side: usize) -> (Vec<f64>, usize) {
    let mut per_belt = Vec::with_capacity(belts.len());
    let mut outside = 0usize;
    for belt in belts.belts() {
        let faces: Vec<&mountain::Face> = facet
            .faces
            .iter()
            .filter(|face| face.attributes.belt == belt.belt)
            .collect();
        if faces.is_empty() {
            outside += 1;
            continue;
        }
        let (mut low_u, mut high_u) = (f32::INFINITY, f32::NEG_INFINITY);
        let (mut low_v, mut high_v) = (f32::INFINITY, f32::NEG_INFINITY);
        for face in &faces {
            for corner in face.uv {
                low_u = low_u.min(corner[0]);
                high_u = high_u.max(corner[0]);
                low_v = low_v.min(corner[1]);
                high_v = high_v.max(corner[1]);
            }
        }
        let last = side - 1;
        let mut worst = 0.0f64;
        for row in 0..last {
            for column in 0..last {
                let (s, t) = (
                    (column as f64 + 0.5) / last as f64,
                    (row as f64 + 0.5) / last as f64,
                );
                let u = low_u + (high_u - low_u) * s as f32;
                let v = low_v + (high_v - low_v) * t as f32;
                let Some(rock) = sample(facet, &faces, u, v) else {
                    outside += 1;
                    continue;
                };
                let cell = (row * last + column) as u32;
                worst = worst.max(rock.distance(belt.surface.patch(cell).evaluate(0.5, 0.5)));
            }
        }
        per_belt.push(worst);
    }
    (per_belt, outside)
}

/// How many distinct belts the massif's faces name.
fn distinct_belts(facet: &FacetMesh) -> usize {
    let mut belts: Vec<BeltRef> = facet
        .faces
        .iter()
        .map(|face| face.attributes.belt)
        .collect();
    belts.sort_by_key(|belt| (belt.valley.0, belt.segment, belt.side));
    belts.dedup();
    belts.len()
}

/// What one group of faces spans in the belt's own `(u, v)`.
#[derive(Clone, Copy, Debug)]
struct Span {
    low_u: f32,
    low_v: f32,
    high_u: f32,
    high_v: f32,
    faces: usize,
}

impl Span {
    fn new() -> Self {
        Self {
            low_u: f32::INFINITY,
            low_v: f32::INFINITY,
            high_u: f32::NEG_INFINITY,
            high_v: f32::NEG_INFINITY,
            faces: 0,
        }
    }

    fn add(&mut self, uv: [f32; 2]) {
        self.low_u = self.low_u.min(uv[0]);
        self.low_v = self.low_v.min(uv[1]);
        self.high_u = self.high_u.max(uv[0]);
        self.high_v = self.high_v.max(uv[1]);
        self.faces += 1;
    }
}

/// Every belt's span, in the massif's own belt order.
/// Every belt's span, in the massif's own belt order.
fn spans_by_belt(facet: &FacetMesh) -> Vec<Span> {
    let mut order: Vec<BeltRef> = Vec::new();
    let mut map: BTreeMap<BeltRef, Span> = BTreeMap::new();
    for face in &facet.faces {
        if !map.contains_key(&face.attributes.belt) {
            order.push(face.attributes.belt);
            map.insert(face.attributes.belt, Span::new());
        }
        if let Some(span) = map.get_mut(&face.attributes.belt) {
            for corner in face.uv {
                span.add(corner);
            }
        }
    }
    order.sort_by_key(|belt| (belt.valley.0, belt.segment, belt.side));
    order.into_iter().map(|belt| map[&belt]).collect()
}

/// Every block's span.
fn spans_by_block(facet: &FacetMesh) -> Vec<Span> {
    let mut map: BTreeMap<u32, Span> = BTreeMap::new();
    for face in &facet.faces {
        let span = map.entry(face.block).or_insert_with(Span::new);
        for corner in face.uv {
            span.add(corner);
        }
    }
    map.into_values().collect()
}

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

fn main() {
    let mut failures: Vec<&str> = Vec::new();
    let massif = match starter(11, Starter::default()).build() {
        Ok(mesh) => mesh,
        Err(problem) => {
            eprintln!("the massif did not expand: {problem:?}");
            std::process::exit(1);
        }
    };
    let largest = massif
        .points
        .iter()
        .flat_map(|point| point.iter().copied())
        .fold(0.0f32, f32::max)
        .abs();
    let floor = if largest == 0.0 {
        f64::MIN_POSITIVE
    } else {
        f64::from(largest - f32::from_bits(largest.to_bits() - 1))
    };
    println!(
        "\n{} triangles in {} blocks; the mesh's own f32 floor is {floor:.3e}",
        massif.faces.len(),
        massif.blocks.len()
    );

    // What each face's `(u, v)` actually spans. **The decisive measurement**, and every number
    // below is unreadable without it: if a belt's domain were the whole belt, every one of its
    // faces would carry `u` in `[0, 1]`, and a belt and a block coming out with the same numbers
    // is the symptom of the grid being laid over the *same* domain whatever it is called.
    let belt_spans = spans_by_belt(&massif);
    let block_spans = spans_by_block(&massif);
    println!(
        "\n  a belt's faces span {} groups; a block's {} -- if these match, the grouping changes nothing",
        belt_spans.len(),
        block_spans.len()
    );
    // **The span's own extent, not whether it touches 1.** A block's domain reaching `[0,1]` says
    // nothing if it also reaches `[0,1]` at every block -- so the number that separates them is how
    // much of the `[0,1]` range each group actually uses.
    let extent = |spans: &Vec<Span>| -> (f64, f64) {
        let mut width: Vec<f64> = spans
            .iter()
            .map(|span| (span.high_u - span.low_u) as f64)
            .collect();
        let mut depth: Vec<f64> = spans
            .iter()
            .map(|span| (span.high_v - span.low_v) as f64)
            .collect();
        (median(&mut width), median(&mut depth))
    };
    let (belt_width, belt_depth) = extent(&belt_spans);
    let (block_width, block_depth) = extent(&block_spans);
    println!("  median u-width of a group: belt {belt_width:.4}   block {block_width:.4}");
    println!("  median v-depth of a group: belt {belt_depth:.4}   block {block_depth:.4}");
    println!(
        "  median faces per group:    belt {:.1}   block {:.1}",
        median(
            &mut belt_spans
                .iter()
                .map(|span| span.faces as f64)
                .collect::<Vec<_>>()
        ),
        median(
            &mut block_spans
                .iter()
                .map(|span| span.faces as f64)
                .collect::<Vec<_>>()
        )
    );

    let side = 4;
    let belts = blockmesh::from_belts(&massif, side);
    println!(
        "{} belts built at {side}x{side} over {} blocks, {} patches in total",
        belts.len(),
        belts.blocks(),
        belts.patch_count()
    );

    let mut worst_split = 0.0f64;
    let mut worst_normal = 0.0f64;
    for belt in belts.belts() {
        let (split, normal) = belt_seams(&belt.surface);
        worst_split = worst_split.max(split);
        worst_normal = worst_normal.max(normal);
    }
    println!(
        "  every belt's interior seams   split {worst_split:.3e}   normal {worst_normal:.3e} rad"
    );

    if belts.len() == 0 {
        failures.push("1  at least one belt became a surface");
    }
    if worst_split > 1.0e-12 {
        failures.push("1  no belt's interior seam opens");
    }
    if worst_normal > C1_CEILING {
        failures.push("2  every belt is exactly C1 inside itself");
    }

    let (mut departures, outside) = belt_departures(&massif, &belts, side);
    let middle = median(&mut departures);
    let worst = departures.iter().copied().fold(0.0f64, f64::max);
    println!(
        "  departure per belt: median {middle:.3e}   worst {worst:.3e}   floor {floor:.3e}   off-rock points {outside}"
    );
    if middle == 0.0 {
        failures.push("3  the departure can fail");
    }

    // 4. **Does the grouping change anything?** The numbers above are identical to the block
    //    construction's, because on this massif a belt **is** a block: `Face::attributes.belt` and
    //    `Face::block` name the same 182 groups. `blockmesh::from_belts` is therefore not a second
    //    front end here but a different *name* for the first, and the measurement that would
    //    separate them needs a massif with more than one block per belt.
    //
    //    That is asserted here rather than left to be noticed later, because a route whose measured
    //    benefit is identical to the route it replaces is a rename until proven otherwise.
    let belts_per_block = distinct_belts(&massif);
    if belts_per_block == block_spans.len() && belt_spans.len() == block_spans.len() {
        println!(
            "\n  NOTE: this massif has {} distinct belts over {} blocks, so every block is its own\n\
             \x20 belt and the belt construction measures the same numbers as the block one. It is\n\
             \x20 not a second front end until a massif exists with more than one block per belt.",
            belts_per_block,
            block_spans.len()
        );
    }

    println!("\n  side   patches   median departure   off-rock points");
    for candidate in [2usize, 3, 4, 6, 8] {
        let belts = blockmesh::from_belts(&massif, candidate);
        let (mut values, outside) = belt_departures(&massif, &belts, candidate);
        let middle = median(&mut values);
        println!(
            "  {:>4}   {:>7}   {:>16.3e}   {outside}",
            candidate,
            belts.patch_count(),
            middle
        );
    }

    // The control: a resolution of 1 has no cells and cannot report a departure, so the measurement
    // above has to be shown to be able to fail on something. The assertion `middle == 0.0` above is
    // that control, and it is why it is there.

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
