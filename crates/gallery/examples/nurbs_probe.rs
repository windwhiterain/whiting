//! Reports what the patch surface in the `nurbs` scene actually is, and that the scene compiles.
//!
//! Run with `cargo run -p whiting-gallery --example nurbs_probe`. A diagnostic rather than a test.
//!
//! A scene that builds is worth very little on its own: a document can validate, compile, and draw a
//! flat sheet, and every one of those is compatible with the model being wrong. So this reports the
//! **model**, and measures the two things the scene is built to show:
//!
//! 1. **The scene compiles.** The document validates against the registry and the standard pipeline
//!    compiles it, which is what says the points, the point schema and the instances agree.
//! 2. **What the surface is.** Patches, degree, soft and hard edges, displacement layers, and the
//!    triangles the sheet is finally drawn as.
//! 3. **The crease is the sharpest thing on the sheet.** The angle between the two faces' own normals
//!    across an edge, split by what the edge is tagged. The crease edges must fold by **more** than
//!    every soft edge folds by, because that ordering is the whole of what the scene is for: a sheet
//!    whose creases are softer than its seams has no crease in it however it is lit.
//!
//!    What it is **not** asked to show is that a soft edge reads zero. It does not, and an earlier
//!    version of this probe asserted that it did and so reported a failure that was a fact about the
//!    crate rather than about the scene: `patch::continuity` states plainly that a soft edge is
//!    measurably smoother than a crease and **not** continuous across the seam. The probe asserts
//!    what is actually promised -- the ordering, and a ceiling the gap may not grow past.
//! 4. **The drawn triangles agree with the surface.** Every triangle's own geometric normal against
//!    the patch's, which is the check `raster_probe` runs on the massif and which costs nothing here.
//!
//! Measurement 3 is the one the scene exists for. The other three are what make it readable.

use definition::{Definition, DefinitionIdentifier, Registry, SchemaIdentifier};
use graphics::{GRAPHICS_SCHEMA, INSTANCE_SCHEMA};
use patch::topology::EdgeKey;
use patch::{Continuity, FaceId, Level, PatchMesh, Point3};
use scene::{CompileContext, Compiler, STANDARD_SCENE_SCHEMA, StandardScene};
use whiting_gallery::{Gallery, nurbs};

/// How much sharper than every soft edge the crease has to be, as a ratio.
///
/// One is "the crease merely wins", which is the assertion; it is not a margin. A sheet whose soft
/// edges all read far below it and whose crease just clears it is exactly the sheet this scene is
/// for, and a sheet that needs slack here is a sheet whose smoothness is a picture rather than a fact.
const CREASE_SEPARATION: f64 = 1.0;

/// The largest angle a soft edge on this sheet may fold by, in radians.
///
/// A regression ceiling rather than a target. Measured on this sheet the soft edges run from `2.4e-2`
/// at the median to `2.1e-1` at the worst, against a crease at `4.7e-1` -- and the gap between a
/// soft seam and a crease is not free: it is the price of `patch`'s soft-edge invariant, which leaves
/// the ends of a slave's row un-derived and cannot satisfy two soft edges on one face at once. This
/// number is here so that a change which makes that worse cannot do so unnoticed; it says nothing
/// about where the gap ought to be.
const SOFT_CEILING: f64 = 4.0e-1;

/// The smallest angle the crease may fold by, in radians, for it to be a crease at all.
///
/// Not a target but a floor on the word. A fold shallower than this is a ridge, and a scene built to
/// show a crease that does not have one has been built wrong.
const CREASE_FOLD: f64 = 1.0e-1;

fn report(name: &str, detail: &str) {
    MEASUREMENTS.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    println!("\n== {name} ==\n{detail}");
}

/// How many measurements have been reported, so the closing line cannot drift out of step with them.
///
/// Counted rather than written out: `raster_probe` printed a confident "all 4" over five
/// measurements for a whole session before this was noticed.
static MEASUREMENTS: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

/// Compiles the scene, and says so if it will not.
fn compiles() -> bool {
    let mut registry = Registry::new();
    registry.put_schema(
        SchemaIdentifier::new(GRAPHICS_SCHEMA),
        graphics::graphics_schema(),
    );
    registry.put_schema(
        SchemaIdentifier::new(INSTANCE_SCHEMA),
        graphics::instance_schema(),
    );
    scene::register_standard_scene_schema(&mut registry);
    let source = Definition::new(
        DefinitionIdentifier::new("nurbs.source"),
        SchemaIdentifier::new(STANDARD_SCENE_SCHEMA),
        Gallery::build("nurbs"),
    );
    if let Err(error) = source.validate(&registry) {
        eprintln!("the scene does not validate: {error}");
        return false;
    }
    let compiled = StandardScene::new(
        DefinitionIdentifier::new("nurbs.graphics"),
        DefinitionIdentifier::new("nurbs.instance"),
    )
    .compile(&CompileContext {
        scene: &source,
        previous_graphics: None,
        previous_instance: None,
        registry: &registry,
        camera: 0,
        // No machine behind this diagnostic, so the compile is made against the floor.
        limits: None,
    });
    if let Err(error) = compiled {
        eprintln!("the scene does not compile: {error}");
        return false;
    }
    report(
        "1  the scene compiles",
        "validated against the registry and compiled by the standard pipeline",
    );
    true
}

/// The angle between the two faces of `edge` at the middle of that edge, in radians.
///
/// Asked of each face's **composed** patch, at the parameter that face assigns to that position of
/// its own boundary. Comparing the two is the whole of what a continuity tag means on a crease, and
/// it is a question about the surface rather than about any one level of it.
///
/// The slot names the parameter an edge **fixes**, so the other one is what `along` moves. Reading
/// them the other way round asks each face for a normal off its *perpendicular* edge, at a point that
/// is not on the shared boundary at all -- which returns a number, a plausible one, and a fold that
/// is not there.
fn fold_at(mesh: &PatchMesh, composed: &Level, edge: &EdgeKey) -> Option<f64> {
    let faces = mesh.topology().edge_faces.get(edge)?;
    if faces.len() != 2 {
        return None;
    }
    // The middle of the edge, rather than one of the integer positions `interior_index` hands out:
    // the two ends of a row are where `patch`'s invariant costs the most, and the middle is where the
    // sheet is actually judged from.
    let along = 0.5;
    let normal_of = |(face, slot): (FaceId, patch::topology::EdgeSlot)| {
        let (u, v) = match slot {
            patch::topology::EdgeSlot::MinV => (along, 0.0),
            patch::topology::EdgeSlot::MaxV => (along, 1.0),
            patch::topology::EdgeSlot::MinU => (0.0, along),
            patch::topology::EdgeSlot::MaxU => (1.0, along),
        };
        composed.face(face as usize).map(|net| net.normal(u, v))
    };
    let first = normal_of(faces[0])?;
    let second = normal_of(faces[1])?;
    Some(first.angle(second))
}

fn main() {
    let mut failures: Vec<String> = Vec::new();
    let sheet = nurbs::sheet();
    let crease: Vec<EdgeKey> = nurbs::crease();

    if !compiles() {
        std::process::exit(1);
    }

    // 2 -- what the surface is.
    let soft = sheet.soft_edges().len();
    let hard_interior = sheet
        .topology()
        .edge_faces
        .values()
        .filter(|faces| faces.len() == 2)
        .count()
        - soft;
    report(
        "2  what the surface is",
        &format!(
            "patches                       {}\n\
             degree every patch carries    {}\n\
             soft edges                    {soft}\n\
             hard interior edges           {hard_interior}\n\
             of which the crease           {}\n\
             displacement layers           {}\n\
             control points per layer      {}\n\
             triangles the sheet is drawn as  {}\n\
             points the scene document holds   {}",
            sheet.quads().len(),
            sheet.degree(),
            crease.len(),
            sheet.offset_count(),
            sheet.level().point_count(),
            nurbs::TRIANGLE_COUNT,
            nurbs::POINT_COUNT
        ),
    );

    if sheet.quads().is_empty() {
        failures.push("2  the surface has patches".to_owned());
    }
    if sheet.offset_count() == 0 {
        failures.push("2  the surface carries a displacement layer".to_owned());
    }
    if crease
        .iter()
        .any(|edge| sheet.continuity(edge).is_some_and(Continuity::is_soft))
    {
        failures.push("2  the crease is hard on every one of its edges".to_owned());
    }

    // 3 -- the crease against the seams, measured as the angle between the two faces across an edge.
    let composed = sheet.composed();
    let mut soft_folds: Vec<f64> = Vec::new();
    let mut crease_folds: Vec<f64> = Vec::new();
    let mut unreadable = 0usize;
    for (edge, faces) in &sheet.topology().edge_faces {
        if faces.len() != 2 {
            continue;
        }
        let Some(fold) = fold_at(&sheet, &composed, edge) else {
            unreadable += 1;
            continue;
        };
        if !fold.is_finite() {
            // Counted, not folded away. `f64::max` and `f64::min` both return the other operand when
            // one is NaN, so a reading that could not be made disappears silently into whatever the
            // rest of the run said — which is how a probe reports a clean zero for a measurement it
            // never made.
            unreadable += 1;
            continue;
        }
        if sheet.continuity(edge).is_some_and(Continuity::is_soft) {
            soft_folds.push(fold);
        } else if crease.contains(edge) {
            crease_folds.push(fold);
        }
    }
    // Both aggregates are over readings already known finite, so `max` and `min` cannot swallow one.
    let worst_soft = soft_folds.iter().copied().fold(0.0f64, f64::max);
    let median_soft = {
        let mut sorted = soft_folds.clone();
        sorted.sort_by(f64::total_cmp);
        sorted.get(sorted.len() / 2).copied().unwrap_or(f64::NAN)
    };
    let smallest_crease = crease_folds.iter().copied().fold(f64::INFINITY, f64::min);
    let largest_crease = crease_folds.iter().copied().fold(0.0f64, f64::max);
    let separation = smallest_crease / worst_soft.max(f64::MIN_POSITIVE);

    if unreadable != 0 {
        failures.push(format!(
            "3  every edge's fold could be measured ({unreadable} could not, and a reading that is \
             silently dropped reports whatever the rest of the run said)"
        ));
    }
    if crease_folds.is_empty() || soft_folds.is_empty() {
        failures.push(format!(
            "3  both kinds of edge were measured ({} crease, {} soft, and a comparison against an \
             empty side is not a comparison)",
            crease_folds.len(),
            soft_folds.len()
        ));
    } else if separation < CREASE_SEPARATION {
        failures.push(format!(
            "3  the crease is the sharpest thing on the sheet ({smallest_crease:.3e} against a soft \
             worst of {worst_soft:.3e}, a separation of {separation:.2} where {CREASE_SEPARATION:.2} \
             is the claim)"
        ));
    }
    if worst_soft > SOFT_CEILING {
        failures.push(format!(
            "3  the soft-edge gap has not grown ({} edges, worst {worst_soft:.3e} \
             against a ceiling of {SOFT_CEILING:.3e})",
            soft_folds.len(),
        ));
    }
    if smallest_crease < CREASE_FOLD {
        failures.push(format!(
            "3  the crease folds, so its two faces disagree on a normal ({smallest_crease:.3e} \
             against a floor of {CREASE_FOLD:.3e})"
        ));
    }
    report(
        "3  the crease is the sharpest thing on the sheet",
        &format!(
            "soft edges measured                  {}\n\
             their median fold                    {median_soft:.3e} rad ({:.2} deg)\n\
             their worst fold                     {worst_soft:.3e} rad ({:.2} deg)\n\
             crease edges measured               {}\n\
             smallest fold along the crease     {smallest_crease:.3e} rad ({:.2} deg)\n\
             largest fold along the crease      {largest_crease:.3e} rad ({:.2} deg)\n\
             separation, crease over soft worst  {separation:.2}\n\
             \n\
             These are two numbers with opposite required signs, which is what makes the pair a\n\
             measurement rather than a description: a seam must fold by *less* than the crease does.\n\
             A soft edge reading zero is **not** what is asserted here. `patch::continuity` states that\n\
             a soft edge is measurably smoother than a crease and not continuous across the seam, and\n\
             an earlier version of this probe asserted zero, read a real 0.2 rad, and reported a\n\
             failure that was a fact about the crate rather than about this scene.",
            soft_folds.len(),
            median_soft.to_degrees(),
            worst_soft.to_degrees(),
            crease_folds.len(),
            smallest_crease.to_degrees(),
            largest_crease.to_degrees(),
        ),
    );

    // 4 -- the drawn triangles agree with the surface under them.
    let tessellation = sheet.tessellate(7);
    let triangles = tessellation.triangles();
    let points = tessellation.points();
    let last = 7 - 1;
    let di_dj = [(2.0 / 3.0, 1.0 / 3.0), (1.0 / 3.0, 2.0 / 3.0)];
    let mut facing_away = 0usize;
    let mut faces_down = 0usize;
    for (index, triangle) in triangles.iter().enumerate() {
        let per_face = last * last * 2;
        let face = index / per_face;
        let within = index % per_face;
        let cell = within / 2;
        let (i, j) = (cell / last, cell % last);
        let (di, dj) = di_dj[within % 2];
        let reference = composed
            .face(face)
            .expect("a face below the triangle count resolves")
            .normal((i as f64 + di) / last as f64, (j as f64 + dj) / last as f64);
        if reference.y < 0.0 {
            faces_down += 1;
        }
        let [a, b, c] = triangle.map(|index| points[index]);
        if (b - a).cross(c - a).dot(reference) <= 0.0 {
            facing_away += 1;
        }
    }
    if facing_away != 0 {
        failures.push(format!(
            "4  every triangle is wound to agree with the patch under it ({facing_away} of {} face \
             away, and {} of {} patches point down at their own middle)",
            triangles.len(),
            faces_down,
            composed.len()
        ));
    }
    report(
        "4  the drawn triangles agree with the surface",
        &format!(
            "triangles                    {}\n\
             facing away from their patch {facing_away}\n\
             patches pointing down         {faces_down} of {}\n\
             \n\
             The same check `raster_probe` runs on the massif, on a surface whose winding this scene\n\
             controls rather than inherits. It is here because a scene document holds points and no\n\
             winding of its own: the winding is decided when the surface is sampled, and nothing\n\
             downstream of that point can check it.\n\
             \n\
             The second number separates the two ways this can fail. A triangle facing away while every\n\
             patch points up is a winding mistake and nothing else. A patch pointing **down** is the\n\
             surface folding over itself, which no winding fixes.",
            triangles.len(),
            composed.len()
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

/// The geometric normal a triangle's own corners give it, used above through `cross` directly.
#[allow(dead_code)]
fn triangle_normal(points: &[Point3], triangle: [usize; 3]) -> Point3 {
    let [a, b, c] = triangle.map(|index| points[index]);
    (b - a).cross(c - a)
}
