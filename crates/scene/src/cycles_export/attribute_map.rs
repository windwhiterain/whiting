//! Which of a mesh's point attributes Cycles will read, and what happens to the rest.
//!
//! # The hard limit
//!
//! `cycles_xml.cpp` reads exactly five vertex attributes, each behind an explicit
//! `xml_read_float*_array` call with a hardcoded attribute name, and its own comment says
//! `we don't yet support arbitrary attributes`. There is no loop over a declared set, no
//! name-to-attribute mapping, and no escape hatch. A field an author named themselves
//! cannot reach the renderer, whatever it is called.
//!
//! Cycles' renderer itself is not the limit. `AttributeNode` is registered as `attribute`
//! in `shader_nodes.cpp` and can read any mesh attribute by name. The renderer would accept
//! a generic attribute if the file format could carry one; the file format cannot.
//!
//! So this table is a statement about one loader at one commit, not about Cycles. When the
//! pin moves, re-read that function and extend the table if the loader grew.

use super::LossyMode;
use super::error::UnsupportedField;
use super::lossy::LossyReport;

/// The attribute name a Cycles XML `normal` array is written under.
///
/// The loader calls `Attribute::standard_name(ATTR_STD_VERTEX_NORMAL)` to get this string
/// rather than hardcoding it, so its literal is a property of the kernel headers rather than
/// of the loader. It is kept here as one name rather than written at each use so that a
/// change in the kernel is a one-line change.
///
/// **It is `N`, and it is a one-character difference that silently did nothing for the whole
/// life of this backend.** the enumeration is called `ATTR_STD_VERTEX_NORMAL` and the string
/// it maps to is not its own name: `scene/attribute.cpp:460-462` reads
/// `case ATTR_STD_VERTEX_NORMAL: case ATTR_STD_CORNER_NORMAL: return "N";`. this module wrote
/// `standard_vertex_normal` — the enum's name rather than the wire's — so the loader never
/// found the array, and **a missing attribute is not an error**. measured: writing the array,
/// deleting it, and replacing it with per-face normals all render byte-identical, which is
/// what a name that matches nothing looks like from the outside.
///
/// UV escapes the same trap only because the loader accepts two spellings for it
/// (`cycles_xml.cpp:482-483` reads `UV` *or* `Attribute::standard_name(ATTR_STD_UV)`). the
/// normal array has no alias.
pub const CYCLES_VERTEX_NORMAL: &str = "N";

/// The attribute name for UV coordinates.
///
/// The loader accepts a short `UV` alias as well, and the short form is what this emitter
/// writes because it is a literal in the source rather than a computed name.
pub const CYCLES_UV: &str = "UV";

/// Decides what to do about a `point_schema` field Cycles has no attribute for.
///
/// # Invariant
///
/// In [`LossyMode::Reject`] this returns `Err` for every field not in the table, and the
/// emitter stops at the first one. In [`LossyMode::Degrade`] it records the field and
/// returns `Ok`, so the caller keeps going and remembers.
///
/// The two are not the same code path with a flag read at the end: reject has to stop, and
/// degrade has to keep going and record. Splitting them here is what keeps that difference
/// from being re-derived at every call site.
pub fn handle_unsupported_point_attribute(
    field: &str,
    mesh: &str,
    mode: LossyMode,
    report: &mut LossyReport,
) -> super::error::EmitResult<()> {
    let unsupported = UnsupportedField::PointAttribute {
        field: field.to_owned(),
        mesh: mesh.to_owned(),
    };
    match mode {
        LossyMode::Reject => Err(super::error::EmitError::Unsupported(unsupported)),
        LossyMode::Degrade => {
            report.record(unsupported);
            Ok(())
        }
    }
}

/// The field names a point's texture coordinate may be written under, in the order they are
/// tried.
///
/// This is **one** list rather than the two the emitter used to keep. The readers that pull UVs
/// out of a point and the table that decides whether the field is carryable are answering the
/// same question — "is this name a texcoord?" — and they disagreed: `is_mappable` omitted
/// `texture`, which is the name this workspace's own point schema uses. The UVs were read and
/// written correctly the whole time and the loss report claimed otherwise, on every mesh of
/// every scene, naming a field the format carries perfectly well.
///
/// `texture` is first because that is what the workspace's own point schema calls it
/// (`kit::shaded_point_schema`); the rest are the spellings a document written by hand is likely
/// to use, since nothing in the workspace writes those.
pub const UV_FIELDS: &[&str] = &["texture", "uv", "texcoord", "uv_map"];

/// Whether `field` is one of the point attributes the loader reads.
///
/// `position` is handled by the geometry writer rather than here, because the loader reads
/// it from the `P` attribute and the mesh is invalid without it.
pub fn is_mappable(field: &str) -> bool {
    matches!(field, "position" | "normal") || UV_FIELDS.contains(&field)
}
