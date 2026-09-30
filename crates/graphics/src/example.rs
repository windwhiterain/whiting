//! A small, valid pair of graphics and instance documents for readers and the doctest.

use definition::Value;

/// Builds a graphics document with one rasterize pass and one compute pass.
///
/// The document validates against [`graphics_schema`](crate::graphics_schema).
pub fn example_graphics() -> Value {
    Value::structure([
        (
            "passes".to_owned(),
            Value::List(vec![
                Value::variant(
                    "Rasterize",
                    [
                        ("label".to_owned(), Value::String("backdrop".to_owned())),
                        (
                            "vertex_shader".to_owned(),
                            Value::String("shape_vertex".to_owned()),
                        ),
                        (
                            "fragment_shader".to_owned(),
                            Value::String("shape_fragment".to_owned()),
                        ),
                        (
                            "targets".to_owned(),
                            Value::List(vec![Value::String("rgba8unorm".to_owned())]),
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
                            "shader".to_owned(),
                            Value::String("blur_compute".to_owned()),
                        ),
                        ("entry_point".to_owned(), Value::String("main".to_owned())),
                        (
                            "workgroup".to_owned(),
                            Value::List([8, 8, 1, 16, 16, 1].into_iter().map(Value::Int).collect()),
                        ),
                    ],
                ),
            ]),
        ),
        // Authored independently, which is why the schema leaves this slot open.
        (
            "instance_schema".to_owned(),
            Value::dictionary([("version".to_owned(), Value::Int(1))]),
        ),
    ])
}

/// Builds the instance that supplies the binders the example graphics refers to.
///
/// The document validates against [`instance_schema`](crate::instance_schema), which is
/// open, so every entry is legal.
pub fn example_instance() -> Value {
    Value::dictionary([
        (
            "shape_vertex".to_owned(),
            Value::variant(
                "Shader",
                [
                    (
                        "path".to_owned(),
                        Value::String("shaders/shape.wgsl".to_owned()),
                    ),
                    ("entry_point".to_owned(), Value::String("main".to_owned())),
                ],
            ),
        ),
        (
            "shape_fragment".to_owned(),
            Value::variant(
                "Shader",
                [
                    (
                        "path".to_owned(),
                        Value::String("shaders/shape.wgsl".to_owned()),
                    ),
                    (
                        "entry_point".to_owned(),
                        Value::String("fragment_main".to_owned()),
                    ),
                ],
            ),
        ),
        (
            "blur_compute".to_owned(),
            Value::variant(
                "Shader",
                [
                    (
                        "path".to_owned(),
                        Value::String("shaders/blur.wgsl".to_owned()),
                    ),
                    ("entry_point".to_owned(), Value::String("main".to_owned())),
                ],
            ),
        ),
        (
            "backdrop_target".to_owned(),
            Value::variant(
                "Resource",
                [(
                    "resource".to_owned(),
                    Value::variant(
                        "Texture",
                        [
                            ("width".to_owned(), Value::Int(512)),
                            ("height".to_owned(), Value::Int(512)),
                            ("format".to_owned(), Value::String("rgba8unorm".to_owned())),
                        ],
                    ),
                )],
            ),
        ),
    ])
}
