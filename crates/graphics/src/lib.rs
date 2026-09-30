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
//!     instance_schema: <schema document>,
//! }
//! ```
//!
//! A pass carries declarations and never payloads. Its `uniform` is the data it uploads,
//! its `shader` names the instance entry holding the shader, and its `binders` name the
//! instance entries the rest of its data is in. The instance holds those payloads, and
//! the two halves are checked against each other when a pass is bound rather than when a
//! document is written, because each document is authored on its own.
//!
//! # Example
//!
//! ```
//! use definition::{Definition, DefinitionIdentifier, Registry, Schema, SchemaIdentifier, SchemaKind, Value};
//! use whiting_graphics::{
//!     BinderKind, PassKind, example_graphics, example_instance, graphics_schema, instance_schema,
//!     passes, resolve_binder, verify_pass_inputs, vertex_layout, GRAPHICS_SCHEMA, INSTANCE_SCHEMA,
//! };
//!
//! let mut registry = Registry::new();
//! registry.put_schema(SchemaIdentifier::new(GRAPHICS_SCHEMA), graphics_schema());
//! registry.put_schema(SchemaIdentifier::new(INSTANCE_SCHEMA), instance_schema());
//!
//! let graphics = example_graphics();
//! let instance = example_instance();
//! graphics_schema().validate(&graphics).expect("the example graphics validates");
//!
//! // The instance schema a graphics document implies is derived from its binders.
//! let derived = whiting_graphics::derived_instance_schema(&graphics).expect("the binders agree");
//! derived.validate(&instance).expect("the example instance satisfies its binders");
//!
//! let found = passes(&graphics).expect("the pass list reads");
//! assert_eq!(found.len(), 2);
//! assert_eq!(found[0].kind(), PassKind::Rasterize);
//! assert_eq!(found[0].label().expect("a label"), "backdrop");
//! assert_eq!(found[1].kind(), PassKind::Compute);
//!
//! // A pass names its entries; the instance supplies them, and the shader's own
//! // interface is what says the pass supplies the right data.
//! for pass in &found {
//!     verify_pass_inputs(pass, &instance).expect("the pass inputs match the shader interface");
//! }
//! let shader = resolve_binder(&instance, "shape").expect("the shader entry resolves");
//! assert_eq!(shader.kind(), Some(BinderKind::Shader));
//! assert!(resolve_binder(&instance, "absent").is_err());
//!
//! // A buffer carries the schema it was packed with, so the vertex layout is derived
//! // from a point schema rather than declared a second time.
//! let vertices = resolve_binder(&instance, "shape_vertices").expect("the vertex buffer resolves");
//! let packed = Schema::from_value(
//!     vertices.value().variant_fields().expect("a typed buffer").get("schema").expect("a schema"),
//! )
//! .expect("the packed schema reads");
//! let layout = vertex_layout(&packed).expect("a point schema has a layout");
//! assert_eq!(layout.stride_bytes, 12);
//! assert_eq!(layout.vertex_count(36), Some(3));
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

pub mod binders;
mod example;
mod inputs;
mod instance;
pub mod layout;
pub mod pass;
mod read;

pub use binders::{
    Binder, BinderKind, Declaration, binder_declaration_schema, binder_payload_schema,
    buffer_binding_schema, resolve_binder, resolve_for_pass, texture_binding_schema,
    uniform_binding_schema,
};
pub use example::{example_graphics, example_instance};
pub use inputs::{shader_interface, uniform_schema, verify_pass_inputs};
pub use instance::derived_instance_schema;
pub use layout::{VertexAttribute, VertexLayout, uniform_bytes, value_bytes, vertex_layout};
pub use pass::{
    BINDERS, CLEAR_COLOR, ENTRY_POINT, LABEL, SHADER, TARGETS, UNIFORM, WORKGROUP, pass_schema,
};
pub use read::{PassKind, PassView, passes};

/// Name of the schema a graphics document is verified against.
pub const GRAPHICS_SCHEMA: &str = "whiting.graphics";

/// Name of the schema an instance document is verified against.
pub const INSTANCE_SCHEMA: &str = "whiting.instance";

/// Field of the graphics root that holds the pass list.
pub const PASSES_FIELD: &str = "passes";

/// Field of the graphics root that holds the schema of the instance data.
pub const INSTANCE_SCHEMA_FIELD: &str = "instance_schema";

/// Field of the graphics root that names the definition its binders read from.
///
/// # Invariants
///
/// The same edge the spec names from the other side, as `GraphicsInstance.referencing_graphics`,
/// declared here because the instance document is a flat map of binder payloads: a dictionary
/// schema carries one value schema, so it has no room for a field of another kind. Declaring it
/// on the reader means the pair can be checked, since the reader is the document that knows
/// which definition its binders were written against.
pub const INSTANCE_DEFINITION_FIELD: &str = "instance_definition";

/// Builds the schema of a graphics instruction set document.
///
/// The `instance_schema` slot holds a schema document. A compiler may leave it permissive
/// and let an agent edit the instance freely, or derive it from the binders its passes
/// declare; see [`derived_instance_schema`].
pub fn graphics_schema() -> Schema {
    Schema::new(SchemaKind::structure([
        (
            PASSES_FIELD.to_owned(),
            Schema::new(SchemaKind::list(pass_schema()))
                .with_documentation("Ordered instruction list, executed in order."),
        ),
        (
            INSTANCE_SCHEMA_FIELD.to_owned(),
            Schema::document_schema()
                .with_documentation("Schema of the instance data, authored or derived."),
        ),
        (
            INSTANCE_DEFINITION_FIELD.to_owned(),
            Schema::new(SchemaKind::String)
                .optional()
                .with_documentation(
                    "Name of the definition the binders read from. A renderer refuses a graphics \
                 document that does not name one, so a document meant to be rendered sets it.",
                ),
        ),
    ]))
    .with_documentation("An instruction set of low level graphics computation.")
}

/// Builds the default permissive schema of instance data.
///
/// # Why this is permissive
///
/// The instance is authored rather than derived from the passes that read it, so that a
/// human or an agent can edit it freely: any entry is legal, so a new buffer needs no
/// schema change. The cost is that binding is resolved by name at render time rather than
/// checked up front, which is why [`resolve_binder`] reports a
/// [`GraphicsError::Binding`] rather than a validation failure.
pub fn instance_schema() -> Schema {
    Schema::new(SchemaKind::dictionary(Schema::new(SchemaKind::Any))).with_documentation(
        "Open instance data. Entries are bound to passes by name, so any entry is legal.",
    )
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
        reason: String,
    },
    #[error("`{binder_name}` is declared as {declared} but the instance entry is {found}")]
    BinderMismatch {
        binder_name: String,
        declared: &'static str,
        found: &'static str,
    },
    #[error("`{binder_name}` is read as {declared} bytes but the instance entry holds {found}")]
    BinderSize {
        binder_name: String,
        declared: i64,
        found: i64,
    },
    #[error("`{entry}` is bound as {first} by one pass and as {second} by another")]
    BinderConflict {
        entry: String,
        first: &'static str,
        second: &'static str,
    },
    #[error("pass `{label}` does not match the interface of shader `{shader}`: {reason}")]
    UnverifiedInputs {
        label: String,
        shader: String,
        reason: String,
    },
    #[error("the interface of shader `{shader}` used by pass `{label}` is not a schema: {reason}")]
    UnreadableSchema {
        label: String,
        shader: String,
        reason: String,
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
