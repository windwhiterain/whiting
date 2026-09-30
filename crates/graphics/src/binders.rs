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

use definition::{Path, Schema, SchemaKind, Value};

use crate::GraphicsError;

/// Field naming the instance entry a declaration binds.
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

/// Field holding a texture format name.
pub const FORMAT: &str = "format";

/// Field holding the source of a shader, resolved against the shader root.
pub const PATH: &str = "path";

/// Field holding the entry point a compute pass invokes.
pub const ENTRY_POINT: &str = "entry_point";

/// Field holding the interface a shader declares.
pub const SCHEMA: &str = "schema";

/// Field holding the field names a uniform payload is laid out as.
pub const LAYOUT: &str = "layout";

/// Which kind of payload an instance entry holds, or a declaration binds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinderKind {
    /// A buffer of bytes.
    Buffer,
    /// A texture.
    Texture,
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
            Self::Uniform => "Uniform",
            Self::Shader => "Shader",
        }
    }

    /// Returns the kind stored under a tag, or `None` for an unknown tag.
    pub fn from_tag(tag: &str) -> Option<Self> {
        match tag {
            "Buffer" => Some(Self::Buffer),
            "Texture" => Some(Self::Texture),
            "Uniform" => Some(Self::Uniform),
            "Shader" => Some(Self::Shader),
            _ => None,
        }
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
    [buffer_resource(), texture_resource()].into_iter()
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

/// Builds a texture declaration body, which names the entry and the format used.
fn texture_resource() -> (String, Schema) {
    (
        "Texture".to_owned(),
        Schema::new(SchemaKind::structure([
            (
                SOURCE.to_owned(),
                Schema::new(SchemaKind::String)
                    .with_documentation("Instance entry the texture is in."),
            ),
            (WIDTH.to_owned(), Schema::new(SchemaKind::Int)),
            (HEIGHT.to_owned(), Schema::new(SchemaKind::Int)),
            (FORMAT.to_owned(), Schema::new(SchemaKind::String)),
        ])),
    )
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
            Schema::new(SchemaKind::structure([
                (WIDTH.to_owned(), Schema::new(SchemaKind::Int)),
                (HEIGHT.to_owned(), Schema::new(SchemaKind::Int)),
                (FORMAT.to_owned(), Schema::new(SchemaKind::String)),
            ])),
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
                    PATH.to_owned(),
                    Schema::new(SchemaKind::String)
                        .with_documentation("Source, resolved against the shader root."),
                ),
                (ENTRY_POINT.to_owned(), Schema::new(SchemaKind::String)),
                (
                    SCHEMA.to_owned(),
                    Schema::document_schema().optional().with_documentation(
                        "The interface this shader reads, checked against the pass.",
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
    let Some(entries) = instance.as_dict() else {
        return Err(GraphicsError::Binding {
            label: "<instance>".to_owned(),
            binder_name: binder_name.to_owned(),
            reason: "the instance is not a dict".to_owned(),
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
