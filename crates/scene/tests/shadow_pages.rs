//! Which lights are given a shadow page, and what a scene is told when the pool cannot hold
//! them all.
//!
//! # What these prove
//!
//! The pool is a budget, and a budget is only honest if it is accounted for. The cases here are
//! the ones where it is easy to lose a light quietly:
//!
//! - a pool larger than the scene's lights, where every light that reaches the view gets a page
//!   and nothing is reported;
//! - a pool smaller, where the ranking keeps the brightest rather than the first, the ones that
//!   missed are counted, and the note carries the numbers an author needs to act on;
//! - a light that reaches no cluster, which is given no page and is not reported as having been
//!   dropped by a full pool, because nothing was dropped — it was never a candidate;
//! - a light of a kind that takes no cluster slot, which casts no shadow and so is given no
//!   page however bright it is;
//! - the pool's own shape, refused rather than truncated at the array limit.

use definition::{Definition, DefinitionIdentifier, Path, Registry, SchemaIdentifier, Value};
use whiting_gallery::Gallery;
use whiting_scene::CompileContext;
use whiting_scene::Compiler;
use whiting_scene::lighting::{
    ClusterLight, Clustering, Clusters, LightPool, MAX_TEXTURE_ARRAY_LAYERS, NO_PAGE, SHADOW_PAGES,
    assign, read_light_pool,
};
use whiting_scene::standard::{Bounds, Camera, FRAME, Projection, register_standard_scene_schema};
use whiting_scene::{CompileError, CompileNote, STANDARD_SCENE_SCHEMA, StandardScene};

#[test]
fn a_pool_larger_than_the_scene_gives_every_reaching_light_a_page_and_reports_nothing() {
    let lights = vec![light("dim", 0.5), light("bright", 1.0)];
    let clusters = assign(&grid(), &lights, &parallel(), &pool_of(8)).expect("the assignment runs");

    assert_eq!(
        clusters.page_of(1),
        Some(0),
        "the brightest light takes the first page, whatever order the document lists them in"
    );
    assert_eq!(clusters.page_of(0), Some(1));
    assert!(
        notes(&lights, &pool_of(8), &clusters).is_empty(),
        "nothing went without a page, so there is nothing to report"
    );
}

#[test]
fn a_pool_that_cannot_hold_the_scene_keeps_the_brightest_and_says_so_with_numbers() {
    // Three lights and one page. The first in document order is the dimmest and the furthest,
    // so a pool that kept the first one to be offered would be keeping the wrong light.
    let lights = vec![
        light("dim", 0.05),
        light("bright", 4.0),
        light("middle", 1.0),
    ];
    let full = pool_of(1);
    let clusters = assign(&grid(), &lights, &parallel(), &full).expect("the assignment runs");

    assert_eq!(clusters.page_of(1), Some(0), "the brightest took the page");
    assert_eq!(
        (clusters.page_of(0), clusters.page_of(2)),
        (None, None),
        "the other two are lit by the approximation instead"
    );

    let notes = notes(&lights, &full, &clusters);
    assert_eq!(notes.len(), 1, "one note, about the pool as a whole");
    let message = &notes[0].message;
    assert!(
        message.contains("1 pages") && message.contains("1 of the 3 lights"),
        "the note counts the pages, the total and how many were given: {message}"
    );
    assert!(
        message.contains("4194304"),
        "the note states what the whole pool costs in bytes: {message}"
    );
    assert!(
        notes[0].path.to_string().contains(SHADOW_PAGES),
        "and it points at the field the author changes: {}",
        notes[0].path
    );
}

#[test]
fn a_light_that_reaches_nothing_is_not_reported_as_dropped_by_a_full_pool() {
    // Behind the eye, so it lights nothing and reaches no cluster. A full pool did not cost it
    // a page, so counting it as dropped would send an author to raise a pool that was never
    // the problem.
    let behind = ClusterLight {
        label: "behind".to_owned(),
        position: [0.0, 0.0, 20.0],
        colour: [1.0, 1.0, 1.0],
        radius: 1.0,
        shadow_projection: identity(),
        clustered: true,
        casts_shadow: true,
    };
    let clusters =
        assign(&grid(), &[behind.clone()], &parallel(), &pool_of(1)).expect("the assignment runs");

    assert_eq!(clusters.page_of(0), None, "nothing it lights has a page");
    assert!(
        notes(&[behind], &pool_of(1), &clusters).is_empty(),
        "a pool that was not full reports nothing"
    );
}

#[test]
fn a_light_that_takes_no_cluster_slot_and_casts_no_shadow_is_given_no_page() {
    // The environment light: exempt from every cluster **and** casting nothing, so a page for
    // it would be a page rendering nothing however bright it is.
    let sky = ClusterLight {
        label: "sky".to_owned(),
        position: [0.0, 0.0, 0.0],
        colour: [1.0, 1.0, 1.0],
        radius: 1000.0,
        shadow_projection: identity(),
        clustered: false,
        casts_shadow: false,
    };
    let clusters =
        assign(&grid(), &[sky.clone()], &parallel(), &pool_of(4)).expect("the assignment runs");

    assert_eq!(clusters.page_of(0), None);
    assert!(
        notes(&[sky], &pool_of(4), &clusters).is_empty(),
        "a light that was never a candidate is not a light the pool dropped"
    );
}

#[test]
fn a_light_that_takes_no_cluster_slot_but_casts_a_shadow_is_still_given_a_page() {
    // The directional light, and the one place this branch can be wrong in a way that renders
    // fine. It is exempt from every cluster, so the candidate walk never records it as having
    // reached one, and a test that asked only whether the walk saw it would hand a sun no
    // page — and a fragment tested against page minus one is unshadowed, so the scene would
    // render with no shadow in it and nothing to say so.
    let sun = ClusterLight {
        label: "sun".to_owned(),
        position: [0.0, 0.0, 0.0],
        colour: [1.0, 1.0, 1.0],
        radius: 1000.0,
        shadow_projection: identity(),
        clustered: false,
        casts_shadow: true,
    };
    let clusters =
        assign(&grid(), &[sun.clone()], &parallel(), &pool_of(4)).expect("the assignment runs");

    assert_eq!(
        clusters.page_of(0),
        Some(0),
        "the exemption is a decision about a cluster's cap, and a page is not a cluster slot"
    );
    assert!(
        notes(&[sun], &pool_of(4), &clusters).is_empty(),
        "a light the pool ranked first was not dropped"
    );
}

#[test]
fn a_record_says_no_page_with_a_value_no_real_page_can_carry() {
    // The record's page is minus one for a light with none. If it were zero, a shader reading
    // it would look up the first page for a light that has no shadow map, which is a lookup
    // into somebody else's shadow.
    let clusters = assign(
        &grid(),
        &[light("reaching", 0.0), light("hidden", 0.0)],
        &parallel(),
        &pool_of(1),
    )
    .expect("the assignment runs");

    assert_eq!(clusters.page_of(0), Some(0), "page zero is the first page");
    assert_eq!(
        clusters.page_of(1),
        None,
        "and a light with no page is not page zero"
    );
    assert!(
        NO_PAGE < 0.0,
        "the sentinel is below every legal page index"
    );
}

#[test]
fn a_pool_deeper_than_the_array_limit_is_refused_rather_than_truncated() {
    let too_deep = Value::structure([(
        FRAME.to_owned(),
        Value::structure([(
            SHADOW_PAGES.to_owned(),
            Value::Int(MAX_TEXTURE_ARRAY_LAYERS as i64 + 1),
        )]),
    )]);
    let refused = read_light_pool(&too_deep).expect_err("257 pages is refused");
    assert!(
        matches!(refused, CompileError::PoolTooDeep { limit, .. } if limit == 256),
        "the refusal names the limit: {refused}"
    );

    // And a pool of nothing is refused on the same grounds a grid of nothing is.
    let empty = Value::structure([(
        FRAME.to_owned(),
        Value::structure([(SHADOW_PAGES.to_owned(), Value::Int(0))]),
    )]);
    assert!(
        matches!(
            read_light_pool(&empty),
            Err(CompileError::EmptyClustering { .. })
        ),
        "a pool of nothing is a scene whose every light is approximate"
    );
}

#[test]
fn a_pool_the_scene_says_nothing_about_is_the_documented_one() {
    let pool = read_light_pool(&Value::structure([])).expect("a scene with no pool reads");
    assert_eq!(
        (pool.page_width, pool.page_height, pool.pages),
        (1024, 1024, 64)
    );
    assert_eq!(
        pool.total_bytes(),
        1024 * 1024 * 4 * 64,
        "four bytes a texel, the format the standard pipeline renders depth in"
    );
}

/// The acceptance bar for the whole effort: the **lighting** pass count is a function of the
/// scene's geometry and not of how many lights it has.
///
/// # Why this compares the lighting passes and not all of them
///
/// It used to compare every pass, and it could only be written that way because every pass
/// named a light. It cannot be written that way now, and the reason is a limit of the api
/// rather than a choice: **a render pass attaches exactly one depth image.** A light's page
/// is rendered from that light's own eye, so two lights are two passes however the shading
/// reads them, and no amount of clustering collapses the shadowing stage. A bar comparing
/// every label would be a bar that can never go green, and a bar that reports nothing is
/// worse than no bar.
///
/// What replaced it is the second test: the shadowing count is bounded by the **pool**, which
/// is the budget the scene controls, rather than by the light count. Together the two say
/// what the design actually claims — the shading is O(groups) and the shadow rendering is
/// O(min(lights, pages) * groups).
#[test]
fn the_lighting_pass_count_does_not_move_when_a_light_is_added() {
    let before = Gallery::build("lanterns");
    let one = light_count(&before);
    assert!(one > 1, "lanterns starts with several lights, not {one}");

    let after = with_one_more_light(&before);
    assert_eq!(
        light_count(&after),
        one + 1,
        "the added light is in the document"
    );
    assert_eq!(
        lighting_pass_labels(&after),
        lighting_pass_labels(&before),
        "the lighting passes are the same, which is the point: a lighting pass count that \
         moved with the light count is exactly what the clustered pass replaces"
    );
}

/// The shadowing pass count follows the **pool**, not the scene's light count.
///
/// A light the pool gave no page is drawn into nothing and is lit unshadowed, so a scene with
/// more lights than pages emits no more shadowing passes than it has pages. That is the part
/// of the old `lights * groups` that survived, and this is what bounds it: the scene's
/// `shadow_pages` field is the whole budget.
#[test]
fn the_shadowing_pass_count_is_bounded_by_the_pool_and_not_by_the_light_count() {
    let scene = with_a_pool_of_one_page(Gallery::build("lanterns"));
    let three = light_count(&scene);
    assert_eq!(
        shadowing_pass_count(&scene),
        3,
        "lanterns has {three} groups and a pool of one page gives one light a page, so exactly \
         one shadowing pass per group"
    );

    let scene = with_a_pool_of_one_page(with_one_more_light(&scene));
    assert_eq!(
        light_count(&scene),
        three + 1,
        "the added light is in the document and the pool is still one page"
    );
    assert_eq!(
        shadowing_pass_count(&scene),
        3,
        "the pool did not grow, so the added light is lit unshadowed and no pass was added for \
         it: the shadowing count is a function of the pool, not of the lights"
    );
    // And the whole document is untouched, which is the stronger claim and the reason the
    // pool is the budget: with the pool no larger than one page, a fourth light costs
    // nothing at all in passes, in either stage.
    assert_eq!(
        pass_labels(&scene),
        pass_labels(&with_a_pool_of_one_page(Gallery::build("lanterns"))),
        "every pass of the scene is the same, so a light the pool turned away is not drawn \
         into anything and is lit unshadowed instead"
    );
}

/// Returns the scene with one page of shadow pool, which is the smallest budget there is.
fn with_a_pool_of_one_page(scene: Value) -> Value {
    let mut fields = scene.as_field_map().expect("a scene is a struct").clone();
    let frame = Value::structure([(SHADOW_PAGES.to_owned(), Value::Int(1))]);
    fields.insert(FRAME.to_owned(), frame);
    Value::structure(fields)
}

/// Returns the labels of every lighting pass a scene emits, in order.
fn lighting_pass_labels(scene: &Value) -> Vec<String> {
    graphics::passes(&compiled(scene).graphics.value)
        .expect("the passes read")
        .iter()
        .filter(|pass| {
            pass.label()
                .expect("a pass is labelled")
                .starts_with("lighting/")
        })
        .map(|pass| pass.label().expect("a pass is labelled").to_owned())
        .collect()
}

/// Returns how many shadowing passes a scene emits.
fn shadowing_pass_count(scene: &Value) -> usize {
    graphics::passes(&compiled(scene).graphics.value)
        .expect("the passes read")
        .iter()
        .filter(|pass| {
            pass.label()
                .expect("a pass is labelled")
                .starts_with("shadow/")
        })
        .count()
}

/// Returns the labels of every pass a scene emits, in order.
fn pass_labels(scene: &Value) -> Vec<String> {
    graphics::passes(&compiled(scene).graphics.value)
        .expect("the passes read")
        .iter()
        .map(|pass| pass.label().expect("a pass is labelled").to_owned())
        .collect()
}

/// Returns how many lights a scene declares.
fn light_count(scene: &Value) -> usize {
    scene
        .as_field_map()
        .and_then(|fields| fields.get("lights"))
        .and_then(Value::as_list)
        .expect("a scene names its lights")
        .len()
}

/// Returns the scene with one more light appended, the first one copied under a new label.
///
/// A copy rather than a differently shaped light, so the only difference is the count: a scene
/// that gained a new kind of light would be testing something other than the pass count. The
/// label has to differ, because two lights sharing one is refused as a duplicate before
/// anything else about the light is read.
fn with_one_more_light(scene: &Value) -> Value {
    let mut scene_fields = scene.as_field_map().expect("a scene is a struct").clone();
    let mut lights: Vec<Value> = scene_fields
        .get("lights")
        .and_then(Value::as_list)
        .expect("a scene names its lights")
        .to_vec();
    let first = lights[0].clone();
    let tag = first
        .variant_tag()
        .expect("a light is a variant of its kind");
    let mut light_fields = first
        .variant_fields()
        .expect("a light carries its fields")
        .clone();
    light_fields.insert("label".to_owned(), Value::String("added".to_owned()));
    lights.push(Value::variant(tag, light_fields));
    scene_fields.insert("lights".to_owned(), Value::List(lights));
    Value::structure(scene_fields)
}

/// Returns the grid these tests use, which is small enough to read.
fn grid() -> Clustering {
    Clustering {
        width_extents: 2,
        height_extents: 2,
        depth_extents: 2,
        lights_per_cluster: 4,
    }
}

/// Returns a pool of the given number of pages.
fn pool_of(pages: usize) -> LightPool {
    LightPool {
        page_width: 1024,
        page_height: 1024,
        pages,
    }
}

/// Returns what a host is told about a pool that could not hold every candidate.
fn notes(lights: &[ClusterLight], pool: &LightPool, assignment: &Clusters) -> Vec<CompileNote> {
    whiting_scene::lighting::pool_notes(lights, pool, assignment)
}

/// Returns a point light with the given brightness, standing in the middle of the view.
fn light(label: &str, brightness: f64) -> ClusterLight {
    ClusterLight {
        label: label.to_owned(),
        position: [0.0, 0.0, 0.0],
        colour: [brightness; 3],
        radius: 8.0,
        shadow_projection: identity(),
        clustered: true,
        casts_shadow: true,
    }
}

/// Returns the identity matrix, which is the projection a light no shadow is looked up
/// through carries.
///
/// Nothing in the pool reads it — a page is a page and a light either has one or does not —
/// so what a projection holds is of no interest to which lights are given a page.
fn identity() -> [f64; 16] {
    [
        1.0, 0.0, 0.0, 0.0, //
        0.0, 1.0, 0.0, 0.0, //
        0.0, 0.0, 1.0, 0.0, //
        0.0, 0.0, 0.0, 1.0,
    ]
}

/// Returns the parallel camera the convention gives a box of half width four.
fn parallel() -> Projection {
    Camera::read(None, &Path::root(), &bounds(), 1.0)
        .expect("the convention camera reads")
        .projection()
}

/// Returns the box the cameras frame.
fn bounds() -> Bounds {
    Bounds::around([0.0, 0.0, 0.0], 4.0)
}

/// Compiles a scene the way a host would, and returns what it emitted.
fn compiled(scene: &Value) -> whiting_scene::CompileOutput {
    let mut registry = Registry::new();
    registry.put_schema(
        SchemaIdentifier::new(graphics::GRAPHICS_SCHEMA),
        graphics::graphics_schema(),
    );
    registry.put_schema(
        SchemaIdentifier::new(graphics::INSTANCE_SCHEMA),
        graphics::instance_schema(),
    );
    register_standard_scene_schema(&mut registry);
    let source = Definition::new(
        DefinitionIdentifier::new("pool.source"),
        SchemaIdentifier::new(STANDARD_SCENE_SCHEMA),
        scene.clone(),
    );
    source
        .validate(&registry)
        .expect("the scene validates against the schema");
    StandardScene::new(
        DefinitionIdentifier::new("pool.graphics"),
        DefinitionIdentifier::new("pool.instance"),
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
