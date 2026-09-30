//! The `Graphics` and `GraphicsInstance` schemas, authored as data.
//!
//! This crate contains no domain types. A graphics instruction set is a pattern of
//! [`Value`] accepted by the [`Schema`] built here, never a `struct` or an `enum` in this
//! codebase. That is what lets an agent edit a graphics document, and lets a schema be
//! replaced, with no recompilation of this crate or of anything downstream of it.
//!
//! # Document shape
//!
//! ```text
//! {
//!     passes: [ Pass, ... ],
//!     instance_schema: Any,
//! }
//! ```
//!
//! A pass is an enum tagged `Rasterize` or `Compute`. A shader or a resource is named in
//! the pass and resolved against the instance dict at bind time; see [`resolve_binder`].
//!
//! # Example
//!
//! ```
//! use definition::{Definition, DefinitionIdentifier, Registry, SchemaIdentifier};
//! use whiting_graphics::{
//!     example_graphics, example_instance, graphics_schema, instance_schema, passes,
//!     resolve_binder, BinderKind, PassKind, GRAPHICS_SCHEMA, INSTANCE_SCHEMA,
//! };
//!
//! let mut registry = Registry::new();
//! registry.put_schema(SchemaIdentifier::new(GRAPHICS_SCHEMA), graphics_schema());
//! registry.put_schema(SchemaIdentifier::new(INSTANCE_SCHEMA), instance_schema());
//!
//! let graphics = example_graphics();
//! let instance = example_instance();
//! graphics_schema().validate(&graphics).expect("the example graphics validates");
//! instance_schema().validate(&instance).expect("the example instance validates");
//!
//! let found = passes(&graphics).expect("the pass list reads");
//! assert_eq!(found.len(), 2);
//! assert_eq!(found[0].kind(), PassKind::Rasterize);
//! assert_eq!(found[0].label().expect("a label"), "backdrop");
//! assert_eq!(found[1].kind(), PassKind::Compute);
//!
//! // Binders are internal: a pass names them, the instance supplies them.
//! let binder = resolve_binder(&instance, "shape_vertex").expect("the binder resolves");
//! assert_eq!(binder.kind(), Some(BinderKind::Shader));
//! assert!(resolve_binder(&instance, "absent").is_err());
//!
//! let identifier = DefinitionIdentifier::new("scene");
//! registry
//!     .insert_definition(Definition::new(
//!         identifier,
//!         SchemaIdentifier::new(GRAPHICS_SCHEMA),
//!         graphics,
//!     ))
//!     .expect("the definition is new");
//! ```

use definition::{Path, Schema, SchemaKind, Value};
use thiserror::Error;

mod binders;
mod example;
mod read;

pub use binders::{resolve_binder, resolve_for_pass, Binder, BinderKind};
pub use example::{example_graphics, example_instance};
pub use read::{passes, PassKind, PassView};

/// Name of the schema a graphics document is verified against.
pub const GRAPHICS_SCHEMA: &str = "whiting.graphics";

/// Name of the schema an instance document is verified against.
pub const INSTANCE_SCHEMA: &str = "whiting.instance";

/// Field of the graphics root that holds the pass list.
pub const PASSES_FIELD: &str = "passes";

/// Field of the graphics root that holds the authored instance schema.
pub const INSTANCE_SCHEMA_FIELD: &str = "instance_schema";

/// Builds the schema of a graphics instruction set document.
///
/// The `instance_schema` slot is `Any` on purpose. The instance schema is authored
/// independently of the binder names, so that an agent can read and edit it freely
/// without the graphics schema constraining it. See [`instance_schema`].
pub fn graphics_schema() -> Schema {
    Schema::new(SchemaKind::structure([
        (
            PASSES_FIELD.to_owned(),
            Schema::new(SchemaKind::list(pass_schema()))
                .with_documentation("Ordered instruction list, executed in order."),
        ),
        (
            INSTANCE_SCHEMA_FIELD.to_owned(),
            Schema::new(SchemaKind::Any)
                .with_documentation("Schema of the instance data, authored independently."),
        ),
    ]))
    .with_documentation("An instruction set of low level graphics computation.")
}

/// Builds the schema of a single pass.
pub fn pass_schema() -> Schema {
    Schema::new(SchemaKind::enumeration([
        (
            "Rasterize".to_owned(),
            Schema::new(SchemaKind::structure([
                (
                    "label".to_owned(),
                    Schema::new(SchemaKind::String)
                        .with_documentation("Name used in errors and in rendered output."),
                ),
                (
                    "vertex_shader".to_owned(),
                    Schema::new(SchemaKind::String)
                        .with_documentation("Binder name resolving to a shader."),
                ),
                (
                    "fragment_shader".to_owned(),
                    Schema::new(SchemaKind::String)
                        .with_documentation("Binder name resolving to a shader."),
                ),
                (
                    "targets".to_owned(),
                    Schema::new(SchemaKind::list(Schema::new(SchemaKind::String)))
                        .with_documentation("Colour target formats, in attachment order."),
                ),
                (
                    "clear_color".to_owned(),
                    Schema::new(SchemaKind::list(Schema::new(SchemaKind::Float)))
                        .with_documentation("Red, green, blue and alpha, each in zero to one."),
                ),
            ]))
            .with_documentation("Draws geometry into the bound targets."),
        ),
        (
            "Compute".to_owned(),
            Schema::new(SchemaKind::structure([
                (
                    "label".to_owned(),
                    Schema::new(SchemaKind::String)
                        .with_documentation("Name used in errors."),
                ),
                (
                    "shader".to_owned(),
                    Schema::new(SchemaKind::String)
                        .with_documentation("Binder name resolving to a shader."),
                ),
                (
                    "entry_point".to_owned(),
                    Schema::new(SchemaKind::String)
                        .with_documentation("Entry function invoked in the shader."),
                ),
                (
                    "workgroup".to_owned(),
                    Schema::new(SchemaKind::list(Schema::new(SchemaKind::Int)))
                        .with_documentation("Workgroup size, then the dispatch count."),
                ),
            ]))
            .with_documentation("Runs a compute shader over the bound resources."),
        ),
    ]))
}

/// Builds the default permissive schema of instance data.
///
/// # Why this is permissive
///
/// The instance schema is authored, not derived from the binder names a pass uses. That
/// keeps it easy for a human or an agent to read and edit: any entry is legal, so a new
/// buffer needs no schema change. The cost is that binding is resolved by name at render
/// time rather than checked up front, which is why [`resolve_binder`] reports a
/// [`GraphicsError::Binding`] rather than a validation failure.
pub fn instance_schema() -> Schema {
    Schema::new(SchemaKind::dictionary(Schema::new(SchemaKind::Any)))
        .with_documentation(
            "Open instance data. Entries are bound to passes by name, so any entry is legal.",
        )
}

/// Builds the schema of a bound instance entry, for documentation and for tooling that
/// wants to describe what a renderer expects to find under a binder name.
pub fn binder_schema() -> Schema {
    Schema::new(SchemaKind::enumeration([
        (
            "Resource".to_owned(),
            Schema::new(SchemaKind::structure([(
                "resource".to_owned(),
                Schema::new(SchemaKind::enumeration([
                    (
                        "Buffer".to_owned(),
                        Schema::new(SchemaKind::structure([
                            ("size_bytes".to_owned(), Schema::new(SchemaKind::Int)),
                            (
                                "usage".to_owned(),
                                Schema::new(SchemaKind::list(Schema::new(SchemaKind::String))),
                            ),
                        ])),
                    ),
                    (
                        "Texture".to_owned(),
                        Schema::new(SchemaKind::structure([
                            ("width".to_owned(), Schema::new(SchemaKind::Int)),
                            ("height".to_owned(), Schema::new(SchemaKind::Int)),
                            ("format".to_owned(), Schema::new(SchemaKind::String)),
                        ])),
                    ),
                ])),
            )])),
        ),
        (
            "Uniform".to_owned(),
            Schema::new(SchemaKind::structure([(
                "layout".to_owned(),
                Schema::new(SchemaKind::list(Schema::new(SchemaKind::String))),
            )])),
        ),
        (
            "Shader".to_owned(),
            Schema::new(SchemaKind::structure([
                ("path".to_owned(), Schema::new(SchemaKind::String)),
                ("entry_point".to_owned(), Schema::new(SchemaKind::String)),
            ])),
        ),
    ]))
    .with_documentation("A named instance entry that a pass binds.")
}

/// Something in a graphics or instance document could not be read.
#[derive(Debug, Error)]
pub enum GraphicsError {
    #[error("at {path}: `{field}` is not {expected}, found {found}")]
    Malformed {
        path: Path,
        field: String,
        expected: &'static str,
        found: &'static str,
    },
    #[error("pass {index} is neither `Rasterize` nor `Compute`, found `{tag}`")]
    UnknownPass { index: usize, tag: String },
    #[error("pass `{label}` cannot bind `{binder_name}`: {reason}")]
    Binding {
        label: String,
        binder_name: String,
        reason: &'static str,
    },
}

/// Reports a field that is not the shape the document requires.
pub(crate) fn malformed(
    path: &Path,
    field: &str,
    expected: &'static str,
    value: Option<&Value>,
) -> GraphicsError {
    GraphicsError::Malformed {
        path: path.clone(),
        field: field.to_owned(),
        expected,
        found: value.map_or("nothing", Value::kind_name),
    }
}
