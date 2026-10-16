//! The `grade` channel: the frame before the post stage's grade, beside the frame after it.
//!
//! # What this is for
//!
//! Two runs lost hours to exposure, and both lost them to the same thing: a transform they
//! could neither observe nor query. One cut the exposure 44% and the image barely moved, and
//! found out why by sampling its own output png from PowerShell and printing
//! `(760,300) = 208,206,204` — a sun glow term in the grade was adding to the sky. The other
//! darkened a clay albedo 60% and the pumpkins stayed orange, because ACES pushes bright red
//! towards yellow-white at the high end, so the fix was the emissive peak: 92 minutes.
//!
//! Both were reasoning about a black box and both eventually left the framework to measure the
//! output file. This channel is the black box opened: the **input** of the grade beside its
//! **output**, at the same pixels.
//!
//! # What is pinned here
//!
//! - **it copies the shaded image and interprets nothing**, so the two files differ by the
//!   grade and by nothing else;
//! - **its contract is where the numbers came from, not what its pass drew** — a pass that
//!   copies a buffer covers the buffer, and saying so is what keeps a caller from believing
//!   the image over the picture when the two disagree;
//! - **it says what it cannot show**: the sky is black because the post stage draws it. It used
//!   to say a second thing here — that a value at one was clipped by the eight bit target before
//!   the grade ever saw it — and `0257706` ended that by giving the shaded image `rgba16float`,
//!   so the file is now the radiance the grade reads and is not bounded by one;
//! - **it moves no pixel of the beauty image**, which is the contract every channel holds and
//!   the one a channel that tinted the frame in place would break;
//! - **it does not borrow the depth channels' note**, because a full frame of a scene that
//!   draws nothing is not an empty image.

use definition::{Definition, DefinitionIdentifier, Registry, SchemaIdentifier, Value};
use graphics::pass::{BINDERS, LABEL};
use whiting_gallery::Gallery;
use whiting_scene::standard::PRESENT_TARGET;
use whiting_scene::{
    CompileContext, Compiler, DebugChannel, STANDARD_SCENE_SCHEMA, StandardScene,
    register_standard_scene_schema,
};

/// The scene the measurements below are made on: it has a post chain, and it is the scene the
/// proposal's evidence came from.
fn tidewater() -> Value {
    Gallery::build("tidewater")
}

#[test]
fn the_grade_channel_is_a_channel_of_its_own_and_not_a_tag() {
    // A channel a caller cannot ask for is not a channel, and one that answers to another
    // channel's name is worse: the picture and the file beside it would be describing
    // different things under one word.
    assert_eq!(DebugChannel::from_tag("grade"), Some(DebugChannel::Grade));
    for channel in DebugChannel::ALL {
        assert_eq!(DebugChannel::from_tag(channel.tag()), Some(channel));
        assert_eq!(DebugChannel::from_output(channel.output()), Some(channel));
    }
    assert_eq!(DebugChannel::from_tag("exposure"), None);
}

#[test]
fn the_grade_channel_reads_the_shaded_image_and_not_the_depth_buffer() {
    // The two contracts are different and the compiler has to know which is which, because it
    // raises a note about an empty image for a channel that reads depth. A `grade` image of a
    // scene that draws nothing is a full frame of black, not an empty image, and the note
    // would be false about it.
    assert!(!DebugChannel::Grade.reads_depth());
    assert!(DebugChannel::Position.reads_depth());
    assert!(DebugChannel::Depth.reads_depth());
    let points = [
        DebugChannel::Position.point(),
        DebugChannel::Depth.point(),
        DebugChannel::Grade.point(),
    ];
    assert_eq!(
        points,
        ["debug_position", "debug_depth", "debug_grade"],
        "and each writes a target of its own, which is what makes 'must not move a pixel of the \
         picture' a structural fact rather than a promise: two channels sharing one target \
         would make the image being diagnosed a different image"
    );
}

#[test]
fn the_grade_channel_declares_the_shaded_image_it_copies() {
    // The interface is the contract between the compiler's own shader and the pass it emits,
    // and a binder the source cannot read is a number in the layout that means nothing. What is
    // checked here is that the interface names the shaded image and does not name the depth
    // buffer: this channel copies a colour and recovers nothing.
    let interface = DebugChannel::Grade.pass_schema().to_value();
    let said = format!("{interface:?}");
    assert!(
        said.contains("color_target"),
        "the grade pass binds the image the grade reads: {said}"
    );
    assert!(
        !said.contains("depth_buffer"),
        "and does not bind the depth buffer, which it has no use for: {said}"
    );
    assert!(
        said.contains("debug_grade"),
        "and writes a target of its own, so no pixel of the picture is touched: {said}"
    );
}

#[test]
fn the_grade_shader_is_a_copy_and_not_an_interpretation() {
    // The channel's whole claim is that the two files differ by the grade, so its shader must
    // add nothing: no camera, no matrix, no test, no branch. A second convention between the
    // caller and the image is what the module refuses everywhere else.
    let source = DebugChannel::Grade.source();
    assert!(
        source.contains("textureLoad(color_target, pixel, 0)"),
        "the value is the texel, verbatim: {source}"
    );
    let fragment = source.split("@fragment").nth(1).expect("a fragment stage");
    for forbidden in [
        "ray_through",
        "view_projection",
        "depth_buffer",
        "normalize(",
        "if (",
    ] {
        assert!(
            !fragment.contains(forbidden),
            "and the fragment stage contains no `{forbidden}`, because a copy that interpreted \
             would be a second convention between the caller and the image the grade reads: \
             {fragment}"
        );
    }
}

#[test]
fn the_grade_header_says_what_it_cannot_show() {
    // A file of numbers whose header does not say what it leaves out is a measurement somebody
    // will believe. Two things are left out here and both are stated: the sky, and the pixels
    // with no surface.
    //
    // **What the radiance bullet asserts, and why it is stronger than the one it replaced.**
    // It used to hold that the header carries "eight bit", which pinned a claim about the
    // *shaded image's format*: that the lighting stage clipped what was above one, so a value
    // at one here might have been one or might have been forty. `0257706` gave the shaded image
    // `rgba16float` and that claim became false, so holding the text would have pinned a lie.
    // What is asserted now is the claim that *survives* and is the one a reader needs: that
    // these numbers are scene-linear and **are not bounded by one**, which is a property of the
    // file rather than of a format constant elsewhere, and which fails loudly if a later change
    // puts the ceiling back. The negative assertion below is what keeps it honest in the other
    // direction — a header that regained the words "eight bit" has regained the lie with them.
    let header = DebugChannel::Grade.description();
    for said in [
        "before that grade has run",
        "not bounded by",
        "sky is",
        "alpha marks nothing",
    ] {
        assert!(
            header.contains(said),
            "the header carries `{said}`, which is what makes the numbers readable rather than \
             merely present: {header}"
        );
    }
    assert!(
        !header.contains("world position") && !header.contains("depth buffer"),
        "and it is not either other channel's sentence: {header}"
    );
    assert!(
        !header.contains("eight bit"),
        "and it does not claim the shaded image is eight bits a channel again, which `0257706` \
         made false when it gave that image `rgba16float`: {header}"
    );
}

#[test]
fn asking_for_the_grade_channel_moves_no_pixel_of_the_picture() {
    // The contract every channel holds, checked where it can be checked without an adapter: the
    // emitted documents differ **only** by the channel's own pass, target, shader entry and
    // declared output. The beauty image is the `present` output either way, and a channel that
    // could reach it would change what that output names. The picture half of this is measured
    // on the adapter in the handoff, by sha-256 over the png.
    let quiet = compiled(&tidewater(), &[]);
    let loud = compiled(&tidewater(), &[DebugChannel::Grade]);
    assert_eq!(
        frame_output(&quiet),
        frame_output(&loud),
        "the frame is named the same way with and without the channel, so the picture comes back \
         from the same image either way"
    );
    assert_eq!(
        quiet.output_names(),
        vec!["present".to_owned()],
        "a scene asked for no channel declares one output"
    );
    assert_eq!(
        loud.output_names(),
        vec!["grade".to_owned(), "present".to_owned()],
        "and a scene asked for the grade channel declares that image beside the frame, under the \
         name the harness looks it up by"
    );
    assert_eq!(
        loud.passes() - quiet.passes(),
        1,
        "for one pass of its own, which writes its own target and reads the shaded image"
    );
    assert!(
        reaches_the_frame_target(&loud).is_empty(),
        "and nothing a channel emits binds the image the frame is written to, which is what \
         'must not move a pixel of the beauty image' means in a document: {:?}",
        reaches_the_frame_target(&loud)
    );
}

/// Returns the pass count an emitted document declares.
trait Passes {
    fn passes(&self) -> usize;
    fn output_names(&self) -> Vec<String>;
    fn frame_output(&self) -> Option<String>;
}

impl Passes for whiting_scene::CompileOutput {
    fn passes(&self) -> usize {
        self.graphics
            .value
            .as_field_map()
            .and_then(|fields| fields.get(graphics::PASSES_FIELD))
            .and_then(Value::as_list)
            .map(|list| list.len())
            .unwrap_or(0)
    }

    fn output_names(&self) -> Vec<String> {
        self.graphics
            .value
            .as_field_map()
            .and_then(|fields| fields.get(graphics::OUTPUTS_FIELD))
            .and_then(Value::as_dict)
            .map(|outputs| outputs.keys().cloned().collect())
            .unwrap_or_default()
    }

    /// The frame's point, read out of the declaration rather than off the whole value.
    ///
    /// An `outputs` entry is a struct of three fields rather than a bare point, so reading the
    /// frame means reading one field out of one entry — and a test that read the entry itself
    /// would be asserting the shape of the declaration rather than which point is the frame.
    fn frame_output(&self) -> Option<String> {
        self.graphics
            .value
            .as_field_map()
            .and_then(|fields| fields.get(graphics::OUTPUTS_FIELD))
            .and_then(Value::as_dict)
            .and_then(|outputs| outputs.get(graphics::PRESENT_OUTPUT))
            .and_then(Value::as_struct)
            .and_then(|fields| fields.get(graphics::POINT_FIELD))
            .and_then(Value::as_str)
            .map(str::to_owned)
    }
}

/// Returns the frame an emitted document names, or the panic message says there is none.
fn frame_output(emitted: &whiting_scene::CompileOutput) -> Option<String> {
    emitted.frame_output()
}

/// Returns whether any pass the compile emitted other than a post process binds the image the
/// frame is written to.
///
/// This is what "must not move a pixel of the beauty image" means in a document rather than in
/// a promise: the post stage is the one thing that is supposed to write `present_target`, and a
/// channel's pass reaching it would be a channel that changed the picture it is a picture of.
fn reaches_the_frame_target(emitted: &whiting_scene::CompileOutput) -> Vec<String> {
    let passes = emitted
        .graphics
        .value
        .as_field_map()
        .and_then(|fields| fields.get(graphics::PASSES_FIELD))
        .and_then(Value::as_list)
        .unwrap_or_default();
    passes
        .iter()
        .filter(|pass| {
            let fields = pass.variant_fields();
            let label = fields
                .and_then(|fields| fields.get(LABEL))
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_owned();
            let writes_the_frame = fields
                .and_then(|fields| fields.get(BINDERS))
                .and_then(Value::as_dict)
                .is_some_and(|binders| binders.contains_key(PRESENT_TARGET));
            writes_the_frame && !label.starts_with("post/")
        })
        .map(|pass| format!("{pass:?}"))
        .collect()
}

/// Compiles a scene with the channels a caller asked for.
fn compiled(scene: &Value, channels: &[DebugChannel]) -> whiting_scene::CompileOutput {
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
        DefinitionIdentifier::new("grade-channel.source"),
        SchemaIdentifier::new(STANDARD_SCENE_SCHEMA),
        scene.clone(),
    );
    source
        .validate(&registry)
        .expect("the scene validates against the schema");
    StandardScene::new(
        DefinitionIdentifier::new("grade-channel.graphics"),
        DefinitionIdentifier::new("grade-channel.instance"),
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
    .expect("the scene compiles")
}
