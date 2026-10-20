//! The exposure field: what it means, and what a scene that does not name it gets.
//!
//! # What these prove
//!
//! Two independent runs lost hours to exposure, both to the same thing: a number that does not
//! do what its name says, in a transform that could be neither observed nor queried. One cut
//! the exposure by 44% and the image barely moved — the light was a glow term in the grade
//! adding to the sky — and found this by sampling its own output png from PowerShell. The
//! other darkened a clay albedo by 60% and the pumpkins stayed orange, because ACES pushes
//! bright red towards yellow-white at the high end, so the fix was the **emissive** peak and
//! not the albedo. That one took 92 minutes.
//!
//! Both were reasoning about a black box. What is pinned here is the box being open:
//!
//! - **the field is a linear multiplier and not a curve parameter**, so a document naming one
//!   number cannot mean two things — and the *consequence* of that choice is stated rather than
//!   discovered: a multiplier reaches a radiance above one, which is only true since `0257706`
//!   gave the shaded image `rgba16float`. It was an eight bit target, so anything above one was
//!   clipped before the grade saw it and no exposure could bring it back; that is recorded here
//!   rather than deleted, because it is why this field reads as a limitation until it does not;
//! - **it is additive**: a scene that names no exposure emits the documents it emitted before
//!   the field existed, and the two post interfaces differ by exactly one field;
//! - **naming it is a three-place opt-in**, and a scene that names it without declaring it in
//!   its own post shader is refused rather than quietly drawn unexposed;
//! - **a scene that names it and runs no grade is told**, because a field nothing reads is the
//!   state this project calls out as one to avoid.

use definition::{
    Definition, DefinitionIdentifier, Path, Registry, Schema, SchemaIdentifier, SchemaKind, Value,
};
use graphics::PASSES_FIELD;
use graphics::pass::{LABEL, UNIFORM};
use whiting_scene::standard::{
    EXPOSURE, PASS_SCHEMA, SOURCE, example_standard_scene, post_pass_schema,
    post_pass_schema_exposed,
};
use whiting_scene::{
    CompileContext, Compiler, STANDARD_SCENE_SCHEMA, StandardScene, register_standard_scene_schema,
};

/// The label the document under test gives its post shader.
const POST_SHADER: &str = "post";

/// The field a scene lists its post processes under.
///
/// Written out rather than imported because the schema's own constant is not re-exported from
/// `standard`, and adding a re-export for one test would be a public API change this does not
/// need.
const POST_PROCESSES: &str = "post_processes";

#[test]
fn the_two_post_interfaces_differ_by_exactly_one_field() {
    // This is the additive property stated as a shape rather than as a render: an absent
    // exposure is not a field of one, and if it were, every field after it would move and
    // every existing scene's post shader would describe the wrong layout.
    let plain = uniform_fields(&post_pass_schema());
    assert!(
        !plain.contains(&EXPOSURE.to_owned()),
        "a scene that names no exposure gets an interface that does not declare the field: \
         {plain:?}"
    );
    let exposed = uniform_fields(&post_pass_schema_exposed());
    let added: Vec<&String> = exposed
        .iter()
        .filter(|field| !plain.contains(field))
        .collect();
    assert_eq!(
        added,
        [&EXPOSURE.to_owned()],
        "and the exposed one adds exactly that field"
    );
    let removed: Vec<&String> = plain
        .iter()
        .filter(|field| !exposed.contains(field))
        .collect();
    assert!(
        removed.is_empty(),
        "and moves nothing: {removed:?}. A field that came and went with the document would put \
         every field after it somewhere else, and the layout is derived from the names"
    );
}

#[test]
fn a_scene_naming_no_exposure_carries_no_exposure_in_its_documents() {
    // The number half of the additive property, on a real scene with a real post chain: the
    // word does not appear in what the compiler emitted. The picture half is measured on the
    // adapter and is in the handoff; this is the same claim where it can be asserted on every
    // run of the suite.
    let emitted = exposure_compile(&example_standard_scene());
    assert!(
        !mentions_exposure(&emitted.graphics.value) && !mentions_exposure(&emitted.instance.value),
        "a scene that names no exposure emits no exposure, so its frame is the frame it drew \
         before the field existed"
    );
}

#[test]
fn a_scene_naming_an_exposure_carries_it_in_the_post_uniform() {
    let scene = with_an_exposure(&exposing_post(&example_standard_scene()), 0.5);
    let emitted = exposure_compile(&scene);
    let exposure = post_uniform_field(&emitted.graphics.value, EXPOSURE)
        .expect("the post uniform carries the exposure the scene named");
    assert_eq!(
        exposure.as_float(),
        Some(0.5),
        "and it carries the number the document wrote, not one the compiler chose"
    );
}

#[test]
fn a_scene_naming_an_exposure_whose_shader_does_not_declare_it_is_refused() {
    // The half of the opt-in that catches a scene in the middle of making the other half. A
    // post shader that declares the plain interface and a scene that names an exposure is a
    // document that would be drawn unexposed on every machine, which the page pool's policy
    // makes a fact about the document and so a refusal rather than a note.
    let scene = with_an_exposure(&example_standard_scene(), 0.5);
    let Err(said) = compile_failing(&scene) else {
        panic!("a scene whose post shader cannot read the field is refused rather than drawn");
    };
    assert!(
        said.contains(EXPOSURE) && said.contains("post_pass_schema_exposed"),
        "and the refusal names the field, the shader's role and the one call that declares it, \
         rather than saying only that something did not line up: {said}"
    );
}

#[test]
fn a_scene_naming_an_exposure_and_no_post_process_is_told() {
    // A field nothing reads is a state this project calls out as one to avoid, and the answer
    // is a note rather than a refusal: a scene with no grade is a legitimate scene, and the
    // frame of one is the shaded image itself.
    let mut scene = example_standard_scene();
    scene
        .as_field_map_mut()
        .expect("a scene is a struct")
        .insert(POST_PROCESSES.to_owned(), Value::List(Vec::new()));
    let scene = with_an_exposure(&scene, 0.5);
    let notes = exposure_compile(&scene).notes;
    let note = notes
        .iter()
        .find(|note| note.path == Path::root().key(EXPOSURE))
        .unwrap_or_else(|| {
            panic!("a field nothing reads is named: {notes:#?}");
        });
    assert!(
        note.message.contains("no post process") && note.message.contains("0.5"),
        "and it names what nothing read it and what it was: {}",
        note.message
    );
}

#[test]
fn an_exposure_that_is_not_a_positive_number_is_refused() {
    // The page pool's line: a count above what the machine has is bounded and named, a count
    // below one is refused. An exposure of zero is a count below one, and a negative one has
    // no meaning as a multiplier of light. Both are refused rather than clamped, because a
    // clamp hands back a picture and says nothing.
    for (written, expected) in [(0.0, "positive"), (-1.0, "positive")] {
        let scene = with_an_exposure(&exposing_post(&example_standard_scene()), written);
        let Err(error) = compile_failing(&scene) else {
            panic!("{written} is refused rather than clamped: it compiled");
        };
        assert!(
            error.contains(expected),
            "and the refusal says what it wanted: {error}"
        );
    }
}

/// Returns the document under test with its post shader's interface and source declaring an
/// `exposure`.
///
/// This is the whole opt-in a scene makes, in the order a scene author makes it: the field on
/// the document, the field in the interface the shader declares, and the field in the shader's
/// own uniform. The declaration is inserted in **name order** — `camera`, `depth_range`,
/// `exposure`, `projection`, `time` — because the uniform's layout is derived from the names
/// and a shader that declared the same field anywhere else would be a shader whose offsets are
/// wrong.
fn exposing_post(scene: &Value) -> Value {
    let mut scene = scene.clone();
    let shaders = scene
        .as_field_map_mut()
        .and_then(|fields| fields.get_mut("shaders"))
        .and_then(Value::as_list_mut)
        .expect("a scene declares its shaders");
    let post = shaders
        .iter_mut()
        .find(|shader| {
            shader
                .variant_fields()
                .and_then(|fields| fields.get("label"))
                .and_then(Value::as_str)
                == Some(POST_SHADER)
        })
        .expect("the document declares a post shader");
    let Some(Value::Enum { fields, .. }) = Some(&mut *post) else {
        panic!("a shader is a variant carrying the stage it belongs to");
    };
    let declared = fields
        .get_mut(PASS_SCHEMA)
        .expect("a shader declares the interface of its pass");
    *declared = post_pass_schema_exposed().to_value();
    let Some(Value::String(text)) = fields.get_mut(SOURCE) else {
        panic!("a shader's source is a string");
    };
    let anchor = "    depth_range: vec2<f32>,\n";
    assert!(
        text.contains(anchor),
        "the post uniform declares its fields in name order, which is where an \
         exposure goes: {text}"
    );
    let exposed = text.replacen(anchor, &format!("{anchor}    exposure: f32,\n"), 1);
    *text = exposed;
    scene
}

/// Returns the scene with an `exposure` written on it.
fn with_an_exposure(scene: &Value, exposure: f64) -> Value {
    let mut scene = scene.clone();
    scene
        .as_field_map_mut()
        .expect("a scene is a struct")
        .insert(EXPOSURE.to_owned(), Value::Float(exposure));
    scene
}

/// Returns the exposure the emitted post uniform carries, or none when it carries no such field.
fn post_uniform_field(graphics: &Value, field: &str) -> Option<Value> {
    let passes = graphics.as_field_map()?.get(PASSES_FIELD)?.as_list()?;
    let post = passes.iter().find(|pass| {
        pass.variant_fields()
            .and_then(|fields| fields.get(LABEL))
            .and_then(Value::as_str)
            .is_some_and(|label| label.starts_with("post/"))
    })?;
    post.variant_fields()?
        .get(UNIFORM)?
        .as_field_map()?
        .get(field)
        .cloned()
}

/// Returns what a refusal said, in the words a compile error carries.
fn said_about(error: &whiting_scene::CompileError) -> String {
    format!("{error:?}")
}

/// Returns the names an interface's uniform declares, which is the layout the pass is uploaded
/// with.
fn uniform_fields(interface: &Schema) -> Vec<String> {
    let SchemaKind::Struct { fields } = &interface.kind else {
        panic!("an interface is a struct");
    };
    let Some(SchemaKind::Struct { fields }) = fields.get(UNIFORM).map(|field| &field.kind) else {
        panic!("an interface declares a uniform block");
    };
    fields.keys().cloned().collect()
}

/// Returns whether any field map in a document carries the name.
fn mentions_exposure(value: &Value) -> bool {
    match value {
        Value::Struct(fields) | Value::Enum { fields, .. } => {
            fields.contains_key(EXPOSURE) || fields.values().any(mentions_exposure)
        }
        Value::Dict(fields) => {
            fields.contains_key(EXPOSURE) || fields.values().any(mentions_exposure)
        }
        Value::List(items) => items.iter().any(mentions_exposure),
        _ => false,
    }
}

/// Compiles a scene the way a host would.
fn exposure_compile(scene: &Value) -> whiting_scene::CompileOutput {
    compile_failing(scene).unwrap_or_else(|said| panic!("the scene compiles, and said: {said}"))
}

/// Compiles a scene, returning what the refusal said rather than panicking on it.
///
/// The message is the thing under test in two of these cases, so it comes back as a string
/// rather than as an error the test would have to know the shape of.
fn compile_failing(scene: &Value) -> Result<whiting_scene::CompileOutput, String> {
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
        DefinitionIdentifier::new("exposure.source"),
        SchemaIdentifier::new(STANDARD_SCENE_SCHEMA),
        scene.clone(),
    );
    source
        .validate(&registry)
        .expect("the scene validates against the schema");
    StandardScene::new(
        DefinitionIdentifier::new("exposure.graphics"),
        DefinitionIdentifier::new("exposure.instance"),
    )
    .compile(&CompileContext {
        scene: &source,
        previous_graphics: None,
        previous_instance: None,
        registry: &registry,
        camera: 0,
        limits: None,
    })
    .map_err(|error| said_about(&error))
}
