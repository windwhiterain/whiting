//! What the device-rasterize crate's subjects share: the helpers that more than one of them
//! uses.
//!
//! A helper that one subject uses stays inside that subject, private to it. A helper that two
//! or more use is the same helper, so it is written once here and the subjects that use it
//! name it from here. Where two subjects had a helper of the same name and the two were
//! different helpers, each keeps its own body and takes a name of its own where it lives.

use definition::{Schema, SchemaKind, Value};
use whiting_device_rasterize::WgpuRenderer;

/// The message a test says when the machine has no adapter to draw on.
pub const NO_ADAPTER: &str = "skipped: no gpu adapter is present";

/// The size of the frame. Large enough that the centre of the viewport is well inside one
/// texel of a one texel texture.
pub const FRAME: u32 = 64;

/// The bytes of a triangle of three points.
pub const TRIANGLE_BYTES: i64 = 3 * 3 * 4;

/// The bytes of one instance row of three points.
pub const INSTANCE_ROW_BYTES: i64 = 3 * 4;

/// Returns a renderer, or nothing when the machine has no adapter.
pub fn renderer() -> Option<WgpuRenderer> {
    let runtime = tokio::runtime::Runtime::new().expect("a runtime");
    runtime.block_on(async {
        if !WgpuRenderer::probe().await {
            return None;
        }
        WgpuRenderer::new().await.ok()
    })
}

/// Builds a buffer payload of three component points packed as floats.
pub fn packed_buffer(values: &[f32], field: &str, usage: &str) -> Value {
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
                Schema::new(SchemaKind::structure([(
                    field.to_owned(),
                    graphics::layout::components_schema(3),
                )]))
                .to_value(),
            ),
        ],
    )
}

/// Builds a vertex buffer of three points that cover the whole viewport.
///
/// The third point is at three in both axes, so the triangle a rasteriser clips out of it covers
/// every pixel of the target rather than half of it — which is what makes every texel of both
/// attachments the value the shader wrote.
pub fn triangle_vertices() -> Value {
    packed_buffer(
        &[-1.0, -1.0, 0.0, 3.0, -1.0, 0.0, -1.0, 3.0, 0.0],
        "position",
        "vertex",
    )
}

/// Builds the single instance row, which carries nothing but has to exist to be counted.
pub fn instance_row() -> Value {
    packed_buffer(&[0.0, 0.0, 0.0], "filler", "instance")
}

/// Builds a buffer declaration of the given size.
pub fn buffer_declaration(source: &str, size: i64) -> Value {
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
///
/// It is written through the shared helper rather than spelled out here, so that a document this
/// test builds and one the emitter builds are the same shape — the reader in `graphics` is the
/// only thing that has an opinion about it, and there is one.
pub fn declared(point: &str, location: u32, ty: &str) -> Value {
    Value::structure([
        ("point".to_owned(), Value::String(point.to_owned())),
        ("location".to_owned(), Value::Int(i64::from(location))),
        ("type".to_owned(), Value::String(ty.to_owned())),
    ])
}

/// Builds the payload of a target, which is written by the pass and carries no pixels.
pub fn target_payload(format: &str) -> Value {
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
pub fn texture_declaration(source: &str, format: &str) -> Value {
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
