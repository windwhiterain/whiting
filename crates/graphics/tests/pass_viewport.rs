//! A rasterize pass may name the region of its target it draws into.
//!
//! # What these prove
//!
//! A target is often a container for several separate drawings — an atlas of shadow pages,
//! where each page is the whole render target of the pass that fills it — and a pass that
//! cannot say which part of it it owns can only be run by erasing the parts already there.
//! The field is therefore read as one rectangle that a renderer honours twice, and what
//! these tests hold still is the reading of it:
//!
//! - a pass that names no region names the whole target, so every document written before
//!   the field existed keeps the render it always had;
//! - a region is four texel counts read in the order a viewport is set in, because the two
//!   ways to read it wrongly are swapping origin with extent and swapping x with y, and both
//!   come from an order the document never has to state;
//! - a rectangle with no area is refused rather than read as "draw nothing", because a pass
//!   that covers no texel is a miscounted tile, and the target keeps whatever an earlier pass
//!   left in it as the answer;
//! - the region reaches no further than the rectangle: it changes no blend, no depth state
//!   and no sample count.

use definition::{Schema, SchemaKind, Value};
use whiting_graphics::binders::{FORMAT, HEIGHT, RESOURCE, SOURCE, WIDTH};
use whiting_graphics::pass::{CLEAR_COLOR, LABEL, SHADER, TARGETS, UNIFORM, VIEWPORT};
use whiting_graphics::{
    BINDERS, Blend, GraphicsError, INSTANCE_SCHEMA_FIELD, PASSES_FIELD, PipelineState, Viewport,
    graphics_schema, passes,
};

#[test]
fn a_pass_that_names_no_region_names_the_whole_target_by_not_naming_one() {
    let state = read(&[]).expect("the pass reads");
    assert_eq!(
        state.viewport, None,
        "a document that says nothing about a region draws all of its target"
    );
    assert_eq!(
        state.blend,
        Blend::Replace,
        "and nothing else about the pass changes: {state:?}"
    );
}

#[test]
fn a_region_is_four_texel_counts_in_the_order_a_viewport_is_set_in() {
    let declared = [7_i64, 3, 128, 64];
    let state = read(&[("viewport", ints(&declared))]).expect("the pass reads");
    assert_eq!(
        state.viewport,
        Some(Viewport {
            x: 7,
            y: 3,
            width: 128,
            height: 64,
        }),
        "origin x, origin y, width, height — {state:?}"
    );
    // The other half of the same sentence. A region read with origin and extent the wrong way
    // round lands somewhere else in the atlas and says nothing about it, so the corner it
    // reaches is what pins the four numbers down.
    let region = state.viewport.expect("a named region");
    assert_eq!(
        (region.x + region.width, region.y + region.height),
        (135, 67)
    );
}

#[test]
fn a_region_with_no_area_is_refused() {
    for declared in [[0_i64, 0, 0, 128], [0, 0, 128, 0]] {
        let error = read(&[("viewport", ints(&declared))])
            .expect_err("a rectangle covering nothing is a miscount, not a draw");
        assert!(
            names_viewport(&error),
            "the refusal names the field it is about: {error}"
        );
    }
}

#[test]
fn a_region_that_is_not_four_whole_offsets_is_refused() {
    // Three numbers, five, a negative origin, a fraction, text where a list should be, and a
    // list that is the wrong shape for a texel count.
    let cases = [
        ints(&[0, 0, 128]),
        ints(&[0, 0, 128, 128, 0]),
        ints(&[-1, 0, 128, 128]),
        Value::List(vec![
            Value::Int(0),
            Value::Float(0.5),
            Value::Int(128),
            Value::Int(128),
        ]),
        Value::String("0 0 128 128".to_owned()),
    ];
    for declared in cases {
        let error = read(&[("viewport", declared)]).expect_err("none of these is a rectangle");
        assert!(
            names_viewport(&error),
            "the refusal names the field it is about: {error}"
        );
    }
}

#[test]
fn the_schema_accepts_a_region_and_the_absence_of_one() {
    let document = Value::structure([
        (
            PASSES_FIELD.to_owned(),
            Value::List(vec![
                rasterize(&[]),
                rasterize(&[(VIEWPORT, ints(&[0, 0, 64, 64]))]),
            ]),
        ),
        (
            INSTANCE_SCHEMA_FIELD.to_owned(),
            Schema::new(SchemaKind::Any).to_value(),
        ),
    ]);
    graphics_schema()
        .validate(&document)
        .expect("both spellings are legal graphics documents");
}

/// Reads the pipeline state of a one pass document carrying the given extra fields.
fn read(extra: &[(&'static str, Value)]) -> Result<PipelineState, GraphicsError> {
    let document =
        Value::structure([(PASSES_FIELD.to_owned(), Value::List(vec![rasterize(extra)]))]);
    let found = passes(&document)?;
    PipelineState::read(&found[0])
}

/// Reports whether an error is about the `viewport` field rather than something else.
fn names_viewport(error: &GraphicsError) -> bool {
    matches!(error, GraphicsError::Malformed { field, .. } if field == VIEWPORT)
}

/// Builds a minimal rasterize pass drawing into one texture target.
fn rasterize(extra: &[(&'static str, Value)]) -> Value {
    let mut fields = vec![
        (LABEL.to_owned(), Value::String("fill a page".to_owned())),
        (UNIFORM.to_owned(), Value::Unit),
        (SHADER.to_owned(), Value::String("caster".to_owned())),
        (
            BINDERS.to_owned(),
            Value::dictionary([("atlas".to_owned(), texture_declaration())]),
        ),
        (
            TARGETS.to_owned(),
            Value::List(vec![Value::String("atlas".to_owned())]),
        ),
        (
            CLEAR_COLOR.to_owned(),
            Value::List(vec![
                Value::Float(1.0),
                Value::Float(1.0),
                Value::Float(1.0),
                Value::Float(1.0),
            ]),
        ),
    ];
    fields.extend(
        extra
            .iter()
            .map(|(name, value)| ((*name).to_owned(), value.clone())),
    );
    Value::variant("Rasterize", fields)
}

/// Builds the binder declaration of a pass targeting one depth texture.
fn texture_declaration() -> Value {
    Value::variant(
        "Resource",
        [(
            RESOURCE.to_owned(),
            Value::variant(
                "Texture",
                [
                    (SOURCE.to_owned(), Value::String("atlas".to_owned())),
                    (WIDTH.to_owned(), Value::Int(512)),
                    (HEIGHT.to_owned(), Value::Int(512)),
                    (FORMAT.to_owned(), Value::String("depth32float".to_owned())),
                ],
            ),
        )],
    )
}

/// Builds a list of integers, which is how a document writes a rectangle.
fn ints(values: &[i64]) -> Value {
    Value::List(values.iter().copied().map(Value::Int).collect())
}
