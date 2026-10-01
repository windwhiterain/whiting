//! The sampled texture channel, on a real gpu.
//!
//! # What these prove
//!
//! The binding model can carry a sampled image, and the rule that numbers its sampler is the
//! one the document states rather than one inferred from a format. Four things are checked,
//! and each is something the version this replaces got wrong:
//!
//! - a texture the payload calls `sampled` reaches the fragment stage through a sampler at
//!   the binding after its own, and the pixels that arrive are the ones that were uploaded;
//! - a texture that says nothing about sampling still renders, read by `textureLoad`, which
//!   is the shape every existing scene's post stage already has;
//! - an uploaded texture carries the level chain its size implies, which the pixels at level
//!   zero cannot show;
//! - a shader that names the wrong number is reported as a disagreement about a binding,
//!   naming the pass and the shader's own variable, rather than as a backend error about a
//!   view dimension.
//!
//! Every test needs an adapter, and one that is missing is a **skip with a printed note**. A
//! silent pass would make this file worthless on a machine with no gpu, which is where it is
//! most likely to be run.

#![cfg(feature = "wgpu")]

use definition::{Schema, SchemaKind, Value};
use graphics::binders::SAMPLED;
use whiting_device::{Renderer, WgpuRenderer};

/// The colour one texel of the sampled texture carries.
///
/// Deliberately neither white nor a channel at its limit: a value that survives being read
/// back through the wrong format or the wrong bytes would pass by accident.
const TEXEL: [u8; 4] = [0x40, 0x80, 0xC0, 0xFF];

/// The size of the frame. Large enough that the centre of the viewport is well inside one
/// texel of a one texel texture.
const FRAME: u32 = 64;

/// The binding the sampler takes when the payload declares `sampled`.
///
/// The pass declares five binders, and which of them reach the shader is decided by their
/// usage rather than by their names: `frame` is the pass's target, `row` and `triangle`
/// carry an instance or vertex usage so they become vertex buffers, and `padding` and
/// `sampled_image` are the two bound resources, in name order. The image is therefore at two
/// and its sampler at three.
///
/// Worth a comment rather than being worked out at the call site, because it is the whole
/// point of the numbering: a buffer's usage decides whether it reaches the shader at all, so
/// a shader's number depends on something other than the declaration list.
const SAMPLER_BINDING: u32 = 3;

/// The bytes three two dimensional points of three floats each occupy.
const TRIANGLE_BYTES: i64 = 3 * 3 * 4;

/// The bytes one instance row of three floats occupies.
const INSTANCE_ROW_BYTES: i64 = 3 * 4;

/// The bytes the bound buffer carries, which the shader does not read.
const PADDING_BYTES: i64 = 16;

/// A note printed when there is nothing to test on.
const NO_ADAPTER: &str = "skipped: no gpu adapter is present";

#[test]
fn a_sampled_texture_reaches_the_fragment_stage() {
    let Some(renderer) = renderer() else {
        println!("{NO_ADAPTER}");
        return;
    };
    let stage = Stage::sampling(SAMPLER_BINDING);
    let frame = renderer
        .render(
            &graphics(1, 1),
            &instance(texture_payload(1, 1, &TEXEL), true, &source(&stage)),
        )
        .expect("a sampled texture renders");
    assert_eq!((frame.width, frame.height), (FRAME, FRAME));
    // The whole frame is the texel: a one texel texture has one value to give whatever
    // coordinate it is read at, and whichever way it is filtered.
    let expected: Vec<u8> = TEXEL
        .iter()
        .copied()
        .cycle()
        .take(frame.rgba.len())
        .collect();
    assert_eq!(frame.rgba, expected);
}

#[test]
fn a_texture_that_is_not_sampled_still_renders() {
    let Some(renderer) = renderer() else {
        println!("{NO_ADAPTER}");
        return;
    };
    // The shader reads the image with `textureLoad`, which needs no sampler, and the payload
    // declares none. This is the numbering every existing scene already relies on.
    let stage = Stage::loading();
    let frame = renderer
        .render(
            &graphics(1, 1),
            &instance(texture_payload(1, 1, &TEXEL), false, &source(&stage)),
        )
        .expect("a read texture renders without a sampler");
    assert_eq!(frame.rgba[0..4], TEXEL, "the first pixel is the texel");
}

#[test]
fn an_uploaded_texture_carries_the_levels_a_box_filter_derives() {
    let Some(renderer) = renderer() else {
        println!("{NO_ADAPTER}");
        return;
    };
    // A checkerboard of black and white texels, one texel per pixel. The level a box filter
    // derives from it is the average of black and white, which is a value the base level never
    // holds: it is either black or white and never half. Reading it back is therefore what
    // shows a chain was built rather than declared.
    let checkerboard: Vec<u8> = (0..(FRAME * FRAME) as usize)
        .flat_map(|index| {
            let (x, y) = (index % FRAME as usize, index / FRAME as usize);
            if (x + y) % 2 == 0 {
                [0x00, 0x00, 0x00, 0xFF]
            } else {
                [0xFF, 0xFF, 0xFF, 0xFF]
            }
        })
        .collect();
    // The frame is 64 texels a side, so a fully averaged level is the sixth one down.
    let frame = renderer
        .render(
            &graphics(FRAME as i64, FRAME as i64),
            &instance(
                texture_payload(FRAME as i64, FRAME as i64, &checkerboard),
                true,
                &source(&Stage::a_known_average(SAMPLER_BINDING, 6)),
            ),
        )
        .expect("an uploaded texture renders");
    assert_eq!(
        frame.rgba[0],
        0,
        "the base level holds a texel and not an average: {:?}",
        &frame.rgba[0..4]
    );
    assert_eq!(
        frame.rgba[2],
        128,
        "the sixth level is the average of black and white: {:?}",
        &frame.rgba[0..4]
    );
}

#[test]
fn an_srgb_texture_is_averaged_in_linear_light() {
    let Some(renderer) = renderer() else {
        println!("{NO_ADAPTER}");
        return;
    };
    // Four texels whose two candidate averages are far apart: averaging the stored values
    // gives 112, and decoding them first and encoding the result gives 97. A checkerboard of
    // black and white cannot tell the two apart, because a black and a white texel average to
    // 0.5 either way — which is why this pattern is four values rather than two.
    //
    // The averaged level is read with `textureLoad` rather than sampled, because a sample of
    // an sRGB texture converts the stored value to linear light and both candidates then
    // decode to a similar value.
    let pixels: Vec<u8> = [0x00u8, 0x40, 0x80, 0xFF]
        .into_iter()
        .flat_map(|value| [value, value, value, 0xFF])
        .collect();
    let frame = renderer
        .render(
            &graphics(2, 2),
            &instance(
                texture_payload_in(2, 2, "rgba8unorm-srgb", &pixels),
                true,
                &source(&Stage::a_known_average(SAMPLER_BINDING, 1)),
            ),
        )
        .expect("an sRGB texture renders");
    assert_eq!(
        frame.rgba[2],
        81,
        "an sRGB format averages in linear light: {:?}",
        &frame.rgba[0..4]
    );
}

#[test]
fn a_linear_texture_is_averaged_as_stored() {
    let Some(renderer) = renderer() else {
        println!("{NO_ADAPTER}");
        return;
    };
    // The same four texels in a format that stores linear values, so the difference between
    // this answer and the one above is the filter's doing and not the format's.
    let pixels: Vec<u8> = [0x00u8, 0x40, 0x80, 0xFF]
        .into_iter()
        .flat_map(|value| [value, value, value, 0xFF])
        .collect();
    let frame = renderer
        .render(
            &graphics(2, 2),
            &instance(
                texture_payload(2, 2, &pixels),
                true,
                &source(&Stage::a_known_average(SAMPLER_BINDING, 1)),
            ),
        )
        .expect("a linear texture renders");
    assert_eq!(
        frame.rgba[2],
        112,
        "a linear format averages the stored values: {:?}",
        &frame.rgba[0..4]
    );
}

#[test]
fn a_shader_that_names_the_wrong_binding_is_reported_as_that() {
    let Some(renderer) = renderer() else {
        println!("{NO_ADAPTER}");
        return;
    };
    // The sampler is declared at binding one, where the pass bound the buffer the shader does
    // not read. A wrong numbering rule is meant to arrive here, as a message naming the pass
    // and the shader's own variable, rather than as a backend error naming a view dimension.
    let stage = Stage::sampling(PADDING_BINDING);
    let failure = renderer
        .render(
            &graphics(1, 1),
            &instance(texture_payload(1, 1, &TEXEL), true, &source(&stage)),
        )
        .expect_err("a shader reading the wrong binding is refused");
    let message = failure.to_string();
    assert!(
        message.contains("binding 1"),
        "the message names the number: {message}"
    );
    assert!(
        message.contains("image_sampler") && message.contains("binds a buffer"),
        "the message names the shader's variable and what the pass bound: {message}"
    );
}

/// What one test puts in the fragment stage, and whether it declares a sampler at all.
///
/// The cases differ only in how the image is read and at which number the sampler is
/// declared, so the shader source is one template with those two things filled in. The
/// distinction is not cosmetic: a shader that reads with `textureLoad` must not declare a
/// sampler, because the layout has none for it, and that is the whole reason the numbering
/// rule reads the need from the document rather than from the format.
struct Stage {
    /// What the fragment stage starts with, as a statement binding `sampled`.
    body: &'static str,
    /// Whether the fragment stage reads a second texel, and from which level it reads it.
    averaged_level: Option<i32>,
    /// Whether the shader declares a sampler.
    declares_sampler: bool,
    /// The binding the sampler is declared at, when one is declared.
    sampler_binding: u32,
}

impl Stage {
    /// The stage that samples its texture, with the sampler where the pass put it.
    const fn sampling(sampler_binding: u32) -> Self {
        Self {
            body: "let sampled = textureSample(image, image_sampler, vec2<f32>(0.5, 0.5));",
            averaged_level: None,
            declares_sampler: true,
            sampler_binding,
        }
    }

    /// The stage that reads its texture by integer coordinate and declares no sampler.
    const fn loading() -> Self {
        Self {
            body: "let sampled = textureLoad(image, vec2<i32>(0, 0), 0);",
            averaged_level: None,
            declares_sampler: false,
            sampler_binding: 0,
        }
    }

    /// The stage that reads the base level and a level above it, both by integer coordinate,
    /// so what it reports is what the texture holds rather than what an sRGB sample of it
    /// decodes to.
    ///
    /// A sample of an sRGB texture converts what it reads to linear light, and the two
    /// candidate averages — a filter that linearises and one that does not — decode to
    /// similar values. Reading a stored texel is the only way to ask what the filter wrote.
    const fn a_known_average(sampler_binding: u32, level: i32) -> Self {
        Self {
            body: "let sampled = textureLoad(image, vec2<i32>(0, 0), 0);",
            averaged_level: Some(level),
            declares_sampler: true,
            sampler_binding,
        }
    }
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

/// Builds a texture payload of the given pixels, one texel per four bytes.
fn texture_payload(width: i64, height: i64, pixels: &[u8]) -> Value {
    texture_payload_in(width, height, "rgba8unorm", pixels)
}

/// Builds a texture payload in a named format.
fn texture_payload_in(width: i64, height: i64, format: &str, pixels: &[u8]) -> Value {
    let expected = (width * height * 4) as usize;
    assert_eq!(pixels.len(), expected, "the pixels fill the texture");
    let mut fields = definition::FieldMap::new();
    fields.insert("width".to_owned(), Value::Int(width));
    fields.insert("height".to_owned(), Value::Int(height));
    fields.insert("format".to_owned(), Value::String(format.to_owned()));
    fields.insert("data".to_owned(), Value::Bytes(pixels.to_vec()));
    Value::Enum {
        tag: "Texture".to_owned(),
        fields,
    }
}

/// Builds the instance the pass draws from.
///
/// `sampling` decides whether the texture payload declares that it is sampled. The key is a
/// unit, so what it holds is nothing and its presence is the whole declaration.
fn instance(texture: Value, sampling: bool, source: &str) -> Value {
    let texture = match texture {
        Value::Enum { tag, mut fields } => {
            if sampling {
                fields.insert(SAMPLED.to_owned(), Value::Unit);
            }
            Value::Enum { tag, fields }
        }
        other => other,
    };
    Value::structure([
        (
            "referencing_graphics".to_owned(),
            Value::String("test.graphics".to_owned()),
        ),
        (
            "entries".to_owned(),
            Value::dictionary([
                ("row".to_owned(), instance_row()),
                ("padding".to_owned(), padding_buffer()),
                // The pass renders into this, so it carries no pixels: a target is written
                // rather than uploaded, and giving it pixels would be uploading into a map
                // the clear is about to overwrite.
                ("frame".to_owned(), target_payload()),
                ("sampled_image".to_owned(), texture),
                ("shader".to_owned(), shader(source)),
                ("triangle".to_owned(), triangle_vertices()),
            ]),
        ),
    ])
}

/// Builds the payload of the texture the frame is rendered into.
fn target_payload() -> Value {
    Value::variant(
        "Texture",
        [
            ("width".to_owned(), Value::Int(FRAME as i64)),
            ("height".to_owned(), Value::Int(FRAME as i64)),
            ("format".to_owned(), Value::String("rgba8unorm".to_owned())),
        ],
    )
}

/// Builds a vertex buffer of three points that cover the whole viewport.
///
/// The third point is at three in both axes, so the triangle a rasteriser clips out of it
/// covers every pixel of the target rather than half of it.
fn triangle_vertices() -> Value {
    packed_buffer(
        &[-1.0, -1.0, 0.0, 3.0, -1.0, 0.0, -1.0, 3.0, 0.0],
        "position",
        "vertex",
    )
}

/// Builds the single instance row, which carries nothing but has to exist to be counted.
fn instance_row() -> Value {
    packed_buffer(&[0.0, 0.0, 0.0], "filler", "instance")
}

/// Builds a buffer the shader binds and does not read.
///
/// It is here to give the numbering something above the image to collide with. Without it a
/// sampler declared at the wrong number would land where nothing is bound, and the test would
/// be checking the weaker of the two messages.
fn padding_buffer() -> Value {
    let mut data = vec![0u8; PADDING_BYTES as usize];
    data[0] = 1;
    Value::variant(
        "Buffer",
        [
            ("size_bytes".to_owned(), Value::Int(PADDING_BYTES)),
            (
                "usage".to_owned(),
                Value::List(vec![Value::String("uniform".to_owned())]),
            ),
            ("data".to_owned(), Value::Bytes(data)),
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
                Schema::new(SchemaKind::structure([(
                    field.to_owned(),
                    graphics::layout::components_schema(3),
                )]))
                .to_value(),
            ),
        ],
    )
}

/// Builds the instance entry the pass names as its shader.
fn shader(source: &str) -> Value {
    Value::variant(
        "Shader",
        [
            ("source".to_owned(), Value::String(source.to_owned())),
            ("schema".to_owned(), interface()),
        ],
    )
}

/// The binding the bound buffer the shader does not read takes.
///
/// `padding` precedes `sampled_image` in name order, so it is the first bound resource.
const PADDING_BINDING: u32 = 1;

/// The binding the image itself takes, which does not move between the cases.
const IMAGE_BINDING: u32 = 2;

/// Builds the wgsl of the test shader.
///
/// The uniform is `time` alone: a pass always carries one, and what it holds is not what
/// these tests are about, so the shader declares the smallest interface that satisfies the
/// check and the pass supplies exactly that.
fn source(stage: &Stage) -> String {
    let image_binding = IMAGE_BINDING;
    let padding_binding = PADDING_BINDING;
    let body = stage.body;
    // A stage that names a level reads it as well, by integer coordinate, and reports the two
    // texels in different channels of the same pixel so the readback can tell them apart.
    let returned = match stage.averaged_level {
        Some(level) => format!(
            "let averaged = textureLoad(image, vec2<i32>(0, 0), {level});\n    \
             return vec4<f32>(sampled.g, 0.0, averaged.g, 1.0);"
        ),
        None => "return sampled;".to_owned(),
    };
    let sampler = if stage.declares_sampler {
        let sampler_binding = stage.sampler_binding;
        format!("@group(0) @binding({sampler_binding}) var image_sampler: sampler;\n")
    } else {
        String::new()
    };
    format!(
        r#"
struct Uniforms {{
    time: f32,
}};

@group(0) @binding(0) var<uniform> uniforms: Uniforms;
@group(0) @binding({image_binding}) var image: texture_2d<f32>;
@group(0) @binding({padding_binding}) var<uniform> padding: vec4<f32>;
{sampler}
@vertex
fn vertex_main(@location(0) position: vec3<f32>) -> @builtin(position) vec4<f32> {{
    return vec4<f32>(position, 1.0);
}}

@fragment
fn fragment_main() -> @location(0) vec4<f32> {{
    {body}
    {returned}
}}
"#
    )
}

/// The interface the test shader declares.
///
/// A binder field is one the pass must have, so the interface lists every binder the pass
/// declares, not only the one the fragment stage reads. What it is *not* required to do is
/// read them: the shader's body samples one image and ignores the two buffers, which is
/// legal and is what makes an interface a statement about what a pass offers.
fn interface() -> Value {
    Schema::new(SchemaKind::structure([
        (
            "uniform".to_owned(),
            Schema::new(SchemaKind::structure([(
                "time".to_owned(),
                Schema::new(SchemaKind::Float),
            )])),
        ),
        (
            "binders".to_owned(),
            Schema::new(SchemaKind::structure([
                ("frame".to_owned(), graphics::texture_binding_schema()),
                ("padding".to_owned(), graphics::buffer_binding_schema()),
                ("row".to_owned(), graphics::buffer_binding_schema()),
                (
                    "sampled_image".to_owned(),
                    graphics::texture_binding_schema(),
                ),
                ("triangle".to_owned(), graphics::buffer_binding_schema()),
            ])),
        ),
    ]))
    .to_value()
}

/// Builds a graphics document of one rasterize pass that renders into the texture it reads.
fn graphics(width: i64, height: i64) -> Value {
    let pass = Value::variant(
        "Rasterize",
        [
            ("label".to_owned(), Value::String("sampling".to_owned())),
            (
                "uniform".to_owned(),
                Value::structure([("time".to_owned(), Value::Float(0.0))]),
            ),
            ("shader".to_owned(), Value::String("shader".to_owned())),
            (
                "binders".to_owned(),
                Value::dictionary([
                    ("frame".to_owned(), target_declaration()),
                    (
                        "padding".to_owned(),
                        buffer_declaration("padding", PADDING_BYTES),
                    ),
                    (
                        "row".to_owned(),
                        buffer_declaration("row", INSTANCE_ROW_BYTES),
                    ),
                    (
                        "sampled_image".to_owned(),
                        texture_declaration("sampled_image", width, height),
                    ),
                    (
                        "triangle".to_owned(),
                        buffer_declaration("triangle", TRIANGLE_BYTES),
                    ),
                ]),
            ),
            // The frame is what the renderer reads back, and the image is what is sampled:
            // one target and one source, which is what keeps the uploaded pixels from being
            // cleared before the fragment stage reads them.
            (
                "targets".to_owned(),
                Value::List(vec![Value::String("frame".to_owned())]),
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
            "instance_schema".to_owned(),
            graphics::example_graphics()
                .as_field_map()
                .and_then(|fields| fields.get("instance_schema"))
                .cloned()
                .expect("the example carries an instance schema"),
        ),
    ])
}

/// Builds the declaration of the texture the frame is rendered into.
fn target_declaration() -> Value {
    Value::variant(
        "Resource",
        [(
            "resource".to_owned(),
            Value::variant(
                "Texture",
                [
                    ("source".to_owned(), Value::String("frame".to_owned())),
                    ("width".to_owned(), Value::Int(FRAME as i64)),
                    ("height".to_owned(), Value::Int(FRAME as i64)),
                    ("format".to_owned(), Value::String("rgba8unorm".to_owned())),
                ],
            ),
        )],
    )
}

/// Builds a texture declaration naming an entry of the instance.
fn texture_declaration(source: &str, width: i64, height: i64) -> Value {
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
                    ("format".to_owned(), Value::String("rgba8unorm".to_owned())),
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
