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
//! # The two claims this file exists to hold
//!
//! **A light record is one size whatever kind of light it is.** That is what the separate cone run
//! bought: a scene of a thousand point lights pays nothing for spot lights it does not have. A test
//! that only checked a spot light renders would pass with the record quietly grown to a hundred and
//! twenty eight, which is the other way this could have been built.
//!
//! **A page is drawn into the slot its table numbers it into.** A spot light's shadow stopped being
//! one texture and became a set of them, written by one pass apiece and read by a fragment that
//! finds the piece by walking a table packed behind the records. Nothing at run time compares the
//! two, and a frame cannot tell a page drawn where nothing looks from a page read before anything
//! was drawn into it, so [`the_pages_a_spot_light_emits_are_the_pages_its_table_answers`] reads
//! both halves out of one compiled document and holds them against each other.

use definition::{Definition, DefinitionIdentifier, Registry, SchemaIdentifier, Value};
use graphics::{
    BINDERS, ENTRIES, GRAPHICS_SCHEMA, INSTANCE_SCHEMA, LABEL, PASSES_FIELD, layers_of,
};
use std::collections::BTreeMap;
use whiting_gallery::Gallery;
use whiting_scene::lighting::record_table_base;
use whiting_scene::standard::RADIUS;
use whiting_scene::standard::example_standard_scene;
use whiting_scene::standard::{SHADOW_ATLAS, VIEW_PAGE};
use whiting_scene::vshadow::{PAGE_SIZE, levels_for, pages_at_level, unpack_high, unpack_low};
use whiting_scene::{
    CompileContext, CompileError, CompileOutput, Compiler, STANDARD_SCENE_SCHEMA, StandardScene,
};

/// The instance entry holding the cone of every light.
const SPOT_CONES: &str = "spot_cones";

/// The instance entry holding the record of every light.
const LIGHT_RECORDS: &str = "light_records";

/// A light record is seven four byte blocks: a colour and a page, a position and a radius, a four
/// by four projection, and the address of the light's virtual shadow pages. It is the size the cone
/// run's existence is protecting.
const RECORD_BYTES: i64 = 112;

/// A cone record is two four component vectors, because a `WGSL` `array<T>` in a storage buffer
/// lays its elements out at `roundUp(align_of_T, size_of_T)` and a struct holding a vector is
/// aligned to sixteen bytes.
const CONE_BYTES: i64 = 32;

#[test]
fn the_light_record_grew_by_a_page_address_and_not_by_a_cone() {
    // **A scene of nothing but point lights**, because a spot light's record run carries its
    // virtual page table appended to the records, and a size read off that run is a size that has
    // to be subtracted before it means anything. Three records of a fixed size is the claim; a
    // run that holds three records and a table is a different measurement.
    let points = compiled(&scene(&[point("first"), point("second"), point("third")]));
    assert_eq!(
        size_of(&points, LIGHT_RECORDS),
        3 * RECORD_BYTES,
        "a record is one size whatever kind of light it is, so the million light path is \
         unchanged by spot lights existing"
    );

    // Three lights, one of them a spot. If the cone's fields were in the record, this size would
    // be the cone's thirty two bytes times three rather than what it is.
    //
    // The record did grow once, by the sixteen bytes of a virtual page address, and it grew for
    // every kind of light rather than only for the one that has a grid. That is the same trade the
    // cone run is written to avoid, made the other way on purpose: the two numbers a virtual
    // lookup needs are eight bytes, and a record has to stay a whole number of sixteen byte
    // blocks, so eight bytes of pad came with them whether a scene wanted them or not.
    let compiled = compiled(&scene(&[
        point("first"),
        spot("lamp", [0.0, -1.0, 0.0], 0.16, 0.56),
        point("second"),
    ]));
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
            // A point light is required to say how far it reaches; the field has no default.
            // The spot above deliberately names none: it is culled by the reach of its cone.
            (RADIUS.to_owned(), Value::Float(20.0)),
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
        // No machine behind this compile, so it is made against the floor.
        limits: None,
    })
}

/// Where a light record's virtual page address starts, in the words a shader counts: the four
/// after the record's shadow projection.
///
/// It is an offset rather than a field because the record is packed as a run of floats, and the
/// same four numbers are spelled out beside `light_record` in `crates/gallery/src/shaders.rs`.
const VIRTUAL_PAGE_WORD: usize = 24;

/// One page of one light's grid, as each end of the emission describes it.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
struct Page {
    /// Which level of the grid it is on, zero being the finest.
    level: u32,
    /// Where in that level's row it sits, in pages.
    column: u32,
    /// Which row of that level.
    row: u32,
}

#[test]
fn the_pages_a_spot_light_emits_are_the_pages_its_table_answers() {
    // A page is written down twice, by two halves of one emitter that do not call each other. The
    // allocator numbers it into the table a fragment walks; the pass that draws it names the atlas
    // slot its viewport fills and the window of the light's plane it fills that slot with. A pass
    // on a slot its table does not resolve to is a shadow drawn where nobody looks, and a window
    // that is not the page the bit was set for is a fragment reading a rectangle of somebody
    // else's depth. Both of those render, and render wrong by one lamp's worth of shadow, which is
    // why this compares the two documents instead of trusting either.
    let scene = Gallery::build("spotlight");
    let compiled = compiled(&scene);
    let records = packed_words(&compiled, LIGHT_RECORDS);
    let header = records[VIRTUAL_PAGE_WORD];
    let layer = records[VIRTUAL_PAGE_WORD + 1];
    assert!(
        header >= 0.0,
        "the lamp of the one scene built to be lit by a cone is recorded as a grid of pages"
    );
    assert_eq!(
        header as usize,
        record_table_base(lights_of(&scene)) as usize,
        "the table is appended to the records that address it, because four storage buffers is \
         the whole budget and the records already hold one"
    );

    let shape = word(&records, header as usize);
    let kinds = word(&records, header as usize + 1);
    let pages_per_side = unpack_low(shape);
    let atlas_side = unpack_high(shape).max(1);
    let levels = unpack_low(kinds);
    assert_eq!(unpack_high(kinds), 1, "a spot light has one face");
    assert_eq!(
        levels,
        levels_for(pages_per_side),
        "the grid carries every level of it that is still a whole page"
    );
    let side = f64::from(pages_per_side * PAGE_SIZE);

    // The table's answer, walked exactly as the fragment walks it: a row's base slot, plus how
    // many bits of that row lie before the one that names this page.
    let mut mapped = BTreeMap::new();
    for level in 0..levels {
        let section = word(&records, header as usize + 2 + level as usize) as usize;
        let pages = pages_at_level(pages_per_side, level);
        let words = 1 + pages.div_ceil(32) as usize;
        for row in 0..pages {
            let start = header as usize + section + row as usize * words;
            let base = word(&records, start);
            let mut rank = 0;
            for word_index in 0..words - 1 {
                let mut bits = word(&records, start + 1 + word_index);
                while bits != 0 {
                    let column = word_index as u32 * 32 + bits.trailing_zeros();
                    assert!(
                        column < pages,
                        "a bit names a page past the last column of a {pages} page row"
                    );
                    assert_eq!(
                        mapped.insert(base + rank, Page { level, column, row }),
                        None,
                        "one slot of the atlas is one page"
                    );
                    rank += 1;
                    bits &= bits - 1;
                }
            }
        }
    }
    assert!(
        !mapped.is_empty(),
        "a scene whose lamp casts nothing has no pages to compare, and this test would pass on \
         an empty pair of maps"
    );

    // The passes' answer, read off the region each one draws into and the window it draws through.
    let mut drawn = BTreeMap::new();
    for pass in graphics::passes(&compiled.graphics.value).expect("the passes read") {
        let Some(Value::List(region)) = pass.optional(graphics::VIEWPORT) else {
            continue;
        };
        let [x, y, width, height] = four_ints(region, "a region is four texel counts");
        assert_eq!(
            (width, height),
            (i64::from(PAGE_SIZE), i64::from(PAGE_SIZE)),
            "a page is the size the allocator allocates, whatever level it is on"
        );
        assert_eq!(
            (x % i64::from(PAGE_SIZE), y % i64::from(PAGE_SIZE)),
            (0, 0),
            "a page begins on a slot boundary, because a slot is a page and the atlas is a grid \
             of them"
        );
        let slot =
            (x / i64::from(PAGE_SIZE) + (y / i64::from(PAGE_SIZE)) * i64::from(atlas_side)) as u32;

        let window = pass
            .uniform()
            .expect("a page pass carries its light's projection and its own window")
            .as_field_map()
            .and_then(|fields| fields.get(VIEW_PAGE))
            .and_then(Value::as_list)
            .expect("the window is four numbers of the light's plane");
        let [min_x, min_y, extent_x, extent_y] =
            four_floats(window, "a window is four plane edges");
        assert_eq!(
            extent_x, extent_y,
            "a page is square, because a texel of it covers the same world either way"
        );
        // The level is not named by a pass and cannot be: what a pass carries is how much of the
        // light's plane its viewport fills, and the level is that number read the other way.
        let shift = extent_x * side / 2.0;
        let level = (shift / f64::from(PAGE_SIZE)).log2();
        assert!(
            level.fract() == 0.0 && level >= 0.0 && level < levels as f64,
            "a window of {extent_x} of the plane is {shift} texels across, which is no level of \
             a grid with {levels} of them"
        );
        let column = ((min_x + 1.0) * side / (2.0 * shift)).round() as u32;
        // The plane's vertical axis runs the other way to the atlas's, which is why the row is
        // read off the window's *far* edge and one page subtracted.
        let row = (((1.0 - min_y) * side / (2.0 * shift)).round() - 1.0) as u32;
        let page = Page {
            level: level as u32,
            column,
            row,
        };
        // One page is drawn by as many passes as there are groups that cast into it, so what a
        // slot has to be unique about is not the pass but the rectangle. Two windows on one slot
        // is a page whose shadow is whichever group happened to draw last, and the depth test
        // cannot tell that a page was meant to hold two different pieces of the light's plane.
        if let Some(previous) = drawn.insert(slot, page) {
            assert_eq!(
                previous, page,
                "two passes drew slot {slot} with different windows"
            );
        }
    }

    assert_eq!(
        drawn, mapped,
        "every page the table maps is drawn into the slot it names, at the window its row and \
         column say, and no page is drawn that the table has no bit for"
    );

    let atlas = compiled
        .instance
        .value
        .as_field_map()
        .and_then(|fields| fields.get(ENTRIES))
        .and_then(Value::as_dict)
        .and_then(|entries| entries.get(SHADOW_ATLAS))
        .expect("the atlas is in the instance document, bound by every pass that numbers one");
    let layers = layers_of(atlas).expect("the shadow atlas is an array");
    assert!(
        (layer as i64) < layers,
        "the record sends a fragment to layer {layer} of an array with {layers} of them"
    );
}

/// Returns one word of a packed run, as the bit pattern the shader's own `bitcast` sees.
fn word(words: &[f32], at: usize) -> u32 {
    words
        .get(at)
        .unwrap_or_else(|| {
            panic!(
                "the run holds {} words and this table asked for the {at}th",
                words.len()
            )
        })
        .to_bits()
}

/// Returns every float of a packed instance run, in the order the compiler wrote them.
fn packed_words(compiled: &CompileOutput, entry: &str) -> Vec<f32> {
    let bytes = compiled
        .instance
        .value
        .as_field_map()
        .and_then(|fields| fields.get(ENTRIES))
        .and_then(Value::as_dict)
        .and_then(|entries| entries.get(entry))
        .and_then(Value::variant_fields)
        .and_then(|fields| fields.get("data"))
        .and_then(Value::as_bytes)
        .unwrap_or_else(|| {
            panic!("{entry} is packed into the instance document, and it has no data field")
        });
    bytes
        .chunks_exact(4)
        .map(|chunk| f32::from_le_bytes(chunk.try_into().expect("a word is four bytes")))
        .collect()
}

/// Reads a four element run of texel counts.
fn four_ints(values: &[Value], what: &str) -> [i64; 4] {
    let mut out = [0; 4];
    for (index, value) in values.iter().enumerate() {
        out[index] = value
            .as_int()
            .unwrap_or_else(|| panic!("{what}, and element {index} is not one"));
    }
    assert_eq!(values.len(), 4, "{what}");
    out
}

/// Reads a four element run of plane coordinates.
fn four_floats(values: &[Value], what: &str) -> [f64; 4] {
    let mut out = [0.0; 4];
    for (index, value) in values.iter().enumerate() {
        out[index] = value
            .as_float()
            .unwrap_or_else(|| panic!("{what}, and element {index} is not one"));
    }
    assert_eq!(values.len(), 4, "{what}");
    out
}

/// Returns how many lights a scene declares.
fn lights_of(scene: &Value) -> usize {
    scene
        .as_field_map()
        .and_then(|fields| fields.get("lights"))
        .and_then(Value::as_list)
        .expect("a scene names its lights")
        .len()
}
