//! What a texel of a declared output holds, and which format can hold it.
//!
//! # Why a type and not a format
//!
//! A texture format is how the numbers are **stored**: eight bits a channel, a half float, a
//! depth value. A texel type is what the **value is**: three floats, four floats, one number.
//! The two are not the same question, and a document that answered the second by naming the
//! first would be answering "how wide" when a reader asked "of what" — which is the same mistake
//! [proposals/texture-channel.md](../../.agents/proposals/texture-channel.md) records from the
//! other direction, where a rule that *inferred* a sampler's need from a texture's format broke
//! six gallery scenes at once. So a declaration says the type, and the format is derived from
//! it through [`TexelType::format`], which is a table with a refusal rather than a guess.
//!
//! # The table's unit is a format this pipeline can create **and read back**
//!
//! A format a backend could hold is not enough, and the difference is not theoretical:
//! `r32uint` exists in `wgpu` and this device does not create it, and even if it did, the
//! readback decodes into four `f32` and a `u32` above 2^24 is not one. A type whose only format
//! fails either half therefore has **no row**, and a declaration naming it is refused by name
//! rather than producing an image nothing can read. That refusal is the whole of
//! [`TexelType::parse`]'s second error, and it is why an integer output is a message rather than
//! a picture of zeros.
//!
//! # The vocabulary is closed, and the closure is the contract
//!
//! Three spellings. A wider set would be easy to add and would be a promise this pipeline cannot
//! keep: every row is a claim that a texel of that type survives a round trip out of the gpu and
//! back into the four floats [`crate::Output`]'s reader returns.
//!
//! # Why three components is a row, and what its fourth channel is
//!
//! **A declaration is the abstract layer and a target's width is the compiler's business.** That
//! is the whole of it, and it is why `vec3<f32>` has a row: the scene says how many numbers its
//! value *is*, and how those numbers are laid out in a texture is a question the document never
//! asked and should not have to answer. A `vec3` normal is a normal, and a rule that refused it
//! would put a graphics-protocol detail into the middle of a scene's own code.
//!
//! So [`TexelType::format`] gives it a four channel row, and **the surplus channel is padding**:
//! it carries no number, marks nothing, and means nothing to a reader. A scene that wants a
//! fourth number declares `vec4<f32>`, and a scene that wants a *marker* has no vocabulary for it
//! here, because a marker is a reading convention rather than a value, and the one place in this
//! workspace that owns a marker — the built-in `Position` and `Depth` images — is compiler code
//! whose fourth channel it fills itself.
//!
//! # Two questions, and the declaration is not the thing both of them are asked against
//!
//! A declaration states how many numbers a texel **is**; [`TexelType::format`] then says which
//! image holds them, and that image is four components wide for two of the three spellings. Those
//! are two facts, they answer two different questions, and the device asks them separately —
//! see `check_outputs` in the device crate, which is where the comparison lives because it is a
//! fact about two documents rather than about one:
//!
//! - **Can the shader's fragment output fill the image the declaration resolves to?** That is
//!   [`TexelType::image_width`], and the fragment output has to be **exactly** that wide, because
//!   an image is filled one whole texel at a time: a `vec3<f32>` declaration resolves to a four
//!   component image, so the shader writes four. A narrower write is refused, because it leaves a
//!   number in every texel that the scene never wrote and a number nobody wrote reads as though it
//!   had been. A wider one is refused for the same reason, in the other direction.
//! - **How many components does the declaration claim carry meaning?** That is
//!   [`TexelType::components`], and it is never more than the image holds. The components from
//!   that count onward are **padding**: written, present in the file, and meaning nothing.
//!
//! So the padding case is the ordinary one — a `vec3<f32>` normal is drawn four wide and its
//! fourth channel is padding — and the question a scene author actually asks, *how many numbers
//! does this value have*, is answered by the declaration rather than by the drawing.

use definition::{Schema, SchemaKind, Value};

/// The three facts one entry of the `outputs` map states.
///
/// # Why a struct rather than three loose values
///
/// The document reads them back as a borrowed struct, and the emitter writes them as an owned
/// one, and both directions are the *same* three facts: a point is useless without a location
/// to read it at and a type to read it as. A helper that takes them together is what stops the
/// writer and the reader from disagreeing about the order or the spelling, and it is what lets
/// a built-in's declaration be written by the same code that built its pass.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OutputDeclaration<'document> {
    /// The binding point a pass renders the image into.
    pub point: &'document str,
    /// The `@location` of the writing shader's fragment output that carries it.
    pub location: u32,
    /// The type of a texel, spelled as [`TexelType`] spells it.
    pub ty: &'document str,
}

impl OutputDeclaration<'_> {
    /// Writes the declaration as a document value, in the order the schema names the fields.
    pub fn to_value(self) -> Value {
        Value::structure([
            (POINT_FIELD.to_owned(), Value::String(self.point.to_owned())),
            (
                LOCATION_FIELD.to_owned(),
                Value::Int(i64::from(self.location)),
            ),
            (TYPE_FIELD.to_owned(), Value::String(self.ty.to_owned())),
        ])
    }
}

/// Field of an output declaration naming the binding point the image is rendered into.
pub const POINT_FIELD: &str = "point";

/// Field of an output declaration naming the `@location` of the writing shader's fragment output
/// that carries it.
pub const LOCATION_FIELD: &str = "location";

/// Field of an output declaration naming the type of one texel, spelled as [`TexelType`] spells
/// it.
pub const TYPE_FIELD: &str = "type";

/// The type of one texel, in the vocabulary an `outputs` declaration spells it in.
///
/// # Invariants
///
/// Every variant has **exactly one** format that can hold it, and it is
/// [`TexelType::format`]. There is no second opinion and no fallback: a single float and a
/// vector of four are not the same texture, and a reader told "some float format" has been
/// told nothing it can size an image from.
///
/// **The set is as small as the round trip allows, and that is what makes it small.** A row
/// exists only for a type a fragment output can be *written as* **and** read back out of, and a
/// three component value qualifies because the surplus channel of a wider row is padding — the
/// module documentation says why, and [`TexelType::image_width`] is the half of the rule the
/// device's check enforces against a shader. A wider set would be easy to add and would be a
/// promise this pipeline cannot keep: every row is a claim that a texel of that type survives a
/// round trip out of the gpu and back into the four floats [`crate::Output`]'s reader returns.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TexelType {
    /// One float, spelled `f32`. The one case where the image is exactly as wide as the
    /// declaration, so there is no surplus and no padding.
    Float,
    /// Three floats, spelled `vec3<f32>`. What a surface normal is: three numbers, and a fourth
    /// component in the image it is drawn in that carries nothing.
    Vec3Float,
    /// Four floats, spelled `vec4<f32>`. What a colour is, what the frame is, and what a
    /// built-in image is written as.
    Vec4Float,
}

impl TexelType {
    /// Every type a declaration may spell, for a message about one that is not among them.
    pub const ALL: [Self; 3] = [Self::Float, Self::Vec3Float, Self::Vec4Float];

    /// Returns how a type is spelled in a document and in an error.
    pub const fn name(self) -> &'static str {
        match self {
            Self::Float => "f32",
            Self::Vec3Float => "vec3<f32>",
            Self::Vec4Float => "vec4<f32>",
        }
    }

    /// Returns how many components a texel of this type **claims to mean something**, which is
    /// what a reader of the image is entitled to find in it.
    ///
    /// It is a property of the **declaration** and never of the row below: a `vec3<f32>` holds
    /// three numbers however wide the image it is drawn in is, and the components from this count
    /// onward are padding rather than values. It is not what a fragment output is compared
    /// against — that is [`TexelType::image_width`], and the two are asked as two questions
    /// because they answer two different ones.
    pub const fn components(self) -> u8 {
        match self {
            Self::Float => 1,
            Self::Vec3Float => 3,
            Self::Vec4Float => 4,
        }
    }

    /// Returns how many components the image a texel of this type is drawn in holds, which is
    /// what a shader's fragment output is compared against.
    ///
    /// It is a property of the **row** and never of the declaration: `vec3<f32>` is three numbers
    /// and the image holding them is four numbers wide, and it is the image that is filled.
    ///
    /// # Invariants
    ///
    /// **No row claims more numbers than the image it resolves to holds**, and the surplus is
    /// padding. A row that broke this would be a declaration that could never be drawn, and the
    /// device's check refuses it in its own words rather than the table carrying the rule: see
    /// `check_outputs` in the device crate, which asks this against the fragment output and
    /// [`TexelType::components`] against the same number.
    pub const fn image_width(self) -> u8 {
        match self {
            Self::Float => 1,
            Self::Vec3Float | Self::Vec4Float => 4,
        }
    }

    /// Returns the format a texel of this type is drawn in.
    ///
    /// # Invariants
    ///
    /// **Every row is a format this device can create and read back**, and the row is the
    /// authority rather than a preference: a target is created from this for a built-in output,
    /// and a scene-declared one is refused when its type has no row at all. A row that named a
    /// format the readback did not know would produce an image that renders and cannot be read,
    /// which is the failure this table exists to make impossible.
    ///
    /// - `f32` is `r32float` because a single channel is a single channel: a four channel
    ///   target would put three numbers in the image that the shader never wrote, and a four
    ///   channel target is also the *only* one a `vec4` can be attached to.
    /// - **`vec3<f32>` is `rgba16float`, and the fourth channel is padding.** A three component
    ///   value is three numbers, and a document says how many numbers its value is rather than how
    ///   a texture lays them out — that division is the one this whole table rests on, and a row
    ///   that refused a `vec3` because of a rule about how a target is written would be a
    ///   protocol detail sitting in the middle of a scene's own code. The image is four wide, so
    ///   the shader writes **four** components; the fourth is padding by the same argument, and
    ///   the three the declaration names are the three a reader is entitled to. The channel is
    ///   **not** a marker, and nothing in a declaration can make it one: a marker is a reading
    ///   convention, and the built-in images that own one are compiler code that fills its own
    ///   fourth channel.
    /// - `vec4<f32>` is `rgba16float` because a position is three numbers an eight bit target
    ///   would quantise to a fraction of whatever the scene's extent happens to be, so the
    ///   image would answer *which region* and not *where*; and a depth value in zero to one
    ///   held to eight bits steps by four thousandths, which is enough to see **that** a
    ///   fragment lost a depth test and not enough to say how near it was. The cost of the
    ///   wider format is the size of the file, and the cost of the narrower one is a
    ///   measurement that lies about its own precision.
    /// - **A `grade` buys no precision at all, and says so.** The image it copies is itself an
    ///   eight bit target, so every value in it is already a multiple of a two hundred and
    ///   fifty fifth and a wider format cannot recover what the lighting stage clipped. What
    ///   the float target buys is the **sidecar**: a file of numbers beside the picture, which
    ///   is the instrument the comparison that channel exists for is actually made with. An
    ///   eight bit copy would be byte exact in the png and would have no numbers at all.
    /// - **A built-in image is a `vec4<f32>` while a scene's own normal is a `vec3<f32>`, and
    ///   the difference is a marker.** `Position` and `Depth` use their fourth channel to say
    ///   that no surface was drawn on a pixel, which is a fact only compiler code can know and
    ///   which therefore has to be paid for in a channel the scene never asked about. A scene
    ///   that wants three numbers declares three, and its fourth channel is padding. The two are
    ///   different kinds of image and they are allowed to say so: the type says how wide a value
    ///   is, and the built-in's own description says what its last channel means.
    pub const fn format(self) -> &'static str {
        match self {
            Self::Float => "r32float",
            Self::Vec3Float | Self::Vec4Float => "rgba16float",
        }
    }

    /// Returns the type a spelling names, or the reason it cannot be one.
    ///
    /// # Invariants
    ///
    /// **A real WGSL type this pipeline cannot round-trip is recognised and refused by name**
    /// rather than reported as a typo, because a reader told "not a type" goes looking for a
    /// misspelling and never learns that the value they wanted is real. A whole number and a
    /// half float are both such types and both have a format behind them, so they name the one
    /// that would have held them: the reader is then told that the plumbing is missing rather
    /// than that their type is wrong.
    ///
    /// A width the vocabulary does not carry — `vec2<f32>` is the only one — is **not** given a
    /// reason it does not have. It is a real type this pipeline's emitted passes do not write and
    /// this table makes no claim about, so it is answered as an unknown spelling, which is true:
    /// nothing here is broken, the vocabulary simply does not carry it. Adding it is one row.
    pub fn parse(spelled: &str) -> Result<Self, TexelError> {
        match spelled {
            "f32" => Ok(Self::Float),
            "vec3<f32>" => Ok(Self::Vec3Float),
            "vec4<f32>" => Ok(Self::Vec4Float),
            // A whole number is a real WGSL type with a real format behind it, and neither half
            // of the chain exists here: the format is not one `format_of` will create, and the
            // readback decodes to four `f32`. Naming the format it would have taken is what
            // turns "unsupported" into "the plumbing for this is not built".
            "u32" | "vec2<u32>" | "vec3<u32>" | "vec4<u32>" => Err(TexelError::Unreadable {
                spelled: spelled.to_owned(),
                format: "r32uint",
            }),
            "i32" | "vec2<i32>" | "vec3<i32>" | "vec4<i32>" => Err(TexelError::Unreadable {
                spelled: spelled.to_owned(),
                format: "r32sint",
            }),
            "f16" | "vec2<f16>" | "vec3<f16>" | "vec4<f16>" => Err(TexelError::Unreadable {
                spelled: spelled.to_owned(),
                format: "r16float",
            }),
            _ => Err(TexelError::Unknown {
                spelled: spelled.to_owned(),
            }),
        }
    }
}

/// A spelled type that is not one a declaration may carry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TexelError {
    /// A type a shader can declare, whose only holding format this pipeline cannot read back.
    Unreadable {
        /// The spelling the document used.
        spelled: String,
        /// The format that would have held it, and the half of it that is missing.
        format: &'static str,
    },
    /// A spelling nothing in this vocabulary answers to.
    Unknown {
        /// The spelling the document used.
        spelled: String,
    },
}

impl TexelError {
    /// Returns the whole reason, as a sentence a message can carry after its own facts.
    ///
    /// It is prose rather than a bare name because **the reason is the actionable part**. A
    /// reader told "not a supported type" edits the type; a reader told that `r32uint` cannot
    /// be read back on this device learns the value they wanted is real and the plumbing for it
    /// is not. Those are two different pieces of work, and one vague refusal would send both to
    /// the wrong place.
    ///
    /// **The one format a reader is told about is the one their own declaration resolves to**,
    /// because that is a fact about the document rather than about the protocol underneath it,
    /// and it is the difference between a message that can be acted on and one that sends a scene
    /// author to read a specification about a rule their scene never asked to be subject to.
    pub fn reason(&self) -> String {
        let offered = TexelType::ALL
            .iter()
            .map(|ty| format!("`{}`", ty.name()))
            .collect::<Vec<String>>()
            .join(", ");
        match self {
            Self::Unreadable { spelled, format } => format!(
                "`{spelled}` is a type a shader can declare, and `{format}` is a format that \
                 could hold it, but this device neither creates that format nor decodes a \
                 readback of it: a readback copies colour images, and a texel comes back as four \
                 `f32`, which a number above 2^24 is not one of. A declared `{spelled}` would \
                 therefore type-check and produce an image nothing can read, which is the \
                 outcome this table refuses rather than permits. The types a declaration may \
                 carry are {offered}."
            ),
            Self::Unknown { spelled } => format!(
                "`{spelled}` is not a type an output declaration may carry. The ones it may are \
                 {offered}, and each of them has exactly one format that can hold it. A \
                 declaration states how many numbers a texel holds rather than how an image lays \
                 them out, so a three component value is `vec3<f32>` and a four component one is \
                 `vec4<f32>`, and the two are told apart by the count rather than by the width \
                 of anything underneath them."
            ),
        }
    }
}

/// Builds the schema of one entry of the `outputs` map.
///
/// # Invariants
///
/// **All three fields are required**, and none of them has a default. A declaration that left
/// the location out would have it inferred, and inference is the one thing this shape exists to
/// stop: a location derived from a name's position in the map mis-binds the instant a name is
/// inserted or removed, and the failure is a wrong picture rather than an error. A declaration
/// that left the type out would be an image with nothing said about what is in it, which is the
/// state this field was added to close.
pub fn output_schema() -> Schema {
    Schema::new(SchemaKind::structure([
        (
            POINT_FIELD.to_owned(),
            Schema::new(SchemaKind::String).with_documentation(
                "The binding point of a pass's `targets` that this image is written into. It is \
                 where the image is rendered, which is not the same fact as what the image is.",
            ),
        ),
        (
            LOCATION_FIELD.to_owned(),
            Schema::new(SchemaKind::Int).with_documentation(
                "The `@location` of the writing shader's fragment output that carries this \
                 image. It is stated rather than inferred: a pass numbers its colour targets \
                 from zero in attachment order, and a location derived from where a name sits \
                 in this map would move every name below it the moment one was inserted. The \
                 device checks this against the pass's own attachment order, so a point and the \
                 location claiming it cannot drift apart.",
            ),
        ),
        (
            TYPE_FIELD.to_owned(),
            Schema::new(SchemaKind::String).with_documentation(
                "The type of one texel, spelled `f32`, `vec3<f32>` or `vec4<f32>`. It says how \
                 many numbers a texel holds, and the format the image is drawn in is derived from \
                 it, so a document says what the value is rather than how it is stored. That image \
                 is wider than the declaration whenever the type resolves to a row with more than \
                 one channel, and the fragment output fills it at the image's width, so a shader \
                 writes as many components as the image is wide; the components from the declared \
                 count onward are padding. A shader that writes fewer would leave numbers a reader \
                 would take for values the scene wrote, and the device refuses it, because only \
                 the device can see both documents at once. A type this pipeline cannot read back \
                 is refused by the device rather than accepted here, because the refusal has to \
                 name the shader's own variable and the location it would have written.",
            ),
        ),
    ]))
    .with_documentation(
        "One image a render produces: the point it is rendered into, the fragment output that \
         carries it, and the type of a texel of it.",
    )
}
