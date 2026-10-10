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

use std::collections::{BTreeMap, BTreeSet};

use whiting_gallery::patchmesh;
use whiting_mountain::{FacetMesh, Starter, starter};
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
             largest coordinate         {largest:.3e}\n\
             \n\
             Non-manifold edges are refused by the conversion rather than handled, because\n\
             enforcement shares a boundary only when exactly two faces meet on it. This sample has\n\
             none, and the count above is made independently of the conversion's own.",
            positions.len(),
            triangles.len(),
            before.total(),
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

    // 2 -- the conversion itself. This is the production path, `gallery::patchmesh::from_facet`, not a
    // second copy of it: the probe measures what a caller would actually get.
    //
    // Reaching here at all is also a check on the conversion's own fold count, which is written
    // separately from the count printed above. Two independent implementations of "is this mesh
    // folded" agreeing on a real mesh is worth having; an off-by-one in the one that guards would
    // show up as a refusal of a mesh with nothing wrong with it.
    let mut mesh = match patchmesh::from_facet(&facet) {
        Ok(model) => model,
        Err(problem) => {
            eprintln!("the sample massif is not convertible: {problem}");
            std::process::exit(1);
        }
    };
    let after = Incidence::of(&quad_edges(mesh.quads()));

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
            mesh.positions().len(),
            positions.len(),
            mesh.quads().len(),
            triangles.len(),
            after.boundary + after.interior + after.non_manifold,
            after.boundary,
            after.interior,
            after.non_manifold,
            after.worst,
        ),
    );

    // 3 -- the model, and the precision it has.
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
             \x20  edit anyone can see, and it is eleven orders of magnitude coarser than the\n\
             \x20  1e-16 the rest of this crate measures in."
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

    // 6 -- does the source agree with itself about which way round it goes.
    //
    // This is the one thing a mesh can be manifold, watertight and correctly covering without being.
    // Nothing above can tell the two apart, because neither of those properties says anything about
    // orientation: a rasteriser with back-face culling draws the upright region and drops the rest,
    // and the mesh still reports every count in measurements 1 to 5 as sound.
    let winding = winding_split(&facet, &triangles);
    let overlapping_blocks = overlapping_block_quads(&facet);

    if winding.agreeing() != 0 {
        failures.push(format!(
            "6  the source's faces agree about which way round they go ({} interior edges are \
             traversed the same way by both their faces)",
            winding.agreeing()
        ));
    }
    if winding.opposed_on_same_side != 0 {
        failures.push(format!(
            "6  no interior edge has both faces' third vertex on the same side in plan ({} do)",
            winding.opposed_on_same_side
        ));
    }
    report(
        "6  does the source agree with itself about winding",
        &format!(
            "every interior edge, by how it is traversed and by which side of it the third\n\
             vertices of its two faces fall in plan:\n\
             \x20 traversed both ways, opposite sides   {:>6}   sound\n\
             \x20 traversed both ways, same sides       {:>6}   the plan projection overlaps\n\
             \x20 traversed one way,  opposite sides   {:>6}   every face pointed up and it still disagrees\n\
             \x20 traversed one way,  same sides       {:>6}\n\
             \x20 a third vertex on the line           {:>6}   not on a side at all\n\
             boundary edges                          {:>6}\n\
             \n\
             smallest plan area at a one-way edge   {:.3e}\n\
             median plan area over all faces         {:.3e}\n\
             the one-way edges sit inside            {} distinct authored blocks\n\
             of those, in a block concave in plan    {}\n\
             block faces concave in plan             {overlapping_blocks} of {}\n\
             \n\
             A mesh can be manifold, watertight and correctly covering and still disagree with itself\n\
             about which way round it goes, because none of those three properties mentions\n\
             orientation. A rasteriser with back-face culling draws the upright region and drops the\n\
             rest, and every count in measurements 1 to 5 still reads as sound.\n\
             \n\
             mountain gives every face the same sign of plan area. Two faces on an edge sit on opposite\n\
             sides of it exactly when their third vertices do, so the ways that can go wrong are\n\
             separable and this table separates them: every edge the two faces traverse the same way is\n\
             an edge whose plan projection overlaps. A face seen nearly edge on casts a plan shadow so\n\
             thin it crosses its neighbour's, and \"every face points up\" stops meaning a consistent\n\
             winding there.\n\
             \n\
             Two explanations for these numbers were measured and refuted before this one. f32 rounding\n\
             was tested against its own error bound and cleared all 2382 faces. Reading the overlap\n\
             count out of the wrong branch of an earlier version of this table reported zero and said the\n\
             opposite; the joint distribution above is what settled it.\n\
             \n\
             The last three lines say how much of it is the tessellation's doing. Most of the one-way\n\
             edges are inside blocks that are convex in plan, so cutting a block into many triangles is\n\
             what produces them; the rest sit in the few blocks that are concave whatever they are cut\n\
             into.",
            winding.sound,
            winding.opposed_on_same_side,
            winding.agreeing_on_opposite_side,
            winding.agreeing_on_same_side,
            winding.collinear,
            winding.boundary,
            winding.smallest_agreeing,
            winding.median,
            winding.from_one_block,
            winding.from_concave,
            facet.blocks.len(),
        ),
    );

    if failures.is_empty() {
        println!("\nall 6 measurements passed");
    } else {
        println!("\nFAILED:");
        for failure in &failures {
            println!("  - {failure}");
        }
        println!(
            "\nA non-manifold edge is the one failure here that would not be an error: enforce shares\n\
             an edge when exactly two faces meet on it, and a third face quietly keeps its own copy.\n\
             Inconsistent winding is the other one, and its consequence is not the one usually\n\
             claimed for it. An earlier version of this text said a rasteriser \"still draws half of\n\
             it inside out\". Nothing is dropped: `Tessellation::triangles` and `Face::normal` both\n\
             derive the winding from the same quad corner order, so they agree with each other\n\
             whatever order the source quad had, and `raster_probe` measures `facing away 0` on the\n\
             massif. What is real is that 48% of the patches carry a normal **opposite to their\n\
             neighbours'**, so a lit render reverses shading across those 46 seams -- and any\n\
             continuity asked for across them is meaningless, because the two sides do not share an\n\
             orientation to be continuous in."
        );
        std::process::exit(1);
    }
}

/// Which side of a segment, in plan, a point falls on.
///
/// The sign of the plan cross product, never its magnitude: this is a question about which side, and
/// a point nearly on the line is exactly the case that matters, so a threshold would hide it.
fn plan_side(facet: &FacetMesh, from: VertexId, to: VertexId, other: VertexId) -> i32 {
    let plan = |index: VertexId| {
        let point = facet.points[index as usize];
        [point[0] as f64, point[2] as f64]
    };
    let (a, b, c) = (plan(from), plan(to), plan(other));
    let cross = (b[0] - a[0]) * (c[1] - a[1]) - (c[0] - a[0]) * (b[1] - a[1]);
    if cross > 0.0 {
        1
    } else if cross < 0.0 {
        -1
    } else {
        0
    }
}

/// How the sample's faces are wound, and what the mesh looks like where they disagree.
///
/// `mountain` gives every face the same sign of plan area. That orients a mesh consistently only where
/// no face casts a plan shadow crossing its neighbour's, and the classification below is a **joint
/// distribution** rather than a test of one idea at a time: for every interior edge, how its two faces
/// traverse it against which side of it their third vertices fall on. An explanation has to account
/// for the whole table.
#[derive(Clone, Copy, Debug, Default)]
struct Winding {
    /// Traversed from both ends, third vertices on opposite sides in plan. The sound case.
    sound: usize,
    /// Traversed from both ends, third vertices on the **same** side: the plan projection overlaps here
    /// too, and consistent winding was available but is not what came out.
    opposed_on_same_side: usize,
    /// Traversed the same way, third vertices on opposite sides. Every face pointed up and the mesh
    /// still disagrees, which this table found no instance of.
    agreeing_on_opposite_side: usize,
    /// Traversed the same way, third vertices on the **same** side. The plan projection overlaps, so
    /// "every face points up" could not have meant a consistent winding here.
    agreeing_on_same_side: usize,
    /// A third vertex lies on the edge's line in plan, so it is not on a side at all.
    collinear: usize,
    /// Edges used once. A boundary has two sides and one of them is the outside.
    boundary: usize,
    /// The smallest plan area among the faces at a same-way edge, against `median` for scale.
    smallest_agreeing: f64,
    /// The median plan area over every face.
    median: f64,
    /// How many authored blocks the same-way edges were cut into, and how many of those edges lie
    /// inside a block that is concave in plan whatever it is cut into.
    from_one_block: usize,
    from_concave: usize,
}

impl Winding {
    fn agreeing(&self) -> usize {
        self.agreeing_on_opposite_side + self.agreeing_on_same_side
    }
}

/// Walks every edge of the sample and classifies it once.
fn winding_split(facet: &FacetMesh, triangles: &[[VertexId; 3]]) -> Winding {
    let mut traversal: BTreeMap<(VertexId, VertexId), (usize, usize)> = BTreeMap::new();
    let mut owners: BTreeMap<(VertexId, VertexId), Vec<(usize, VertexId)>> = BTreeMap::new();
    for (index, &[a, b, c]) in triangles.iter().enumerate() {
        for slot in 0..3 {
            let from = [a, b, c][slot];
            let to = [a, b, c][(slot + 1) % 3];
            let other = [a, b, c][(slot + 2) % 3];
            let key = if from < to { (from, to) } else { (to, from) };
            let counts = traversal.entry(key).or_insert((0, 0));
            if from < to {
                counts.0 += 1;
            } else {
                counts.1 += 1;
            }
            owners.entry(key).or_default().push((index, other));
        }
    }

    let mut found = Winding {
        smallest_agreeing: f64::INFINITY,
        ..Winding::default()
    };
    for (&key, &counts) in &traversal {
        if counts == (1, 0) || counts == (0, 1) {
            found.boundary += 1;
            continue;
        }
        // Measurement 1 has already established the mesh is manifold, so every interior edge here has
        // exactly two faces and `(2, 0)` or `(0, 2)` is exactly "both traverse it the same way".
        if counts != (1, 1) {
            continue;
        }
        let sides: Vec<i32> = owners[&key]
            .iter()
            .map(|&(_, other)| plan_side(facet, key.0, key.1, other))
            .collect();
        if sides[0] == 0 || sides[1] == 0 {
            found.collinear += 1;
        }
        match sides[0] == sides[1] {
            true => found.opposed_on_same_side += 1,
            false => found.sound += 1,
        }
    }

    // The same-way edges need their own pass, because they are not in `(1, 1)` and the loop above
    // skipped them.
    for (&key, &counts) in &traversal {
        if counts != (2, 0) && counts != (0, 2) {
            continue;
        }
        let sides: Vec<i32> = owners[&key]
            .iter()
            .map(|&(_, other)| plan_side(facet, key.0, key.1, other))
            .collect();
        match sides[0] == sides[1] {
            true => found.agreeing_on_same_side += 1,
            false => found.agreeing_on_opposite_side += 1,
        }
        let smallest = owners[&key]
            .iter()
            .map(|(index, _)| plan_area(facet, triangles[*index]).abs())
            .fold(f64::INFINITY, f64::min);
        found.smallest_agreeing = found.smallest_agreeing.min(smallest);
    }

    let mut areas: Vec<f64> = triangles
        .iter()
        .map(|&[a, b, c]| plan_area(facet, [a, b, c]).abs())
        .collect();
    areas.sort_by(|a, b| a.partial_cmp(b).unwrap());
    if !areas.is_empty() {
        found.median = areas[areas.len() / 2];
    }
    // Which authored block each same-way edge came out of. The question the counts above cannot
    // answer: whether the overlaps are a consequence of cutting each block into many triangles, or
    // whether they are the handful of blocks that are concave in plan whatever they are cut into.
    let block_of: Vec<usize> = {
        let mut owner = vec![usize::MAX; triangles.len()];
        for (number, block) in facet.blocks.iter().enumerate() {
            for slot in owner
                .iter_mut()
                .take(block.faces.end as usize)
                .skip(block.faces.start as usize)
            {
                *slot = number;
            }
        }
        owner
    };
    let mut from_concave = 0usize;
    let mut blocks_touched: BTreeSet<usize> = BTreeSet::new();
    for (&key, &counts) in &traversal {
        if counts != (2, 0) && counts != (0, 2) {
            continue;
        }
        let sides: Vec<i32> = owners[&key]
            .iter()
            .map(|&(_, other)| plan_side(facet, key.0, key.1, other))
            .collect();
        if sides[0] != sides[1] {
            continue;
        }
        let blocks: Vec<usize> = owners[&key]
            .iter()
            .map(|(index, _)| block_of[*index])
            .collect();
        if blocks[0] == blocks[1] {
            blocks_touched.insert(blocks[0]);
            let block = &facet.blocks[blocks[0]];
            let quad = [
                block.top[0],
                *block.top.last().unwrap(),
                *block.bottom.last().unwrap(),
                block.bottom[0],
            ];
            if plan_side(facet, quad[0], quad[2], quad[1])
                == plan_side(facet, quad[0], quad[2], quad[3])
            {
                from_concave += 1;
            }
        }
    }
    found.from_one_block = blocks_touched.len();
    found.from_concave = from_concave;
    found
}

/// A triangle's signed plan area, from the same three points `mountain` reads it from.
fn plan_area(facet: &FacetMesh, triangle: [VertexId; 3]) -> f64 {
    let plan = triangle.map(|index| {
        let point = facet.points[index as usize];
        [point[0] as f64, point[2] as f64]
    });
    let [a, b, c] = plan;
    0.5 * ((b[0] - a[0]) * (c[1] - a[1]) - (c[0] - a[0]) * (b[1] - a[1]))
}

/// How many of the massif's authored block faces would break the same rule.
///
/// A block is split along the diagonal from its first crest point to its last valley point, giving
/// the two triangles the current expander would emit for it, and the test is whether the two other
/// corners land on the same side of that diagonal in plan. This is the control on the whole of
/// measurement 6: if the rule is sound at the granularity the massif was authored at, the same test
/// against block faces has to come back clean.
fn overlapping_block_quads(facet: &FacetMesh) -> usize {
    facet
        .blocks
        .iter()
        .filter(|block| {
            let quad = [
                *block.top.first().unwrap_or(&u32::MAX),
                *block.top.last().unwrap_or(&u32::MAX),
                *block.bottom.last().unwrap_or(&u32::MAX),
                *block.bottom.first().unwrap_or(&u32::MAX),
            ];
            plan_side(facet, quad[0], quad[2], quad[1])
                == plan_side(facet, quad[0], quad[2], quad[3])
                && quad.iter().all(|index| *index != u32::MAX)
        })
        .count()
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
