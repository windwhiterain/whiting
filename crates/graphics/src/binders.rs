//! Binders: what a pass declares, and the instance entry a declaration resolves to.
//!
//! # Two halves of one binding
//!
//! A pass never carries instance data. It carries *declarations*: a binding point name
//! and the instance entry the data lives in, plus the size and format the pass needs.
//! The instance carries the *payloads*: the bytes, the dimensions, the shader source.
//! Splitting the two is what keeps a graphics document cheap to diff, because the
//! changing part of a reload is nearly always the uniform data and the declarations,
//! never the megabytes of vertex data.
//!
//! Because the two halves are authored independently, a declaration that names a
//! missing entry, or an entry of the wrong kind, is reported as a binding error rather
//! than a schema validation failure. Both documents validate against their own schemas
//! and still fail to agree with each other.
//!
//! # What a texture carries, and the sampler beside it
//!
//! A texture entry carries its own pixels and the sampler its reader filters them with, and
//! a pass declares neither separately. A binding carries one resource, so a sampled image is
//! two: the texture and the sampler that reads it. Rather than making every shader author
//! write a second declaration that can disagree with the first, the two are allocated
//! together and in a stated order — **a bound resource takes one binding number, and a
//! texture the document says is `sampled` takes the sampler that reads it at the next one**.
//! A shader therefore reads its texture at a binding it can count to, and reads its sampler
//! one above.
//!
//! The need is *declared rather than inferred*. A colour texture is not necessarily sampled:
//! the post stage reads the shaded image with `textureLoad`, at the coordinates of the pixel
//! it is writing, and wants no sampler at all. Format does not carry the distinction, so the
//! document states it, and a texture that says nothing gets no sampler and keeps the
//! numbering every existing document already has.
//!
//! The settings live in the payload rather than in the pass because they belong to the
//! image: two passes that bind one texture bind one sampler, and a texture's filtering is a
//! fact about the texture. A texture a pass renders *into* is an attachment and is never
//! bound, so its sampler does not exist and its settings are ignored.
//!
//! Because `sampled` is a property of the payload and not of the declaration,
//! [`crate::verify_pass_inputs`] cannot report a disagreement about it: a shader's declared
//! interface says it reads a texture, and carries no opinion on how it is read. The device
//! is where the two are compared, against the layout it actually built.
//!
//! # A texture array is a second kind, not a count on the first
//!
//! A texture may have **layers**, which are images of the same size stacked in one texture,
//! and a document that holds one says `TextureArray` where a document that holds a flat
//! image says `Texture`. The two are separate kinds rather than one kind with a count in
//! it, and the reason is that a shader has no way to read them the same way: a flat
//! texture is `texture_depth_2d` and an array is `texture_depth_2d_array`, two WGSL types,
//! and `textureLoad` on an array takes an index argument that a flat texture has nowhere
//! to put. A single variant with a number in it would accept a document that reads
//! `Texture` while meaning something no shader can name, which is a document nobody can
//! edit honestly and no reader could catch.
//!
//! This is the same argument the light kinds made: a reader *can* tell a flat texture from
//! a sixteen layer one, so the document says which it is rather than leaving the reader to
//! infer it from a field that might be absent.
//!
//! The two halves of an array's addressing live on opposite sides of the binding, because
//! they answer different questions. **How many** layers a texture has is a fact about the
//! image, so `layers` is a payload field and the texture is created with it. **Which**
//! layer a pass draws into is a fact about that pass, so `layer` is a declaration field: a
//! pool is one texture with many layers and one pass per light, and a payload naming the
//! layer would name one page for the whole image, which is the addressing a pool exists to
//! replace. A pass that only *reads* a pool names no layer, because a shader indexes the
//! layers itself with the page its own light record carries.

use definition::{Path, Schema, SchemaKind, Value};

use crate::GraphicsError;

/// Field naming where content comes from: the instance entry a declaration binds, and, in
/// a shader entry, the source itself. A shader carries its own text because the instruction
/// has to be able to travel to whoever runs it.
pub const SOURCE: &str = "source";

/// Field holding the resource of a `Resource` declaration.
pub const RESOURCE: &str = "resource";

/// Field holding the size in bytes a buffer carries or is declared to carry.
pub const SIZE_BYTES: &str = "size_bytes";

/// Field holding what a buffer may be used for.
pub const USAGE: &str = "usage";

/// Field holding the bytes of a buffer or a uniform payload.
pub const DATA: &str = "data";

/// Field holding how the points of a vertex buffer are read as primitives.
pub const TOPOLOGY: &str = "topology";

/// Field holding a texture width in pixels.
pub const WIDTH: &str = "width";

/// Field holding a texture height in pixels.
pub const HEIGHT: &str = "height";

/// Field holding how many layers a texture array has.
///
/// # Invariants
///
/// It is a required field of the `TextureArray` variant and is not a field of `Texture` at
/// all, so there is no default to agree on: a document that wants a pool says how deep it
/// is, and a document that wants a flat texture cannot say a count that would be ignored.
pub const LAYERS: &str = "layers";

/// Field holding which layer of a texture array a declaration draws into.
///
/// This is the *declaration's* field, and only the `TextureArray` variant carries it: a
/// flat texture has no layer to choose between. A pool is one texture with many layers and
/// one pass per light, so the pass says which of them it writes and every other pass that
/// binds the same entry still sees the whole array.
pub const LAYER: &str = "layer";

/// Field holding a texture format name.
pub const FORMAT: &str = "format";

/// Field holding how a texture is magnified and minified.
pub const FILTER: &str = "filter";

/// Field holding how a texture is addressed outside zero to one.
pub const ADDRESS: &str = "address";

/// Field holding how levels of a texture are blended between.
pub const MIP_FILTER: &str = "mip_filter";

/// Field whose *presence* declares that a texture is read with a sampler.
///
/// # Why presence rather than a boolean
///
/// The key carries no value that could contradict it, so there is one reading and not two.
/// A `Bool` field would have to be read for its value, which gives `sampled: false` a
/// meaning, and both candidate meanings are wrong for somebody: it would either duplicate
/// the absent case or name a third state. A unit key is present or it is not, which is the
/// distinction the binding model actually makes — see the module documentation.
pub const SAMPLED: &str = "sampled";

/// Field holding the interface a shader declares.
pub const SCHEMA: &str = "schema";

/// Field holding the field names a uniform payload is laid out as.
pub const LAYOUT: &str = "layout";

/// Which kind of payload an instance entry holds, or a declaration binds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinderKind {
    /// A buffer of bytes.
    Buffer,
    /// A texture of one layer.
    Texture,
    /// A texture of more than one layer.
    ///
    /// A separate kind rather than a [`Texture`] carrying a count, because a shader names
    /// the two with different types — `texture_depth_2d` and `texture_depth_2d_array` —
    /// and reads them differently, a `textureLoad` on an array taking an index a flat
    /// texture has nowhere to put. One kind with a number in it would let a document say
    /// `Texture` and mean something no shader can name, which is the failure a reader of
    /// the document could not see.
    TextureArray,
    /// A uniform block.
    Uniform,
    /// A shader, addressed by path.
    Shader,
}

impl BinderKind {
    /// Returns the variant tag this kind is stored under.
    pub const fn tag(self) -> &'static str {
        match self {
            Self::Buffer => "Buffer",
            Self::Texture => "Texture",
            Self::TextureArray => "TextureArray",
            Self::Uniform => "Uniform",
            Self::Shader => "Shader",
        }
    }

    /// Returns the kind stored under a tag, or `None` for an unknown tag.
    pub fn from_tag(tag: &str) -> Option<Self> {
        match tag {
            "Buffer" => Some(Self::Buffer),
            "Texture" => Some(Self::Texture),
            "TextureArray" => Some(Self::TextureArray),
            "Uniform" => Some(Self::Uniform),
            "Shader" => Some(Self::Shader),
            _ => None,
        }
    }

    /// Returns whether this kind is a texture, whichever shape it has.
    ///
    /// # Invariants
    ///
    /// This is the question "is this an image", as against "which image is this". A caller
    /// that only needs to read a size or a format asks it here rather than matching one
    /// variant, so that the day a third texture shape is added the callers that did not
    /// care are the ones that need no change.
    pub const fn is_texture(self) -> bool {
        matches!(self, Self::Texture | Self::TextureArray)
    }
}

/// Builds the schema of a binder declaration, which is what a pass carries.
///
/// A declaration names the instance entry its data is in, and the size or format the
/// pass needs of it. It never carries the data itself.
pub fn binder_declaration_schema() -> Schema {
    let mut variants = vec![("Resource".to_owned(), resource_field(all_resources()))];
    variants.push(("Uniform".to_owned(), uniform_field()));
    Schema::new(SchemaKind::enumeration(variants))
        .with_documentation("What a pass binds at one binding point.")
}

/// Builds the schema of a binding point that reads a buffer.
///
/// A shader interface that wants a specific resource at a specific point names it here,
/// which is a stronger statement than [`binder_declaration_schema`] and the reason the
/// per kind builders exist.
pub fn buffer_binding_schema() -> Schema {
    Schema::new(SchemaKind::enumeration([(
        "Resource".to_owned(),
        resource_field([buffer_resource()]),
    )]))
    .with_documentation("A binding point that reads a buffer.")
}

/// Builds the schema of a binding point that reads a texture.
pub fn texture_binding_schema() -> Schema {
    Schema::new(SchemaKind::enumeration([(
        "Resource".to_owned(),
        resource_field([texture_resource()]),
    )]))
    .with_documentation("A binding point that reads a texture.")
}

/// Builds the schema of a binding point that reads a uniform block.
pub fn uniform_binding_schema() -> Schema {
    Schema::new(SchemaKind::enumeration([(
        "Uniform".to_owned(),
        uniform_field(),
    )]))
}

/// Builds the `resource` field of a `Resource` declaration around its variants.
fn resource_field(variants: impl IntoIterator<Item = (String, Schema)>) -> Schema {
    Schema::new(SchemaKind::structure([(
        RESOURCE.to_owned(),
        Schema::new(SchemaKind::enumeration(variants)),
    )]))
}

/// Builds the body of a `Uniform` declaration.
fn uniform_field() -> Schema {
    Schema::new(SchemaKind::structure([(
        SOURCE.to_owned(),
        Schema::new(SchemaKind::String)
            .with_documentation("Instance entry the uniform block is in."),
    )]))
}

/// Every resource a declaration may name, as the variants of its `resource` field.
fn all_resources() -> impl Iterator<Item = (String, Schema)> {
    [
        buffer_resource(),
        texture_resource(),
        texture_array_resource(),
    ]
    .into_iter()
}

/// Builds a texture declaration body around the fields its own shape adds.
///
/// # Why one builder and not two bodies
///
/// A flat texture and a texture array name their entry, give their size and give their
/// format the same way, and the only differences are the array's layer count and the layer
/// a pass draws into. Written out twice, that shared half would be free to drift, and a
/// drift is a declaration that validates against a payload it cannot bind.
fn texture_declaration(
    tag: &str,
    own: impl IntoIterator<Item = (String, Schema)>,
) -> (String, Schema) {
    let mut fields = vec![
        (
            SOURCE.to_owned(),
            Schema::new(SchemaKind::String)
                .with_documentation("Instance entry the texture is in."),
        ),
        (WIDTH.to_owned(), Schema::new(SchemaKind::Int)),
        (HEIGHT.to_owned(), Schema::new(SchemaKind::Int)),
    ];
    // The shape's own fields sit between the size and the format, where a reader looks for
    // what makes this texture the shape it is.
    fields.extend(own);
    fields.push((FORMAT.to_owned(), Schema::new(SchemaKind::String)));
    (tag.to_owned(), Schema::new(SchemaKind::structure(fields)))
}

/// Builds the fields a texture array adds to what both shapes declare.
///
/// The count is required and states what it costs, because a reader choosing a pool depth
/// is choosing memory: the whole image is stored once per layer, so sixty four pages of a
/// thousand square depth texture cost sixty four times what one page does. The device caps
/// the count at what it can build, and refuses rather than clamps, so an author who asks
/// for more learns it at the document rather than in a frame.
fn texture_array_fields() -> [(String, Schema); 2] {
    [
        (
            LAYERS.to_owned(),
            Schema::new(SchemaKind::Int).with_documentation(
                "How many layers the array has, where a layer is one image of the same \
                 size. The cost is the whole image once per layer. A count above what the \
                 device can build is refused by name rather than clamped.",
            ),
        ),
        (
            LAYER.to_owned(),
            Schema::new(SchemaKind::Int).optional().with_documentation(
                "Which layer this pass draws into, counted from zero. Absent means the \
                 first. A pass that only reads the array names none, because a shader \
                 reads a layer by the index its own data carries.",
            ),
        ),
    ]
}

/// Builds the declaration of a texture of one layer.
fn texture_resource() -> (String, Schema) {
    texture_declaration("Texture", [])
}

/// Builds the declaration of a texture of more than one layer.
fn texture_array_resource() -> (String, Schema) {
    texture_declaration("TextureArray", texture_array_fields())
}

/// Builds a buffer declaration body, which names the entry and the size read.
fn buffer_resource() -> (String, Schema) {
    (
        "Buffer".to_owned(),
        Schema::new(SchemaKind::structure([
            (
                SOURCE.to_owned(),
                Schema::new(SchemaKind::String)
                    .with_documentation("Instance entry the bytes are in."),
            ),
            (
                SIZE_BYTES.to_owned(),
                Schema::new(SchemaKind::Int).with_documentation("Bytes the pass reads."),
            ),
        ])),
    )
}

/// How a sampled texture is filtered and addressed.
///
/// # Why this is a value and not a declaration
///
/// The settings belong to the image rather than to the pass that reads it: a texture is one
/// entry, two passes that bind it bind one sampler, and a filter is as much a fact about a
/// texture as its size. They are read from the payload the texture entry already owns, so a
/// shader author writes no second declaration that could disagree with the first.
///
/// The default is the one a tiling surface wants: magnified and minified linearly, blended
/// between mip levels, and repeating past its edge.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SamplerSettings {
    /// Whether magnification and minification interpolate between texels.
    pub linear_filter: bool,
    /// Whether a level is blended into the next, or one level is chosen.
    pub linear_mip_filter: bool,
    /// How a coordinate outside zero to one is folded back in.
    pub address: SamplerAddress,
}

/// How a sampled texture is addressed outside zero to one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SamplerAddress {
    /// The edge texel is repeated outwards.
    ClampToEdge,
    /// The image tiles, which is what a surface texture wants.
    Repeat,
    /// The image tiles, reflected at every edge.
    MirrorRepeat,
}

impl Default for SamplerSettings {
    fn default() -> Self {
        Self {
            linear_filter: true,
            linear_mip_filter: true,
            address: SamplerAddress::Repeat,
        }
    }
}

impl SamplerSettings {
    /// Reads settings out of a texture payload, defaulting what it does not declare.
    ///
    /// # Invariants
    ///
    /// An unrecognised tag selects the default rather than failing, because the payload is
    /// already validated against [`binder_payload_schema`] before it reaches here and a
    /// reader that refused a tag the schema admits would be a second, disagreeing schema.
    pub fn read(payload: &Value) -> Self {
        let fields = payload.variant_fields();
        let text = |name: &str| {
            fields
                .and_then(|fields| fields.get(name))
                .and_then(Value::as_str)
        };
        let mut settings = Self::default();
        if let Some(filter) = text(FILTER) {
            settings.linear_filter = filter != "Nearest";
            // A mip filter the payload does not name follows the filter, which is the
            // pairing a sampler means by `Linear`: both or neither.
            settings.linear_mip_filter = settings.linear_filter;
        }
        if let Some(mip) = text(MIP_FILTER) {
            settings.linear_mip_filter = mip == "Linear";
        }
        if let Some(address) = text(ADDRESS) {
            settings.address = match address {
                "ClampToEdge" => SamplerAddress::ClampToEdge,
                "MirrorRepeat" => SamplerAddress::MirrorRepeat,
                _ => SamplerAddress::Repeat,
            };
        }
        settings
    }
}

/// Returns whether a texture payload declares that it is read with a sampler.
///
/// # Invariants
///
/// The key's presence is the whole of the declaration, so this reads the field map and not
/// the value under it. A payload that is not a texture struct is not sampled, which matches
/// the schema: only a texture payload has the key, and both texture shapes declare it.
///
/// The question is the same for an array as for a flat texture, so a shader that samples a
/// pool of shadow pages still says so. What it may not do is *compare*: a depth format is
/// read with `textureLoad` in this project, and the device refuses a sampled depth texture
/// whatever its shape.
pub fn is_sampled(payload: &Value) -> bool {
    payload
        .variant_fields()
        .is_some_and(|fields| fields.contains_key(SAMPLED))
}

/// Returns how many layers a texture payload has, which only an array declares.
///
/// # Invariants
///
/// `None` is the flat texture and **not a defaulted count**. A `Texture` payload has no
/// layer field to read, and a `TextureArray` payload that somehow had none is a document
/// the schema already refuses, so a caller that needs a number asks
/// [`BinderKind::is_texture`] and reads one for a flat texture itself. Returning a one here
/// would put back the erasure the two kinds exist to remove: a reader could no longer tell
/// a flat texture from an array by asking.
pub fn layers_of(payload: &Value) -> Option<i64> {
    if BinderKind::from_tag(payload.variant_tag()?) != Some(BinderKind::TextureArray) {
        return None;
    }
    payload.variant_fields()?.get(LAYERS)?.as_int()
}

/// Returns whether a texture payload carries pixels to upload.
///
/// # Invariants
///
/// The presence of the bytes is the declaration, exactly as [`is_sampled`] reads its key.
/// A texture with none was rendered into rather than uploaded, which is also the texture
/// whose levels cannot be built: a chain generated at upload would be stale the moment a
/// pass wrote the base level.
pub fn has_data(payload: &Value) -> bool {
    payload
        .variant_fields()
        .is_some_and(|fields| fields.contains_key(DATA))
}

/// Builds the schema of how a texture is magnified and minified.
pub fn sampler_filter_schema() -> Schema {
    Schema::new(SchemaKind::enumeration([
        ("Nearest".to_owned(), Schema::new(SchemaKind::structure([]))),
        ("Linear".to_owned(), Schema::new(SchemaKind::structure([]))),
    ]))
    .with_documentation("Whether a sample is one texel or a blend of its neighbours.")
}

/// Builds the schema of how a texture is addressed outside zero to one.
pub fn sampler_address_schema() -> Schema {
    Schema::new(SchemaKind::enumeration([
        (
            "ClampToEdge".to_owned(),
            Schema::new(SchemaKind::structure([])),
        ),
        ("Repeat".to_owned(), Schema::new(SchemaKind::structure([]))),
        (
            "MirrorRepeat".to_owned(),
            Schema::new(SchemaKind::structure([])),
        ),
    ]))
    .with_documentation("How a coordinate outside zero to one is folded back in.")
}

/// Builds the schema of how the levels of a texture are blended between.
pub fn sampler_mip_filter_schema() -> Schema {
    Schema::new(SchemaKind::enumeration([
        ("Nearest".to_owned(), Schema::new(SchemaKind::structure([]))),
        ("Linear".to_owned(), Schema::new(SchemaKind::structure([]))),
    ]))
    .with_documentation("Whether two levels are blended or one is chosen.")
}

/// Builds the schema of how the points of a vertex buffer are read.
pub fn topology_schema() -> Schema {
    Schema::new(SchemaKind::enumeration([
        (
            "Triangles".to_owned(),
            Schema::new(SchemaKind::structure([])),
        ),
        (
            "Segments".to_owned(),
            Schema::new(SchemaKind::structure([])),
        ),
    ]))
    .with_documentation("How a packed run of points is read as primitives.")
}

/// Returns which topology a buffer declares, defaulting to triangles.
pub fn topology_of(payload: &Value) -> Option<&str> {
    payload.variant_fields()?.get(TOPOLOGY)?.variant_tag()
}

/// Builds the fields a texture payload has, beside the ones its own shape adds.
///
/// # Why one builder and not two bodies
///
/// The size, the format, the pixels and the four sampler settings mean exactly the same
/// thing to a flat texture and to an array, and the only difference between the two is the
/// layer count. Written out twice, that shared half would be free to drift, and a drift is
/// a payload that validates against a shape and then builds the other one.
fn texture_payload_fields(own: impl IntoIterator<Item = (String, Schema)>) -> Vec<(String, Schema)> {
    let mut fields = vec![
        (WIDTH.to_owned(), Schema::new(SchemaKind::Int)),
        (HEIGHT.to_owned(), Schema::new(SchemaKind::Int)),
    ];
    // The shape's own fields sit between the size and the format, where a reader looks for
    // what makes this texture the shape it is.
    fields.extend(own);
    fields.extend([
        (FORMAT.to_owned(), Schema::new(SchemaKind::String)),
        (
            DATA.to_owned(),
            Schema::new(SchemaKind::Bytes)
                .optional()
                .with_documentation(
                    "The pixels, one texel per four bytes, top row first. Absent on a \
                     texture a pass renders into, which has nothing to upload: a shadow \
                     map is written, never read from the instance.",
                ),
        ),
        (
            SAMPLED.to_owned(),
            Schema::new(SchemaKind::Unit).optional().with_documentation(
                "Present when the shader reads this texture with `textureSample`, which is \
                 what gives it a sampler at the binding after it. Absent means \
                 `textureLoad`, which is how a render target is read: the post stage reads \
                 the shaded image at the coordinates of the pixel it is writing and needs no \
                 sampler. The key is a unit because its presence is the whole declaration.",
            ),
        ),
        (
            FILTER.to_owned(),
            sampler_filter_schema().optional().with_documentation(
                "How the image is magnified and minified. Absent means `Linear`.",
            ),
        ),
        (
            ADDRESS.to_owned(),
            sampler_address_schema().optional().with_documentation(
                "How the image is addressed outside zero to one. Absent means `Repeat`, \
                 which is what a tiling surface wants; `ClampToEdge` is what a picture that \
                 must not wrap wants.",
            ),
        ),
        (
            MIP_FILTER.to_owned(),
            sampler_mip_filter_schema().optional().with_documentation(
                "How the levels of the image are blended between. Absent means the filter; \
                 `Nearest` selects one level rather than blending two, which is a hard edge \
                 at every level boundary.",
            ),
        ),
    ]);
    fields
}

/// Builds the schema of an instance entry, which is what a declaration resolves to.
pub fn binder_payload_schema() -> Schema {
    Schema::new(SchemaKind::enumeration([
        (
            "Buffer".to_owned(),
            Schema::new(SchemaKind::structure([
                (SIZE_BYTES.to_owned(), Schema::new(SchemaKind::Int)),
                (
                    USAGE.to_owned(),
                    Schema::new(SchemaKind::list(Schema::new(SchemaKind::String))),
                ),
                (DATA.to_owned(), Schema::new(SchemaKind::Bytes)),
                (
                    TOPOLOGY.to_owned(),
                    topology_schema().optional().with_documentation(
                        "How the points are read, for a vertex buffer. Absent means triangles.",
                    ),
                ),
                (
                    SCHEMA.to_owned(),
                    Schema::document_schema().optional().with_documentation(
                        "The schema the bytes were packed with. A renderer derives the \
                             vertex layout from it, rather than the layout being declared twice.",
                    ),
                ),
            ])),
        ),
        (
            "Texture".to_owned(),
            Schema::new(SchemaKind::structure(texture_payload_fields([]))),
        ),
        (
            "TextureArray".to_owned(),
            Schema::new(SchemaKind::structure(texture_payload_fields(
                // The array's payload declares how deep the array is, and not which layer a
                // pass writes: a payload is the image, and the image has no opinion about
                // who is drawing into it. So the shared array fields are filtered down to
                // the count rather than the count and the layer.
                texture_array_fields()
                    .into_iter()
                    .filter(|(name, _)| name != LAYER),
            ))),
        ),
        (
            "Uniform".to_owned(),
            Schema::new(SchemaKind::structure([
                (
                    LAYOUT.to_owned(),
                    Schema::new(SchemaKind::list(Schema::new(SchemaKind::String))),
                ),
                (DATA.to_owned(), Schema::new(SchemaKind::Bytes)),
            ])),
        ),
        (
            "Shader".to_owned(),
            Schema::new(SchemaKind::structure([
                (
                    SOURCE.to_owned(),
                    Schema::new(SchemaKind::String).with_documentation(
                        "The shader source. An instruction carries its own text so that it \
                         renders wherever it is sent, rather than naming a file the reader \
                         may not have.",
                    ),
                ),
                (
                    SCHEMA.to_owned(),
                    Schema::document_schema().optional().with_documentation(
                        "The interface this shader reads, checked against the pass. A \
                         rasterize shader exposes `vertex_main` and `fragment_main`, which \
                         is the backend's convention rather than data.",
                    ),
                ),
            ])),
        ),
    ]))
    .with_documentation("One entry of an instance document, which a pass binds by name.")
}

/// A borrowed view of one bound instance entry.
#[derive(Debug, Clone, Copy)]
pub struct Binder<'instance> {
    value: &'instance Value,
}

impl<'instance> Binder<'instance> {
    /// Borrows the bound value.
    pub const fn value(&self) -> &'instance Value {
        self.value
    }

    /// Returns which kind of payload this is, or `None` when the entry is not a typed
    /// binder document.
    pub fn kind(&self) -> Option<BinderKind> {
        BinderKind::from_tag(self.value.variant_tag()?)
    }

    /// Borrows a string field of a typed binder document.
    pub fn text(&self, field: &'static str) -> Option<&'instance str> {
        self.value.variant_fields()?.get(field)?.as_str()
    }

    /// Borrows the interface the bound shader declares, or `None` when it declares none.
    pub fn schema(&self) -> Option<Result<definition::Schema, definition::SchemaError>> {
        Some(definition::Schema::from_value(
            self.value.variant_fields()?.get(SCHEMA)?,
        ))
    }
}

/// A borrowed view of one binder declaration in a pass.
#[derive(Debug, Clone)]
pub struct Declaration<'pass> {
    pub(crate) path: Path,
    pub(crate) point: &'pass str,
    pub(crate) value: &'pass Value,
}

impl<'pass> Declaration<'pass> {
    /// Returns the binding point name this declaration is written under.
    pub const fn point(&self) -> &'pass str {
        self.point
    }

    /// Returns the path of this declaration, for diagnostics.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Borrows the declaration document.
    pub const fn value(&self) -> &'pass Value {
        self.value
    }

    /// Returns which kind of payload this declaration binds.
    pub fn kind(&self) -> Result<BinderKind, GraphicsError> {
        let kind = match self.value.variant_tag() {
            Some("Resource") => self
                .value
                .variant_fields()
                .and_then(|fields| fields.get(RESOURCE))
                .and_then(Value::variant_tag),
            Some("Uniform") => Some("Uniform"),
            _ => None,
        };
        kind.and_then(BinderKind::from_tag).ok_or_else(|| {
            crate::malformed(
                &self.path,
                RESOURCE,
                "a binder declaration",
                Some(self.value),
            )
        })
    }

    /// Returns the name of the instance entry this declaration binds.
    pub fn source(&self) -> Result<&'pass str, GraphicsError> {
        let fields = self.value.variant_fields();
        let source = match self.value.variant_tag() {
            Some("Resource") => fields
                .and_then(|fields| fields.get(RESOURCE))
                .and_then(Value::variant_fields)
                .and_then(|fields| fields.get(SOURCE)),
            Some("Uniform") => fields.and_then(|fields| fields.get(SOURCE)),
            _ => None,
        };
        match source.and_then(Value::as_str) {
            Some(name) => Ok(name),
            None => Err(crate::malformed(
                &self.path,
                SOURCE,
                "an instance entry name",
                source,
            )),
        }
    }

    /// Returns the size in bytes this declaration requires, for the kinds that have one.
    pub fn declared_size(&self) -> Option<i64> {
        if self.value.variant_tag() != Some("Resource") {
            return None;
        }
        self.value
            .variant_fields()?
            .get(RESOURCE)?
            .variant_fields()?
            .get(SIZE_BYTES)?
            .as_int()
    }

    /// Returns which layer of a texture array this declaration draws into, or `None` when
    /// it names none and so means the first.
    ///
    /// # Invariants
    ///
    /// Only a `TextureArray` declaration has a layer, and the count it would be checked
    /// against is not defaulted here: the two are fields of the same variant, so a
    /// declaration that named a layer of a flat texture would be a document the schema
    /// refuses. A pass that binds a texture without drawing into it names no layer either,
    /// which is every document written before the field existed.
    pub fn layer(&self) -> Option<i64> {
        if self.value.variant_tag() != Some("Resource") {
            return None;
        }
        let resource = self.value.variant_fields()?.get(RESOURCE)?;
        if BinderKind::from_tag(resource.variant_tag()?) != Some(BinderKind::TextureArray) {
            return None;
        }
        resource.variant_fields()?.get(LAYER)?.as_int()
    }

    /// Resolves this declaration against an instance document.
    ///
    /// The declared kind and the kind of the entry it names must agree, and a payload
    /// must be at least as large as the declaration asks for. A declaration that says
    /// `Buffer` over a `Texture` entry, or that reads past the end of a buffer, would
    /// otherwise bind successfully and produce nonsense, so both are refused here.
    pub fn bind<'instance>(
        &self,
        instance: &'instance Value,
    ) -> Result<Binder<'instance>, GraphicsError> {
        let declared = self.kind()?;
        let source = self.source()?;
        let binder = resolve_binder(instance, source)?;
        let found = binder.kind();
        if found != Some(declared) {
            return Err(GraphicsError::BinderMismatch {
                binder_name: source.to_owned(),
                declared: declared.tag(),
                found: found.map_or("nothing", BinderKind::tag),
            });
        }
        if let Some(needed) = self.declared_size() {
            let available = binder
                .value()
                .variant_fields()
                .and_then(|fields| fields.get(SIZE_BYTES))
                .and_then(Value::as_int);
            if available.is_some_and(|available| available < needed) {
                return Err(GraphicsError::BinderSize {
                    binder_name: source.to_owned(),
                    declared: needed,
                    found: available.unwrap_or_default(),
                });
            }
        }
        Ok(binder)
    }
}

/// Resolves a binder name against an instance document.
///
/// # Why this is not a validation error
///
/// The two documents are authored independently of each other, so a graphics document
/// and its instance can each validate against their own schema and still fail to bind.
/// That outcome is reported as [`GraphicsError::Binding`], which is deliberately a
/// distinct variant from [`GraphicsError::Malformed`].
pub fn resolve_binder<'instance>(
    instance: &'instance Value,
    binder_name: &str,
) -> Result<Binder<'instance>, GraphicsError> {
    let entries = instance
        .as_field_map()
        .and_then(|fields| fields.get(crate::ENTRIES))
        .and_then(Value::as_dict);
    let Some(entries) = entries else {
        return Err(GraphicsError::Binding {
            label: "<instance>".to_owned(),
            binder_name: binder_name.to_owned(),
            reason: "the instance holds no entries".to_owned(),
        });
    };
    let Some(value) = entries.get(binder_name) else {
        return Err(GraphicsError::Binding {
            label: "<instance>".to_owned(),
            binder_name: binder_name.to_owned(),
            reason: "no instance entry carries that name".to_owned(),
        });
    };
    Ok(Binder { value })
}

/// Resolves a binder name on behalf of a named pass, so the error names the pass.
pub fn resolve_for_pass<'instance>(
    pass_label: &str,
    instance: &'instance Value,
    binder_name: &str,
) -> Result<Binder<'instance>, GraphicsError> {
    resolve_binder(instance, binder_name).map_err(|error| match error {
        GraphicsError::Binding {
            binder_name,
            reason,
            ..
        } => GraphicsError::Binding {
            label: pass_label.to_owned(),
            binder_name,
            reason,
        },
        other => other,
    })
}
