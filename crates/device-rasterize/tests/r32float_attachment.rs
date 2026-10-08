//! A measurement: can an `r32float` colour attachment be drawn, and does it read back exactly?
//!
//! # Why this file exists
//!
//! The content-identity design puts an integer id in a rendered image and resolves it to an
//! authored path. An integer id in a float target is only usable if the target is `r32float`:
//! `TexelType::format` sends `vec3<f32>` and `vec4<f32>` to `rgba16float`, whose 11 bit mantissa
//! represents integers exactly only up to 2048, and a landscape of any size has more belts than
//! that. So the whole design rests on two facts that were asserted rather than measured:
//!
//! 1. **`r32float` is a renderable colour attachment** on this adapter. `rgba16float` was measured
//!    when it landed ([agent-debug-scenes.md]); `r32float` was never rendered into.
//! 2. **A fragment output of `f32` is accepted and read back without loss**, so an id above 2048
//!    comes back as the same integer rather than as a neighbouring float.
//!
//! **Whether the current renderer can do this at all is the open part.** `raster.rs` builds every
//! attachment of a pass with the same `blend_of(state.blend)`, and `r32float` is not blendable
//! without a feature the device does not request. A pipeline that failed here would not be a
//! defect in this file; it would be the measurement saying the blend has to become per-attachment.


use definition::Value;
use device::Renderer;
use whiting_device_rasterize::WgpuRenderer;

/// The size of the frame.
const FRAME: u32 = 16;

/// The id written into the second attachment.
///
/// Chosen rather than written down: 4093 is **not** exactly representable in `rgba16float` — its
/// 11 bit mantissa steps by four past 2048, so the nearest half float is 4092 — while `r32float`
/// holds every integer below 2^24 exactly. A readback that returned 4092 would say the target is
/// the wrong format, and a readback that returned 4093 would say it is the right one. The number
/// is the assertion.
const ID: f32 = 4093.0;

/// A note printed when there is nothing to test on.
const NO_ADAPTER: &str = "skipped: no gpu adapter is present";

#[test]
fn an_r32float_attachment_is_drawn_and_reads_an_integer_back_exactly() {
    let Some(renderer) = renderer() else {
        println!("{NO_ADAPTER}");
        return;
    };

    let images = match renderer.render_outputs(&graphics(), &instance()) {
        Ok(images) => images,
        Err(error) => {
            // A pipeline that would not build is a result, not a skip: it is the reason the
            // renderer has to change, and the backend's own words are the evidence.
            panic!("a pass with an r32float colour attachment does not render: {error}");
        }
    };

    let image = images
        .iter()
        .find(|image| image.name == "identity")
        .expect("the r32float attachment is returned under the name it was declared as");
    assert_eq!(
        image.format, "r32float",
        "the attachment is read in the format it was drawn in"
    );

    let mut checked = 0usize;
    for y in 0..FRAME {
        for x in 0..FRAME {
            let Some(texel) = image.texel(x, y) else {
                continue;
            };
            assert_eq!(
                texel[0], ID,
                "({x}, {y}) holds {}, and the shader wrote {ID}. `r32float` has 24 bits of mantissa, \
                 so an integer this size survives the round trip exactly; a value that came back \
                 nearby instead is a target that quantised it",
                texel[0]
            );
            checked += 1;
        }
    }
    assert!(
        checked == (FRAME * FRAME) as usize,
        "the triangle covers the whole target, so all {FRAME}² texels hold the value and only \
         {checked} do — the attachment is not being read where it was drawn"
    );
}

/// Returns a renderer, or nothing when the machine has no adapter.
fn renderer() -> Option<WgpuRenderer> {
    let runtime = tokio::runtime::Runtime::new().expect("a runtime");
    runtime.block_on(async {
        if !WgpuRenderer::probe().await {
            return None;
        }
        WgpuRenderer::new().await.ok()
    })
}

/// Builds the graphics document: one rasterize pass writing two colour attachments.
///
/// The first is `rgba8unorm` and the second is `r32float`, so the second is the case in question
/// and the first is the shape every document in the workspace already has. They are declared in
/// name order, so the second target is the second attachment and `@location(1)` is the one it was
/// built as.
fn graphics() -> Value {
    let pass = Value::variant(
        "Rasterize",
        [
            ("label".to_owned(), Value::String("identity".to_owned())),
            (
                "uniform".to_owned(),
                Value::structure([("time".to_owned(), Value::Float(0.0))]),
            ),
            ("shader".to_owned(), Value::String("shader".to_owned())),
            (
                "binders".to_owned(),
                Value::dictionary(vec![
                    (
                        "frame".to_owned(),
                        texture_declaration("frame", "rgba8unorm"),
                    ),
                    (
                        "identity".to_owned(),
                        texture_declaration("identity", "r32float"),
                    ),
                    (
                        "row".to_owned(),
                        buffer_declaration("row", INSTANCE_ROW_BYTES),
                    ),
                    (
                        "triangle".to_owned(),
                        buffer_declaration("triangle", TRIANGLE_BYTES),
                    ),
                ]),
            ),
            (
                "targets".to_owned(),
                Value::List(vec![
                    Value::String("frame".to_owned()),
                    Value::String("identity".to_owned()),
                ]),
            ),
            (
                "clear_color".to_owned(),
                Value::List([0.0, 0.0, 0.0, 1.0].into_iter().map(Value::Float).collect()),
            ),
        ],
    );
    Value::structure([
        ("passes".to_owned(), Value::List(vec![pass])),
        (
            graphics::OUTPUTS_FIELD.to_owned(),
            Value::dictionary(vec![
                (
                    graphics::PRESENT_OUTPUT.to_owned(),
                    declared("frame", 0, "vec4<f32>"),
                ),
                ("identity".to_owned(), declared("identity", 1, "f32")),
            ]),
        ),
        (
            "instance_schema".to_owned(),
            graphics::example_graphics()
                .as_field_map()
                .and_then(|fields| fields.get("instance_schema"))
                .cloned()
                .expect("the example carries an instance schema"),
        ),
    ])
}

/// Builds the instance the pass draws from.
fn instance() -> Value {
    Value::structure([
        (
            "referencing_graphics".to_owned(),
            Value::String("test.graphics".to_owned()),
        ),
        (
            "entries".to_owned(),
            Value::dictionary(vec![
                (
                    "row".to_owned(),
                    packed_buffer(&[0.0, 0.0, 0.0], "filler", "instance"),
                ),
                ("frame".to_owned(), target_payload("rgba8unorm")),
                ("identity".to_owned(), target_payload("r32float")),
                ("shader".to_owned(), shader()),
                (
                    "triangle".to_owned(),
                    packed_buffer(
                        &[-1.0, -1.0, 0.0, 3.0, -1.0, 0.0, -1.0, 3.0, 0.0],
                        "position",
                        "vertex",
                    ),
                ),
            ]),
        ),
    ])
}

/// The bytes three two dimensional points of three floats each occupy.
const TRIANGLE_BYTES: i64 = 3 * 3 * 4;

/// The bytes one instance row of three floats occupies.
const INSTANCE_ROW_BYTES: i64 = 3 * 4;

/// The shader the pass is drawn with.
///
/// The identity is written as a **bare `f32`**, not a `vec4` of a normalised integer: a single
/// channel is a single channel, and this is the spelling `TexelType::Float` resolves to
/// `r32float`.
fn shader() -> Value {
    Value::variant(
        "Shader",
        [
            (
                "source".to_owned(),
                Value::String(
                    r#"
struct Uniforms {
    time: f32,
};

@group(0) @binding(0) var<uniform> uniforms: Uniforms;

@vertex
fn vertex_main(@location(0) position: vec3<f32>) -> @builtin(position) vec4<f32> {
    return vec4<f32>(position, 1.0);
}

struct Written {
    @location(0) frame: vec4<f32>,
    @location(1) identity: f32,
};

@fragment
fn fragment_main() -> Written {
    var out: Written;
    out.frame = vec4<f32>(0.25, 0.5, 0.75, 1.0);
    out.identity = 4093.0;
    return out;
}
"#
                    .to_owned(),
                ),
            ),
            (
                "schema".to_owned(),
                definition::Schema::new(definition::SchemaKind::structure([
                    (
                        "uniform".to_owned(),
                        definition::Schema::new(definition::SchemaKind::structure([(
                            "time".to_owned(),
                            definition::Schema::new(definition::SchemaKind::Float),
                        )])),
                    ),
                    (
                        "binders".to_owned(),
                        definition::Schema::new(definition::SchemaKind::structure(vec![
                            ("frame".to_owned(), graphics::texture_binding_schema()),
                            ("identity".to_owned(), graphics::texture_binding_schema()),
                            ("row".to_owned(), graphics::buffer_binding_schema()),
                            ("triangle".to_owned(), graphics::buffer_binding_schema()),
                        ])),
                    ),
                ]))
                .to_value(),
            ),
        ],
    )
}

/// Builds a buffer payload of three component points packed as floats.
fn packed_buffer(values: &[f32], field: &str, usage: &str) -> Value {
    let bytes: Vec<u8> = values
        .iter()
        .flat_map(|value| value.to_le_bytes())
        .collect();
    Value::variant(
        "Buffer",
        [
            ("size_bytes".to_owned(), Value::Int(bytes.len() as i64)),
            (
                "usage".to_owned(),
                Value::List(vec![Value::String(usage.to_owned())]),
            ),
            ("data".to_owned(), Value::Bytes(bytes)),
            ("topology".to_owned(), Value::variant("Triangles", [])),
            (
                "schema".to_owned(),
                definition::Schema::new(definition::SchemaKind::structure([(
                    field.to_owned(),
                    graphics::layout::components_schema(3),
                )]))
                .to_value(),
            ),
        ],
    )
}

/// Builds the payload of a target, which is written by the pass and carries no pixels.
fn target_payload(format: &str) -> Value {
    Value::variant(
        "Texture",
        [
            ("width".to_owned(), Value::Int(FRAME as i64)),
            ("height".to_owned(), Value::Int(FRAME as i64)),
            ("format".to_owned(), Value::String(format.to_owned())),
        ],
    )
}

/// Builds a texture declaration naming an entry of the instance.
fn texture_declaration(source: &str, format: &str) -> Value {
    Value::variant(
        "Resource",
        [(
            "resource".to_owned(),
            Value::variant(
                "Texture",
                [
                    ("source".to_owned(), Value::String(source.to_owned())),
                    ("width".to_owned(), Value::Int(FRAME as i64)),
                    ("height".to_owned(), Value::Int(FRAME as i64)),
                    ("format".to_owned(), Value::String(format.to_owned())),
                ],
            ),
        )],
    )
}

/// Builds a buffer declaration of the given size.
fn buffer_declaration(source: &str, size: i64) -> Value {
    Value::variant(
        "Resource",
        [(
            "resource".to_owned(),
            Value::variant(
                "Buffer",
                [
                    ("source".to_owned(), Value::String(source.to_owned())),
                    ("size_bytes".to_owned(), Value::Int(size)),
                ],
            ),
        )],
    )
}

/// Builds one `outputs` entry: the point, the location and the type of a texel.
fn declared(point: &str, location: u32, ty: &str) -> Value {
    Value::structure([
        ("point".to_owned(), Value::String(point.to_owned())),
        ("location".to_owned(), Value::Int(i64::from(location))),
        ("type".to_owned(), Value::String(ty.to_owned())),
    ])
}
