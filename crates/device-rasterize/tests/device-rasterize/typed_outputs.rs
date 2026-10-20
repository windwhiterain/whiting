//! What a document declares it renders out, and the check that it can actually write it.
//!
//! # What is here and why it is one file
//!
//! Four things, in the order a reader meets them:
//!
//! 1. **The check itself, with no adapter.** [`whiting_device_rasterize::check_outputs`] is a free
//!    function over a parsed `naga` module, on the same grounds `check_resources` is: a scene's
//!    fragment shader is authored on a laptop far more often than it is drawn, and a check that
//!    needed a gpu would only run where a gpu already is.
//! 2. **The mutations**, each showing the message it produces. A test that has never been
//!    seen to fail is not evidence that a check works, so each of these asserts on the *whole*
//!    message rather than on "it errored": the failures are different from each other and a check
//!    that reported one of them for all of them would pass every one of them. The width table's
//!    seven rows are walked in one test, each with the words its own refusal has to contain.
//! 3. **A declared output no pass writes**, which is the hole the declaration opened: a
//!    general `outputs` makes "declared and never written" reachable, where the closed set of
//!    channels could not express it.
//! 4. **One scene-declared output end to end**, on a real adapter: a fragment shader that
//!    writes a second `@location`, declared, type-checked and read back — plus the same
//!    declaration filled with the width the image is *not* drawn at, which is refused before the
//!    pipeline is built.
//!
//! The last two need an adapter and skip with a printed note without one, following
//! `multi_attachment_render.rs`.

use super::support::{
    FRAME, INSTANCE_ROW_BYTES, NO_ADAPTER, TRIANGLE_BYTES, buffer_declaration, instance_row,
    renderer, target_payload, texture_declaration, triangle_vertices,
};
use definition::{Schema, SchemaKind, Value};
use device::{RenderError, Renderer};
use graphics::Output;
use whiting_device_rasterize::check_outputs;

/// The colour the frame's own attachment carries, and a value eight bits holds exactly.
const FRAME_VALUE: [f32; 4] = [0.25, 0.5, 0.75, 1.0];

/// How many components the second image's declaration claims, for the sentence that says the
/// rest of what comes back is padding.
const DECLARED: usize = 3;

/// The value the second, scene-written attachment carries.
///
/// Negative and above one on purpose: neither survives an `rgba8unorm` target, so a value that
/// came back as `0.0` in the blue channel is a value that was never written where it was read
/// from, and a swapped pair cannot produce anything that looks right.
const NORMAL_VALUE: [f32; 4] = [0.0, 0.0, 1.0, 1.0];

/// A shader whose fragment stage writes `colour` at location zero and `normal` at location one.
const TWO_OUTPUTS: &str = r#"
struct Uniforms {
    time: f32,
};

@group(0) @binding(0) var<uniform> uniforms: Uniforms;

@vertex
fn vertex_main(@location(0) position: vec3<f32>) -> @builtin(position) vec4<f32> {
    return vec4<f32>(position, 1.0);
}

struct Written {
    @location(0) colour: vec4<f32>,
    @location(1) normal: vec4<f32>,
};

@fragment
fn fragment_main() -> Written {
    var out: Written;
    out.colour = vec4<f32>(0.25, 0.5, 0.75, 1.0);
    out.normal = vec4<f32>(0.0, 0.0, 1.0, 1.0);
    return out;
}
"#;

/// The same shader with its second output at a location the declaration does not name.
///
/// This is **mutation one**: the values are identical and only the `@location` moved, so a
/// check that compared values rather than locations would pass it.
const WRONG_LOCATION: &str = r#"
struct Uniforms {
    time: f32,
};

@group(0) @binding(0) var<uniform> uniforms: Uniforms;

@vertex
fn vertex_main(@location(0) position: vec3<f32>) -> @builtin(position) vec4<f32> {
    return vec4<f32>(position, 1.0);
}

struct Written {
    @location(0) colour: vec4<f32>,
    @location(2) normal: vec3<f32>,
};

@fragment
fn fragment_main() -> Written {
    var out: Written;
    out.colour = vec4<f32>(0.25, 0.5, 0.75, 1.0);
    out.normal = vec4<f32>(0.0, 0.0, 1.0, 1.0);
    return out;
}
"#;

/// The same shader with its second output **narrower** than the declaration names.
///
/// This is **mutation two**: the location is the one the declaration names, the variable name is
/// the one it would look for, and only the type disagrees — and it disagrees in the direction a
/// real G-buffer goes wrong, a shader that fills three of the four components the image claims
/// to hold. It is also a shader no backend would accept, which is the point: the check fires
/// first and names both documents, where the backend's own message names neither.
const WRONG_TYPE: &str = r#"
struct Uniforms {
    time: f32,
};

@group(0) @binding(0) var<uniform> uniforms: Uniforms;

@vertex
fn vertex_main(@location(0) position: vec3<f32>) -> @builtin(position) vec4<f32> {
    return vec4<f32>(position, 1.0);
}

struct Written {
    @location(0) colour: vec4<f32>,
    @location(1) normal: vec3<f32>,
};

@fragment
fn fragment_main() -> Written {
    var out: Written;
    out.colour = vec4<f32>(0.25, 0.5, 0.75, 1.0);
    out.normal = vec3<f32>(0.0, 0.0, 1.0);
    return out;
}
"#;

/// The same shader writing a whole number, which is a type no format on this device can hold.
///
/// This is **mutation three**: the declaration and the shader agree completely — same location,
/// same variable, same type — and the output is still refused, because a declared type is
/// refused on its own before it is compared with anything. That ordering is the point: a check
/// that compared first would report a *disagreement* about two documents that agree, and send a
/// reader to edit the WGSL.
const INTEGER_OUTPUT: &str = r#"
struct Uniforms {
    time: f32,
};

@group(0) @binding(0) var<uniform> uniforms: Uniforms;

@vertex
fn vertex_main(@location(0) position: vec3<f32>) -> @builtin(position) vec4<f32> {
    return vec4<f32>(position, 1.0);
}

struct Written {
    @location(0) colour: vec4<f32>,
    @location(1) identifier: u32,
};

@fragment
fn fragment_main() -> Written {
    var out: Written;
    out.colour = vec4<f32>(0.25, 0.5, 0.75, 1.0);
    out.identifier = 7u;
    return out;
}
"#;

/// A shader writing a **three component** value at location one, which is the width a
/// `vec3<f32>` declaration claims.
///
/// This is the shape the proposal's own worked example wanted, and it is the shape the width rule
/// exists to refuse: the declaration is three numbers, the image it is drawn in is four wide, and
/// three numbers do not fill four. A reader would find one number per texel that the scene never
/// wrote, so the same document with `vec4<f32>` at this location is the row that draws.
const THREE_COMPONENTS: &str = r#"
struct Uniforms {
    time: f32,
};

@group(0) @binding(0) var<uniform> uniforms: Uniforms;

@vertex
fn vertex_main(@location(0) position: vec3<f32>) -> @builtin(position) vec4<f32> {
    return vec4<f32>(position, 1.0);
}

struct Written {
    @location(0) colour: vec4<f32>,
    @location(1) normal: vec3<f32>,
};

@fragment
fn fragment_main() -> Written {
    var out: Written;
    out.colour = vec4<f32>(0.25, 0.5, 0.75, 1.0);
    out.normal = vec3<f32>(0.0, 0.0, 1.0);
    return out;
}
"#;

/// A shader writing a **scalar** at location one, where a `vec3<f32>` declaration sits.
///
/// This is the same row with a smaller supply, and it is the case the message has to be exact
/// about: three of the four numbers a reader would find in the image are values nothing supplied,
/// and they read as though they had been written.
const ONE_COMPONENT: &str = r#"
struct Uniforms {
    time: f32,
};

@group(0) @binding(0) var<uniform> uniforms: Uniforms;

@vertex
fn vertex_main(@location(0) position: vec3<f32>) -> @builtin(position) vec4<f32> {
    return vec4<f32>(position, 1.0);
}

struct Written {
    @location(0) colour: vec4<f32>,
    @location(1) normal: f32,
};

@fragment
fn fragment_main() -> Written {
    var out: Written;
    out.colour = vec4<f32>(0.25, 0.5, 0.75, 1.0);
    out.normal = 1.0;
    return out;
}
"#;

#[test]
fn a_declaration_the_shader_agrees_with_passes() {
    let module = parse(TWO_OUTPUTS);
    let error = check_outputs(&module, &declared_both(), &["first", "normal"], "gbuffer");
    assert!(
        error.is_ok(),
        "a declaration naming the point, the location and the type the fragment output actually \
         has is the case the check exists to allow: {error:?}"
    );
}

#[test]
fn a_declaration_at_another_passs_point_is_not_this_passs_business() {
    let module = parse(TWO_OUTPUTS);
    // The same two declarations, and a pass that renders into only the first of their points.
    let error = check_outputs(&module, &declared_both(), &["first"], "frame");
    assert!(
        error.is_ok(),
        "a point this pass does not render into is another pass's to answer for, and a pass that \
         checked it would fail on every document in the workspace for writing the colour the \
         frame is made of: {error:?}"
    );
}

#[test]
fn a_module_with_no_fragment_output_at_a_declared_point_is_the_writing_passs_answer() {
    let module = parse(
        r#"
@vertex
fn vertex_main() -> @builtin(position) vec4<f32> {
    return vec4<f32>(0.0, 0.0, 0.0, 1.0);
}
"#,
    );
    // A module with no fragment stage and no declaration against it is a shape rather than a
    // disagreement, so the check says nothing at all.
    assert!(
        check_outputs(&module, &[], &["depth_buffer"], "depth-only").is_ok(),
        "a depth-only pass has no fragment stage, and nothing declared against it is a shape \
         rather than a disagreement"
    );
    // A declaration at one of its **colour** points is different: the pass has a target and
    // nothing to fill it, which is precisely the uninitialised-memory case the check exists for.
    // The backend would reject the pipeline for the same reason and with a message naming
    // neither document.
    let message = message(
        &module,
        &declared_both(),
        &["first", "normal"],
        "depth-only",
    );
    assert!(message.contains("writes nothing there"), "{message}");
    assert!(
        message.contains("has no fragment output at all"),
        "{message}"
    );
    assert!(
        message.contains("whatever the image was cleared to"),
        "{message}"
    );
}

#[test]
fn every_row_of_the_width_table_gives_its_own_verdict() {
    // The rule, as a table. **A fragment output has to be exactly as wide as the image the
    // declaration resolves to**, because an image is filled one whole texel at a time — and the
    // declared count is a *different* question, answered by the same row: it says how many numbers
    // the value is, and everything from that count onward is padding. The second column is the
    // resolved width, read back out of the table rather than written out here, so a row that moved
    // would move the column with it and the test would say so.
    //
    // Each row is a document and a shader that are both valid on their own, and the verdict is the
    // whole of the change. The refused rows also assert on what the message *says*, because a
    // check that refused all of them with one sentence would pass a table this shape.
    let rows: [(&str, &str, &str, &str, &str, bool, &str); 7] = [
        // declared, written, source, what it says, resolved width, allowed, what it must say
        (
            "vec3<f32>",
            "vec4<f32>",
            TWO_OUTPUTS,
            "the image is drawn four wide and the fourth component is padding",
            "4",
            true,
            "",
        ),
        (
            "vec3<f32>",
            "vec3<f32>",
            THREE_COMPONENTS,
            "the image is drawn four wide and the shader supplies three",
            "4",
            false,
            "This image is drawn 4 components wide",
        ),
        (
            "vec3<f32>",
            "f32",
            ONE_COMPONENT,
            "the same, with a scalar, and the message has to say how many it supplies",
            "4",
            false,
            "supplies 1 of the 4",
        ),
        (
            "vec4<f32>",
            "vec4<f32>",
            TWO_OUTPUTS,
            "equal all the way: the value is four numbers and the image is four wide",
            "4",
            true,
            "",
        ),
        (
            "vec4<f32>",
            "vec3<f32>",
            THREE_COMPONENTS,
            "the declared count agrees and the image still is not filled",
            "4",
            false,
            "A reader would find 1 number the scene never wrote",
        ),
        (
            "f32",
            "f32",
            ONE_COMPONENT,
            "the one row whose image is exactly as wide as the value, and the only one with no \
             padding at all",
            "1",
            true,
            "",
        ),
        (
            "f32",
            "vec4<f32>",
            TWO_OUTPUTS,
            "a one component image cannot be filled with four, and the surplus is dropped rather \
             than refused",
            "1",
            false,
            "have nowhere to go",
        ),
    ];

    for (declared, written, source, why, width, allowed, must_say) in rows {
        let module = parse(source);
        let outputs = vec![
            output("present", "first", 0, "vec4<f32>"),
            output("normal", "normal", 1, declared),
        ];
        // The column is read back out of the table rather than trusted, so the two numbers in the
        // rule — the declared count and the width of the image it is drawn in — are both visible
        // in the failure if either of them is not what this test believes.
        let parsed = graphics::TexelType::parse(declared).expect("the table has this row");
        assert_eq!(
            parsed.image_width().to_string(),
            width,
            "a `{declared}` declaration resolves to an image {width} components wide"
        );
        assert!(
            parsed.components() <= parsed.image_width(),
            "a `{declared}` declaration claims {} numbers of the {width} the image holds, and a \
             declaration may never claim more than the image has",
            parsed.components()
        );

        let verdict = verdict(&module, &outputs, &["first", "normal"], "gbuffer");
        if allowed {
            assert!(
                verdict.is_ok(),
                "a declaration of `{declared}` and a fragment output of `{written}` is {why}, and \
                 is allowed: {verdict:?}"
            );
        } else {
            let refused = verdict.expect_err(&format!(
                "a declaration of `{declared}` and a fragment output of `{written}` is {why}, and \
                 is refused"
            ));
            // The two documents, in both vocabularies, and the image they disagree about.
            assert!(
                refused.contains(&format!("declared at location 1 as `{declared}`")),
                "{refused}"
            );
            assert!(
                refused.contains(&format!("writes `normal` there as `{written}`")),
                "{refused}"
            );
            assert!(
                refused.contains(must_say),
                "the refusal for `{declared}` filled by `{written}` has to say {must_say:?}, \
                 which is the fact the reader acts on: {refused}"
            );
            println!("\n--- refused: declared `{declared}`, written `{written}` ---\n{refused}");
        }
    }
}

#[test]
fn the_four_width_refusals_are_four_different_messages_that_name_no_protocol() {
    // The four refusals the width rule can raise, each a document and a shader that are both
    // valid and disagree about the image they are about. A check that reported one of them for
    // all four would pass every one of them, which is why this compares the whole messages.
    let image_wider_than_the_write = message(
        &parse(THREE_COMPONENTS),
        &[
            output("present", "first", 0, "vec4<f32>"),
            output("normal", "normal", 1, "vec3<f32>"),
        ],
        &["first", "normal"],
        "gbuffer",
    );
    let a_scalar = message(
        &parse(ONE_COMPONENT),
        &[
            output("present", "first", 0, "vec4<f32>"),
            output("normal", "normal", 1, "vec3<f32>"),
        ],
        &["first", "normal"],
        "gbuffer",
    );
    let declared_count_agrees_and_the_image_is_still_short = message(
        &parse(THREE_COMPONENTS),
        &declared_both(),
        &["first", "normal"],
        "gbuffer",
    );
    let image_narrower_than_the_write = message(
        &parse(TWO_OUTPUTS),
        &[
            output("present", "first", 0, "vec4<f32>"),
            output("normal", "normal", 1, "f32"),
        ],
        &["first", "normal"],
        "gbuffer",
    );
    // The one message in this path that is allowed to name a format, because it is phrased about
    // the device that cannot create or read it back rather than about a rule.
    let no_format = message(
        &parse(INTEGER_OUTPUT),
        &[
            output("present", "first", 0, "vec4<f32>"),
            output("identifier", "normal", 1, "u32"),
        ],
        &["first", "normal"],
        "gbuffer",
    );

    // Five refusals, five sentences.
    let said = [
        (
            "a `vec3` declaration and a `vec3` write",
            &image_wider_than_the_write,
        ),
        ("a `vec3` declaration and a scalar", &a_scalar),
        (
            "a `vec4` declaration and a `vec3` write",
            &declared_count_agrees_and_the_image_is_still_short,
        ),
        (
            "an `f32` declaration and a `vec4` write",
            &image_narrower_than_the_write,
        ),
        ("a type this device cannot read back", &no_format),
    ];
    for (index, (left, left_message)) in said.iter().enumerate() {
        for (right, right_message) in &said[index + 1..] {
            assert_ne!(
                left_message, right_message,
                "the refusal for {left} and the refusal for {right} are different defects, and \
                 one sentence for both would pass a table this shape"
            );
        }
    }

    // Each of the four says how wide the image is and how many the shader supplies, because those
    // are the two numbers the reader acts on and neither is in the shader.
    for (label, message, wide, supplied) in [
        (
            "a `vec3` declaration and a `vec3` write",
            &image_wider_than_the_write,
            "This image is drawn 4 components wide",
            "the shader supplies 3 of the 4",
        ),
        (
            "a `vec3` declaration and a scalar",
            &a_scalar,
            "This image is drawn 4 components wide",
            "the shader supplies 1 of the 4",
        ),
        (
            "a `vec4` declaration and a `vec3` write",
            &declared_count_agrees_and_the_image_is_still_short,
            "This image is drawn 4 components wide",
            "the shader supplies 3 of the 4",
        ),
        (
            "an `f32` declaration and a `vec4` write",
            &image_narrower_than_the_write,
            "This image is drawn 1 component wide",
            "3 of the 4 components the shader writes have nowhere to go",
        ),
    ] {
        assert!(message.contains(wide), "{label}: {message}");
        assert!(message.contains(supplied), "{label}: {message}");
    }

    // The remedy each one names is the surplus, because that is the edit that fixes it.
    assert!(
        a_scalar.contains("write 4 components at this location"),
        "a scalar is a shortfall like any other and takes the same edit: {a_scalar}"
    );
    assert!(
        image_narrower_than_the_write.contains("The declared type is what decides how wide"),
        "a surplus is the other direction, and its edit is the declared type: \
         {image_narrower_than_the_write}"
    );

    println!(
        "\n--- 1. a `vec3` declaration, a `vec3` write ---\n{image_wider_than_the_write}\n\
         --- 2. a `vec3` declaration, a scalar ---\n{a_scalar}\n\
         --- 3. a `vec4` declaration, a `vec3` write ---\n{declared_count_agrees_and_the_image_is_still_short}\n\
         --- 4. an `f32` declaration, a `vec4` write ---\n{image_narrower_than_the_write}\n\
         --- 5. a type this device cannot read back ---\n{no_format}"
    );
}

/// The words a message in the output path may not contain, and what each of them is.
///
/// **The list is the rule, written out.** A refusal is a fact about two documents disagreeing
/// about one image, and a word from underneath — the backend's name, its format spellings, its own
/// vocabulary for the thing being filled, or the two words that carry a rule without naming one —
/// turns that fact back into a rule a scene author has to know. `Float32x` is the spelling the
/// pipeline rejection recorded in the handoff came in as, which is why the list exists rather than
/// being hypothetical.
const PROTOCOL_WORDS: [&str; 13] = [
    "wgpu",
    "WebGPU",
    "naga",
    "texture",
    "attachment",
    "target",
    "renderable",
    "is_subtype_of",
    "pipeline creation",
    "channel",
    "subtype",
    "attachment state",
    "blend",
];

/// The words a **width** refusal may not contain, which is the whole of the table's own vocabulary
/// plus the backend's spelling of it.
///
/// The refusal for a type this device cannot read back is exempt from this list and held to
/// [`PROTOCOL_WORDS`] instead: naming the format that would have held the type is what turns
/// "unsupported" into "the plumbing for this is not built".
const FORMAT_WORDS: [&str; 6] = [
    "format",
    "rgba16float",
    "r32float",
    "r32uint",
    "Float32x",
    "Rgba16Float",
];

/// Returns whether a message says a word, matched on word boundaries.
///
/// Boundaries matter: a message may name the document's own `targets` field, and a substring test
/// would call that a render target. Every entry is matched the same way, so a phrase such as
/// `colour target` is only reported when both of its words are there.
fn names_word(message: &str, word: &str) -> bool {
    let words = |text: &str| {
        text.split(|character: char| !character.is_alphanumeric())
            .filter(|token| !token.is_empty())
            .map(str::to_owned)
            .collect::<Vec<String>>()
    };
    let said = words(message);
    words(word)
        .iter()
        .all(|token| said.iter().any(|found| found.eq_ignore_ascii_case(token)))
}

#[test]
fn no_refusal_in_the_output_path_names_a_word_from_underneath() {
    // Every refusal `check_outputs` can raise, one document and one shader each, and the word
    // list applied to all of them. This is the sweep rather than one test's worth of it: a leak in
    // the message for the *scalar* would say nothing about the message for a swapped location, and
    // a list checked against one refusal is a list checked against one sentence.
    let refusals = [
        (
            "a location that is not its point's",
            message(
                &parse(TWO_OUTPUTS),
                &[
                    output("present", "first", 1, "vec4<f32>"),
                    output("normal", "normal", 0, "vec4<f32>"),
                ],
                &["first", "normal"],
                "gbuffer",
            ),
            true,
        ),
        (
            "a type no format can hold",
            message(
                &parse(INTEGER_OUTPUT),
                &[
                    output("present", "first", 0, "vec4<f32>"),
                    output("identifier", "normal", 1, "u32"),
                ],
                &["first", "normal"],
                "gbuffer",
            ),
            false,
        ),
        (
            "a spelling the vocabulary does not carry",
            message(
                &parse(TWO_OUTPUTS),
                &[
                    output("present", "first", 0, "vec4<f32>"),
                    output("normal", "normal", 1, "vec2<f32>"),
                ],
                &["first", "normal"],
                "gbuffer",
            ),
            false,
        ),
        (
            "a location the shader does not write",
            message(
                &parse(WRONG_LOCATION),
                &declared_both(),
                &["first", "normal"],
                "gbuffer",
            ),
            true,
        ),
        (
            "a `vec3` declaration and a `vec3` write",
            message(
                &parse(THREE_COMPONENTS),
                &[
                    output("present", "first", 0, "vec4<f32>"),
                    output("normal", "normal", 1, "vec3<f32>"),
                ],
                &["first", "normal"],
                "gbuffer",
            ),
            true,
        ),
        (
            "a `vec3` declaration and a scalar",
            message(
                &parse(ONE_COMPONENT),
                &[
                    output("present", "first", 0, "vec4<f32>"),
                    output("normal", "normal", 1, "vec3<f32>"),
                ],
                &["first", "normal"],
                "gbuffer",
            ),
            true,
        ),
        (
            "a `vec4` declaration and a `vec3` write",
            message(
                &parse(THREE_COMPONENTS),
                &declared_both(),
                &["first", "normal"],
                "gbuffer",
            ),
            true,
        ),
        (
            "an `f32` declaration and a `vec4` write",
            message(
                &parse(TWO_OUTPUTS),
                &[
                    output("present", "first", 0, "vec4<f32>"),
                    output("normal", "normal", 1, "f32"),
                ],
                &["first", "normal"],
                "gbuffer",
            ),
            true,
        ),
        (
            "a whole number written into a float image",
            message(
                &parse(INTEGER_OUTPUT),
                &declared_both(),
                &["first", "normal"],
                "gbuffer",
            ),
            true,
        ),
        // The second coverage contract's refusal, built rather than drawn: it is the other message
        // in the output path, and a sweep that left it out would be a sweep of one file. The point
        // is named without a word from the list in it, because this message quotes the document's
        // own names and a document is entitled to call a point what it likes.
        (
            "an output no pass writes",
            RenderError::UnwrittenOutput {
                name: "nowhere".to_owned(),
                point: "absent_point".to_owned(),
                ty: "vec4<f32>".to_owned(),
                location: 0,
                labels: vec!["gbuffer".to_owned()],
            }
            .to_string(),
            true,
        ),
    ];

    for (label, said, names_no_format) in &refusals {
        // The spellings a message carries are set in backticks, and an **unbalanced** one is a
        // message that renders as a code span running to the end of the paragraph — which is how
        // a `vec3<f32>` declaration came to read "the width a `f32 declaration resolves to" in
        // the fourth refusal below. Every other assertion here passed on that one.
        assert_eq!(
            said.matches('`').count() % 2,
            0,
            "the refusal for {label} has an unbalanced backtick, so a spelling in it runs away \
             with the rest of the sentence: {said}"
        );
        for leak in PROTOCOL_WORDS {
            assert!(
                !names_word(said, leak),
                "the refusal for {label} names `{leak}`, which is a word from underneath rather than \
                 a fact about the two documents: {said}"
            );
        }
        if !names_no_format {
            continue;
        }
        for format in FORMAT_WORDS {
            assert!(
                !names_word(said, format),
                "the refusal for {label} names `{format}`. A disagreement about two documents is a \
                 fact about the image and about how many numbers the shader supplies, and a width, \
                 a location and a kind of value are all of it: {said}"
            );
        }
    }

    // The two refusals that may name a format, and why. Both are refusals of the **declared
    // type** rather than of a disagreement between two documents, and both are about what this
    // device can do with the value rather than about how an image is written.
    assert!(
        refusals[1].1.contains("r32uint"),
        "the type that would have held it is named, because \"unsupported\" sends a reader \
         looking for a typo: {}",
        refusals[1].1
    );
    assert!(
        refusals[1]
            .1
            .contains("this device neither creates that format"),
        "and the half of it that is missing is named as this device's: {}",
        refusals[1].1
    );
    assert!(
        refusals[2].1.contains("`vec3<f32>`"),
        "the vocabulary the reader has to choose from is named instead: {}",
        refusals[2].1
    );
}

#[test]
fn a_width_the_vocabulary_does_not_carry_is_not_given_a_protocol_reason() {
    // `vec2<f32>` is a real WGSL type and this table makes no claim about it, so it is refused
    // as an unknown spelling — which is true, and which does not invent a rule from underneath to
    // justify. The message says what the vocabulary is and stops there.
    let refused = message(
        &parse(TWO_OUTPUTS),
        &[
            output("present", "first", 0, "vec4<f32>"),
            output("normal", "normal", 1, "vec2<f32>"),
        ],
        &["first", "normal"],
        "gbuffer",
    );
    assert!(refused.contains("`vec2<f32>` is not a type"), "{refused}");
    assert!(refused.contains("`vec3<f32>`"), "{refused}");
    // The same list every other refusal in this path is held to, rather than a second shorter one:
    // two lists drift apart, and the shorter one is the one that stops being checked.
    for leak in PROTOCOL_WORDS {
        assert!(!names_word(&refused, leak), "names `{leak}`: {refused}");
    }
}

#[test]
fn a_location_the_shader_does_not_write_is_named_with_the_ones_it_does() {
    let module = parse(WRONG_LOCATION);
    let message = message(&module, &declared_both(), &["first", "normal"], "gbuffer");

    // The output, the location it asked for, and the locations the shader does write.
    assert!(message.contains("`normal`"), "{message}");
    assert!(message.contains("location 1"), "{message}");
    // The shader's own variable, which is what a reader opens the file to find.
    assert!(message.contains("`normal` at location 2"), "{message}");
    assert!(message.contains("`colour` at location 0"), "{message}");
    // Why the image would have been worthless rather than merely wrong.
    assert!(
        message.contains("whatever the image was cleared to"),
        "{message}"
    );
}

#[test]
fn a_type_the_shader_does_not_write_is_reported_in_both_spellings() {
    let module = parse(WRONG_TYPE);
    let message = message(&module, &declared_both(), &["first", "normal"], "gbuffer");

    assert!(
        message.contains("declared at location 1 as `vec4<f32>`"),
        "{message}"
    );
    assert!(
        message.contains("writes `normal` there as `vec3<f32>`"),
        "{message}"
    );
    assert!(message.contains("`gbuffer`"), "{message}");
}

#[test]
fn a_type_no_format_can_hold_is_refused_before_it_is_compared() {
    let module = parse(INTEGER_OUTPUT);
    // The declaration names the whole number, and the shader writes the whole number at the
    // location it names: the two agree, and the output is still refused.
    let outputs = vec![
        output("present", "first", 0, "vec4<f32>"),
        output("identifier", "normal", 1, "u32"),
    ];
    let message = message(&module, &outputs, &["first", "normal"], "gbuffer");

    assert!(message.contains("`u32`"), "{message}");
    assert!(message.contains("location 1"), "{message}");
    // The format that would have held it, named, because "unsupported" sends a reader looking
    // for a typo and "r32uint cannot be read back here" tells them the value is real.
    assert!(message.contains("r32uint"), "{message}");
    // The shader's own variable at its own location, so the message says the two agree.
    assert!(message.contains("`identifier` at location 1"), "{message}");
    assert!(message.contains("as `u32`"), "{message}");
    // And what a declaration *may* carry, so the reader is not left without an alternative.
    assert!(message.contains("`f32`"), "{message}");
    assert!(message.contains("`vec4<f32>`"), "{message}");
}

#[test]
fn the_three_failures_are_three_different_messages() {
    let location = message(
        &parse(WRONG_LOCATION),
        &declared_both(),
        &["first", "normal"],
        "gbuffer",
    );
    let ty = message(
        &parse(WRONG_TYPE),
        &declared_both(),
        &["first", "normal"],
        "gbuffer",
    );
    let format = message(
        &parse(INTEGER_OUTPUT),
        &[
            output("present", "first", 0, "vec4<f32>"),
            output("identifier", "normal", 1, "u32"),
        ],
        &["first", "normal"],
        "gbuffer",
    );

    assert_ne!(
        location, ty,
        "a wrong location and a wrong type are different defects"
    );
    assert_ne!(
        ty, format,
        "a wrong type and an unholdable type are different defects"
    );
    assert_ne!(
        location, format,
        "a wrong location and an unholdable type are different defects"
    );
    // Each names the one thing that is wrong, and not the other two.
    assert!(!location.contains("as `vec4<f32>`"), "{location}");
    assert!(!ty.contains("writes nothing there"), "{ty}");
    assert!(!format.contains("writes nothing there"), "{format}");

    // Printed so that the three can be **read** rather than only compared. The substring
    // assertions above are what holds each one to its content; this is what lets a reviewer see
    // that the three are actually three sentences and not one sentence with three names in it.
    println!("\n--- 1. the location the shader does not write ---\n{location}");
    println!("\n--- 2. the type the shader does not write ---\n{ty}");
    println!("\n--- 3. a type no format can hold ---\n{format}");
}

#[test]
fn a_location_that_is_not_its_points_is_named_before_the_type_is_ever_read() {
    // The mutation this check exists for: **two attachments of the same type, with their
    // declarations' locations swapped.** Both are `vec4<f32>`, both are written by a shader that
    // writes `vec4<f32>` at both locations, and the type check — the one this change relaxed to
    // a width — has nothing to say about either. Before the cross-check this document rendered
    // an image of one thing under the other's name.
    let module = parse(TWO_OUTPUTS);
    let swapped = vec![
        output("present", "first", 1, "vec4<f32>"),
        output("normal", "normal", 0, "vec4<f32>"),
    ];
    let said = message(&module, &swapped, &["first", "normal"], "gbuffer");

    // Both sides of the disagreement, by name: the point the declaration names, the location it
    // claims, and the location that point actually holds.
    assert!(said.contains("`first`"), "{said}");
    assert!(said.contains("location 1"), "{said}");
    assert!(said.contains("renders into at location 0"), "{said}");
    // And why the two are one fact rather than two.
    assert!(
        said.contains("numbered from zero in the order of its `targets`"),
        "{said}"
    );
    assert!(said.contains("both documents would be valid"), "{said}");

    // It is a different message from each of the three the width rule raises, because it is a
    // different defect: nothing here is a type, and nothing here is a format.
    let narrow = message(
        &parse(THREE_COMPONENTS),
        &[
            output("present", "first", 0, "vec4<f32>"),
            output("normal", "normal", 1, "vec4<f32>"),
        ],
        &["first", "normal"],
        "gbuffer",
    );
    assert_ne!(said, narrow);
    assert!(!said.contains("as `vec4<f32>`"), "{said}");
}

#[test]
fn the_declared_both_helper_is_the_same_document_the_swap_mutates() {
    // The mutation above is only a mutation if the un-swapped document passes, so this states
    // that in one line rather than leaving it to be inferred from the neighbouring test.
    let module = parse(TWO_OUTPUTS);
    let error = check_outputs(&module, &declared_both(), &["first", "normal"], "gbuffer");
    assert!(
        error.is_ok(),
        "the same declarations in the same order, which is what the swap changes: {error:?}"
    );
}

#[test]
fn a_three_component_output_is_written_read_back_and_the_frame_is_unchanged() {
    let Some(renderer) = renderer() else {
        println!("{NO_ADAPTER}");
        return;
    };
    // A scene's own fragment shader writes **four** components at `@location(1)` and the
    // document declares that image `vec3<f32>`. That is the padding row, and it is the shape a
    // `vec3` declaration is drawn in: the declaration states how many numbers a texel holds and
    // the drawing is one component wider than that, so the fourth is padding.
    let images = renderer
        .render_outputs(&three_component_graphics(), &typed_instance())
        .expect("a `vec3<f32>` declaration a shader fills with four components renders");

    let normal = images
        .iter()
        .find(|image| image.name == "normal")
        .unwrap_or_else(|| panic!("`normal` is returned, having been declared an output"));
    assert_eq!(
        normal.format, "rgba16float",
        "the format is the one a `vec3<f32>` resolves to, and the image is read back as the float \
         it was drawn as"
    );
    let texel = normal
        .texel(FRAME / 2, FRAME / 2)
        .unwrap_or_else(|| panic!("`normal` holds a texel at the centre"));
    assert_eq!(
        &texel[..3],
        &NORMAL_VALUE[..3],
        "the three components the declaration named are the three the scene's shader wrote, read \
         back from the image the declaration pointed at"
    );
    // **The fourth channel is padding.** The shader wrote a one there because it writes four
    // floats, and the declaration claims three, so nothing about the image depends on it and
    // the check did not refuse the document over it.
    println!(
        "\n--- a `vec3<f32>` output, read back ---\n{texel:?}\n\
         the declaration claims {DECLARED} of these; the fourth is padding the drawing needed \
         and carries no number the declaration asked for."
    );

    // And the beauty frame is where it was, which is the property declaring a second output
    // beside it must not cost.
    let frame = images
        .iter()
        .find(|image| image.name == graphics::PRESENT_OUTPUT)
        .unwrap_or_else(|| panic!("the frame is returned"));
    let frame_texel = frame
        .texel(FRAME / 2, FRAME / 2)
        .unwrap_or_else(|| panic!("`present` holds a texel at the centre"));
    for (axis, (found, written)) in frame_texel.iter().zip(FRAME_VALUE).enumerate() {
        let quantised = (written * 255.0).round();
        assert!(
            (found * 255.0 - quantised).abs() <= 1.0,
            "declaring a `vec3<f32>` output beside the frame does not move a pixel of the frame: \
             component {axis} reads {found} where the shader wrote {written}"
        );
    }
}

#[test]
fn a_three_component_declaration_filled_by_three_components_is_refused_before_the_draw() {
    let Some(renderer) = renderer() else {
        println!("{NO_ADAPTER}");
        return;
    };
    // **The case this whole rule is for.** A `vec3<f32>` declaration filled by a `vec3<f32>`
    // fragment output, which is what the previous rule called equal and allowed. It is drawn four
    // components wide, so the backend rejects the pipeline for it — and the check fires first and
    // names both documents, which is the one outcome the check exists to prevent. The error is
    // matched rather than string-matched to prove *which* code refused it: a `Backend` error from
    // this crate is the check, and anything else would be the pipeline rejection arriving late.
    let error = renderer
        .render_outputs(
            &three_component_graphics(),
            &instance_with(THREE_COMPONENTS),
        )
        .expect_err("a `vec3<f32>` write cannot fill the four wide image a `vec3<f32>` draws");
    let RenderError::Backend(said) = &error else {
        panic!("the check refuses this, rather than the pipeline: {error}");
    };

    assert!(
        said.contains("declared at location 1 as `vec3<f32>`"),
        "{said}"
    );
    assert!(
        said.contains("writes `normal` there as `vec3<f32>`"),
        "{said}"
    );
    assert!(said.contains("drawn 4 components wide"), "{said}");
    assert!(
        said.contains("write 4 components at this location"),
        "the remedy is the surplus, because that is the edit that draws: {said}"
    );
    println!("\n--- 6. a `vec3<f32>` declaration and a `vec3<f32>` write ---\n{said}");
}

#[test]
fn a_swapped_pair_of_same_typed_declarations_is_refused_before_the_draw() {
    let Some(renderer) = renderer() else {
        println!("{NO_ADAPTER}");
        return;
    };
    // The same document with the two declarations' locations exchanged. Both are `vec4<f32>`
    // and both are filled by a shader writing `vec4<f32>`, so this is refused by the location
    // cross-check alone and by nothing else.
    let mut graphics = typed_graphics();
    let Value::Struct(ref mut fields) = graphics else {
        unreachable!("a graphics document is a structure value")
    };
    let Value::Dict(outputs) = fields
        .get_mut(graphics::OUTPUTS_FIELD)
        .expect("the document declares its outputs")
    else {
        unreachable!("`outputs` is a dict")
    };
    outputs.insert(
        "present".to_owned(),
        declared_value("first", 1, "vec4<f32>"),
    );
    outputs.insert(
        "normal".to_owned(),
        declared_value("normal", 0, "vec4<f32>"),
    );

    let said = renderer
        .render_outputs(&graphics, &typed_instance())
        .expect_err("a location that is not its point's is refused before the draw")
        .to_string();
    // Which of the two swapped declarations is named first depends on the order the map reads
    // in, so this asserts on the shape of the refusal and on the pass it names rather than on
    // which half of the swap tripped.
    assert!(said.contains("renders into at location"), "{said}");
    assert!(said.contains("`gbuffer`"), "{said}");
    assert!(
        said.contains("numbered from zero in the order of its `targets`"),
        "{said}"
    );
    // And it is not a type message: nothing about this document is a type, which is what makes
    // it the case the width rule could never have caught.
    assert!(!said.contains("as `vec4<f32>`"), "{said}");
    println!("\n--- 5. a location that is not its point's ---\n{said}");
}

#[test]
fn a_declared_output_no_pass_writes_is_an_error_rather_than_an_empty_image() {
    let Some(renderer) = renderer() else {
        println!("{NO_ADAPTER}");
        return;
    };
    // The document declares an image at a point **no pass renders into**, which is the case the
    // closed set of channels could not express: there was no way to name an image nothing drew.
    let mut graphics = typed_graphics();
    let Value::Struct(ref mut fields) = graphics else {
        unreachable!("a graphics document is a structure value")
    };
    let Value::Dict(outputs) = fields
        .get_mut(graphics::OUTPUTS_FIELD)
        .expect("the document declares its outputs")
    else {
        unreachable!("`outputs` is a dict")
    };
    outputs.insert(
        "nowhere".to_owned(),
        declared_value("absent_target", 0, "vec4<f32>"),
    );

    let error = renderer
        .render_outputs(&graphics, &typed_instance())
        .expect_err("an output no pass writes is refused rather than answered with nothing");

    let RenderError::UnwrittenOutput {
        name,
        point,
        ty,
        location,
        labels,
    } = &error
    else {
        panic!("the failure is the unwritten-output error, found {error}");
    };
    assert_eq!(
        (name.as_str(), point.as_str()),
        ("nowhere", "absent_target")
    );
    assert_eq!((ty.as_str(), *location), ("vec4<f32>", 0));
    assert_eq!(
        labels.as_slice(),
        ["gbuffer".to_owned()].as_slice(),
        "the passes that did draw are named, because they are what separates an emitter that \
         never enumerated a bucket from a shader that does not write the location"
    );
    // The second coverage contract, in its own words: an enumeration, not a missing buffer.
    let said = error.to_string();
    println!("\n--- 4. declared, and no pass writes it ---\n{said}");
    assert!(
        said.contains("exactly what the writing pass enumerated"),
        "{said}"
    );
    assert!(said.contains("not a missing buffer"), "{said}");
    assert!(said.contains("gbuffer"), "{said}");
}

#[test]
fn a_scene_declared_output_is_written_read_back_and_carries_its_own_type() {
    let Some(renderer) = renderer() else {
        println!("{NO_ADAPTER}");
        return;
    };
    let images = renderer
        .render_outputs(&typed_graphics(), &typed_instance())
        .expect("a pass whose fragment shader writes the location the document declares renders");

    let names: Vec<&str> = images.iter().map(|image| image.name.as_str()).collect();
    assert_eq!(
        names,
        vec!["normal", "present"],
        "both declared images come back under the names the document gave them, the frame among \
         them, and nothing else: a declared output that is not written is an error rather than a \
         shorter list"
    );

    let read = |name: &str| {
        images
            .iter()
            .find(|image| image.name == name)
            .unwrap_or_else(|| panic!("`{name}` is returned, having been declared an output"))
    };
    let normal = read("normal");
    assert_eq!(
        normal.format, "rgba16float",
        "the format is the one the declared `vec4<f32>` resolves to, and the image is read back \
         as the float it was drawn as rather than as a byte"
    );
    let texel = normal
        .texel(FRAME / 2, FRAME / 2)
        .unwrap_or_else(|| panic!("`normal` holds a texel at the centre"));
    assert_eq!(
        &texel[..3],
        &NORMAL_VALUE[..3],
        "the second attachment holds the value the scene's own fragment shader wrote at \
         `@location(1)`, and not the frame's: the two differ in every channel an eight bit \
         target could not have carried, so a swapped pair is not merely a different picture"
    );
    // The frame is untouched by the declaration, which is the property the additive habit bought
    // and which this change gives up only where it has to.
    let frame = read("present");
    assert_eq!(frame.format, "rgba8unorm");
    // Within one eight bit step rather than exactly, because the frame is an `rgba8unorm` and
    // `texel` documents that an eight bit image comes back over 255: what is under test is
    // *which* texel the frame is, and a quantisation of one step in 255 is the whole of what
    // the round trip costs. The second attachment, being a float target, is compared exactly.
    let frame_texel = frame
        .texel(FRAME / 2, FRAME / 2)
        .unwrap_or_else(|| panic!("`present` holds a texel at the centre"));
    for (axis, (found, written)) in frame_texel.iter().zip(FRAME_VALUE).enumerate() {
        let quantised = (written * 255.0).round();
        assert!(
            (found * 255.0 - quantised).abs() <= 1.0,
            "declaring a second output beside the frame does not move a pixel of the frame: \
             component {axis} reads {found} where the shader wrote {written}"
        );
    }
}

#[test]
fn the_same_scene_with_a_broken_shader_is_refused_rather_than_drawn() {
    let Some(renderer) = renderer() else {
        println!("{NO_ADAPTER}");
        return;
    };
    let error = renderer
        .render_outputs(&typed_graphics(), &instance_with(WRONG_TYPE))
        .expect_err("a shader that does not write the declared type is refused before the draw");
    let said = error.to_string();
    assert!(
        said.contains("declared at location 1 as `vec4<f32>`"),
        "{said}"
    );
    assert!(
        said.contains("writes `normal` there as `vec3<f32>`"),
        "{said}"
    );
}

/// Parses a shader into the form the check reads, which is all it ever asks a machine for.
fn parse(source: &str) -> wgpu::naga::Module {
    wgpu::naga::front::wgsl::parse_str(source)
        .unwrap_or_else(|error| panic!("the test shader parses: {error}"))
}

/// Returns the message a failed check produced, with the panic saying which case wanted it.
fn message(
    module: &wgpu::naga::Module,
    declared: &[Output<'_>],
    points: &[&str],
    label: &str,
) -> String {
    verdict(module, declared, points, label).expect_err("this case is a failure")
}

/// Returns what the check said, whether it allowed the document or refused it.
///
/// The verdict is kept as a [`Result`] rather than as a message so that a case the table says
/// should pass is asserted on the *absence* of a failure, which a helper returning a `String`
/// could not say.
fn verdict(
    module: &wgpu::naga::Module,
    declared: &[Output<'_>],
    points: &[&str],
    label: &str,
) -> Result<(), String> {
    match check_outputs(module, declared, points, label) {
        Ok(()) => Ok(()),
        Err(RenderError::Backend(message)) => Err(message),
        Err(other) => panic!("the failure is a backend one, found {other}"),
    }
}

/// One declaration, borrowed from string literals for the lifetime of the check.
///
/// The borrows are the point: a document's declarations are borrows out of its own value tree,
/// and a check that took owned copies would be checking a different thing from the one that
/// runs against a real document.
fn output(
    name: &'static str,
    point: &'static str,
    location: u32,
    ty: &'static str,
) -> Output<'static> {
    Output {
        name,
        point,
        location,
        ty,
    }
}

/// The two declarations every pure case above is checked against.
fn declared_both() -> Vec<Output<'static>> {
    vec![
        output("present", "first", 0, "vec4<f32>"),
        output("normal", "normal", 1, "vec4<f32>"),
    ]
}

/// Builds the graphics document: one rasterize pass, two colour attachments, two declarations.
fn typed_graphics() -> Value {
    let binders = vec![
        (
            "first".to_owned(),
            texture_declaration("first", "rgba8unorm"),
        ),
        (
            "normal".to_owned(),
            texture_declaration("normal", "rgba16float"),
        ),
        (
            "row".to_owned(),
            buffer_declaration("row", INSTANCE_ROW_BYTES),
        ),
        (
            "triangle".to_owned(),
            buffer_declaration("triangle", TRIANGLE_BYTES),
        ),
    ];
    let pass = Value::variant(
        "Rasterize",
        [
            ("label".to_owned(), Value::String("gbuffer".to_owned())),
            (
                "uniform".to_owned(),
                Value::structure([("time".to_owned(), Value::Float(0.0))]),
            ),
            ("shader".to_owned(), Value::String("shader".to_owned())),
            ("binders".to_owned(), Value::dictionary(binders)),
            // `first` and `normal` in the order the binders sort into, which is the order
            // `@location(0)` and `@location(1)` resolve against.
            (
                "targets".to_owned(),
                Value::List(vec![
                    Value::String("first".to_owned()),
                    Value::String("normal".to_owned()),
                ]),
            ),
            (
                "clear_color".to_owned(),
                Value::List([0.0, 0.0, 0.0, 1.0].into_iter().map(Value::Float).collect()),
            ),
        ],
    );
    Value::structure(vec![
        ("passes".to_owned(), Value::List(vec![pass])),
        (
            graphics::OUTPUTS_FIELD.to_owned(),
            Value::dictionary(vec![
                (
                    graphics::PRESENT_OUTPUT.to_owned(),
                    declared_value("first", 0, "vec4<f32>"),
                ),
                (
                    "normal".to_owned(),
                    declared_value("normal", 1, "vec4<f32>"),
                ),
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

/// Builds the same graphics document with the second image declared `vec3<f32>` instead.
///
/// Only the declared type differs, and it is the whole of the change: the point, the location
/// and the pass are the ones the other tests use, so a difference in what this document does is
/// a difference about the width and about nothing else.
fn three_component_graphics() -> Value {
    let mut graphics = typed_graphics();
    let Value::Struct(ref mut fields) = graphics else {
        unreachable!("a graphics document is a structure value")
    };
    let Value::Dict(outputs) = fields
        .get_mut(graphics::OUTPUTS_FIELD)
        .expect("the document declares its outputs")
    else {
        unreachable!("`outputs` is a dict")
    };
    outputs.insert(
        "normal".to_owned(),
        declared_value("normal", 1, "vec3<f32>"),
    );
    graphics
}

/// Builds the instance the pass draws from, carrying the source that fills both locations.
fn typed_instance() -> Value {
    instance_with(TWO_OUTPUTS)
}

/// Builds the instance with a different shader, which is how a broken one is rendered.
fn instance_with(source: &str) -> Value {
    Value::structure([
        (
            "referencing_graphics".to_owned(),
            Value::String("test.graphics".to_owned()),
        ),
        (
            "entries".to_owned(),
            Value::dictionary(vec![
                ("row".to_owned(), instance_row()),
                // Both targets are written rather than uploaded, so neither carries pixels:
                // giving a target pixels would be uploading into a map the clear overwrites.
                ("first".to_owned(), target_payload("rgba8unorm")),
                ("normal".to_owned(), target_payload("rgba16float")),
                ("shader".to_owned(), typed_shader(source)),
                ("triangle".to_owned(), triangle_vertices()),
            ]),
        ),
    ])
}

/// Builds one `outputs` entry, the three fields a declaration states.
fn declared_value(point: &str, location: u32, ty: &str) -> Value {
    Value::structure([
        (
            graphics::POINT_FIELD.to_owned(),
            Value::String(point.to_owned()),
        ),
        (
            graphics::LOCATION_FIELD.to_owned(),
            Value::Int(i64::from(location)),
        ),
        (
            graphics::TYPE_FIELD.to_owned(),
            Value::String(ty.to_owned()),
        ),
    ])
}

/// Builds the instance entry the pass names as its shader.
fn typed_shader(source: &str) -> Value {
    Value::variant(
        "Shader",
        [
            ("source".to_owned(), Value::String(source.to_owned())),
            ("schema".to_owned(), typed_interface()),
        ],
    )
}

/// The interface the test shader declares.
///
/// The targets are listed because a binder field is one the pass **must** have, even though the
/// shader reads none of them: a target takes a binding number with no entry behind it, so there
/// is nothing for a shader to declare.
fn typed_interface() -> Value {
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
            Schema::new(SchemaKind::structure(vec![
                ("first".to_owned(), graphics::texture_binding_schema()),
                ("normal".to_owned(), graphics::texture_binding_schema()),
                ("row".to_owned(), graphics::buffer_binding_schema()),
                ("triangle".to_owned(), graphics::buffer_binding_schema()),
            ])),
        ),
    ]))
    .to_value()
}
