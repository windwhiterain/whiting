//! A field whose value is an image rather than a number.
//!
//! # Why this is an attribute and not a kind
//!
//! A scene material's custom data is one slot: it holds a `tint`, a `roughness`, and — with
//! this — an image. Most of those values are numbers that are packed into the instance row
//! and read as vertex attributes. An image is not, so the compiler has to find it and bind it
//! instead of packing it, and to do that it has to know which fields are images **before** it
//! looks at any value.
//!
//! The schema is where that is already written down. It is the only place that says what a
//! field is before a document fills it in, and it is what the emitter reads to know which
//! fields to pack. So an image is a marker on a field, and the marker lives in the schema's
//! extension map — the same mechanism a point field uses to declare how many components it
//! has.
//!
//! It is deliberately **not** a new [`definition::SchemaKind`]. `definition` is a general
//! data representation that knows nothing about graphics, and a texture is a graphics
//! concept; adding it there would put one crate's domain into the crate that exists not to
//! have one.
//!
//! # What a field with the marker holds
//!
//! A **name**, not pixels. The pixels live once in a scene's list of images and a material
//! names the one it wants, so that two materials naming one image share the one texture. A
//! material that carried its own pixels could not share, and a scene that inlined them could
//! not keep them out of a document that is re-sent on every reload.

use definition::{Schema, SchemaKind, Value};

/// Extension on a field whose value names an image rather than carrying a number.
pub const IMAGE: &str = "image";

/// The spelling of an image field, and the only thing the marker holds.
///
/// A unit rather than a boolean: a marker whose presence is the whole statement cannot
/// disagree with itself, and there is no second reading of it to get wrong.
pub const MARKER: &str = "image";

/// Builds the schema of a field whose value names an image.
///
/// # Invariants
///
/// The field is a string, because what it holds is the name of an image in the scene that
/// declares it. It is optional, so a scene and a shader that predate images are unaffected:
/// a field that is absent is a material that draws no image, which is every material today.
#[must_use]
pub fn image_field() -> Schema {
    Schema::new(SchemaKind::String)
        .with_attributes(
            definition::Attributes::new().with_extension(IMAGE, Value::String(MARKER.to_owned())),
        )
        .optional()
        .with_documentation(
            "The label of an image this scene declares, which the compiler binds as a texture \
             for the shader to sample. Absent means the material draws no image.",
        )
}

/// Returns whether a field is one whose value names an image.
///
/// # Invariants
///
/// The marker's presence is the whole declaration, so this reads the extension map and not
/// the value under it. A field without the marker is a field the compiler packs.
#[must_use]
pub fn is_image(field: &Schema) -> bool {
    field.attributes.extensions.contains_key(IMAGE)
}
