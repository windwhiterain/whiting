//! The `WGSL` attributes a scene's points and instance rows are read at.
//!
//! # Why the list is derived rather than written
//!
//! A shader reads a vertex at numbered locations, and which field is at which location is
//! decided by the schemas: a point's fields first, in the order the schema sorts them, and
//! the instance row's fields after, in the order it sorts them too. Writing the list by
//! hand means transcribing that decision, and a transcription that is one field out is a
//! shader that reads a normal where a position is and draws nothing at all — or, worse,
//! draws something, wrongly.
//!
//! Nothing checks it. The uniform of a shader is checked against the interface it
//! declares, and a vertex layout is not: the device derives offsets from the same schemas
//! but a shader that names the wrong attribute at a right location is not a failure any
//! layer can see. So the list is written once, here, from the schemas themselves.
//!
//! # Invariants
//!
//! - **A field of more than four components is four attributes of four.** That is what a
//!   backend reads at one location, and it is why a matrix arrives as four consecutive
//!   names rather than as one.
//! - **A field of sixteen components is a matrix, and its attributes are named
//!   `<field>0` to `<field>3`.** A shader assembles one with `mat4x4<f32>(model0, model1,
//!   model2, model3)`, and a convention the generator and the shader both follow is what
//!   lets a shader written against a row of any size read its matrices.
//! - **The point comes before the row.** A vertex buffer holds points and an instance
//!   buffer holds rows, and a backend reads the two in that order, so the row continues
//!   from wherever the point's attributes ended.

use definition::{Schema, SchemaKind};

use graphics::layout::{MAX_ATTRIBUTE_COMPONENTS, components};

use crate::gallery::kit::{gallery_custom_schema, gallery_instance_schema};

/// Returns the attributes a gallery material's points and row are read at.
///
/// The row is the gallery's own instance schema merged with the gallery's own custom
/// schema, which are the two a gallery shader declares, so the list cannot disagree with
/// the declarations a scene hands to [`crate::gallery::kit::shader`].
pub fn gallery_vertex_inputs(point: &Schema) -> String {
    vertex_inputs(point, &gallery_row_schema())
}

/// Returns the schema one gallery material's instance row is packed with.
///
/// # Invariants
///
/// The row is the two schemas a gallery material declares, merged. The compiler merges the
/// same two when it packs the row, from the same pair, so the attributes a shader reads
/// are the fields the bytes carry.
fn gallery_row_schema() -> Schema {
    merge(&gallery_instance_schema(), &gallery_custom_schema())
}

/// Returns the attributes a point schema and a row schema lay out to.
pub fn vertex_inputs(point: &Schema, row: &Schema) -> String {
    let mut declared = String::new();
    let mut location = 0;
    for schema in [point, row] {
        let SchemaKind::Struct { fields } = &schema.kind else {
            continue;
        };
        for (name, field) in fields {
            let width = components(name, field).unwrap_or(1);
            // A field a backend reads as one attribute is named after itself; a field it
            // reads as several — which is every matrix — is named after itself and the
            // index of the attribute, because a shader assembles it from the four.
            if width <= MAX_ATTRIBUTE_COMPONENTS {
                declared.push_str(&format!(
                    "    @location({location}) {name}: {},\n",
                    attribute_type(width)
                ));
                location += 1;
                continue;
            }
            for index in 0..width / MAX_ATTRIBUTE_COMPONENTS {
                declared.push_str(&format!(
                    "    @location({location}) {name}{index}: {},\n",
                    attribute_type(MAX_ATTRIBUTE_COMPONENTS)
                ));
                location += 1;
            }
        }
    }
    declared
}

/// Returns the two schemas of a row merged into the one the bytes are packed with.
fn merge(left: &Schema, right: &Schema) -> Schema {
    let (SchemaKind::Struct { fields: left }, SchemaKind::Struct { fields: right }) =
        (&left.kind, &right.kind)
    else {
        return left.clone();
    };
    let mut fields = left.clone();
    for (name, field) in right {
        fields.insert(name.clone(), field.clone());
    }
    Schema::new(SchemaKind::structure(fields))
}

/// Returns the `WGSL` type an attribute of this many components is read as.
fn attribute_type(components: usize) -> &'static str {
    match components {
        1 => "f32",
        2 => "vec2<f32>",
        3 => "vec3<f32>",
        _ => "vec4<f32>",
    }
}
