//! What an emitted document says it renders out, read back through the reader both sides use.
//!
//! # Why this is on the scene side
//!
//! The check itself is the device's, and it is proved there without a gpu
//! (`crates/device/tests/typed_outputs.rs`). What this file answers is the question on the other
//! side of the compile: **does the emitter write the declaration the check is going to read?**
//! A shape that compiles, renders and is then misread is a defect in the seam, and neither
//! half's own test can see it.
//!
//! It reads through [`graphics::outputs`] rather than through the raw value, on purpose: a
//! reader that went round the shared one would be testing a second implementation of the
//! declaration's shape, which is precisely the third copy this change exists to remove.

use definition::{Definition, DefinitionIdentifier, Registry, SchemaIdentifier, Value};
use graphics::{LOCATION_FIELD, Output, POINT_FIELD, TYPE_FIELD};
use whiting_gallery::Gallery;
use whiting_scene::{
    CompileContext, Compiler, DebugChannel, STANDARD_SCENE_SCHEMA, StandardScene,
    register_standard_scene_schema,
};

/// Compiles a scene with the channels a caller asked for.
fn compiled(channels: &[DebugChannel]) -> whiting_scene::CompileOutput {
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
        DefinitionIdentifier::new("typed-outputs.scene"),
        SchemaIdentifier::new(STANDARD_SCENE_SCHEMA),
        // The gallery is a dev-dependency of this crate, which is the only reason a test here
        // can reach a real scene: the cycle exists for tests alone and the published library's
        // dependency graph never sees it.
        Gallery::build("tidewater"),
    );
    StandardScene::new(
        DefinitionIdentifier::new("typed-outputs.graphics"),
        DefinitionIdentifier::new("typed-outputs.instance"),
    )
    .with_debug(channels.iter().copied())
    .compile(&CompileContext {
        scene: &source,
        previous_graphics: None,
        previous_instance: None,
        registry: &registry,
        camera: 0,
        limits: None,
    })
    .unwrap_or_else(|error| panic!("the scene compiles: {error}"))
}

/// Returns the declaration a name is emitted under, or the panic says there is none.
fn declared<'document>(outputs: &[Output<'document>], name: &str) -> Output<'document> {
    outputs
        .iter()
        .find(|output| output.name == name)
        .unwrap_or_else(|| panic!("`{name}` is declared"))
        .to_owned()
}

#[test]
fn the_frame_is_declared_with_a_point_a_location_and_a_type() {
    let emitted = compiled(&[]);
    let outputs = graphics::outputs(&emitted.graphics.value).expect("the outputs read");
    let frame = declared(&outputs, graphics::PRESENT_OUTPUT);

    // The three facts, and each of them carrying its own weight: the point is where the image
    // is rendered, the location is which fragment output fills it, and the type is what a texel
    // of it holds. A document that stated only the point would be the shape this change replaced.
    assert_eq!(frame.point, "present_target", "the post stage's own target");
    assert_eq!(
        frame.location, 0,
        "the frame is the pass's only colour target"
    );
    assert_eq!(
        frame.ty, "vec4<f32>",
        "every shader in this pipeline returns four floats at location zero, and the declaration \
         says so rather than leaving the type to be read out of the bytes"
    );
}

#[test]
fn a_builtin_channel_is_declared_from_its_own_registry_entry() {
    let emitted = compiled(&[DebugChannel::Position, DebugChannel::Depth]);
    let outputs = graphics::outputs(&emitted.graphics.value).expect("the outputs read");

    // Each built-in's three facts come from the registry entry rather than from the emitter, so
    // the compiler cannot declare one thing and draw as another. That is the whole of the split:
    // a built-in is scene-independent and states its own type, and the check never compares it
    // against a shader because the shader beside it is the compiler's own.
    for (channel, point) in [
        (DebugChannel::Position, "debug_position"),
        (DebugChannel::Depth, "debug_depth"),
    ] {
        let found = declared(&outputs, channel.output());
        assert_eq!(found.point, point);
        assert_eq!(found.location, channel.location());
        assert_eq!(found.ty, channel.ty().name());
    }
}

#[test]
fn the_built_in_format_is_derived_from_the_declared_type_and_is_one_table() {
    // The thing the change was for: `DebugChannel::format` is no longer a `match` on the
    // variant. Every built-in and every scene-declared output resolve their format through the
    // one table beside the vocabulary, so a type that is added in one place gets a format in the
    // same place rather than in a second private list.
    for channel in DebugChannel::ALL {
        assert_eq!(channel.format(), channel.ty().format());
        assert_eq!(channel.format(), "rgba16float");
    }
    assert_eq!(
        graphics::TexelType::Float.format(),
        "r32float",
        "the table has one row per type and each row is a format this device creates and reads \
         back; a single channel is a single channel"
    );
}

#[test]
fn every_row_claims_no_more_numbers_than_the_image_it_is_drawn_in_holds() {
    // The second of the two questions the device's check asks, which is a property of the table
    // rather than of any document: a declaration states how many numbers a texel is, and the image
    // holding it has to have a component for each one. A row that broke this would be a
    // declaration the device refuses against its own table, so the invariant is held here where a
    // new row is added, and the device's check refuses it in its own words rather than the table
    // carrying the rule.
    //
    // The two numbers are read through the table and not written out here, so this states no fact
    // of its own beyond the relation between them.
    for texel in graphics::TexelType::ALL {
        assert!(
            texel.components() <= texel.image_width(),
            "`{}` claims {} numbers and is drawn in an image {} components wide, and an image has \
             no component for a number beyond its own width",
            texel.name(),
            texel.components(),
            texel.image_width(),
        );
    }
    // And the two are genuinely different numbers for the row that makes the padding case, which
    // is the whole reason the check asks them separately: a `vec3<f32>` is three numbers drawn
    // four wide, and a `f32` is one drawn one wide, so it is the first that has padding at all.
    assert_eq!(graphics::TexelType::Vec3Float.components(), 3);
    assert_eq!(graphics::TexelType::Vec3Float.image_width(), 4);
    assert_eq!(graphics::TexelType::Float.components(), 1);
    assert_eq!(graphics::TexelType::Float.image_width(), 1);
    assert_eq!(graphics::TexelType::Vec4Float.components(), 4);
    assert_eq!(graphics::TexelType::Vec4Float.image_width(), 4);
}

#[test]
fn a_three_component_sidecar_says_its_fourth_channel_carries_nothing() {
    // The padding-versus-marker decision, read off the two files a caller would actually hold
    // side by side. A `vec3<f32>` declaration draws an image with a fourth channel in it, and
    // that channel is padding; `Position` fills its own fourth channel as a marker. **The two
    // headers must not read the same**, because a reader who took a padding channel for a marker
    // would drop every texel the pass did not draw — and for a `vec3<f32>` whose padding is low
    // that is every texel, which is a file of zeros made by a rule rather than by a render.
    let three = whiting_scene::declared_description("normal", "vec3<f32>", 1);
    let marker = DebugChannel::Position.description().to_owned();

    assert_ne!(
        three, marker,
        "a scene-declared `vec3<f32>` image and a built-in position image are different kinds of \
         image and their headers say so"
    );

    // The padding side: the fourth channel is named, and named as nothing.
    assert!(three.contains("padding"), "{three}");
    assert!(three.contains("carries no number"), "{three}");
    assert!(three.contains("marks nothing"), "{three}");
    assert!(three.contains("It is not a marker"), "{three}");
    // And it says what follows from that, because a header that only said "padding" and then
    // measured over a marker's pixels would be the contradiction this test exists to prevent.
    assert!(
        three.contains("every texel rather than over the ones a marker would have selected"),
        "{three}"
    );
    assert!(
        !three.contains("alpha is above one half"),
        "a scene-declared image has no marker, so a header about pixels above one half is a \
         claim about a convention it does not have: {three}"
    );

    // The marker side is unchanged: the built-in still says its alpha marks the pixels it drew
    // nothing on, which is the fact only compiler code can know.
    assert!(marker.contains("alpha is the marker"), "{marker}");
    assert!(marker.contains("0 where it drew none"), "{marker}");
    // And the two say opposite things about their fourth component, which is the whole of the
    // decision in one comparison.
    assert!(!marker.contains("padding"), "{marker}");

    println!("\n--- a `vec3<f32>` sidecar header ---\n{three}");
    println!("\n--- the `position` sidecar header ---\n{marker}");
}

#[test]
fn a_four_component_scene_declared_header_says_its_fourth_is_a_value_not_a_marker() {
    // The other half of the same decision: a scene's own `vec4<f32>` has four components the
    // shader wrote and no fifth to account for, and its fourth is one of them rather than a
    // convention borrowed from the built-ins.
    let four = whiting_scene::declared_description("lit", "vec4<f32>", 0);
    assert!(four.contains("marks nothing"), "{four}");
    assert!(
        !four.contains("padding"),
        "a `vec4<f32>` has no surplus: {four}"
    );
    assert_ne!(four, DebugChannel::Grade.description().to_owned());
}

#[test]
fn a_document_without_outputs_still_declares_nothing() {
    // The absent case is the convention, and it is unchanged: a document that omits the field
    // is answered by the colour target of the last pass that wrote one. Nothing here is
    // inferred to fill the gap, which is what a default would be.
    let graphics = definition::Value::structure([(
        graphics::PASSES_FIELD.to_owned(),
        Value::List(Vec::new()),
    )]);
    assert!(
        graphics::outputs(&graphics)
            .expect("the outputs read")
            .is_empty(),
        "an absent field is an empty list rather than an error, and means what every document \
         written before the field existed meant"
    );
}

#[test]
fn a_declaration_missing_a_field_is_refused_rather_than_defaulted() {
    // None of the three has a default, and this is the case that says so. A declaration with no
    // location would otherwise have one inferred, and inference is the one thing the shape
    // exists to stop: a location derived from a name's position in the map mis-binds the instant
    // a name is inserted, and the failure is a wrong picture rather than an error.
    for missing in [POINT_FIELD, LOCATION_FIELD, TYPE_FIELD] {
        let mut fields = vec![
            (POINT_FIELD.to_owned(), Value::String("target".to_owned())),
            (LOCATION_FIELD.to_owned(), Value::Int(0)),
            (TYPE_FIELD.to_owned(), Value::String("vec4<f32>".to_owned())),
        ];
        fields.retain(|(name, _)| name != missing);
        let document = Value::structure([(
            graphics::OUTPUTS_FIELD.to_owned(),
            Value::dictionary([("image".to_owned(), Value::structure(fields))]),
        )]);
        let error =
            graphics::outputs(&document).expect_err("a declaration missing a field is refused");
        assert!(
            error.to_string().contains(missing),
            "the error names the field that is absent, not the map: {error}"
        );
    }
}

#[test]
fn a_declaration_with_a_negative_location_is_refused() {
    // `@location(n)` is a non-negative number in WGSL. Wrapping a negative one would be the
    // difference between an error naming the field and a shader asked for an output at four
    // billion, so the reader refuses it rather than converting.
    let document = Value::structure([(
        graphics::OUTPUTS_FIELD.to_owned(),
        Value::dictionary([(
            "image".to_owned(),
            Value::structure([
                (POINT_FIELD.to_owned(), Value::String("target".to_owned())),
                (LOCATION_FIELD.to_owned(), Value::Int(-1)),
                (TYPE_FIELD.to_owned(), Value::String("vec4<f32>".to_owned())),
            ]),
        )]),
    )]);
    let error = graphics::outputs(&document).expect_err("a negative location is refused");
    assert!(
        error.to_string().contains("non-negative"),
        "the error says what the field has to be rather than only that it is wrong: {error}"
    );
}
