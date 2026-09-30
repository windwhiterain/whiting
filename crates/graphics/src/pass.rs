//! The pass schema: one shared struct merged into the variants of the pass enum.
//!
//! # Why a shared struct merged into an enum
//!
//! A pass has two kinds of field: the ones every pass has, and the ones that make a
//! rasterize pass different from a compute pass. Writing the shared fields once and
//! merging them into each variant keeps a field such as `uniform` in a single place, so
//! adding one is a one line change that cannot be applied to one variant and forgotten
//! on the other.
//!
//! # Document shape
//!
//! ```text
//! Rasterize(
//!     label: "shadow",
//!     uniform: <data>,
//!     shader: "shadowing",
//!     binders: { "shadow_map": Resource(Texture(..)), .. },
//!     targets: [ "depth32float" ],
//!     clear_color: [ 1.0, 1.0, 1.0, 1.0 ],
//! )
//! Compute(
//!     label: "blur",
//!     uniform: <data>,
//!     shader: "blur",
//!     binders: { .. },
//!     workgroup: [ 8, 8, 1, 16, 16, 1 ],
//!     entry_point: "main",
//! )
//! ```

use definition::{Schema, SchemaKind, SchemaMap};

/// Name of a pass, used in errors and in rendered output.
pub const LABEL: &str = "label";

/// Field holding the uniform data of a pass.
pub const UNIFORM: &str = "uniform";

/// Field naming the instance entry that holds the shader of a pass.
pub const SHADER: &str = "shader";

/// Field holding the binder declarations of a pass, keyed by binding point.
pub const BINDERS: &str = "binders";

/// Field holding the colour target formats of a rasterize pass, in attachment order.
pub const TARGETS: &str = "targets";

/// Field holding the colour a rasterize pass clears to.
pub const CLEAR_COLOR: &str = "clear_color";

/// Field holding the workgroup size and then the dispatch count of a compute pass.
pub const WORKGROUP: &str = "workgroup";

/// Field holding the entry point a compute pass invokes.
pub const ENTRY_POINT: &str = "entry_point";

/// Builds the schema of a single pass.
pub fn pass_schema() -> Schema {
    Schema::new(SchemaKind::enumeration([
        (
            "Rasterize".to_owned(),
            variant(shared_fields(), [targets(), clear_color()]),
        ),
        (
            "Compute".to_owned(),
            variant(shared_fields(), [workgroup(), entry_point()]),
        ),
    ]))
    .with_documentation("One step of the instruction list, executed in order.")
}

/// The fields every pass carries.
///
/// A field belongs here when every pass has it. A field that only one kind of pass has
/// belongs in that variant instead.
fn shared_fields() -> SchemaMap {
    [
        (
            LABEL.to_owned(),
            Schema::new(SchemaKind::String)
                .with_documentation("Name used in errors and in rendered output."),
        ),
        (
            UNIFORM.to_owned(),
            Schema::new(SchemaKind::Any).with_documentation(
                "Uniform data, uploaded verbatim. Its shape is the shader's business.",
            ),
        ),
        (
            SHADER.to_owned(),
            Schema::new(SchemaKind::String)
                .with_documentation("Name of the instance entry holding this pass's shader."),
        ),
        (
            BINDERS.to_owned(),
            Schema::new(SchemaKind::dictionary(crate::binder_declaration_schema()))
                .with_documentation("What the pass binds, keyed by binding point."),
        ),
    ]
    .into_iter()
    .collect()
}

/// Builds the schema of a rasterize target list.
fn targets() -> (String, Schema) {
    (
        TARGETS.to_owned(),
        Schema::new(SchemaKind::list(Schema::new(SchemaKind::String)))
            .with_documentation("Colour target formats, in attachment order."),
    )
}

/// Builds the schema of a clear colour.
fn clear_color() -> (String, Schema) {
    (
        CLEAR_COLOR.to_owned(),
        Schema::new(SchemaKind::list(Schema::new(SchemaKind::Float)))
            .with_documentation("Red, green, blue and alpha, each in zero to one."),
    )
}

/// Builds the schema of a compute workgroup description.
fn workgroup() -> (String, Schema) {
    (
        WORKGROUP.to_owned(),
        Schema::new(SchemaKind::list(Schema::new(SchemaKind::Int)))
            .with_documentation("Workgroup size, then the dispatch count."),
    )
}

/// Builds the schema of a compute entry point name.
fn entry_point() -> (String, Schema) {
    (
        ENTRY_POINT.to_owned(),
        Schema::new(SchemaKind::String).with_documentation("Entry function invoked in the shader."),
    )
}

/// Merges the shared fields with the fields of one variant.
fn variant(shared: SchemaMap, own: impl IntoIterator<Item = (String, Schema)>) -> Schema {
    let mut fields = shared;
    fields.extend(own);
    Schema::new(SchemaKind::Struct { fields })
}
