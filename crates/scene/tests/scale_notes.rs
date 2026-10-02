//! What a compile says about a scene that is bigger than the constants it inherited.
//!
//! # What these prove
//!
//! Three agents ran the same task against this framework. Two of them inherited a light
//! falloff they could neither see nor set — `1 / (1 + 0.0035r + 0.00002r²)`, whose own
//! comment places it in "a scene a couple of hundred units across" — and applied it to scenes
//! five and fourteen times that size, so their suns were attenuated by factors of twenty and
//! eighty and their intensities were those factors of a radiance. One of them built a roof from
//! a rise in metres against a ridge in world coordinates, computed "a 6.6-meter-tall roof in a
//! 14-meter-wide house", and found it with a fake-colour world-position image rather than by
//! reading the code it had just written.
//!
//! The compiler knows the extent of every scene at the moment it emits the frame. This is where
//! the note that says what that extent costs is pinned:
//!
//! - **it names the fold the pipeline's falloff gives at the scene's own size**, with the span
//!   that produced it and the reference it is measured against;
//! - **the bar is pinned from both sides on scenes whose size the test chooses**, and against
//!   the gallery it is pinned as a set: six of the ten are reported and the four small ones are
//!   not;
//! - **it is a note and never a refusal** — a scene is legitimately any size;
//! - **it is measured against the template's own constant**, so a change to one without the
//!   other fails here rather than quietly making the note a lie.
//!
//! **The measure is a box and not a surface**, which is the same limitation the camera notes
//! carry and the same reason their numbers are in their messages. `tidewater`'s box is nearly
//! three thousand units across and its far shore is a few hundred away, so the box says a fold
//! of eighty-six and the picture may have a fold of two. The number is quoted rather than the
//! verdict.

use definition::{
    Definition, DefinitionIdentifier, FieldMap, Path, Registry, SchemaIdentifier, Value,
};
use whiting_gallery::Gallery;
use whiting_scene::{
    CompileContext, CompileNote, Compiler, STANDARD_SCENE_SCHEMA, StandardScene,
    register_standard_scene_schema,
};

/// The falloff the gallery's shading template applies, written as it writes it.
///
/// This string is the copy the note in `emit.rs` is a statement about, and the test that holds
/// the two together is `the_falloff_the_note_quotes_is_the_templates_own`.
const TEMPLATE_FALLOFF: &str =
    "1.0 / (1.0 + 0.0035 * light_range + 0.00002 * light_range * light_range)";

/// The field the note points at, which is the field that sets how far apart anything is.
const INSTANCES: &str = "instances";

/// The half diagonal of a cube `across` units across, which is the distance the note measures
/// at: the furthest a light at the centre of the scene's box is from one of its corners.
fn half_diagonal_of_a_cube(across: f64) -> f64 {
    (3.0_f64 / 4.0 * across * across).sqrt()
}

/// The fold the template's law gives at a distance, which is the number the note quotes.
fn fold_at(reach: f64) -> f64 {
    1.0 + 0.0035 * reach + 0.00002 * reach * reach
}

#[test]
fn the_falloff_the_note_quotes_is_the_templates_own() {
    // The note cannot be honest about a constant it does not hold, so it holds one — a copy.
    // A copy is only honest while it is a copy, and this is what says so: change the template
    // and this fails, naming the constant in the emitter that has to change with it.
    let source = whiting_gallery::shaders::lighting("VERTEX_IN", "", "");
    assert!(
        source.contains(TEMPLATE_FALLOFF),
        "the gallery's lighting template no longer applies `{TEMPLATE_FALLOFF}`, so the falloff \
         constants FALLOFF_LINEAR and FALLOFF_QUADRATIC in crates/scene/src/standard/emit.rs are \
         a copy of nothing. The template's own comment on the attenuation says what the new one \
         is for; the note has to quote that."
    );
}

#[test]
fn a_scene_small_enough_for_the_constant_says_nothing() {
    // The example scene is four units across: the same constant divides a light by 1.01 between
    // the middle of it and its furthest corner, which is nothing a picture has ever noticed. A
    // note here would be a note about every scene.
    assert_eq!(
        scale_notes_of(&Gallery::build("studio")),
        Vec::new(),
        "one percent of a brightness gradient is not a fact about a scene"
    );
}

#[test]
fn a_scene_bigger_than_the_bar_is_named_with_its_span_and_its_fold() {
    // A cube three hundred units across, which is bigger than the extent the constant was tuned
    // for and so is reported. The message has to carry the two numbers a reader needs to argue
    // with it: the span the author wrote, and the fold that span costs — and the two have to
    // agree with each other, which is what parsing them back out of the sentence checks.
    let scene = a_cube_across(&Gallery::build("studio"), 300.0);
    let notes = scale_notes_of(&scene);
    assert_eq!(notes.len(), 1, "one note about the size: {notes:#?}");
    let note = &notes[0];
    assert_eq!(
        note.path,
        Path::root().key(INSTANCES),
        "the path is the field that sets how far apart anything is; a note that pointed at the \
         camera would send the reader after a field a falloff has nothing to do with"
    );
    let reported = fold_of(note);
    assert!(
        reported >= 2.0,
        "a three hundred unit scene is at or above the bar: {reported}"
    );
    assert!(
        (stops_of(note) - reported.log2()).abs() < 0.05,
        "the stops it quotes are the fold's own logarithm, so the sentence cannot carry a \
         number and a restatement of it that disagree: {}",
        note.message
    );
    for wanted in [
        "units across its largest axis".to_owned(),
        "200 units".to_owned(),
        format!("{:.1}", fold_at(half_diagonal_of_a_cube(200.0))),
    ] {
        assert!(
            note.message.contains(&wanted),
            "the message carries `{wanted}`, so the reader can check it rather than take it: {}",
            note.message
        );
    }
    assert!(
        note.message.contains("drawn as written"),
        "and it says the compile did not refuse, because a scene is legitimately any size: {}",
        note.message
    );
    assert!(
        note.message.contains("box") && note.message.contains("not a surface"),
        "and it says what the measure is, so a reader who thinks the fold is wrong can say why: \
         {}",
        note.message
    );
}

#[test]
fn the_bar_sits_between_a_scene_that_is_reported_and_one_that_is_not() {
    // Both sides of the bar, on scenes whose extent the test writes down rather than measures
    // out of a document. The folds are read back out of the notes, so what is checked is the
    // compiler's arithmetic and not this file's.
    let quiet = scale_notes_of(&a_cube_across(&Gallery::build("studio"), 100.0));
    assert!(
        quiet.is_empty(),
        "a scene a fifth the size of the one the constant was tuned for is not reported"
    );
    let reported = scale_notes_of(&a_cube_across(&Gallery::build("studio"), 300.0));
    let note = reported
        .first()
        .expect("a three hundred unit scene is reported");
    let fold = fold_of(note);
    assert!(
        fold > 2.0,
        "and the fold it quotes is above the bar, which is what puts the bar in the gap between \
         the two scenes rather than on either of them: {fold}"
    );
}

#[test]
fn the_gallery_says_how_often_a_scene_is_reported() {
    // The threshold was chosen from this table and not from taste, so the set it names is
    // pinned. Measured over the eleven gallery scenes, the fold at the half diagonal of each
    // scene's own box runs: studio 1.01, sunlight 1.05, spotlight 1.05, tiles 1.07, dunes 2.91,
    // terraces 3.51, stilllife 4.79, lanterns 6.09, monoliths 7.31, foldyard 11.14, tidewater
    // 86.35. Three clusters and two gaps; the bar of two sits in the first one, and the second is
    // left alone because a bar above it would report `tidewater` and nothing else.
    //
    // `foldyard` is the eleventh, and it is the first scene in the gallery whose own size is the
    // reason it is here: a valley three hundred units across with a range five hundred beyond it,
    // so the box the fold is measured over is a thousand and the note's own words — the distance
    // is a box and not a surface — are the whole of the story. It lands between `monoliths` and
    // `tidewater` and so it moves neither gap.
    let reported: Vec<&str> = Gallery::names()
        .iter()
        .copied()
        .filter(|name| scale_note_of(&Gallery::build(name)).is_some())
        .collect();
    let quiet: Vec<&str> = Gallery::names()
        .iter()
        .copied()
        .filter(|name| scale_note_of(&Gallery::build(name)).is_none())
        .collect();
    assert_eq!(
        reported,
        [
            "dunes",
            "foldyard",
            "lanterns",
            "monoliths",
            "stilllife",
            "terraces",
            "tidewater"
        ],
        "the seven scenes at or above a one-stop fold are the ones reported, in the gallery's own \
         order. A bar moved into the second gap would report `tidewater` alone, which is a bar \
         fitted to a single observation rather than chosen from the data"
    );
    assert_eq!(
        quiet,
        ["spotlight", "studio", "sunlight", "tiles"],
        "and the four scenes the constant does nothing to are the four the note leaves alone: a \
         scene four units across and a scene twenty-four units across are the same answer"
    );
    // The largest fold among the quiet four is 1.07 and the smallest among the reported seven is
    // 2.91, so any bar between them names this same seven. The bar is two because a doubling of
    // light is the smallest difference a photograph is named for, and because it is a meaning
    // rather than a number somebody picked out of the gap. Both edges are written as the sums
    // the note itself prints, so the claim and the arithmetic cannot drift apart.
    let quiet_edge = fold_at(half_diagonal_of_a_cube(24.0));
    let reported_edge = fold_at(half_diagonal_of_a_cube(300.0));
    assert!(
        quiet_edge < FOLD_BAR && FOLD_BAR < reported_edge,
        "the bar is inside the gap, not on either edge: {quiet_edge} < {FOLD_BAR} < {reported_edge}"
    );
}

/// The bar the emitter compares against, restated here so the arithmetic above and the
/// constant there are one claim rather than two.
const FOLD_BAR: f64 = 2.0;

#[test]
fn a_scene_naming_no_camera_is_still_told_about_its_size() {
    // The camera notes are behind a gate the scale note is not: a camera the scene did not
    // write says nothing, because a note about it points at a field that is not in the
    // document. A scene's **size** is in the document whatever camera it has, and a falloff is
    // a function of the size whether or not anybody chose where to stand. The example scene
    // names no camera.
    let scene = a_cube_across(&Gallery::build("studio"), 1000.0);
    assert!(
        scale_note_of(&scene).is_some(),
        "a scene with no camera and a cube a thousand units across is still a scene whose light \
         falls off, and its box is in the document"
    );
}

/// Returns the notes a compile of this scene raises that are about its size.
fn scale_notes_of(scene: &Value) -> Vec<CompileNote> {
    compiled(scene)
        .notes
        .into_iter()
        .filter(|note| note.path == Path::root().key(INSTANCES))
        .collect()
}

/// Returns the one note a scene raises about its size, if it raises one.
fn scale_note_of(scene: &Value) -> Option<CompileNote> {
    scale_notes_of(scene).into_iter().next()
}

/// Returns the fold a note quotes, read back out of its own sentence.
///
/// Reading the number out of the message rather than recomputing it is the point: it is the
/// **compiler's** fold that is being checked, and a test that recomputed the same law beside it
/// would agree with a compiler that had drifted.
fn fold_of(note: &CompileNote) -> f64 {
    number_after(note, "point light ")
        .expect("the message names the fold, which is the number a reader cannot otherwise get")
}

/// Returns the stops a note quotes, read back out of its own sentence.
fn stops_of(note: &CompileNote) -> f64 {
    number_after(note, "about ").expect("the message names the fold in stops as well")
}

/// Returns the first number that follows a phrase in a message.
fn number_after(note: &CompileNote, phrase: &str) -> Option<f64> {
    let after = note.message.split(phrase).nth(1)?;
    let digits: String = after
        .chars()
        .skip_while(|character| !character.is_ascii_digit())
        .take_while(|character| character.is_ascii_digit() || *character == '.')
        .collect();
    digits.parse().ok()
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
        DefinitionIdentifier::new("scale-notes.source"),
        SchemaIdentifier::new(STANDARD_SCENE_SCHEMA),
        scene.clone(),
    );
    source
        .validate(&registry)
        .expect("the scene validates against the schema");
    StandardScene::new(
        DefinitionIdentifier::new("scale-notes.graphics"),
        DefinitionIdentifier::new("scale-notes.instance"),
    )
    .compile(&CompileContext {
        scene: &source,
        previous_graphics: None,
        previous_instance: None,
        registry: &registry,
        camera: 0,
        // No machine behind this compile, so it is made against the floor: a note in this file
        // is a note about a scene's size rather than a count that asked past a ceiling.
        limits: None,
    })
    .expect("the scene compiles")
}

/// Returns a scene whose every instance draws a cube `across` units across.
///
/// The example's four instances all draw one mesh, so replacing the mesh's points replaces all
/// four and the scene's box is the cube's. A cube rather than a flat triangle because a box with
/// no thickness is a box the falloff's half diagonal cannot be read off.
fn a_cube_across(scene: &Value, across: f64) -> Value {
    let half = across / 2.0;
    let points: Vec<Value> = [
        [
            [-half, -half, half],
            [half, -half, half],
            [-half, half, half],
        ],
        [
            [half, -half, -half],
            [-half, -half, -half],
            [half, half, -half],
        ],
        [
            [half, half, half],
            [half, -half, -half],
            [half, half, -half],
        ],
        [
            [-half, half, -half],
            [-half, half, half],
            [-half, -half, -half],
        ],
    ]
    .iter()
    .flatten()
    .map(|position| {
        Value::structure([
            ("normal".to_owned(), triple([0.0, 1.0, 0.0])),
            ("position".to_owned(), triple(*position)),
        ])
    })
    .collect();
    let mut scene = scene.clone();
    let mesh = scene
        .as_field_map_mut()
        .and_then(|fields| fields.get_mut("meshes"))
        .and_then(Value::as_list_mut)
        .and_then(|meshes| meshes.first_mut())
        .expect("the example has a mesh");
    mesh.as_field_map_mut()
        .expect("a mesh is a struct")
        .insert("points".to_owned(), Value::List(points));
    scene
}

fn triple(vector: [f64; 3]) -> Value {
    Value::List(vector.into_iter().map(Value::Float).collect())
}

/// Unused here, but kept so the next test in this file has the shape this one needed.
#[allow(dead_code)]
fn translation_of(transform: &FieldMap) -> [f64; 3] {
    let mut at = [0.0; 3];
    if let Some(Value::List(components)) = transform.get("translation") {
        for (axis, component) in components.iter().take(3).enumerate() {
            at[axis] = component
                .as_float()
                .or_else(|| component.as_int().map(|number| number as f64))
                .unwrap_or(0.0);
        }
    }
    at
}
