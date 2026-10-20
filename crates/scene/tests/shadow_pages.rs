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
//! - a light whose shadow is recorded as a grid of pages, which asks the pool for nothing at all
//!   and so is neither given a page nor reported as having missed one;
//! - the pool's own shape, bounded rather than refused, with a note naming what was lost.

use definition::{Definition, DefinitionIdentifier, Path, Registry, SchemaIdentifier, Value};
use whiting_scene::CompileContext;
use whiting_scene::Compiler;
use whiting_scene::lighting::{
    ClusterLight, Clustering, Clusters, LightPool, NO_PAGE, SHADOW_PAGE_WIDTH, SHADOW_PAGES,
    assign, read_light_pool,
};
use whiting_scene::standard::example_standard_scene;
use whiting_scene::standard::{Bounds, Camera, FRAME, Projection, register_standard_scene_schema};
use whiting_scene::{
    CompileError, CompileNote, DeviceLimits, STANDARD_SCENE_SCHEMA, StandardScene,
};

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
        global: false,
        casts_shadow: true,
        virtual_shadow: false,
        virtual_page: [NO_PAGE; 4],
    };
    let clusters = assign(
        &grid(),
        std::slice::from_ref(&behind),
        &parallel(),
        &pool_of(1),
    )
    .expect("the assignment runs");

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
        global: false,
        casts_shadow: false,
        virtual_shadow: false,
        virtual_page: [NO_PAGE; 4],
    };
    let clusters = assign(
        &grid(),
        std::slice::from_ref(&sky),
        &parallel(),
        &pool_of(4),
    )
    .expect("the assignment runs");

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
        global: false,
        casts_shadow: true,
        virtual_shadow: false,
        virtual_page: [NO_PAGE; 4],
    };
    let clusters = assign(
        &grid(),
        std::slice::from_ref(&sun),
        &parallel(),
        &pool_of(4),
    )
    .expect("the assignment runs");

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
fn a_light_recorded_as_pages_asks_the_pool_for_nothing_and_is_not_missed() {
    // The migration's one place a note could lie. A virtual light is still a light that casts a
    // shadow, so the candidate walk would rank it with the rest — and on a pool of one page,
    // with a lamp bright enough to win the ranking, that page would be spent on a shadow already
    // drawn as eighty five of them, the one page that was actually needed would go to the dimmer
    // light, and the note would tell the author the pool dropped a light that has a better
    // shadow than anything in the pool. So it is not a candidate at all, and this says so.
    let lamp = ClusterLight {
        label: "lamp".to_owned(),
        position: [0.0, 0.0, 0.0],
        colour: [1.0, 1.0, 1.0],
        radius: 100.0,
        shadow_projection: identity(),
        clustered: true,
        global: false,
        casts_shadow: true,
        virtual_shadow: true,
        virtual_page: [0.0, 0.0, NO_PAGE, NO_PAGE],
    };
    let pool_of_one = pool_of(1);
    let clusters = assign(
        &grid(),
        &[lamp.clone(), light("candle", 0.5)],
        &parallel(),
        &pool_of_one,
    )
    .expect("the assignment runs");

    assert_eq!(
        clusters.page_of(0),
        None,
        "a light with a grid takes no pool page: the fixed path is replaced for a kind that moved, \
         not kept beside it"
    );
    assert_eq!(
        clusters.page_of(1),
        Some(0),
        "and the page the lamp did not spend is the candle's"
    );
    assert!(
        notes(&[lamp, light("candle", 0.5)], &pool_of_one, &clusters).is_empty(),
        "a light that never asked the pool is not a light the pool turned away"
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
        std::hint::black_box(NO_PAGE) < 0.0,
        "the sentinel is below every legal page index"
    );
}

/// The machine a test compile is made against, which is small enough that a document can ask
/// past it. A count of 8 pages and a page of 512 pixels are both inside it, so the only way a
/// note appears in this file is a count that asked for more.
fn small_machine() -> DeviceLimits {
    DeviceLimits {
        max_texture_dimension_2d: 512,
        max_texture_array_layers: 8,
    }
}

#[test]
fn a_pool_deeper_than_the_machine_has_is_bounded_and_named() {
    let too_deep = Value::structure([(
        FRAME.to_owned(),
        Value::structure([(SHADOW_PAGES.to_owned(), Value::Int(64))]),
    )]);
    let (pool, notes) =
        read_light_pool(&too_deep, small_machine()).expect("a pool past the ceiling still reads");
    assert_eq!(
        pool.pages, 8,
        "the pool is given the depth this machine has"
    );
    assert_eq!(
        notes.len(),
        1,
        "one count was bounded, so it is named once: {notes:?}"
    );
    let note = &notes[0];
    assert_eq!(
        note.path,
        Path::root().key(FRAME).key(SHADOW_PAGES),
        "the note points at the number to change"
    );
    for named in ["shadow_pages", "64", "8"] {
        assert!(
            note.message.contains(named),
            "the note names `{named}` so an author can act on it: {}",
            note.message
        );
    }

    // A pool that fits says nothing at all, which is the other half of the contract: a note
    // nobody asked for is noise, and the author of a scene that fits has nothing to read.
    let fitting = Value::structure([(
        FRAME.to_owned(),
        Value::structure([(SHADOW_PAGES.to_owned(), Value::Int(8))]),
    )]);
    let (pool, notes) =
        read_light_pool(&fitting, small_machine()).expect("a pool within the ceiling reads");
    assert_eq!(pool.pages, 8);
    assert!(
        notes.is_empty(),
        "a pool that fits is not a note: {notes:?}"
    );

    // And a pool of nothing is still refused, because a pool of nothing is a wrong image on
    // every machine rather than a machine being too small.
    let empty = Value::structure([(
        FRAME.to_owned(),
        Value::structure([(SHADOW_PAGES.to_owned(), Value::Int(0))]),
    )]);
    assert!(
        matches!(
            read_light_pool(&empty, small_machine()),
            Err(CompileError::EmptyClustering { .. })
        ),
        "a pool of nothing is a scene whose every light is approximate"
    );
}

#[test]
fn a_page_wider_than_the_machine_has_is_bounded_and_named() {
    let too_wide = Value::structure([(
        FRAME.to_owned(),
        Value::structure([(SHADOW_PAGE_WIDTH.to_owned(), Value::Int(4096))]),
    )]);
    let (pool, notes) =
        read_light_pool(&too_wide, small_machine()).expect("a page past the ceiling still reads");
    assert_eq!(
        pool.page_width, 512,
        "the page is given the side this machine can make"
    );
    assert_eq!(
        pool.page_height, 1024,
        "the other side was never asked past the ceiling"
    );
    assert_eq!(notes.len(), 1, "only the side that asked past it is named");
    assert!(
        notes[0].message.contains("coarser"),
        "the note says what a smaller page costs, because a page drawn at the size given is a \
         wrong picture rather than a smaller one: {}",
        notes[0].message
    );
}

#[test]
fn a_pool_the_scene_says_nothing_about_is_the_documented_one() {
    let (pool, notes) = read_light_pool(&Value::structure([]), DeviceLimits::downlevel())
        .expect("a scene with no pool reads");
    assert!(
        notes.is_empty(),
        "a convention is below every machine's ceiling, so it is never bounded: {notes:?}"
    );
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
    let before = with_one_more_light(&example_standard_scene());
    let one = light_count(&before);
    assert!(
        one > 1,
        "the document starts with several lights, not {one}"
    );

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
///
/// The bound is the pool's, and only the pool's. A light recorded as a grid is not in this
/// arithmetic at all — its pass count is its own pages times the groups that cast into them, and
/// the ceiling on that is the atlas rather than the pool — which is why the document below
/// carries only lights of the kind that stayed on the pool. The other half of that claim, that a
/// grid's pages and its table cannot come apart, is
/// `the_pages_a_spot_light_emits_are_the_pages_its_table_answers` in `spot_lights.rs`.
#[test]
fn the_shadowing_pass_count_is_bounded_by_the_pool_and_not_by_the_light_count() {
    let before = with_one_more_light(&example_standard_scene());
    let scene = with_a_pool_of_one_page(before.clone());
    let three = light_count(&scene);
    let groups = shadowing_groups(&scene);
    assert_eq!(
        shadowing_pass_count(&scene),
        groups,
        "the document has {groups} groups of shadow casting geometry and a pool of one page \
         gives one light a page, so exactly one shadowing pass per group"
    );

    let scene = with_a_pool_of_one_page(with_one_more_light(&scene));
    assert_eq!(
        light_count(&scene),
        three + 1,
        "the added light is in the document and the pool is still one page"
    );
    assert_eq!(
        shadowing_pass_count(&scene),
        groups,
        "the pool did not grow, so the added light is lit unshadowed and no pass was added for \
         it: the shadowing count is a function of the pool, not of the lights"
    );
    // And the whole document is untouched, which is the stronger claim and the reason the
    // pool is the budget: with the pool no larger than one page, a fourth light costs
    // nothing at all in passes, in either stage.
    assert_eq!(
        pass_labels(&scene),
        pass_labels(&with_a_pool_of_one_page(before)),
        "every pass of the document is the same, so a light the pool turned away is not drawn \
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

/// Returns how many groups of shadow casting geometry a document places.
///
/// This is the number a pool of one page is spent on: one page is given to one light, and that
/// light draws every group that casts into it, so the count is read off the document rather than
/// written down and does not have to be a number this file was written against.
fn shadowing_groups(scene: &Value) -> usize {
    scene
        .as_field_map()
        .and_then(|fields| fields.get("instances"))
        .and_then(Value::as_dict)
        .and_then(|buckets| buckets.get("shadowing"))
        .and_then(Value::as_list)
        .expect("a document places its instances by stage")
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
        global: false,
        casts_shadow: true,
        virtual_shadow: false,
        virtual_page: [NO_PAGE; 4],
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
        // No machine behind this compile, so it is made against the floor.
        limits: None,
    })
    .expect("the scene compiles")
}
