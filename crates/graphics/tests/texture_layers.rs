//! A texture can say it has layers, and a pass can say which one it draws into.
//!
//! # What these prove
//!
//! Both fields are optional and both are additive, so what a document means today must not
//! change: a payload with no `layers` and a declaration with no `layer` have to validate
//! against the same schemas as before, because every document in the repository omits them.
//!
//! The two halves are also kept apart on purpose. `layers` is a payload field, because how
//! many layers a texture has is a fact about the image, and `layer` is a declaration field,
//! because which layer a pass draws into is a fact about that pass. A test that put both on
//! one side would pass with either placement, so these read the two where they are.

use definition::Value;
use whiting_graphics::binders::{
    FORMAT, HEIGHT, LAYER, LAYERS, RESOURCE, SOURCE, WIDTH, binder_declaration_schema,
    binder_payload_schema,
};
use whiting_graphics::{BINDERS, LABEL, PASSES_FIELD, passes};

#[test]
fn a_payload_that_says_nothing_about_layers_is_the_flat_texture_it_was() {
    let schema = binder_payload_schema();
    schema
        .validate(&payload(None))
        .expect("a payload that names no layers is the texture every document means");
    schema
        .validate(&payload(Some(Value::Int(64))))
        .expect("a payload may name how many layers it has");
}

#[test]
fn a_payload_may_not_name_a_layer_count_of_another_kind() {
    let error = binder_payload_schema()
        .validate(&payload(Some(Value::String("sixty four".to_owned()))))
        .expect_err("a struct is closed and a field is read as the kind it is written as");
    assert!(
        error.to_string().contains(LAYERS),
        "the refusal names the field that is wrong: {error}"
    );
}

#[test]
fn a_pass_names_the_layer_it_draws_into_on_its_declaration() {
    let schema = binder_declaration_schema();
    for layer in [None, Some(Value::Int(3))] {
        let declaration = declaration(layer.clone());
        schema
            .validate(&declaration)
            .unwrap_or_else(|error| panic!("a declaration naming {layer:?} is legal: {error}"));
        let read = passes(&graphics_document(declaration))
            .expect("the pass reads")[0]
                .binders()
                .expect("the binders read")[0]
                .layer();
        assert_eq!(
            read,
            layer.as_ref().and_then(Value::as_int),
            "a declaration reads back the layer it wrote, and names none when it wrote none"
        );
    }
}

/// Builds a texture payload, with a layer count or without one.
fn payload(layers: Option<Value>) -> Value {
    let mut fields = vec![
        (WIDTH.to_owned(), Value::Int(1024)),
        (HEIGHT.to_owned(), Value::Int(1024)),
        (FORMAT.to_owned(), Value::String("depth32float".to_owned())),
    ];
    if let Some(layers) = layers {
        fields.push((LAYERS.to_owned(), layers));
    }
    Value::variant("Texture", fields)
}

/// Builds a texture declaration naming a layer, or naming none.
fn declaration(layer: Option<Value>) -> Value {
    let mut fields = vec![
        (SOURCE.to_owned(), Value::String("shadow_pool".to_owned())),
        (WIDTH.to_owned(), Value::Int(1024)),
        (HEIGHT.to_owned(), Value::Int(1024)),
        (FORMAT.to_owned(), Value::String("depth32float".to_owned())),
    ];
    if let Some(layer) = layer {
        fields.push((LAYER.to_owned(), layer));
    }
    Value::variant("Resource", [(RESOURCE.to_owned(), Value::variant("Texture", fields))])
}

/// Builds a one pass graphics document around one binder declaration.
fn graphics_document(declaration: Value) -> Value {
    Value::structure([(
        PASSES_FIELD.to_owned(),
        Value::List(vec![Value::variant("Rasterize", [
            (LABEL.to_owned(), Value::String("write the page".to_owned())),
            (
                BINDERS.to_owned(),
                Value::dictionary([("pool".to_owned(), declaration)]),
            ),
        ])]),
    )])
}
