//! Which lights reach which part of the view, from the scene document to the three buffers.
//!
//! # What these prove
//!
//! The assignment is the one part of this effort with no adapter behind it, so it is also the
//! one part that can be checked properly. The cases here are the ones that are easy to get
//! quietly wrong:
//!
//! - a light that reaches no cluster, one that reaches every cluster, and a cluster no light
//!   reaches. An empty result is a correct result: a kind that covers the whole view is in no
//!   cluster by design, so a scene whose lights are only those kinds has empty lists and that
//!   is not a missing assignment.
//! - a light whose volume straddles a slice boundary, and the depth distribution at both ends
//!   of the frustum. The slices divide depth rather than distance, which is only visible as a
//!   difference in where a given distance lands, so it is asserted as one.
//! - a cluster over its cap, where the ranking has to keep the highest contributing lights and
//!   not the first ones to arrive. A light that is first in the document order and dim is
//!   dropped in favour of a brighter one behind it, which is the case a truncation gets wrong.
//! - a grid the scene says nothing about, which has to read as the documented default. A
//!   default that is wrong is a number an author silently inherits.
//!
//! The last is the regression bar for the stage: an all `Point` scene emits the passes it
//! emitted before, with the same buffer sizes, and gains three entries that nothing binds yet.

use definition::{Definition, DefinitionIdentifier, Path, Registry, SchemaIdentifier, Value};
use graphics::{ENTRIES, GRAPHICS_SCHEMA, INSTANCE_SCHEMA};
use whiting_gallery::Gallery;
use whiting_scene::CompileContext;
use whiting_scene::CompileOutput;
use whiting_scene::Compiler;
use whiting_scene::lighting::{
    CLUSTER_COUNTS_ENTRY, CLUSTER_INDICES_ENTRY, ClusterLight, Clustering, Clusters,
    LIGHT_RECORDS_ENTRY, LightPool, assign,
};
use whiting_scene::standard::{
    Bounds, CLUSTERING, Camera, DEPTH_EXTENTS, LIGHTS_PER_CLUSTER, Projection, WIDTH_EXTENTS,
    example_standard_scene, register_standard_scene_schema,
};
use whiting_scene::{CompileError, STANDARD_SCENE_SCHEMA, StandardScene};

/// A box the clusters are tested against, chosen so its numbers can be read off by hand.
const WIDE: f64 = 16.0;

#[test]
fn an_all_point_scene_still_emits_the_passes_it_emitted_before() {
    let compiled = compiled(&example_standard_scene());

    let labels: Vec<&str> = graphics::passes(&compiled.graphics.value)
        .expect("the passes read")
        .iter()
        .map(|pass| pass.label().expect("a pass is labelled"))
        .collect();
    // The lighting pass no longer names a light, so its label lost the `key/` that the one
    // light of this scene used to contribute. The shadowing pass still does name one, because
    // it is still one per light that was given a page.
    assert_eq!(
        labels,
        [
            "shadow/key/shadowing/triangle",
            "depth/depthing/triangle",
            "lighting/lighting/triangle",
        ],
        "one pass per stage, and none of them named twice over"
    );

    // Every buffer that existed before this stage, at the size it had.
    for (entry, size) in [
        ("triangle_vertices", 72),
        ("shadow/key/shadowing/triangle_instance_data", 76),
        ("depth/depthing/triangle_instance_data", 76),
        ("lighting/lighting/triangle_instance_data", 152),
    ] {
        assert_eq!(
            buffer_bytes(&compiled.instance, entry),
            size as i64,
            "the entry `{entry}` is the size it was"
        );
    }

    // And the three this stage adds: one record of twenty four floats for the one light —
    // colour, page, position, radius and the light's own projection — one count per cluster of
    // the default grid, and one row of the default cap per cluster.
    let clusters = Clustering::convention().cluster_count() as i64;
    let stride = whiting_scene::lighting::light_record_stride() as i64;
    assert_eq!(
        stride * 4,
        96,
        "the record is twenty four floats, and the stride is stated rather than written down \
         so the assertion and the packing read one number"
    );
    for (entry, size) in [
        (LIGHT_RECORDS_ENTRY, stride * 4),
        (CLUSTER_COUNTS_ENTRY, clusters * 4),
        (CLUSTER_INDICES_ENTRY, clusters * 64 * 4),
    ] {
        assert_eq!(
            buffer_bytes(&compiled.instance, entry),
            size,
            "the entry `{entry}` is the size the grid and the cap imply"
        );
    }

    assert!(
        compiled.notes.is_empty(),
        "a light per cluster is a scene that dropped nothing: {:?}",
        compiled.notes
    );
}

/// How many passes a scene emits of each stage, which is the whole of what a pass count is.
///
/// # Invariants
///
/// The **lighting** number is the scene's groups of *lit* geometry and nothing else: it is what
/// the clustered pass bought, and it is what the acceptance bar in `shadow_pages.rs` holds
/// still while a light is added.
///
/// The **shadowing** number is the lights the pool bought pages for, times the groups of
/// *shadow casting* geometry, so it is bounded by the pool rather than by the scene's light
/// count: a scene with more lights than pages draws only as many shadow maps as it has pages,
/// and lights the rest unshadowed.
///
/// **The lighting count is not the depthing count**, and the difference is real rather than
/// rounding: a mesh may be drawn in the depthing stage and not in the lighting one, as an
/// occluder is. `tidewater` is the case in the gallery — four groups in the depth prepass and
/// three lit — so a claim that the two are equal would be true of five of the seven scenes and
/// wrong about the other two.
#[derive(Debug, PartialEq, Eq)]
struct Stages {
    shadowing: usize,
    depthing: usize,
    lighting: usize,
    other: usize,
}

impl Stages {
    fn of(scene: &Value) -> Self {
        let compiled = compiled(scene);
        let passes = graphics::passes(&compiled.graphics.value).expect("the passes read");
        let count = |prefix: &str| {
            passes
                .iter()
                .filter(|pass| {
                    pass.label()
                        .expect("a pass is labelled")
                        .starts_with(prefix)
                })
                .count()
        };
        let stages = (
            count("shadow/"),
            count("depth/"),
            count("lighting/"),
            passes.len(),
        );
        Self {
            shadowing: stages.0,
            depthing: stages.1,
            lighting: stages.2,
            // Everything else: the emission stage and the post processes, neither of which
            // reads a light and neither of which this stage touched.
            other: stages.3 - stages.0 - stages.1 - stages.2,
        }
    }

    fn total(&self) -> usize {
        self.shadowing + self.depthing + self.lighting + self.other
    }
}

#[test]
fn every_gallery_scene_still_emits_the_pass_count_it_emitted_before() {
    // The counts move, and this is where the movement is pinned. Each is a function of the
    // scene's **geometry** — how many groups of mesh — and, for the shadowing stage, of how
    // many pages the pool bought; none is a function of the scene's light count.
    // The list is `Gallery::names()` rather than a list written out here, because a list
    // written out here is a list that goes stale: `tidewater` landed in the gallery and this
    // test carried on covering the six scenes it was written for. A scene added to the
    // gallery now has to be added to the table and cannot be forgotten.
    let counted: Vec<(String, Stages)> = Gallery::names()
        .iter()
        .map(|name| ((*name).to_owned(), Stages::of(&Gallery::build(name))))
        .collect();
    let expected: Vec<(&str, Stages)> = vec![
        (
            "dunes",
            Stages {
                shadowing: 2,
                depthing: 1,
                lighting: 1,
                other: 1,
            },
        ),
        (
            "lanterns",
            Stages {
                shadowing: 9,
                depthing: 3,
                lighting: 3,
                other: 2,
            },
        ),
        (
            "monoliths",
            Stages {
                shadowing: 2,
                depthing: 2,
                lighting: 2,
                other: 1,
            },
        ),
        (
            "spotlight",
            Stages {
                shadowing: 4,
                depthing: 4,
                lighting: 4,
                other: 1,
            },
        ),
        (
            // Eleven lights and eight groups. The shadowing count is the one that moved: a
            // light with a page costs a pass per group, which is what a scene pays for keeping
            // a window's softness as a grid of point lights.
            "stilllife",
            Stages {
                shadowing: 88,
                depthing: 8,
                lighting: 8,
                other: 1,
            },
        ),
        (
            "studio",
            Stages {
                shadowing: 1,
                depthing: 1,
                lighting: 1,
                other: 0,
            },
        ),
        (
            "sunlight",
            // A floor and four boxes, and one light. A shadowing pass is one per
            // (light, shader, mesh), so a scene of five drawn meshes under one sun emits five
            // of them — **and the sun counts**. It takes no cluster slot and is in no list, so
            // a light that were wrongly given no page would emit **no** shadowing pass here at
            // all, and the scene would render with no shadow in it and nothing to say so.
            Stages {
                shadowing: 5,
                depthing: 5,
                lighting: 5,
                other: 1,
            },
        ),
        (
            "terraces",
            Stages {
                shadowing: 12,
                depthing: 6,
                lighting: 6,
                other: 1,
            },
        ),
        (
            "tidewater",
            Stages {
                shadowing: 6,
                depthing: 4,
                lighting: 3,
                other: 2,
            },
        ),
        (
            "tiles",
            Stages {
                shadowing: 1,
                depthing: 1,
                lighting: 1,
                other: 1,
            },
        ),
    ];
    assert_eq!(
        counted,
        expected
            .into_iter()
            .map(|(name, stages)| (name.to_owned(), stages))
            .collect::<Vec<(String, Stages)>>(),
        "the passes of each gallery scene by stage, after the clustered pass replaced the \
         per light ones"
    );
}

/// The totals the same scenes emitted before, which is the claim the stage above is replacing.
///
/// Kept as a plain list so the move is legible in one place. `dunes` 6 to 5, `lanterns` 23 to
/// 17, `monoliths` 7 to 7, `studio` 3 to 3, `terraces` 31 to 25, `tidewater` 18 to 15, `tiles` 4
/// to 4. Three of the seven did not move at all, which is the honest result: a scene whose
/// groups are few and whose lights are few gains nothing from the swap, and a scene with the
/// most of both is where the count actually moved.
#[test]
fn the_totals_shrank_where_the_scene_had_more_lights_than_groups() {
    let before: Vec<(&str, usize)> = vec![
        ("dunes", 6),
        ("lanterns", 23),
        ("monoliths", 7),
        ("studio", 3),
        ("terraces", 31),
        ("tidewater", 18),
        ("tiles", 4),
    ];
    for (name, was) in before {
        let after = Stages::of(&Gallery::build(name)).total();
        assert!(
            after <= was,
            "{name} emitted {was} passes and emits {after}, so the clustered pass never added one"
        );
    }
}

#[test]
fn a_grid_the_scene_says_nothing_about_is_the_documented_one() {
    let clustering = Clustering::read(&Value::structure([])).expect("a scene with no grid reads");
    assert_eq!(clustering.width_extents, 16);
    assert_eq!(clustering.height_extents, 9);
    assert_eq!(clustering.depth_extents, 24);
    assert_eq!(clustering.lights_per_cluster, 64);
    assert_eq!(clustering.cluster_count(), 3456);

    // One extent on its own, and the rest of the grid every scene gets.
    let written = Value::structure([(
        CLUSTERING.to_owned(),
        Value::structure([(WIDTH_EXTENTS.to_owned(), Value::Int(4))]),
    )]);
    let clustering = Clustering::read(&written).expect("a written extent reads");
    assert_eq!(
        (clustering.width_extents, clustering.height_extents),
        (4, 9)
    );
}

#[test]
fn a_grid_of_nothing_is_refused_rather_than_read_as_no_clusters() {
    let scene = Value::structure([(
        CLUSTERING.to_owned(),
        Value::structure([(DEPTH_EXTENTS.to_owned(), Value::Int(0))]),
    )]);
    let refused = Clustering::read(&scene).expect_err("zero slices is refused");
    assert!(
        matches!(refused, CompileError::EmptyClustering { found: 0, .. }),
        "the refusal names the count: {refused}"
    );

    let cap = Value::structure([(
        CLUSTERING.to_owned(),
        Value::structure([(LIGHTS_PER_CLUSTER.to_owned(), Value::Int(0))]),
    )]);
    assert!(
        matches!(
            Clustering::read(&cap),
            Err(CompileError::EmptyClustering { .. })
        ),
        "a cap of nothing keeps no light at all, which is also refused"
    );
}

#[test]
fn a_light_behind_the_eye_is_recorded_and_reaches_no_cluster() {
    let grid = grid(2, 2, 2, 4);
    // The parallel camera stands at z twelve, so a light at z twenty is eight units behind it.
    let behind = light("behind", [0.0, 0.0, 20.0], 1.0);
    let clusters = assign(&grid, &[behind], &parallel(), &pool()).expect("the assignment runs");

    assert_eq!(clusters.records.len(), 1, "the light has a record");
    assert_eq!(count_of(&clusters, 0), 0.0, "and no cluster holds it");
    assert!(
        clusters.counts.bytes().iter().all(|byte| *byte == 0),
        "every cluster is empty, which is a correct answer rather than a missing one"
    );
    assert!(clusters.overflows.is_empty());
}

#[test]
fn a_light_covering_the_view_is_in_every_cluster() {
    let grid = grid(2, 2, 2, 4);
    let everything = light("everything", [0.0, 0.0, 0.0], 1000.0);
    let clusters = assign(&grid, &[everything], &parallel(), &pool()).expect("the assignment runs");

    assert_eq!(clusters.counts.len(), 8);
    for cluster in 0..8 {
        assert_eq!(count_of(&clusters, cluster), 1.0, "cluster {cluster}");
        assert_eq!(light_at(&clusters, cluster, 0), 0.0);
    }
}

#[test]
fn a_light_off_to_one_side_reaches_no_cluster() {
    let grid = grid(2, 2, 2, 4);
    // A thousand units to the side of a frame eight and a bit wide, at a distance in the middle
    // of the frustum: the failure here is lateral, not one of depth.
    let aside = light("aside", [1000.0, 0.0, -2.0], 1.0);
    let clusters = assign(&grid, &[aside], &parallel(), &pool()).expect("the assignment runs");

    assert_eq!(clusters.records.len(), 1);
    assert_eq!(clusters.counts.len(), 8);
    assert!(
        clusters.counts.bytes().iter().all(|byte| *byte == 0),
        "no cluster of the frame is anywhere near it"
    );
}

#[test]
fn a_volume_straddling_a_slice_boundary_is_in_both_slices() {
    // A parallel view divides depth and distance alike, so its boundaries can be read off: the
    // convention camera frames a box of half width four, stands three box radii back, and
    // therefore has a near plane at five and a far plane at twenty, which four slices divide
    // at 5.0, 8.75, 12.5, 16.25 and 20.0. A light two and a half units across the 12.5
    // boundary is in the slice before it and the slice after, and in neither of the others.
    let grid = grid(1, 1, 4, 4);
    let across = light("across", parallel_at(12.5), 2.5);
    let clusters = assign(&grid, &[across], &parallel(), &pool()).expect("the assignment runs");

    let reached: Vec<usize> = (0..4)
        .filter(|slice| count_of(&clusters, *slice) > 0.0)
        .collect();
    assert_eq!(reached, [1, 2], "the two slices its volume reaches");
}

#[test]
fn the_slices_divide_depth_rather_than_distance() {
    // A perspective view with a near plane at one and a far plane at twenty one puts its slice
    // boundaries at 1.0, 1.31, 1.91, 3.5 and 21.0: the first slice is three tenths of a unit
    // deep and the last is seventeen and a half. A grid that divided the distance instead
    // would put them five apart, so a light two and a half units out would land in its first
    // slice — it is seven and a half per cent of the way along the distances — and here it is
    // in its third. That is the whole of what dividing depth rather than distance means.
    let grid = grid(1, 1, 4, 4);
    let two_and_a_half_out = light("out", perspective_at(2.5), 0.1);
    let clusters =
        assign(&grid, &[two_and_a_half_out], &perspective(), &pool()).expect("the assignment runs");

    let reached: Vec<usize> = (0..4)
        .filter(|slice| count_of(&clusters, *slice) > 0.0)
        .collect();
    assert_eq!(reached, [2], "two thirds of the way along the depth range");
}

#[test]
fn a_light_at_each_plane_lands_in_the_slice_at_that_plane() {
    let grid = grid(1, 1, 4, 4);
    let near = light("near", perspective_at(1.0), 0.1);
    let far = light("far", perspective_at(21.0), 0.1);
    let clusters =
        assign(&grid, &[near, far], &perspective(), &pool()).expect("the assignment runs");

    assert_eq!(
        count_of(&clusters, 0),
        1.0,
        "the near plane is the first slice"
    );
    assert_eq!(count_of(&clusters, 3), 1.0, "the far plane is the last one");
    assert_eq!(count_of(&clusters, 1), 0.0);
    assert_eq!(count_of(&clusters, 2), 0.0);
}

#[test]
fn a_cluster_over_its_cap_keeps_the_highest_contributing_lights() {
    // One cluster, and two lights that reach it. The first in document order is dim and sits
    // its radius away from the box, so it is the weakest light in the cluster; the second is
    // brighter and stands inside it. A cap of one has to keep the second.
    let grid = grid(1, 1, 1, 1);
    let dim = ClusterLight {
        label: "dim".to_owned(),
        position: [WIDE / 2.0 + 4.0, 0.0, -0.5],
        colour: [0.5, 0.5, 0.5],
        radius: 4.0,
        shadow_projection: identity(),
        clustered: true,
        casts_shadow: true,
    };
    let bright = ClusterLight {
        label: "bright".to_owned(),
        position: [0.0, 0.0, -0.5],
        colour: [1.0, 1.0, 1.0],
        radius: 4.0,
        shadow_projection: identity(),
        clustered: true,
        casts_shadow: true,
    };
    let clusters =
        assign(&grid, &[dim, bright], &parallel(), &pool()).expect("the assignment runs");

    assert_eq!(count_of(&clusters, 0), 1.0, "the cap is one light");
    assert_eq!(
        light_at(&clusters, 0, 0),
        1.0,
        "the brighter light is kept and the first one to arrive is not"
    );

    // And the author is told, in one note, with the count and the name of what went missing.
    assert_eq!(clusters.lights_dropped, 1);
    assert_eq!(clusters.overflows.len(), 1);
    assert_eq!(clusters.overflows[0].reached, 2);
    assert_eq!(clusters.overflows[0].dropped, ["dim".to_owned()]);

    // **One** note, not one per overflowing cluster: the cluster is an interior of a grid the
    // author recomputes, and the label is the string they go and edit.
    let notes = clusters.notes(&grid, &Path::root().key(CLUSTERING));
    assert_eq!(
        notes.len(),
        1,
        "one note for the whole view, whatever overflowed"
    );
    assert!(
        notes[0].message.contains("1 of the 1 clusters")
            && notes[0].message.contains("1 of the light entries"),
        "the note counts: {}",
        notes[0].message
    );
    assert!(
        notes[0].message.contains("`dim`"),
        "and it names the light that was dropped, in backticks: {}",
        notes[0].message
    );
    assert!(
        notes[0].path.to_string().contains(LIGHTS_PER_CLUSTER),
        "and it points at the field the author changes: {}",
        notes[0].path
    );
}

#[test]
fn a_light_that_covers_the_view_takes_no_cluster_slot() {
    // The exemption: a kind that reaches everywhere is in no cluster by design, so a scene of
    // nothing but those kinds emits a full record buffer and a wholly empty pair of lists.
    let grid = grid(1, 1, 2, 4);
    let sun = ClusterLight {
        label: "sun".to_owned(),
        position: [0.0, 0.0, -0.5],
        colour: [1.0, 1.0, 1.0],
        radius: 1.0,
        shadow_projection: identity(),
        clustered: false,
        casts_shadow: true,
    };
    let clusters = assign(&grid, &[sun], &parallel(), &pool()).expect("the assignment runs");

    assert_eq!(
        clusters.records.len(),
        1,
        "its record is still there for stage 3b"
    );
    assert!(clusters.counts.bytes().iter().all(|byte| *byte == 0));
    assert!(
        clusters
            .notes(&grid, &Path::root().key(CLUSTERING))
            .is_empty(),
        "nothing was dropped, so there is nothing to report"
    );
}

/// Returns the bytes a named buffer entry carries.
fn buffer_bytes(instance: &Definition, entry: &str) -> i64 {
    entries_of(instance)
        .get(entry)
        .unwrap_or_else(|| panic!("the entry `{entry}` is emitted"))
        .variant_fields()
        .and_then(|fields| fields.get(graphics::binders::SIZE_BYTES))
        .and_then(Value::as_int)
        .unwrap_or_else(|| panic!("the entry `{entry}` is a buffer"))
}

/// Returns the instance entries a compile emitted.
fn entries_of(instance: &Definition) -> std::collections::BTreeMap<String, Value> {
    instance
        .value
        .as_field_map()
        .and_then(|fields| fields.get(ENTRIES))
        .and_then(Value::as_dict)
        .expect("an instance holds its entries")
        .clone()
}

/// Returns the grid these tests use, which is small enough to read.
fn grid(width: usize, height: usize, depth: usize, cap: usize) -> Clustering {
    Clustering {
        width_extents: width,
        height_extents: height,
        depth_extents: depth,
        lights_per_cluster: cap,
    }
}

/// Returns a pool big enough that no test here runs out of pages, so the assignment tests
/// measure the assignment rather than the pool.
///
/// The tests that are about the pool pass their own.
fn pool() -> LightPool {
    LightPool {
        page_width: 1024,
        page_height: 1024,
        pages: 64,
    }
}

/// Returns a point light with the reach the tests need.
fn light(label: &str, position: [f64; 3], radius: f64) -> ClusterLight {
    ClusterLight {
        label: label.to_owned(),
        position,
        colour: [1.0, 1.0, 1.0],
        radius,
        shadow_projection: identity(),
        clustered: true,
        casts_shadow: true,
    }
}

/// Returns the identity matrix, which is the projection a light no shadow is looked up
/// through carries.
///
/// The assignment does not read it and the packing only writes it, so a test that is about
/// which lights reach which clusters has nothing to say about a projection — but the record
/// has to carry one, and a light with no page is never looked up through it.
fn identity() -> [f64; 16] {
    [
        1.0, 0.0, 0.0, 0.0, //
        0.0, 1.0, 0.0, 0.0, //
        0.0, 0.0, 1.0, 0.0, //
        0.0, 0.0, 0.0, 1.0,
    ]
}

/// Returns how many lights a cluster kept.
fn count_of(clusters: &Clusters, cluster: usize) -> f64 {
    clusters
        .counts
        .component(cluster, "count", 0)
        .expect("every cluster has a count")
}

/// Returns the light in one slot of one cluster.
fn light_at(clusters: &Clusters, cluster: usize, slot: usize) -> f64 {
    let cap = clusters.indices.len() / clusters.counts.len();
    clusters
        .indices
        .component(cluster * cap + slot, "light", 0)
        .expect("every cluster has a row of the cap")
}

/// Returns a parallel camera: the convention one, framing a box of half width four.
///
/// Its eye stands at z twelve looking at the origin, its frame is eight and a bit wide each
/// way, and its depth range is five to twenty — so a world position is a distance from the eye
/// by subtracting its z from twelve, which is what the two helpers below do.
fn parallel() -> Projection {
    Camera::read(None, &Path::root(), &bounds(), 1.0)
        .expect("the convention camera reads")
        .projection()
}

/// Returns a world position a given distance in front of the parallel camera.
fn parallel_at(distance: f64) -> [f64; 3] {
    [0.0, 0.0, 12.0 - distance]
}

/// Returns a perspective camera with its near plane at one and its far plane at twenty one,
/// its eye standing at z ten.
fn perspective() -> Projection {
    let triple = |v: [f64; 3]| Value::List(v.into_iter().map(Value::Float).collect());
    Camera::read(
        Some(&Value::structure([
            ("eye".to_owned(), triple([0.0, 0.0, 10.0])),
            ("target".to_owned(), triple([0.0, 0.0, 0.0])),
            ("projection".to_owned(), Value::variant("Perspective", [])),
            ("near".to_owned(), Value::Float(1.0)),
            ("far".to_owned(), Value::Float(21.0)),
        ])),
        &Path::root(),
        &bounds(),
        1.0,
    )
    .expect("a written camera reads")
    .projection()
}

/// Returns a world position a given distance in front of the perspective camera.
fn perspective_at(distance: f64) -> [f64; 3] {
    [0.0, 0.0, 10.0 - distance]
}

/// Returns the box the cameras frame, whose half width the tests read their light positions off.
fn bounds() -> Bounds {
    Bounds::around([0.0, 0.0, 0.0], 4.0)
}

/// Compiles a scene the way a host would, and returns what it emitted.
fn compiled(scene: &Value) -> CompileOutput {
    let mut registry = Registry::new();
    registry.put_schema(
        SchemaIdentifier::new(GRAPHICS_SCHEMA),
        graphics::graphics_schema(),
    );
    registry.put_schema(
        SchemaIdentifier::new(INSTANCE_SCHEMA),
        graphics::instance_schema(),
    );
    register_standard_scene_schema(&mut registry);
    let source = Definition::new(
        DefinitionIdentifier::new("clustered.source"),
        SchemaIdentifier::new(STANDARD_SCENE_SCHEMA),
        scene.clone(),
    );
    source
        .validate(&registry)
        .expect("the scene validates against the schema");
    StandardScene::new(
        DefinitionIdentifier::new("clustered.graphics"),
        DefinitionIdentifier::new("clustered.instance"),
    )
    .compile(&CompileContext {
        scene: &source,
        previous_graphics: None,
        previous_instance: None,
        registry: &registry,
        camera: 0,
    })
    .expect("the scene compiles")
}
