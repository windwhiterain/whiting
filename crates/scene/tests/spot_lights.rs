//! What the compiler does with a `Spot` light, and the shape of the cone run it packs.
//!
//! # What this holds that the pixel test cannot
//!
//! `crates/gallery/src/spotlight.rs` renders a cone, but a frame cannot show that a light whose
//! two angles disagree was **refused** rather than drawn, and it cannot show the size of a
//! buffer. Both are the document side of this stage: the cone run is a second storage buffer,
//! and the whole reason it is a second buffer rather than more fields of the light record is a
//! claim about sizes that has to be pinned by something.
//!
//! # The one claim this file exists to hold
//!
//! **The light record is still ninety six bytes.** That is what the separate cone run bought: a
//! scene of a thousand point lights pays nothing for spot lights it does not have. A test that
//! only checked a spot light renders would pass with the record quietly grown to a hundred and
//! twenty eight, which is the other way this could have been built.

use definition::{Definition, DefinitionIdentifier, Registry, SchemaIdentifier, Value};
use graphics::{BINDERS, ENTRIES, GRAPHICS_SCHEMA, INSTANCE_SCHEMA, LABEL, PASSES_FIELD};
use whiting_scene::standard::example_standard_scene;
use whiting_scene::{
    CompileContext, CompileError, CompileOutput, Compiler, STANDARD_SCENE_SCHEMA, StandardScene,
};

/// The instance entry holding the cone of every light.
const SPOT_CONES: &str = "spot_cones";

/// The instance entry holding the record of every light.
const LIGHT_RECORDS: &str = "light_records";

/// A light record is six four byte blocks: a colour and a page, a position and a radius, and a
/// four by four projection. It is the size the cone run's existence is protecting.
const RECORD_BYTES: i64 = 96;

/// A cone record is two four component vectors, because a `WGSL` `array<T>` in a storage buffer
/// lays its elements out at `roundUp(align_of_T, size_of_T)` and a struct holding a vector is
/// aligned to sixteen bytes.
const CONE_BYTES: i64 = 32;

#[test]
fn the_light_record_is_still_ninety_six_bytes_so_a_point_light_pays_nothing_for_a_cone() {
    // Three lights, one of them a spot. If the cone's fields were in the record, this size would
    // be the cone's thirty two bytes times three rather than what it is.
    let compiled = compiled(&scene(&[
        point("first"),
        spot("lamp", [0.0, -1.0, 0.0], 0.16, 0.56),
        point("second"),
    ]));
    assert_eq!(
        size_of(&compiled, LIGHT_RECORDS),
        3 * RECORD_BYTES,
        "a record is ninety six bytes whatever kind of light it is, so the million light path \
         is unchanged by spot lights existing"
    );
    assert_eq!(
        size_of(&compiled, SPOT_CONES),
        3 * CONE_BYTES,
        "and the cone is a run of its own, one record per light, in the same order"
    );
}

#[test]
fn a_light_with_no_cone_carries_the_whole_sphere_so_a_point_light_is_scaled_by_exactly_one() {
    // The run has one record per light **whatever the kinds are**, because a cluster index is a
    // plain index into it: a light with no cone cannot leave a hole for a fragment to test.
    let compiled = compiled(&scene(&[point("only")]));
    assert_eq!(
        size_of(&compiled, SPOT_CONES),
        CONE_BYTES,
        "one light is one cone record, and a light with no cone of its own carries the whole \
         sphere rather than an absent field"
    );
}

#[test]
fn a_scene_naming_no_light_still_binds_a_cone_run_of_one() {
    // This is the same floor the light record was given: a scene whose only light is the
    // environment has no cluster pointing at any light, and the shader's `array<SpotCone>` wants
    // a whole record, so a run of nothing is four bytes of alignment where a binding wants a
    // record. wgpu refuses that draw by name rather than rendering it.
    let compiled = compiled(&scene(&[]));
    assert_eq!(
        size_of(&compiled, SPOT_CONES),
        CONE_BYTES,
        "no light is still one record, for the same reason the light record is"
    );
    assert_eq!(
        size_of(&compiled, LIGHT_RECORDS),
        RECORD_BYTES,
        "and the light record beside it"
    );
}

#[test]
fn the_cone_run_is_bound_by_a_scene_that_names_no_spot_light() {
    // A pass binds its resources in name order, so a binding that came and went with the
    // document would move every number below it and a shader would have to be written for one
    // scene and not for another. This is the environment map's rule, applied again.
    let compiled = compiled(&scene(&[point("only")]));
    let binders = lighting_pass(&compiled)
        .variant_fields()
        .and_then(|fields| fields.get(BINDERS))
        .and_then(Value::as_dict)
        .cloned()
        .expect("a pass carries its binders");
    assert!(
        binders.contains_key(SPOT_CONES),
        "a scene with no spot light still binds the cone run, the way it still binds the \
         environment map"
    );
}

#[test]
fn a_cone_whose_outer_angle_does_not_exceed_its_inner_one_is_refused() {
    // The two angles describe a cone, and a cone whose outer angle is inside its inner one is
    // inverted: a document whose two answers disagree, which is a refusal rather than a guess.
    for (inner, outer) in [(0.56, 0.16), (0.3, 0.3)] {
        let message = refused(&scene(&[spot("lamp", [0.0, -1.0, 0.0], inner, outer)]));
        assert!(
            message.contains("outer_angle") && message.contains("lamp"),
            "the refusal names the field and the light: {message}"
        );
    }
}

#[test]
fn a_cone_wider_than_a_half_turn_is_refused() {
    // Past a half turn the "outer" region includes the light's own axis, and the falloff between
    // the two angles would be a ramp that goes back up. The same reason a zero radius is refused.
    let message = refused(&scene(&[spot("lamp", [0.0, -1.0, 0.0], 0.2, 4.0)]));
    assert!(
        message.contains("outer_angle"),
        "the refusal names the field: {message}"
    );
}

#[test]
fn a_direction_that_is_not_a_direction_is_refused() {
    // A zero direction has no axis to compare a surface against, and dividing by its length
    // would put a `NaN` in a fragment rather than a refusal in a document.
    let message = refused(&scene(&[spot("lamp", [0.0, 0.0, 0.0], 0.16, 0.56)]));
    assert!(
        message.contains("direction"),
        "the refusal names the field: {message}"
    );
}

/// Builds a spot light standing at a position, shining along a direction between two half angles.
fn spot(label: &str, direction: [f64; 3], inner: f64, outer: f64) -> Value {
    Value::variant(
        "Spot",
        [
            ("label".to_owned(), Value::String(label.to_owned())),
            ("position".to_owned(), triple([4.0, 6.0, 2.0])),
            ("direction".to_owned(), triple(direction)),
            ("inner_angle".to_owned(), Value::Float(inner)),
            ("outer_angle".to_owned(), Value::Float(outer)),
            ("color".to_owned(), triple([1.0, 0.95, 0.9])),
            ("intensity".to_owned(), Value::Float(1.0)),
            ("shadow_extent".to_owned(), Value::Float(2.5)),
        ],
    )
}

/// Builds a point light, which is the kind that has no cone of its own.
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

/// Builds the example scene with its one light replaced, which is the smallest document that
/// compiles and therefore the smallest way to ask what the compiler does with a different light.
fn scene(lights: &[Value]) -> Value {
    let mut fields = example_standard_scene()
        .as_field_map()
        .cloned()
        .expect("the example scene is a structure");
    fields.insert("lights".to_owned(), Value::List(lights.to_vec()));
    Value::structure(fields)
}

/// Builds a three component list of floats.
fn triple(components: [f64; 3]) -> Value {
    Value::List(components.into_iter().map(Value::Float).collect())
}

/// Returns the byte size the instance document gives one packed run.
///
/// A packed run is an instance entry holding the bytes, so the size is the length of its `data`
/// rather than a number read out of a declaration: a declaration states the size a **binding**
/// expects, and this is about what the compiler actually packed.
fn size_of(compiled: &CompileOutput, entry: &str) -> i64 {
    compiled
        .instance
        .value
        .as_field_map()
        .and_then(|fields| fields.get(ENTRIES))
        .and_then(Value::as_dict)
        .and_then(|entries| entries.get(entry))
        .and_then(Value::variant_fields)
        .and_then(|fields| fields.get("data"))
        .and_then(Value::as_bytes)
        .map(<[u8]>::len)
        .unwrap_or_else(|| {
            panic!("{entry} is packed into the instance document, and it has no data field")
        }) as i64
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

/// Compiles a scene, expecting it to succeed.
fn compiled(scene: &Value) -> CompileOutput {
    compile(scene).expect("the scene compiles")
}

/// Compiles a scene expecting it to be refused, and returns what it was told.
///
/// `CompileOutput` is not `Debug`, so `Result::expect_err` cannot be used here; this is the same
/// assertion written out.
fn refused(scene: &Value) -> String {
    match compile(scene) {
        Ok(_) => panic!("the scene compiles, and this test is about a document it must refuse"),
        Err(failure) => failure.to_string(),
    }
}

/// Compiles a scene the way a host would.
fn compile(scene: &Value) -> Result<CompileOutput, CompileError> {
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
        DefinitionIdentifier::new("spot.source"),
        SchemaIdentifier::new(STANDARD_SCENE_SCHEMA),
        scene.clone(),
    );
    source
        .validate(&registry)
        .expect("the scene validates against the schema");
    StandardScene::new(
        DefinitionIdentifier::new("spot.graphics"),
        DefinitionIdentifier::new("spot.instance"),
    )
    .compile(&CompileContext {
        scene: &source,
        previous_graphics: None,
        previous_instance: None,
        registry: &registry,
        camera: 0,
    })
}
