//! A small, valid pair of graphics and instance documents for readers and the doctest.
//!
//! The pair is written to exercise the whole binding path: a pass that declares
//! resources it does not carry, an instance that carries the payloads, and shaders
//! whose own interfaces say which of the two is right.

use definition::{Schema, SchemaKind, Value};

use crate::binders::binder_declaration_schema;

/// Builds a graphics document with one rasterize pass and one compute pass.
///
/// The document validates against [`graphics_schema`](crate::graphics_schema), and its
/// `instance_schema` slot is derived from the binders its passes declare, so the
/// example shows the derived form rather than the permissive one.
pub fn example_graphics() -> Value {
    let passes = example_passes();
    let declared = Value::structure([("passes".to_owned(), Value::List(passes.clone()))]);
    let instance_schema = crate::derived_instance_schema(&declared)
        .expect("the example declares no conflicting binder")
        .to_value();

    Value::structure([
        ("passes".to_owned(), Value::List(passes)),
        ("instance_schema".to_owned(), instance_schema),
    ])
}

/// Builds the pass list of the example, which both documents are assembled from.
fn example_passes() -> Vec<Value> {
    vec![
        Value::variant(
            "Rasterize",
            [
                ("label".to_owned(), Value::String("backdrop".to_owned())),
                (
                    "uniform".to_owned(),
                    Value::structure([(
                        "tint".to_owned(),
                        Value::List([0.9, 0.95, 1.0].into_iter().map(Value::Float).collect()),
                    )]),
                ),
                ("shader".to_owned(), Value::String("shape".to_owned())),
                (
                    "binders".to_owned(),
                    Value::dictionary([
                        (
                            "target".to_owned(),
                            texture_binding("backdrop_target", 512, 512, "rgba8unorm"),
                        ),
                        ("vertices".to_owned(), buffer_binding("shape_vertices", 36)),
                    ]),
                ),
                // The colour attachments, named as the binding points they read.
                (
                    "targets".to_owned(),
                    Value::List(vec![Value::String("target".to_owned())]),
                ),
                (
                    "clear_color".to_owned(),
                    Value::List(
                        [0.1, 0.1, 0.12, 1.0]
                            .into_iter()
                            .map(Value::Float)
                            .collect(),
                    ),
                ),
            ],
        ),
        Value::variant(
            "Compute",
            [
                ("label".to_owned(), Value::String("blur".to_owned())),
                (
                    "uniform".to_owned(),
                    Value::structure([("radius".to_owned(), Value::Float(1.0))]),
                ),
                ("shader".to_owned(), Value::String("blur".to_owned())),
                ("binders".to_owned(), Value::dictionary([])),
                (
                    "workgroup".to_owned(),
                    Value::List([8, 8, 1, 16, 16, 1].into_iter().map(Value::Int).collect()),
                ),
                ("entry_point".to_owned(), Value::String("main".to_owned())),
            ],
        ),
    ]
}

/// The schema the example vertex buffer was packed with.
///
/// A buffer says which schema shaped it, so a renderer derives the vertex layout from
/// that rather than the layout being written down a second time in the declaration.
pub fn vertex_schema() -> Value {
    Schema::new(SchemaKind::structure([(
        "position".to_owned(),
        crate::layout::components_schema(3),
    )]))
    .to_value()
}

/// Builds a buffer declaration for the example.
fn buffer_binding(source: &str, size_bytes: i64) -> Value {
    Value::variant(
        "Resource",
        [(
            "resource".to_owned(),
            Value::variant(
                "Buffer",
                [
                    ("source".to_owned(), Value::String(source.to_owned())),
                    ("size_bytes".to_owned(), Value::Int(size_bytes)),
                ],
            ),
        )],
    )
}

/// Builds a texture declaration for the example.
fn texture_binding(source: &str, width: i64, height: i64, format: &str) -> Value {
    Value::variant(
        "Resource",
        [(
            "resource".to_owned(),
            Value::variant(
                "Texture",
                [
                    ("source".to_owned(), Value::String(source.to_owned())),
                    ("width".to_owned(), Value::Int(width)),
                    ("height".to_owned(), Value::Int(height)),
                    ("format".to_owned(), Value::String(format.to_owned())),
                ],
            ),
        )],
    )
}

/// Builds the instance that supplies the payloads the example graphics declares.
///
/// The document satisfies the schema derived from those declarations, which is the
/// stricter of the two instance schemas on offer.
pub fn example_instance() -> Value {
    Value::dictionary([
        (
            "shape".to_owned(),
            Value::variant(
                "Shader",
                [
                    (
                        "path".to_owned(),
                        Value::String("shaders/shape.wgsl".to_owned()),
                    ),
                    ("schema".to_owned(), shape_interface()),
                ],
            ),
        ),
        (
            "blur".to_owned(),
            Value::variant(
                "Shader",
                [
                    (
                        "path".to_owned(),
                        Value::String("shaders/blur.wgsl".to_owned()),
                    ),
                    ("schema".to_owned(), blur_interface()),
                ],
            ),
        ),
        (
            "shape_vertices".to_owned(),
            Value::variant(
                "Buffer",
                [
                    ("size_bytes".to_owned(), Value::Int(36)),
                    (
                        "usage".to_owned(),
                        Value::List(vec![Value::String("vertex".to_owned())]),
                    ),
                    // Three points of three floats each.
                    ("data".to_owned(), Value::Bytes(vec![0u8; 36])),
                    ("topology".to_owned(), Value::variant("Triangles", [])),
                    ("schema".to_owned(), vertex_schema()),
                ],
            ),
        ),
        (
            "backdrop_target".to_owned(),
            Value::variant(
                "Texture",
                [
                    ("width".to_owned(), Value::Int(512)),
                    ("height".to_owned(), Value::Int(512)),
                    ("format".to_owned(), Value::String("rgba8unorm".to_owned())),
                ],
            ),
        ),
    ])
}

/// The interface the example shape shader declares.
///
/// A shader author who wants a binding point to be one specific kind of resource writes
/// that kind here rather than the general declaration schema.
fn shape_interface() -> Value {
    Schema::new(SchemaKind::structure([
        (
            "uniform".to_owned(),
            Schema::new(SchemaKind::structure([(
                "tint".to_owned(),
                Schema::new(SchemaKind::list(Schema::new(SchemaKind::Float))),
            )])),
        ),
        (
            "binders".to_owned(),
            Schema::new(SchemaKind::structure([
                ("target".to_owned(), binder_declaration_schema()),
                ("vertices".to_owned(), binder_declaration_schema()),
            ])),
        ),
    ]))
    .to_value()
}

/// The interface the example blur shader declares, which reads a uniform and no binders.
fn blur_interface() -> Value {
    Schema::new(SchemaKind::structure([
        (
            "uniform".to_owned(),
            Schema::new(SchemaKind::structure([(
                "radius".to_owned(),
                Schema::new(SchemaKind::Float),
            )])),
        ),
        ("binders".to_owned(), Schema::new(SchemaKind::structure([]))),
    ]))
    .to_value()
}
