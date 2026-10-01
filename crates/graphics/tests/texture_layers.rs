//! A document says which shape of texture it holds, and a pass says which layer it writes.
//!
//! # What these prove
//!
//! A flat texture and a texture array are two kinds rather than one kind with a count in
//! it, so the schema has to keep them apart and the readers have to keep them apart. A
//! single variant with an optional count would validate every document here and mean
//! something a shader cannot name, and a reader of the document could not tell which it
//! held.
//!
//! - a `Texture` payload and declaration are byte for byte what they were, so no existing
//!   document changes meaning;
//! - a `TextureArray` payload must say how many layers it has, and a `Texture` payload has
//!   no such field to fall back on;
//! - `layers_of` answers a count only for an array, so a caller cannot mistake a flat
//!   texture for an array of one;
//! - a declaration reads back the layer it wrote, and a flat texture declaration has no
//!   layer field at all.

use definition::Value;
use whiting_graphics::binders::{
    FORMAT, HEIGHT, LAYER, LAYERS, RESOURCE, SOURCE, WIDTH, binder_declaration_schema,
    binder_payload_schema, layers_of,
};
use whiting_graphics::{BINDERS, LABEL, PASSES_FIELD, passes};

#[test]
fn a_flat_texture_is_exactly_the_document_it_was() {
    let payload = texture_payload("Texture", None);
    binder_payload_schema()
        .validate(&payload)
        .expect("a flat texture payload is unchanged");
    assert_eq!(
        layers_of(&payload),
        None,
        "a flat texture has no count to read, and reading one as a single layer is exactly \
         the inference the two kinds exist to remove"
    );

    let declaration = texture_declaration("Texture", None, None);
    binder_declaration_schema()
        .validate(&declaration)
        .expect("a flat texture declaration is unchanged");
    assert_eq!(
        layer_of(&declaration),
        None,
        "a flat texture has no layer to choose between, so it declares no layer"
    );
}

#[test]
fn a_texture_array_says_how_deep_it_is_and_which_layer_a_pass_writes() {
    let payload = texture_payload("TextureArray", Some(64));
    binder_payload_schema()
        .validate(&payload)
        .expect("an array payload that says how many layers it has is legal");
    assert_eq!(
        layers_of(&payload),
        Some(64),
        "the count is read from the variant that declares it"
    );

    let declaration = texture_declaration("TextureArray", Some(64), Some(3));
    binder_declaration_schema()
        .validate(&declaration)
        .expect("an array declaration that names a layer is legal");
    assert_eq!(
        layer_of(&declaration),
        Some(3),
        "a pass reads back the layer it said it draws into"
    );
    assert_eq!(
        layer_of(&texture_declaration("TextureArray", Some(64), None)),
        None,
        "a pass that names no layer draws into the first, and says nothing"
    );
}

#[test]
fn an_array_may_not_omit_its_count_and_a_flat_texture_may_not_carry_one() {
    let missing = texture_payload("TextureArray", None);
    let error = binder_payload_schema()
        .validate(&missing)
        .expect_err("an array that does not say how deep it is cannot be built");
    assert!(
        error.to_string().contains(LAYERS),
        "the refusal names the field that is missing: {error}"
    );

    let error = binder_payload_schema()
        .validate(&texture_payload("Texture", Some(64)))
        .expect_err("a flat texture has no count field, so a document cannot mean an array by it");
    assert!(
        error.to_string().contains(LAYERS),
        "the refusal names the field that is not allowed here: {error}"
    );
}

/// Builds a texture payload of the given shape, with a layer count or without one.
fn texture_payload(tag: &str, layers: Option<i64>) -> Value {
    let mut fields = vec![
        (WIDTH.to_owned(), Value::Int(1024)),
        (HEIGHT.to_owned(), Value::Int(1024)),
    ];
    if let Some(layers) = layers {
        fields.push((LAYERS.to_owned(), Value::Int(layers)));
    }
    fields.push((FORMAT.to_owned(), Value::String("depth32float".to_owned())));
    Value::variant(tag, fields)
}

/// Builds a texture declaration of the given shape, with a count and a layer or neither.
fn texture_declaration(tag: &str, layers: Option<i64>, layer: Option<i64>) -> Value {
    let mut fields = vec![
        (SOURCE.to_owned(), Value::String("shadow_pool".to_owned())),
        (WIDTH.to_owned(), Value::Int(1024)),
        (HEIGHT.to_owned(), Value::Int(1024)),
    ];
    if let Some(layers) = layers {
        fields.push((LAYERS.to_owned(), Value::Int(layers)));
    }
    if let Some(layer) = layer {
        fields.push((LAYER.to_owned(), Value::Int(layer)));
    }
    fields.push((FORMAT.to_owned(), Value::String("depth32float".to_owned())));
    Value::variant(
        "Resource",
        [(RESOURCE.to_owned(), Value::variant(tag, fields))],
    )
}

/// Reads the layer a declaration names, by building the one pass document that carries it.
fn layer_of(declaration: &Value) -> Option<i64> {
    passes(&graphics_document(declaration.clone())).expect("the pass reads")[0]
        .binders()
        .expect("the binders read")[0]
        .layer()
}

/// Builds a one pass graphics document around one binder declaration.
fn graphics_document(declaration: Value) -> Value {
    Value::structure([(
        PASSES_FIELD.to_owned(),
        Value::List(vec![Value::variant(
            "Rasterize",
            [
                (LABEL.to_owned(), Value::String("write the page".to_owned())),
                (
                    BINDERS.to_owned(),
                    Value::dictionary([("pool".to_owned(), declaration)]),
                ),
            ],
        )]),
    )])
}
