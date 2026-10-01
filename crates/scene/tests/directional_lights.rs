//! What the compiler does with a `Directional` light, and the page it is given one for.
//!
//! # What this holds that the picture cannot
//!
//! `crates/gallery/src/sunlight.rs` draws four parallel shadows, but a frame cannot show that
//! a directional light that was given **no page** rendered a scene with no shadow in it and
//! nothing to say so. It cannot show the size of a packed buffer either, and it cannot show
//! that a document was refused rather than guessed at.
//!
//! # The one claim this file exists to hold
//!
//! **An exempt light that casts a shadow is still given a page.** A directional light takes no
//! cluster slot, so the candidate walk never records it as having reached a cluster, and a test
//! that only asked whether the walk saw it would hand a sun no page. A fragment tested against
//! page minus one is lit unshadowed **by design** — that is what a light the pool had no room
//! for is supposed to do — so the failure is a sun that lights a scene and casts nothing, drawn
//! perfectly, with no note and no error. Every other test in this file could pass with that one
//! line reverted.
//!
//! The page is read out of the **packed record** rather than asked of the assignment, because
//! the record is what a fragment would look the page up in, and the number the test pins is
//! then the number the shader sees.

use definition::{Definition, DefinitionIdentifier, FieldMap, Registry, SchemaIdentifier, Value};
use graphics::{ENTRIES, GRAPHICS_SCHEMA, INSTANCE_SCHEMA, LABEL, PASSES_FIELD};
use whiting_scene::standard::example_standard_scene;
use whiting_scene::{
    CompileContext, CompileError, CompileOutput, Compiler, STANDARD_SCENE_SCHEMA, StandardScene,
};

/// The instance entry holding the record of every light.
const LIGHT_RECORDS: &str = "light_records";

/// The instance entry holding the cone of every light.
const SPOT_CONES: &str = "spot_cones";

/// The uniform field carrying the scene's directional light.
const DIRECTIONAL: &str = "directional";

/// A light record is six four byte blocks: a colour and a page, a position and a radius, and a
/// four by four projection. A directional light is a **stated input of the pass** and not a
/// record, so this is the size it still costs.
const RECORD_BYTES: i64 = 96;

/// A cone record is two four component vectors, because a `WGSL` `array<T>` in a storage buffer
/// lays its elements out at `roundUp(align_of_T, size_of_T)` and a struct holding a vector is
/// aligned to sixteen bytes.
const CONE_BYTES: i64 = 32;

/// The byte the record's page sits at, inside one record.
///
/// A three component vector is sixteen byte aligned and twelve bytes long, so `color` is at
/// zero and the single `f32` after it is at twelve. These are the offsets the shader reads
/// `LightRecord` at, so a number read here is the number a fragment would see.
const PAGE_AT: usize = 12;

/// Where the record's matrix starts, inside one record.
const PROJECTION_AT: usize = 32;

/// The page a light that was given none carries.
///
/// A value in the record's own type rather than a flag beside it, and below every legal page
/// index, so a record that was never written also reads as "no page".
const NO_PAGE: f32 = -1.0;

#[test]
fn an_exempt_light_that_casts_a_shadow_is_still_given_a_page() {
    // The whole reason this file exists. A directional light reaches no cluster, so `reached`
    // is false for it, and a page decided by `reached` alone would be page minus one — which
    // a fragment reads as **unshadowed**, by design. The scene would then render correctly and
    // wrongly at once: a sun that lights everything and casts nothing.
    let compiled = compiled(&scene(&[directional("sun", [0.0, -1.0, 0.0])]));

    assert_ne!(
        page_of(&compiled, 0),
        NO_PAGE,
        "the exemption is a decision about a cluster's cap, and a page is not a cluster slot: \
         one page is paid for once for the whole image however many clusters there are"
    );
}

#[test]
fn a_page_given_to_a_sun_is_drawn_through_a_parallel_projection() {
    // The projection is in the same record, so this reads the matrix the shadowing pass drew
    // the page through and the lighting pass looks it up through.
    //
    // The discriminator is the **bottom row**, and picking the right entry of it took some
    // care. A page is `projection × look_at`, and a column-major matrix acting on a column
    // vector takes its `w` from row three — so row three is `(m[3], m[7], m[11], m[15])`. For
    // a **parallel** projection that row is exactly the identity, so `look_at`'s own bottom row
    // survives the product and `m[15]` is one. For a **perspective** one the bottom row is
    // `(0, 0, -1, 0)`, so the product's bottom row is *minus* `look_at`'s third row and `m[15]`
    // comes out as `dot(back, eye)` — a distance, not a one.
    //
    // `m[11]` is **not** the test, even though it is zero for a sun: for the perspective product
    // it is `-back.z`, which is zero for any light shining straight up or down, so it would pass
    // for a spot light aimed at the floor.
    let compiled = compiled(&scene(&[directional("sun", [0.0, -1.0, 0.0])]));
    let projection = projection_of(&compiled, 0);

    assert_eq!(
        projection[15], 1.0,
        "every point keeps a `w` of one, which is what makes the shader's own `w <= 0` \
         early-out inert for a sun — a page with `w == -z` would fold everything behind the \
         light's plane back into the page and read it as lit"
    );

    // The contrast, so the assertion above cannot pass on a matrix that merely happens to have a
    // one in the corner. A **spot** light is the right kind to contrast with, and not a point
    // light: a spot light's cone gives its page a field of view, while a **point** light's page
    // is `orthographic × look_at` and therefore already a parallel projection. A test that
    // contrasted with a point light would be asserting that two identical shapes differ.
    let cone = compile(&scene(&[spot("lamp")])).expect("a spot light compiles");
    let frustum = projection_of(&cone, 0);
    assert_ne!(
        frustum[15], 1.0,
        "a spot light's page is a frustum, and it is the only kind here whose is"
    );
}

#[test]
fn a_directional_light_costs_nothing_in_the_clustered_buffers() {
    // Three lights, one of them a sun. Its data is a uniform field rather than record fields,
    // so the record is still ninety six bytes and the cone run is still thirty two — a scene
    // of a thousand point lights pays nothing for a sun it does not have.
    let compiled = compiled(&scene(&[
        point("first"),
        directional("sun", [0.0, -1.0, 0.0]),
        point("second"),
    ]));

    assert_eq!(
        size_of(&compiled, LIGHT_RECORDS),
        3 * RECORD_BYTES,
        "one record per light whatever the kind, which is what keeps a cluster index a plain \
         index into an array"
    );
    assert_eq!(
        size_of(&compiled, SPOT_CONES),
        3 * CONE_BYTES,
        "and one cone record per light, so the two runs are indexed by the same numbers"
    );
}

#[test]
fn a_sun_is_given_a_shadowing_pass_exactly_as_a_point_light_is() {
    // The other half of the page. A page with no pass drawn into it is a page of whatever was
    // in the texture array before, and a fragment compared against that reads as lit. So the
    // pass count is checked rather than assumed, and against the other kind rather than
    // against a number: one light either way is one pass either way.
    let with_sun = compiled(&scene(&[directional("sun", [0.0, -1.0, 0.0])]));
    let with_point = compiled(&scene(&[point("lamp")]));

    assert_eq!(
        shadowing_passes(&with_sun),
        shadowing_passes(&with_point),
        "a light given a page is drawn into one, and a sun is a light"
    );
    assert!(
        shadowing_passes(&with_sun) > 0,
        "and there is at least one, or the comparison above is comparing two noughts"
    );
}

#[test]
fn a_scenes_uniform_carries_the_sun_whether_the_scene_names_one_or_not() {
    // The layout is the same for every scene, so the field is written for every scene. A field
    // that came and went with the document would put every field after it somewhere else, and
    // the device's uniform check would report a shader and an interface that disagree — or
    // worse, agree by accident and read the wrong numbers.
    let named = uniform_of(&compiled(&scene(&[directional("sun", [0.0, -1.0, 0.0])])));
    let unnamed = uniform_of(&compiled(&scene(&[point("lamp")])));

    assert!(
        has_field(&named, DIRECTIONAL),
        "a scene naming a sun writes the field"
    );
    assert!(
        has_field(&unnamed, DIRECTIONAL),
        "and so does one naming none, which is the environment's rule applied again"
    );
    assert_eq!(
        vector_fourth(&unnamed, "colour"),
        0.0,
        "a scene naming none writes a zero flag, and the shader takes its path on the flag \
         rather than on the colour being black"
    );
    assert_eq!(
        vector_fourth(&named, "colour"),
        1.0,
        "and one naming a sun writes the flag that says so"
    );
}

#[test]
fn a_suns_uniform_page_is_the_page_its_record_carries() {
    // Two places now hold the page: the record, which is dead weight kept for the index, and
    // the uniform field a fragment actually reads. A sun reads the second and a point light
    // reads the first, and the two are the same number by construction rather than by two
    // answers happening to agree.
    let compiled = compiled(&scene(&[directional("sun", [0.0, -1.0, 0.0])]));
    let page = vector_fourth(&uniform_of(&compiled), "direction") as f32;

    assert_eq!(
        page,
        page_of(&compiled, 0),
        "the page the fragment reads is the page the shadowing pass drew, read from one answer \
         rather than two"
    );
}

#[test]
fn a_second_directional_light_is_refused() {
    // One directional light is what a lighting pass carries, because a light that stands
    // nowhere is in no cluster list and a fragment reaches it through a stated input rather than
    // through a record. A scene naming two would have one of them read and the other ignored,
    // which is a light an author wrote and nobody sees.
    let message = refused(&scene(&[
        directional("first", [0.0, -1.0, 0.0]),
        directional("second", [1.0, -1.0, 0.0]),
    ]));
    assert!(
        message.contains("second") && message.contains("first"),
        "the refusal names the light refused and the one already read: {message}"
    );
}

#[test]
fn a_direction_that_is_not_a_direction_is_refused() {
    // A zero direction has no axis to compare a surface against, and dividing by its length
    // would put a `NaN` in a fragment rather than a refusal in a document. The spot light's own
    // refusal is the same check reached from the other kind that declares the field.
    let message = refused(&scene(&[directional("sun", [0.0, 0.0, 0.0])]));
    assert!(
        message.contains("direction") && message.contains("sun"),
        "the refusal names the field and the light: {message}"
    );
}

/// Builds a directional light shining along a direction, which stands nowhere.
fn directional(label: &str, direction: [f64; 3]) -> Value {
    Value::variant(
        "Directional",
        [
            ("label".to_owned(), Value::String(label.to_owned())),
            ("direction".to_owned(), triple(direction)),
            ("color".to_owned(), triple([1.0, 0.95, 0.9])),
            ("intensity".to_owned(), Value::Float(1.0)),
            ("shadow_extent".to_owned(), Value::Float(6.0)),
        ],
    )
}

/// Builds a point light standing at a position, which is the kind that does have one.
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

/// Builds a spot light, which is the only kind whose page is a perspective frustum.
fn spot(label: &str) -> Value {
    Value::variant(
        "Spot",
        [
            ("label".to_owned(), Value::String(label.to_owned())),
            ("position".to_owned(), triple([4.0, 6.0, 2.0])),
            ("direction".to_owned(), triple([0.0, -1.0, 0.0])),
            ("inner_angle".to_owned(), Value::Float(0.16)),
            ("outer_angle".to_owned(), Value::Float(0.56)),
            ("color".to_owned(), triple([1.0, 0.95, 0.9])),
            ("intensity".to_owned(), Value::Float(1.0)),
            ("shadow_extent".to_owned(), Value::Float(6.0)),
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

/// Returns the page the light at `index` was given, read out of its packed record.
///
/// The record run is the thing a cluster index points into, so this is the number a fragment
/// would look a page up with — and the number a fragment tests against minus one to mean "no
/// page", which is why it is read from the bytes rather than asked of the assignment.
fn page_of(compiled: &CompileOutput, index: usize) -> f32 {
    let bytes = entry_bytes(compiled, LIGHT_RECORDS);
    f32::from_le_bytes(
        bytes[index * RECORD_BYTES as usize + PAGE_AT..][..4]
            .try_into()
            .expect("a page is four bytes"),
    )
}

/// Returns the sixteen floats of the light at `index`'s shadow projection, column major.
fn projection_of(compiled: &CompileOutput, index: usize) -> [f32; 16] {
    let bytes = entry_bytes(compiled, LIGHT_RECORDS);
    let start = index * RECORD_BYTES as usize + PROJECTION_AT;
    std::array::from_fn(|offset| {
        f32::from_le_bytes(
            bytes[start + offset * 4..][..4]
                .try_into()
                .expect("a matrix entry is four bytes"),
        )
    })
}

/// Returns the scene's one lighting pass's uniform, as the named fields it carries.
fn uniform_of(compiled: &CompileOutput) -> FieldMap {
    lighting_pass(compiled)
        .variant_fields()
        .and_then(|fields| fields.get(graphics::UNIFORM))
        .and_then(|uniform| uniform.as_field_map())
        .cloned()
        .unwrap_or_else(|| panic!("a pass carries its uniform"))
}

/// Returns whether the uniform carries a field under that name.
fn has_field(uniform: &FieldMap, field: &str) -> bool {
    uniform.contains_key(field)
}

/// Returns one field of the lighting pass's uniform.
fn field_of(uniform: &FieldMap, field: &str) -> Value {
    uniform
        .get(field)
        .cloned()
        .unwrap_or_else(|| panic!("the uniform carries `{field}`"))
}

/// Returns the fourth component of one of the directional light's two vectors, which is the flag
/// and the page respectively.
fn vector_fourth(uniform: &FieldMap, field: &str) -> f64 {
    field_of(uniform, DIRECTIONAL)
        .as_field_map()
        .and_then(|fields| fields.get(field))
        .and_then(Value::as_list)
        .and_then(|list| list.last())
        .and_then(Value::as_float)
        .unwrap_or_else(|| panic!("`{field}` is a four component vector"))
}

/// Returns the bytes of one packed run in the instance document.
fn entry_bytes(compiled: &CompileOutput, entry: &str) -> Vec<u8> {
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
        .map(<[u8]>::to_vec)
        .unwrap_or_else(|| {
            panic!("{entry} is packed into the instance document, and it has no data field")
        })
}

/// Returns the byte size the instance document gives one packed run.
fn size_of(compiled: &CompileOutput, entry: &str) -> i64 {
    entry_bytes(compiled, entry).len() as i64
}

/// Returns how many shadowing passes the scene drew, which is one per light that was given a
/// page per shadowing group.
///
/// The stage's own prefix is `shadow` and not `shadowing`: a pass is labelled
/// `shadow/<light>/<shader>/<mesh>`, and a filter on the longer word would find nothing and
/// quietly compare two empty counts.
fn shadowing_passes(compiled: &CompileOutput) -> usize {
    compiled
        .graphics
        .value
        .as_field_map()
        .and_then(|fields| fields.get(PASSES_FIELD))
        .and_then(Value::as_list)
        .expect("the graphics document holds passes")
        .iter()
        .filter(|pass| {
            pass.variant_fields()
                .and_then(|fields| fields.get(LABEL))
                .and_then(Value::as_str)
                .is_some_and(|label| label.starts_with("shadow/"))
        })
        .count()
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
        DefinitionIdentifier::new("directional.source"),
        SchemaIdentifier::new(STANDARD_SCENE_SCHEMA),
        scene.clone(),
    );
    source
        .validate(&registry)
        .expect("the scene validates against the schema");
    StandardScene::new(
        DefinitionIdentifier::new("directional.graphics"),
        DefinitionIdentifier::new("directional.instance"),
    )
    .compile(&CompileContext {
        scene: &source,
        previous_graphics: None,
        previous_instance: None,
        registry: &registry,
        camera: 0,
    })
}
