//! The debug channels: the extra images the standard pipeline can be asked to draw.
//!
//! # What a channel is
//!
//! An image about the frame rather than of it. A picture says where things are on the screen;
//! a channel says what is *there*, so a caller that sees a dark band can ask which part of the
//! scene it is instead of authoring a second scene to find out.
//!
//! # Why a channel is the compiler's business and not a screen material
//!
//! Every channel is a fullscreen pass over a depth buffer that already exists, so an author
//! could write one as a post process. What they would write is the same twenty lines of ray
//! recovery and matrix arithmetic at every call site, with the y flip and the matrix row
//! convention to get wrong each time. A channel is that arithmetic written once, against the
//! same camera basis and the same recovery the post stage already uses.
//!
//! # Why the channel is not in the scene document
//!
//! A scene says what is in it. A channel is a question its **caller** is asking, and the
//! caller is usually a debugging loop rather than the author of the picture. So it travels as
//! an input to the compile, beside the camera, which is the other input a compile has that the
//! scene does not express by itself.
//!
//! # What a channel covers, which is a contract on the scene
//!
//! [`DebugChannel::Position`] and [`DebugChannel::Depth`] are reconstructed from the **depth
//! buffer**, so each covers exactly what the scene's `depthing` bucket drew. A scene whose lit
//! stages list an instance its `depthing` bucket does not has pixels in its picture that no
//! such channel can name a position for, and a scene with no depth at all produces an empty
//! image — which the compiler says in a note rather than leaving to be discovered. This is not
//! a limitation of a channel: the depth buffer is where a renderer knows what a pixel shows,
//! and a document whose stages disagree about what is on screen is already a document whose
//! depth test is against the wrong surface.
//!
//! [`DebugChannel::Grade`] is reconstructed from a different buffer — the **shaded image** —
//! and is under the same kind of contract rather than a different kind: it covers exactly what
//! the lighting and emission passes accumulated into that image, and it says so on its own
//! description. The two are the same shape because the shape is *where the numbers come from*.
//!
//! # Which channels that contract binds, and why it cannot bind all of them
//!
//! The contract above is a property of *where the numbers come from*, not of channels as
//! such, and a channel that reads no buffer of the pipeline does not inherit it. A channel
//! drawn by a geometry pass of its own covers **exactly what it drew**, and carries a
//! different contract in its place: **which buckets the pass enumerated**. That is the
//! contract a `Normal` or `Identifier` channel would carry if it were written by a
//! compiler-generated G-buffer, and it is recorded here rather than left to the day one is
//! built.
//!
//! # The two halves, and where the boundary between them is
//!
//! An output is either a **built-in** — one of the entries in this module — or a
//! **scene-declared** output, which is a name a document puts in `outputs` beside a point, a
//! `@location` and a type. A caller cannot tell them apart and is not meant to: both are
//! looked up by name out of one `outputs` map, both come back in the format they were drawn
//! in, and both are written into the same manifest and the same sidecar machinery.
//!
//! The distinction is load-bearing and it is **the compiler's business**, so it is kept at the
//! points where it can actually cause a wrong answer rather than described once and hoped for:
//!
//! | | built-in | scene-declared |
//! | --- | --- | --- |
//! | who fills the target | the compiler | the scene's own fragment shader |
//! | what is checked | the compiler's own declared type, against its own WGSL | the shader, parsed ([`check_outputs`]) |
//! | what bounds coverage | **what the pipeline has** — a depth buffer, a shaded image | **which buckets the writing pass enumerated** |
//! | where the coverage is said | [`DebugChannel::description`], in the file's own header | [`DECLARED_DESCRIPTION`], and the error for one nothing wrote |
//!
//! **The coverage sentences are deliberately different words.** A built-in's is a statement
//! about a buffer, so it can name the buffer and a scene that has none. A scene-declared
//! output's is a statement about an enumeration, so it names the pass and cannot name a
//! buffer: there is no buffer to be missing, the writing pass is simply not among the ones
//! that ran. A single vague sentence about "coverage" would be true of neither and would send
//! a reader to the wrong file in both cases.
//!
//! What no contract may be is silent. A channel that did not say what it covers would be
//! the one lying: its image would honestly report what it drew, and a caller comparing it with
//! a picture that does not agree with it would have no way to tell which of the two to believe.
//!
//! [`check_outputs`]: ../../../../device/src/wgpu_renderer/shader_interface.rs

use definition::Schema;
use graphics::TexelType;

use super::pass_schemas::{
    CAMERA, COLOR_TARGET, DEPTH_BUFFER, PROJECTION, VERTICES, camera_schema, interface,
    projection_schema,
};
use graphics::binders::{buffer_binding_schema, texture_binding_schema};

/// One extra image the standard pipeline can be asked to render.
///
/// # Invariants
///
/// - **The set is built-in, on the same grounds the light kinds are**: a name the compiler
///   cannot draw is refused by name rather than emitted as a pass that produces nothing. What
///   it is *not* is the closed set of all outputs — a scene may declare one of its own beside
///   these, type-checked against its own fragment shader, and a caller looks both up the same
///   way. See the module doc for where the two halves meet.
/// - A closed set of built-ins is also what keeps the *meaning* of an image fixed:
///   [`DebugChannel::Position`] is the world position of the nearest surface, and a caller
///   that has to ask what a channel holds would be reading a picture rather than a fact.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DebugChannel {
    /// The world position of the nearest surface each pixel shows, and none where it shows none.
    Position,
    /// The depth buffer itself: the value the depth test compared against at each pixel, and
    /// none where the scene drew nothing.
    Depth,
    /// The shaded image the post stage's grade reads, before that grade has touched it.
    Grade,
}

impl DebugChannel {
    /// Every channel, for a caller that has to name them all in a message or in a loop.
    pub const ALL: [Self; 3] = [Self::Position, Self::Depth, Self::Grade];

    /// Returns the channel a tag names, or none when no channel is called that.
    pub fn from_tag(tag: &str) -> Option<Self> {
        match tag {
            "position" => Some(Self::Position),
            "depth" => Some(Self::Depth),
            "grade" => Some(Self::Grade),
            _ => None,
        }
    }

    /// Returns the channel the image of this name is declared under, or none for an image no
    /// channel drew.
    ///
    /// # Invariants
    ///
    /// A file of numbers may be written beside **every** float image a render produced, and a
    /// document may declare an output the standard pipeline never drew. Those are the
    /// caller's to describe and not a channel's, so this answers none for them rather than
    /// the nearest channel's name: a file described as a world position because it happened
    /// to be a float image is the defect this lookup exists to prevent.
    pub fn from_output(name: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|channel| channel.output() == name)
    }

    /// Returns the tag the channel is named by, which is how a caller asks for it.
    pub const fn tag(self) -> &'static str {
        match self {
            Self::Position => "position",
            Self::Depth => "depth",
            Self::Grade => "grade",
        }
    }

    /// Returns the name the image is declared under in the emitted graphics document.
    ///
    /// This is what a caller looks the image up by, and it is deliberately not the binding
    /// point: the point is where a pass happens to write, and the name is what the image *is*.
    pub const fn output(self) -> &'static str {
        match self {
            Self::Position => "position",
            Self::Depth => "depth",
            Self::Grade => "grade",
        }
    }

    /// Returns the binding point the image is written into, and the instance entry holding it.
    pub const fn point(self) -> &'static str {
        match self {
            Self::Position => "debug_position",
            Self::Depth => "debug_depth",
            Self::Grade => "debug_grade",
        }
    }

    /// Returns whether this channel's image is read out of the **depth buffer**.
    ///
    /// # Why the emitter and the compiler's note both ask this
    ///
    /// The note the emitter raises when a channel is asked for and the scene lists nothing
    /// under `depthing` is about a channel that *reads depth*: a `grade` image is a copy of
    /// the shaded image and holds a full frame of a scene that draws nothing at all, so the
    /// note would be false about it. It is a property of the channel rather than of one call
    /// site, which is what keeps the note honest as channels are added.
    pub const fn reads_depth(self) -> bool {
        match self {
            Self::Position | Self::Depth => true,
            Self::Grade => false,
        }
    }

    /// Returns the type of a texel of this channel's image, as a declaration spells it.
    ///
    /// # Why a built-in declares its own type
    ///
    /// Every channel in this module is **scene-independent**: the WGSL that fills it is
    /// compiler code, not a scene's, so there is nothing a scene can get wrong about it and
    /// nothing for the type check to compare it against. It is still **stated**, because the
    /// declaration is what a caller and the sidecar read, and a type that was written down
    /// only in the format table would be the format-inferred-from-the-channel that
    /// [proposals/typed-outputs.md] superseded. The statement is a fact about this module's
    /// own shader; the check is a fact about a scene's, and the two are never confused.
    ///
    /// # Why a built-in is four components and a scene's own normal is three
    ///
    /// **This is the padding-versus-marker decision, and it is the reason the two kinds of image
    /// are not the same kind.** A `vec3<f32>` declaration draws an image with a fourth channel
    /// in it, and that channel is **padding: it carries no number and marks nothing.** A
    /// declaration has no way to ask for a marker and this vocabulary deliberately offers no
    /// field for one, because a marker is a *reading convention* rather than a value — it says
    /// something to a reader about a pixel, not something about what a texel holds — and a
    /// declaration is the place where a document says what its value is.
    ///
    /// `Position` and `Depth` are the other half of the decision, and they keep `vec4<f32>`
    /// precisely because they **do** carry a marker: the fourth component is how a built-in
    /// image says that no surface was drawn on a pixel, which is a fact about the frame resource
    /// only this module's own WGSL can know. It pays for that fact in a channel no scene asked
    /// about, and its own [`DebugChannel::description`] is what tells a reader what the channel
    /// means. A scene that wants a marker writes its own fourth component and says so in its own
    /// header; the compiler makes no claim about it either way.
    ///
    /// So the shape is **not** "a declaration is four components when it wants a marker and
    /// three when it does not", which would make a scene's declaration a statement about the
    /// compiler's conventions rather than about the scene's value. It is: *a declaration states a
    /// width, and a built-in happens to need four because of something it does rather than
    /// something it holds.* A cleaner shape than that was looked for — a `marker` field, or a
    /// dedicated type — and both were rejected for the same reason: both move a convention of
    /// this module into the document, which is the one place a scene author should never have to
    /// meet it.
    pub const fn ty(self) -> TexelType {
        // Every channel returns four floats. A position is written into four and a depth is
        // copied into four, and the grade's fourth component is the marker the other two use —
        // which is the reason this is `vec4<f32>` where a scene's own three component value is
        // `vec3<f32>`, and the module documentation says which of the two is padding.
        TexelType::Vec4Float
    }

    /// Returns the `@location` this channel's own fragment output carries its image at.
    ///
    /// It is zero for every built-in because **every built-in pass has exactly one colour
    /// target**, and a channel is a fullscreen pass over a buffer rather than a G-buffer pass
    /// over the scene. A scene-declared output has no such guarantee, which is why its
    /// location is a field of the declaration rather than a property of a variant.
    pub const fn location(self) -> u32 {
        0
    }

    /// Returns the format the image is drawn in, which is **derived from** [`DebugChannel::ty`].
    ///
    /// # Invariants
    ///
    /// There is no `match` on the variant here, and that is the whole of the change. A format
    /// inferred from what a channel *is* is a second, private copy of the answer to a question
    /// the document now asks; a format derived from the declared type has exactly one source,
    /// and a new built-in that declared a different type would get a different format without
    /// anybody editing a second table. See [`TexelType::format`] for why each row is the one
    /// it is.
    pub const fn format(self) -> &'static str {
        self.ty().format()
    }

    /// Returns the label the pass is emitted with.
    pub const fn label(self) -> &'static str {
        match self {
            Self::Position => "debug/position",
            Self::Depth => "debug/depth",
            Self::Grade => "debug/grade",
        }
    }

    /// Returns the prose a file of this channel's numbers carries above them, which is what
    /// says what the image **is**.
    ///
    /// # Invariants
    ///
    /// - **It is a property of the channel and not of the file it is written to.** Every
    ///   float image a render produced may be written out with a header, and the program that
    ///   writes the header knows the file's format and nothing about what is in it. A header
    ///   that described the *format* would be a claim the format cannot make: `Position` and
    ///   `Depth` are both `rgba16float` with the same marker, so nothing about the bytes tells
    ///   the two apart, and a sentence they shared would be one of them being wrong.
    /// - **It answers the three questions a reader brings to a file of numbers**: what the
    ///   numbers are, what the alpha among them means, and what the ranges written below it
    ///   stand for. A header that answers two of the three leaves the third to be guessed at,
    ///   and a depth guessed at reads as a world position — the sky coming back a whole scene
    ///   away, in a file that is otherwise entirely well formed.
    /// - **It describes rather than argues.** The reasoning for the conventions it names lives
    ///   on this module, and a caller reading a header has not read this module. The sentences
    ///   are therefore about the numbers in front of the reader and about nothing else.
    ///
    /// ```
    /// use whiting_scene::DebugChannel;
    ///
    /// // The two channels are the same format with the same marker, so nothing but the channel
    /// // itself can tell a caller which is which. A header that could not tell them apart was a
    /// // header one of them was wrong about.
    /// assert_eq!(DebugChannel::Position.format(), DebugChannel::Depth.format());
    /// assert_ne!(DebugChannel::Position.description(), DebugChannel::Depth.description());
    ///
    /// // A depth image read as a world position puts the sky a whole scene away, in a file
    /// // that is otherwise entirely well formed, so the one sentence it must never carry is
    /// // the other channel's.
    /// assert!(DebugChannel::Position.description().contains("world position"));
    /// assert!(!DebugChannel::Depth.description().contains("world position"));
    ///
    /// // Every image a render names comes back to the channel that drew it, and an image no
    /// // channel drew comes back to none — a file described as a world position because it
    /// // happened to be a float image is the defect this lookup exists to prevent.
    /// for channel in DebugChannel::ALL {
    ///     assert_eq!(DebugChannel::from_output(channel.output()), Some(channel));
    /// }
    /// assert_eq!(DebugChannel::from_output("normal"), None);
    ///
    /// // The grade channel is the same format as the other two and is not read out of the
    /// // depth buffer, so it is the one whose header must say what it copied rather than
    /// // leave it to be inferred from a channel that reads a different buffer.
    /// assert!(DebugChannel::Grade.description().contains("grade"));
    /// assert!(!DebugChannel::Grade.reads_depth());
    /// assert!(DebugChannel::Position.reads_depth() && DebugChannel::Depth.reads_depth());
    /// ```
    pub const fn description(self) -> &'static str {
        match self {
            Self::Position => POSITION_DESCRIPTION,
            Self::Depth => DEPTH_DESCRIPTION,
            Self::Grade => GRADE_DESCRIPTION,
        }
    }

    /// Returns the instance entry the pass's shader lives in.
    pub const fn shader_entry(self) -> &'static str {
        match self {
            Self::Position => "debug_position_shader",
            Self::Depth => "debug_depth_shader",
            Self::Grade => "debug_grade_shader",
        }
    }

    /// Builds the `outputs` entry this channel is declared under, in the shape a document
    /// spells.
    ///
    /// # Invariants
    ///
    /// The declaration is **written here rather than assembled by the emitter**, so that the
    /// point, the location and the type that go into the document are the same three facts the
    /// compiler used to build the pass. A caller reading the emitted document and a caller
    /// reading this module are reading the same declaration, which is the point of declaring
    /// it: the check in the device compares what this says against what this shader returns,
    /// and a type stated anywhere else would be a third copy.
    pub const fn declaration(self) -> graphics::OutputDeclaration<'static> {
        graphics::OutputDeclaration {
            point: self.point(),
            location: self.location(),
            ty: self.ty().name(),
        }
    }

    /// Returns the shader source the channel is drawn with.
    pub const fn source(self) -> &'static str {
        match self {
            Self::Position => POSITION_SOURCE,
            Self::Depth => DEPTH_SOURCE,
            Self::Grade => GRADE_SOURCE,
        }
    }

    /// Builds the interface of the channel's pass, which its own shader is checked against.
    ///
    /// # Invariants
    ///
    /// - **The binders are exactly the ones the channel's own source reads, and no more**,
    ///   because the binding numbers are derived from their **names**: adding a point renumbers
    ///   the shader's own declarations, so the list and the source in this module have to move
    ///   together. A `grade` pass binds the shaded image and writes its own target, and does
    ///   **not** bind the depth buffer — it copies a colour and never tests or recovers
    ///   anything, so a binder it cannot read would be a number in the layout with no meaning.
    /// - **The channel's point sorts before `depth_buffer`**, so the target takes binding one
    ///   as a hole and the depth buffer is read at two, which is what [`POSITION_SOURCE`] and
    ///   [`DEPTH_SOURCE`] declare. Both points begin `debug_`, so both sort before `depth_buffer`
    ///   and a second channel took the same numbers rather than moving the first one's.
    ///   [`GRADE_SOURCE`] reads `color_target` at one, which is where its own name sorts it.
    /// - **Every channel gets the same interface, including the uniform it does not read.** The
    ///   emitter hands each channel's pass the camera and the projection, so an interface that
    ///   declared less would describe a pass the compiler does not emit. A channel reads from
    ///   that uniform what it needs and ignores the rest; the alternative — an interface per
    ///   channel, matching the uniform the emitter writes — is one more place for the two to
    ///   disagree, and the emitter is not the thing this module gets to change per channel.
    pub fn pass_schema(self) -> Schema {
        let target = (
            self.point().to_owned(),
            texture_binding_schema().with_documentation(
                "The image this pass renders into. It is a target, so it takes a \
                 binding number with no entry behind it.",
            ),
        );
        let shaded = (
            COLOR_TARGET.to_owned(),
            texture_binding_schema().with_documentation(
                "The image the lighting and emission passes accumulated, which is the one \
                 the post stage's grade reads. Read with `textureLoad`, so it declares no \
                 sampler: this channel copies the pixels rather than filtering them.",
            ),
        );
        let depth = (
            DEPTH_BUFFER.to_owned(),
            texture_binding_schema().with_documentation(
                "The depth of the scene, which is what a pixel's position is recovered \
                 from. Read with `textureLoad`, so it declares no sampler.",
            ),
        );
        let binders = match self {
            Self::Grade => vec![shaded, target],
            Self::Position | Self::Depth => vec![target, depth],
        };
        interface(
            "A debug pass, which draws one image about the frame rather than of it.",
            binders
                .into_iter()
                .chain([(
                    VERTICES.to_owned(),
                    buffer_binding_schema()
                        .with_documentation("The fullscreen triangle this pass draws."),
                )])
                .collect(),
            vec![
                (
                    CAMERA.to_owned(),
                    camera_schema().with_documentation(
                        "The camera this pass draws through, which is what a ray through a \
                         pixel is recovered from.",
                    ),
                ),
                (
                    PROJECTION.to_owned(),
                    projection_schema().with_documentation(
                        "The camera's own projection, whose last two rows are what a depth \
                         becomes a position through. It is the **whole** view-projection and \
                         not a perspective's coefficients, so the recovery holds for an \
                         orthographic camera too.",
                    ),
                ),
            ],
        )
    }
}

/// The prose a file of [`DebugChannel::Position`] numbers carries above them.
///
/// # Invariants
///
/// It says the numbers are **points in the world** rather than distances or fractions, because
/// that is the one reading the format cannot supply and the one a caller has to be told: a
/// caller who assumes a distance gets a number that is finite, in range, and wrong.
const POSITION_DESCRIPTION: &str = "\
the world position of the nearest surface each pixel shows, as x, y, z in the scene's own
units. It is a point in the world rather than a distance from the camera, so it can be
compared against the scene's own coordinates directly. Nothing here is normalized to a range
and nothing has to be decoded to read one of the numbers.
alpha is the marker: 1 where the scene drew a surface, 0 where it drew none. The colour is
zero at a pixel with no surface, because a position of zero is a perfectly good place to be
and only the alpha can say that nothing is here at all.
the three ranges below are the smallest and the largest value each colour holds across the
pixels the scene drew, and they are the range the png beside this file was normalized by, so
that png is a picture of where the surfaces are rather than a set of positions to read.";

/// The prose a file of [`DebugChannel::Depth`] numbers carries above them.
///
/// # Invariants
///
/// It says the numbers are a **fraction of the camera's depth range** rather than a distance,
/// and it says so with the consequence rather than the definition: two pixels reading 0.98 and
/// 0.99 are not a hundredth of a unit apart, and the whole cost of reading them as though
/// they were is a number that looks measured. It also says what the colour holds at a pixel
/// with no surface, because the buffer's own value there is `1.0` and `1.0` read as a position
/// is a surface at the far plane.
const DEPTH_DESCRIPTION: &str = "\
the value the depth test compared against at each pixel, in zero to one, where 0 is the near
plane and 1 is the far plane. It is a fraction of the camera's own depth range and not a
distance in scene units, so two pixels reading 0.98 and 0.99 are nearer the far plane by a
fraction of the range rather than a hundredth of a unit apart. The colour is the depth buffer
verbatim, so a pixel the scene never reached holds 1.0 — which is what an empty depth buffer
is, and not a measurement of a surface.
alpha is the marker: 1 where the scene drew a surface, 0 where it drew none. It is the marker
`position` uses rather than a second convention, so a caller reading both images reads one
rule and a pixel is either reported by both of them or by neither.
the three ranges below are the smallest and the largest value each colour holds across the
pixels the scene drew, and they are the range the png beside this file was normalized by, so
that png is a picture of how the depths are laid out rather than a set of depths to read.";

/// The prose a file of [`DebugChannel::Grade`] numbers carries above them.
///
/// # Invariants
///
/// It answers the three questions a header has to answer, and then the one that decides
/// whether the numbers can be believed:
///
/// - **what they are** — the radiance the post stage's grade reads, at the pixel, before that
///   grade has run. It is the input of the transform and not its output, which is the whole
///   reason the channel exists: a picture and this file side by side are the two ends of one
///   transform, and a caller who has only ever had the picture has been reasoning about a
///   black box.
/// - **what the alpha means** — nothing, and that is said rather than left to be read as the
///   other two channels' marker. The lighting stage writes one into the shaded image and the
///   pipeline clears it to one, so it is one at a pixel nothing was drawn on as well.
/// - **what it does not cover** — two things, each of which is a way this file is not the
///   frame divided by something:
///
///   1. **The sky.** A post stage draws the sky, so a pixel of sky is black here however bright
///      the sky is in the frame. That is the single most useful thing this file says about a
///      landscape: a sky that is bright in the picture and black here was drawn by the grade.
///   2. **The pixels nothing was drawn on.** A black pixel here is unlit, which is a fact
///      about the light and not about the surface; the `depth` channel is the one that says
///      which pixels have a surface at all.
///
/// **The header says nothing above one any more, and that is a change.** It used to carry a
/// third item, that a value at one here might have been one or might have been forty, because
/// the shaded image was an eight bit target and the lighting stage had clipped it before the
/// grade saw it. `0257706` gave that image `rgba16float` and the claim became false, so it was
/// removed rather than reworded: **a header is a statement about the file the reader is holding,
/// not a changelog**, and one that explains what the numbers used to be is one a reader has to
/// decide how much of to believe. `crates/scene/tests/grade_channel.rs` asserts the claim that
/// survives — that these numbers are not bounded by one — and asserts the old words are gone.
const GRADE_DESCRIPTION: &str = "\
the radiance the post stage's grade reads at each pixel, before that grade has run: this is
the input of the transform, where the picture beside this file is its output. The three are
the same pixels, so a difference between the two is the grade and nothing else.
it is a value in scene-linear light and not a colour the scene chose, and it is not bounded by
one: the shaded image it copies is an rgba16float target, so a light above one arrives here
above one. A value at one is a light that reached one.
The sky is drawn by the post stage rather than by the lighting, so a pixel of sky is
black here however bright the sky is in the frame.
alpha marks nothing. The lighting stage writes one into the shaded image and the pipeline
clears it to one, so it is one at a pixel no surface was drawn on as well. The depth channel
is the one that says which pixels have a surface at all.
the three ranges below are the smallest and the largest value each colour holds across the
pixels of the frame, and they are the range the png beside this file was normalized by, so
that png is a picture of the light the scene made rather than a set of values to read.";

/// The prose a file of a **scene-declared** output's numbers carries above them.
///
/// # Invariants
///
/// - **It states the enumeration, and not a buffer.** Every built-in above is bounded by what
///   the pipeline has — the depth buffer, the shaded image — so its header names the buffer and
///   a scene with no depth can be told from a scene with a dark surface. A scene-declared
///   output is bounded by *which buckets the writing pass enumerated*, so there is no buffer
///   to name: the pass is either there or it is not, and a header that said "the depth buffer"
///   about a normal would be a file describing a buffer it has never seen.
/// - **It states the declared type and the location**, because those are the two facts a
///   caller has to check a number against and neither is in the bytes: a `vec3<f32>` and a
///   `vec4<f32>` are both four float texels in the same image, and a number at a location is
///   not a number at any other.
/// - **It says what the surplus channel is, and for a scene-declared image that is never a
///   marker.** A `vec3<f32>` declaration draws an image with a fourth channel in it and that
///   channel is padding; a `vec4<f32>` one has the four components the shader wrote and no fifth
///   to account for. Either way the fourth component marks nothing, and a header that borrowed
///   `Position`'s convention would be claiming a convention of the built-in registry about a
///   scene's own image. The ranges are therefore measured over every texel, and this header
///   must not read the same as a built-in's.
/// - **It does not say what the value means.** Only the scene's own shader knows that, and a
///   header that guessed would be the one lying. What it can say is that the numbers are
///   exactly what that shader wrote, which is the fact the check already guaranteed.
pub fn declared_description(name: &str, ty: &str, location: u32) -> String {
    format!(
        "the values the fragment output at location {location} of this document's writing pass \
         wrote, as a `{ty}` per texel. The document declares this image as `{name}` with that \
         type and that location, and the device checked the shader against both before drawing \
         it, so what is in the file is what the shader wrote and nothing was substituted for it.\n{surplus}\nthis is a scene-declared output rather than a built-in channel, so it covers **exactly what \
the writing pass enumerated** and not a frame resource: where a channel reads the depth buffer \
or the shaded image and covers what that buffer holds, and marks the pixels it drew nothing on \
in its fourth component, this covers the buckets the pass drew \
and says nothing about the rest of the frame. A pixel outside those buckets is not a value \
this shader produced.
nothing here is normalized and nothing has to be decoded to read one of the numbers. The \
format the image was drawn in is named in the first line above; it is what the declared type \
resolves to, not a choice the writing program made.
the three ranges below are the smallest and the largest value each colour holds across every texel \
of the frame, and they are the range the png beside this file was normalized by.",
        surplus = surplus_of(ty)
    )
}

/// Returns the sentence saying what the fourth channel of a scene-declared image is, for the
/// header that goes above its numbers.
///
/// **The two cases must not read the same**, because a reader who took a padding channel for a
/// marker would throw away every texel the pass did not draw, and for a `vec3<f32>` whose
/// padding happened to be low that is *every* texel, which is a file of zeros produced by a rule
/// rather than by a render.
///
/// **Whether the image is wider than the value is asked of the table and not of the spelling**,
/// so a row added beside the vocabulary cannot leave this file describing a surplus that is not
/// there — and a third case is a sentence of its own, because a one component image has no fourth
/// component to account for at all, and a header that talked about one would be this file
/// describing a component the image does not have.
fn surplus_of(ty: &str) -> &'static str {
    match TexelType::parse(ty) {
        Ok(texel) if texel.components() < texel.image_width() => PADDED_SURPLUS,
        Ok(texel) if texel.image_width() == 1 => SINGLE_SURPLUS,
        _ => EXACT_SURPLUS,
    }
}

/// The header's sentence for a declaration drawn in an image wider than its value.
const PADDED_SURPLUS: &str = "a `vec3<f32>` is drawn one component wider than three, and the fourth channel is \
**padding**: it carries no number, it marks nothing, and nothing in this document gives it a \
meaning. It is not a marker, because marking is a convention of the built-in images above, which \
are compiler code that fills its own fourth channel, and this file is the scene's own value. Do \
not read this image as though a fourth number were in it, and do not drop a texel because its \
fourth component is low. The ranges below are measured over every texel rather than over the \
ones a marker would have selected.\
";

/// The header's sentence for a declaration drawn in an image exactly as wide as its value.
const EXACT_SURPLUS: &str = "a `vec4<f32>` holds exactly the four components the shader wrote and there is no \
fifth to account for. Its fourth component is one of the declared values, whatever the scene's \
shader put there; it marks nothing, because marking is a convention of the built-in images \
rather than a thing a declaration can ask for.\
";

/// The header's sentence for a declaration drawn in a one component image.
const SINGLE_SURPLUS: &str = "a single `f32` is drawn as one number in a one component image, and there is no fourth \
component to account for and none that could mark anything: the image holds the one value the \
shader wrote and nothing else. Nothing here is normalized and nothing has to be decoded.\
";

/// The shader a position image is drawn with.
///
/// # Invariants
///
/// - **The recovery is written against rows two and three of the whole view-projection**, as
///   the post stage's fog recovery is: `z_clip = row_z · world` and `w = row_w · world`, so
///   `world = eye + ray * t` makes `depth = z_clip / w` one linear equation in `t`. Reading
///   `m[2][2]` and `m[3][2]` as a perspective's coefficients instead reads every distance as
///   the far plane, and does it only for a perspective camera, which is the failure that looks
///   like a bug in the camera.
/// - **The ray comes from the camera's basis and not from the matrix**, because a matrix
///   inverse is not a thing `WGSL` has. The emitter hands the basis over for exactly this.
/// - **A pixel no surface covers is none.** The depth buffer is cleared to one, so the test is
///   the cheap one the post stage already makes. A background pixel given a position would tell
///   a caller that the sky is six hundred units away, which is a wrong answer rather than a
///   missing one.
/// - **The value is the position and not a fraction of it.** The image is a float target, so
///   there is nothing to decode and no scale a caller has to be told.
pub const POSITION_SOURCE: &str = r#"
struct Camera {
    forward: vec3<f32>,
    half_extent: vec2<f32>,
    position: vec3<f32>,
    right: vec3<f32>,
    up: vec3<f32>,
};

struct Projection {
    position: vec3<f32>,
    view_projection: mat4x4<f32>,
};

struct Uniforms {
    camera: Camera,
    projection: Projection,
};

@group(0) @binding(0) var<uniform> uniforms: Uniforms;
@group(0) @binding(2) var depth_buffer: texture_depth_2d;

struct Vertex {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vertex_main(@location(0) position: vec3<f32>) -> Vertex {
    var out: Vertex;
    out.position = vec4<f32>(position.xy, 0.0, 1.0);
    // Clip space runs from minus one to one with y up; texture space runs from zero to one
    // with y down, so the two differ by a scale and a flip.
    out.uv = position.xy * vec2<f32>(0.5, -0.5) + vec2<f32>(0.5, 0.5);
    return out;
}

/// Returns the ray through a pixel of the frame.
///
/// A ray through a pixel is recovered from the camera's three axes and the half extent of the
/// frame, not from the projection matrix: recovering it from the matrix would mean inverting
/// it, and WGSL has no matrix inverse.
fn ray_through(uv: vec2<f32>) -> vec3<f32> {
    let offset = vec2<f32>(uv.x * 2.0 - 1.0, 1.0 - uv.y * 2.0);
    return normalize(
        uniforms.camera.forward
        + uniforms.camera.right * offset.x * uniforms.camera.half_extent.x
        + uniforms.camera.up * offset.y * uniforms.camera.half_extent.y,
    );
}

@fragment
fn fragment_main(in: Vertex) -> @location(0) vec4<f32> {
    let size = vec2<f32>(textureDimensions(depth_buffer));
    let pixel = vec2<i32>(in.uv * size);
    let depth = textureLoad(depth_buffer, pixel, 0);
    // A depth of one is where nothing was drawn, because the depth buffer is cleared to the
    // far plane. Alpha is what marks it: a position of zero is a perfectly good position, so
    // the marker cannot be the position itself.
    if (depth >= 1.0) {
        return vec4<f32>(0.0, 0.0, 0.0, 0.0);
    }

    let ray = ray_through(in.uv);
    let eye = uniforms.camera.position;
    let matrix = uniforms.projection.view_projection;
    let row_z = vec4<f32>(matrix[0][2], matrix[1][2], matrix[2][2], matrix[3][2]);
    let row_w = vec4<f32>(matrix[0][3], matrix[1][3], matrix[2][3], matrix[3][3]);
    let at_eye_z = dot(row_z.xyz, eye) + row_z.w;
    let at_eye_w = dot(row_w.xyz, eye) + row_w.w;
    let per_depth = dot(row_z.xyz - depth * row_w.xyz, ray);
    let t = (depth * at_eye_w - at_eye_z) / per_depth;
    return vec4<f32>(eye + ray * t, 1.0);
}
"#;

/// The shader a depth image is drawn with.
///
/// # Invariants
///
/// - **The image is the buffer, not a reading of it.** There is no ray, no camera and no
///   matrix in this shader, because there is nothing to interpret: a depth value the depth
///   test compared against is the answer to "why was this fragment kept", and re-deriving it
///   would put a second, possibly different, convention between the caller and the buffer.
/// - **`rgb` is the buffer verbatim, including at a pixel nothing was drawn on.** A cleared
///   depth buffer holds one, so one is what that pixel reports. Zeroing it would be the
///   opposite error to the one this module keeps refusing: it would report the *near* plane
///   where there is no surface at all, which reads as "something is right in front of the
///   camera" rather than as "nothing is here".
/// - **Alpha is the marker, and it is `Position`'s marker rather than a second convention.**
///   One is where the scene drew a surface, zero is where it drew none, and every channel in
///   this module marks the same way — so a caller reading two of them together reads one rule.
/// - **The uniform is declared and not read.** It is here because the pass carries one and the
///   device checks a shader's uniform struct against the pass's own layout; a source that
///   declared nothing would still pass that check, and would be a second thing to keep in step.
pub const DEPTH_SOURCE: &str = r#"
struct Camera {
    forward: vec3<f32>,
    half_extent: vec2<f32>,
    position: vec3<f32>,
    right: vec3<f32>,
    up: vec3<f32>,
};

struct Projection {
    position: vec3<f32>,
    view_projection: mat4x4<f32>,
};

struct Uniforms {
    camera: Camera,
    projection: Projection,
};

@group(0) @binding(0) var<uniform> uniforms: Uniforms;
@group(0) @binding(2) var depth_buffer: texture_depth_2d;

struct Vertex {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vertex_main(@location(0) position: vec3<f32>) -> Vertex {
    var out: Vertex;
    out.position = vec4<f32>(position.xy, 0.0, 1.0);
    // The same flip the position channel makes, for the same reason: clip space runs from
    // minus one to one with y up, texture space from zero to one with y down, and a channel
    // that flipped one way and not the other would be sampling a mirrored depth buffer.
    out.uv = position.xy * vec2<f32>(0.5, -0.5) + vec2<f32>(0.5, 0.5);
    return out;
}

@fragment
fn fragment_main(in: Vertex) -> @location(0) vec4<f32> {
    let size = vec2<f32>(textureDimensions(depth_buffer));
    let pixel = vec2<i32>(in.uv * size);
    let depth = textureLoad(depth_buffer, pixel, 0);
    // A depth of one is where nothing was drawn, because the depth buffer is cleared to the
    // far plane. The colour is one anyway: it is what the buffer holds, and reporting zero
    // would report the near plane. Alpha is what says whether there is a surface here.
    if (depth >= 1.0) {
        return vec4<f32>(1.0, 1.0, 1.0, 0.0);
    }
    return vec4<f32>(depth, depth, depth, 1.0);
}
"#;

/// The shader a grade image is drawn with.
///
/// # Invariants
///
/// - **The image is the buffer, not a reading of it.** There is no camera, no matrix and no
///   arithmetic: this shader copies four numbers from one texel to another, and any
///   interpretation it added would be a second, possibly different, convention between the
///   caller and the image the grade actually reads. It is the same rule [`DEPTH_SOURCE`] is
///   written under, and for the same reason.
/// - **`textureLoad` and not a sampler**, so the copy is exact. A filtered read of a full
///   screen triangle at its own resolution is the identity in exact arithmetic and a resample
///   in floating point, and a diagnostic that moved its own pixels by a rounding error would be
///   indistinguishable from a grade that did.
/// - **The y flip is the one the other two channels make**, for the same reason: clip space
///   runs from minus one to one with y up and texture space from zero to one with y down, and
///   a copy that flipped one way and not the other would show the frame upside down — which
///   reads as a scene rather than as a mistake.
/// - **The uniform is declared and not read**, for the reason [`DEPTH_SOURCE`] gives: the pass
///   carries one and the device checks a shader's uniform struct against the pass's own layout.
/// - **`color_target` is at binding one**, which is where its name sorts it among this pass's
///   own binders. The point it writes, `debug_grade`, sorts after it and takes two.
pub const GRADE_SOURCE: &str = r#"
struct Camera {
    forward: vec3<f32>,
    half_extent: vec2<f32>,
    position: vec3<f32>,
    right: vec3<f32>,
    up: vec3<f32>,
};

struct Projection {
    position: vec3<f32>,
    view_projection: mat4x4<f32>,
};

struct Uniforms {
    camera: Camera,
    projection: Projection,
};

@group(0) @binding(0) var<uniform> uniforms: Uniforms;
@group(0) @binding(1) var color_target: texture_2d<f32>;

struct Vertex {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vertex_main(@location(0) position: vec3<f32>) -> Vertex {
    var out: Vertex;
    out.position = vec4<f32>(position.xy, 0.0, 1.0);
    // The same flip the other channels make, for the same reason: a copy that sampled one row
    // off would be a channel reporting a picture that is not the one being diagnosed.
    out.uv = position.xy * vec2<f32>(0.5, -0.5) + vec2<f32>(0.5, 0.5);
    return out;
}

@fragment
fn fragment_main(in: Vertex) -> @location(0) vec4<f32> {
    let size = vec2<f32>(textureDimensions(color_target));
    let pixel = vec2<i32>(in.uv * size);
    // Verbatim, in all four components, at the pixel this fragment is. There is no test here
    // and no branch: what a pixel of the shaded image holds is the answer, including at a
    // pixel the scene never reached.
    return textureLoad(color_target, pixel, 0);
}
"#;
