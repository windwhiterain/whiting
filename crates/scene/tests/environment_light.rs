//! What the compiler does with an `Environment` light, and what it still refuses.
//!
//! # What this holds that the pixel test cannot
//!
//! `crates/device/tests/environment_map_render.rs` proves the gpu sampled the map by direction.
//! This file holds the three things about the **document** that a frame cannot show, and two of
//! them are refusals: a scene naming a kind this compiler cannot draw, and a scene naming two
//! environments where a lighting pass carries one. Both are the outcome this project treats as
//! better than a quietly wrong image, and both are invisible in a rendered frame because a
//! refused scene renders nothing at all.
//!
//! The third is the one the frame cannot show in the other direction: a scene naming **no**
//! environment light still binds a map and still carries the flag, because a binding that came
//! and went with the document would move every binding number below it.

use definition::{Definition, DefinitionIdentifier, Registry, SchemaIdentifier, Value};
use graphics::{BINDERS, ENTRIES, GRAPHICS_SCHEMA, INSTANCE_SCHEMA, LABEL, PASSES_FIELD, UNIFORM};
use whiting_scene::standard::example_standard_scene;
use whiting_scene::{
    CompileContext, CompileOutput, Compiler, STANDARD_SCENE_SCHEMA, StandardScene,
};

/// The binding point the environment map is read at, as the pass declares it.
const ENVIRONMENT_MAP: &str = "environment_map";

/// The uniform field the environment's colour and flag are written to.
const ENVIRONMENT: &str = "environment";

/// The size of the environment map these tests declare, in texels a side.
const MAP: i64 = 4;

#[test]
fn an_environment_light_reaches_the_pass_as_a_map_and_a_flag() {
    let compiled = compiled(&scene(&[environment("dusk", "dusk", 2.0)]));

    // The map is uploaded as an **image**, so the entry is named after the image rather than
    // after the light, and the pass binds that entry. One image is one texture however many
    // things read it, so a material naming `dusk` as well would find the same entry.
    assert!(
        entries(&compiled).contains_key("image_dusk"),
        "the environment's image is an image and is uploaded under its own name: {:?}",
        entries(&compiled).keys().collect::<Vec<_>>()
    );
    assert_eq!(
        environment_map_source(&compiled),
        "image_dusk",
        "the pass binds the image the light named rather than an entry of its own"
    );

    // The uniform carries the colour scaled by the intensity and the flag saying the scene
    // named one, and the flag is the fourth number rather than a field beside it.
    assert_eq!(
        environment_uniform(&compiled),
        vec![2.0, 2.0, 2.0, 1.0],
        "a white light at an intensity of two writes two in every channel and one in the flag"
    );
}

#[test]
fn a_scene_naming_no_environment_still_binds_a_map_and_writes_the_flag() {
    let compiled = compiled(&scene(&[point("key")]));

    // Both halves are what make the binding numbers the same in every scene. A map that was
    // absent rather than black would move `environment_map` out of the numbering and every
    // binding above it along, so the shader would be a different shader per scene.
    // A map that is bound even though the scene named no environment, and one that is **not**
    // named after an image: the absent map is not a scene image, and a name in the image
    // namespace standing for a thing no author declared is a name a reader would go looking for.
    assert_eq!(
        environment_map_source(&compiled),
        "environment_absent_map",
        "a scene with no environment light still binds a map, because the binding is a property \
         of the pass and not of the document"
    );
    assert_eq!(
        environment_uniform(&compiled),
        vec![0.0, 0.0, 0.0, 0.0],
        "and what it writes is a zero colour and a **zero flag**, which is the difference \
         between there being no environment and the environment being black"
    );
}

#[test]
fn a_second_environment_light_is_refused_rather_than_one_of_them_being_ignored() {
    let failure = compile(&scene(&[
        environment("dusk", "dusk", 1.0),
        environment("night", "dusk", 1.0),
    ]))
    .err()
    .expect("a scene naming two environments is refused");
    let message = failure.to_string();
    assert!(
        message.contains("night") && message.contains("dusk"),
        "the message names the second light and the first: {message}"
    );
}

/// Builds a scene from the example one, with the given lights and an environment map declared.
///
/// The example scene is a complete document that already compiles, so replacing its one light is
/// the smallest way to ask what the compiler does with a different one.
fn scene(lights: &[Value]) -> Value {
    let mut fields = example_standard_scene()
        .as_field_map()
        .cloned()
        .expect("the example scene is a structure");
    fields.insert("lights".to_owned(), Value::List(lights.to_vec()));
    fields.insert(
        "images".to_owned(),
        Value::List(vec![Value::structure([
            ("label".to_owned(), Value::String("dusk".to_owned())),
            ("width".to_owned(), Value::Int(MAP)),
            ("height".to_owned(), Value::Int(MAP)),
            ("format".to_owned(), Value::String("rgba8unorm".to_owned())),
            (
                "data".to_owned(),
                Value::Bytes(vec![64; (MAP * MAP * 4) as usize]),
            ),
        ])]),
    );
    Value::structure(fields)
}

/// Builds a point light, which is what the example scene had.
fn point(label: &str) -> Value {
    Value::variant(
        "Point",
        [
            ("label".to_owned(), Value::String(label.to_owned())),
            ("position".to_owned(), triple([4.0, 6.0, 2.0])),
            ("color".to_owned(), triple([1.0, 0.95, 0.9])),
            ("intensity".to_owned(), Value::Float(1.0)),
            ("shadow_extent".to_owned(), Value::Float(2.5)),
        ],
    )
}

/// Builds an environment light naming an image, at an intensity.
fn environment(label: &str, image: &str, intensity: f64) -> Value {
    Value::variant(
        "Environment",
        [
            ("label".to_owned(), Value::String(label.to_owned())),
            ("image".to_owned(), Value::String(image.to_owned())),
            ("color".to_owned(), triple([1.0, 1.0, 1.0])),
            ("intensity".to_owned(), Value::Float(intensity)),
        ],
    )
}

/// Builds a three component list of floats.
fn triple(components: [f64; 3]) -> Value {
    Value::List(components.into_iter().map(Value::Float).collect())
}

/// Returns the entries of the emitted instance document.
fn entries(compiled: &CompileOutput) -> definition::FieldMap {
    compiled
        .instance
        .value
        .as_field_map()
        .and_then(|fields| fields.get(ENTRIES))
        .and_then(Value::as_dict)
        .cloned()
        .expect("an instance holds its entries under one field")
}

/// Returns the scene's one lighting pass.
fn lighting_pass(compiled: &CompileOutput) -> Value {
    compiled
        .graphics
        .value
        .as_field_map()
        .and_then(|fields| fields.get(PASSES_FIELD))
        .and_then(Value::as_list)
        .expect("the graphics document holds passes")
        .iter()
        .find(|pass| {
            pass.variant_fields()
                .and_then(|fields| fields.get(LABEL))
                .and_then(Value::as_str)
                .is_some_and(|label| label.starts_with("lighting/"))
        })
        .expect("the example scene has one lighting pass")
        .clone()
}

/// Returns the entry the lighting pass binds the environment map from.
///
/// The source is read out of the declaration rather than asserted to be the binding point's
/// name, because a test that only checked the point existed would pass on a binding to
/// something else — and the whole of this stage is that the binding is to the image the light
/// named.
fn environment_map_source(compiled: &CompileOutput) -> String {
    let declaration = lighting_pass(compiled)
        .variant_fields()
        .and_then(|fields| fields.get(BINDERS))
        .and_then(Value::as_dict)
        .cloned()
        .expect("a pass carries its binders")
        .into_iter()
        .find(|(point, _)| point == ENVIRONMENT_MAP)
        .unwrap_or_else(|| {
            panic!(
                "the lighting pass binds the map at `{ENVIRONMENT_MAP}`, whatever the scene names"
            )
        })
        .1;
    declaration
        .variant_fields()
        .and_then(|fields| fields.get("resource"))
        .and_then(Value::variant_fields)
        .and_then(|fields| fields.get("source"))
        .and_then(Value::as_str)
        .expect("a texture declaration names the entry it reads")
        .to_owned()
}

/// Reads the environment field of the lighting pass's own uniform.
fn environment_uniform(compiled: &CompileOutput) -> Vec<f64> {
    lighting_pass(compiled)
        .variant_fields()
        .and_then(|fields| fields.get(UNIFORM))
        .and_then(Value::as_field_map)
        .and_then(|fields| fields.get(ENVIRONMENT))
        .and_then(Value::as_list)
        .expect("the environment field is written for every lighting pass")
        .iter()
        .map(|value| value.as_float().expect("a uniform field is a number"))
        .collect()
}

/// Compiles a scene the way a host would, and returns what it emitted.
fn compiled(scene: &Value) -> CompileOutput {
    compile(scene).expect("the scene compiles")
}

/// Compiles a scene the way a host would.
fn compile(scene: &Value) -> Result<CompileOutput, whiting_scene::CompileError> {
    let mut registry = Registry::new();
    registry.put_schema(
        SchemaIdentifier::new(GRAPHICS_SCHEMA),
        graphics::graphics_schema(),
    );
    registry.put_schema(
        SchemaIdentifier::new(INSTANCE_SCHEMA),
        graphics::instance_schema(),
    );
    whiting_scene::register_standard_scene_schema(&mut registry);
    let source = Definition::new(
        DefinitionIdentifier::new("environment.source"),
        SchemaIdentifier::new(STANDARD_SCENE_SCHEMA),
        scene.clone(),
    );
    source
        .validate(&registry)
        .expect("the scene validates against the schema");
    StandardScene::new(
        DefinitionIdentifier::new("environment.graphics"),
        DefinitionIdentifier::new("environment.instance"),
    )
    .compile(&CompileContext {
        scene: &source,
        previous_graphics: None,
        previous_instance: None,
        registry: &registry,
        camera: 0,
    })
}
