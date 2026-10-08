//! Materials: our `MeshMaterial` as a Cycles `principled_bsdf`.
//!
//! # Why `principled_bsdf`
//!
//! It is registered in `shader_nodes.cpp` and was confirmed by rendering: two quads sharing
//! one node type and differing only in parameters came out as a matte red and a specular
//! gold. That matters because it removes the earlier suggestion that this emitter would
//! have to hand-assemble a Lambert-plus-GGX subset out of `diffuse_bsdf` and
//! `glossy_bsdf`, and a hand-assembled subset would have made our materials a dialect
//! rather than a mapping.
//!
//! `PrincipledBsdfNode` carries `base_color`, `metallic`, `roughness`, `ior` and
//! `transmission_weight`, which is very nearly the set our `MeshMaterial` names.
//!
//! # Where the custom data goes
//!
//! Cycles has no per-material data channel. A material's parameters are compiled into its
//! shader graph, so anything the document carries as `custom_data` has to be folded into a
//! socket at emit time or it does not exist. `tint` is the one field with a socket to land
//! in, and it folds multiplicatively into `base_color`, which is what a colour multiplier
//! means.
//!
//! Any other custom field has nowhere to go. That is recorded rather than dropped, because
//! a tint that silently failed to apply is the same invisible-wrong-image failure this
//! module is arranged to prevent.

use super::LossyMode;
use super::error::{EmitError, EmitResult, UnsupportedField};
use super::lossy::LossyReport;
use super::read;
use super::writer::Writer;
use definition::{FieldMap, Value};

/// One material, read out of the document.
pub struct Material {
    /// The name an instance refers to this material by.
    pub label: String,
    /// The label of the shader that draws it.
    pub shader: String,
    /// A colour multiplier in zero to one, when the document wrote one.
    pub tint: Option<[f64; 3]>,
    /// Roughness in zero to one.
    pub roughness: f64,
    /// Metalness in zero to one.
    pub metallic: f64,
    /// Index of refraction, for the dielectric case.
    pub ior: f64,
    /// Transmission weight in zero to one, for glass.
    pub transmission: f64,
}

/// Reads every material in a scene.
///
/// The `LightingShader` is the one that draws a lit surface. The other two mesh stages exist
/// to serve the rasterised pipeline's own passes, and Cycles derives shadow and depth rays
/// from its integrator, so a scene that declares only those is reported rather than emitted
/// as an empty frame.
pub fn read_materials(
    materials: &[Value],
    mode: LossyMode,
    report: &mut LossyReport,
) -> EmitResult<Vec<Material>> {
    let mut out = Vec::with_capacity(materials.len());
    for material in materials {
        let fields = material
            .as_field_map()
            .ok_or_else(|| EmitError::Malformed {
                detail: "a material is not a structure".to_owned(),
            })?;
        let label = read::text_field(fields, "label", "a material")?;
        let context = format!("material '{label}'");
        let shader = read::text_field(fields, "shader", &context)?;

        let mut tint = None;
        if let Some(custom) = read::field(fields, "custom_data") {
            if let Some(custom_fields) = custom.as_field_map() {
                for (name, value) in custom_fields {
                    match name.as_str() {
                        "tint" => tint = Some(read::triple(value, &format!("{context}'s tint"))?),
                        other => match mode {
                            LossyMode::Reject => {
                                return Err(EmitError::Unsupported(UnsupportedField::CustomData {
                                    field: other.to_owned(),
                                    material: label,
                                }));
                            }
                            LossyMode::Degrade => report.record(UnsupportedField::CustomData {
                                field: other.to_owned(),
                                material: label.clone(),
                            }),
                        },
                    }
                }
            }
        }

        out.push(Material {
            label,
            shader,
            tint,
            roughness: optional_number(fields, "roughness").unwrap_or(0.5),
            metallic: optional_number(fields, "metallic").unwrap_or(0.0),
            ior: optional_number(fields, "ior").unwrap_or(1.45),
            transmission: optional_number(fields, "transmission").unwrap_or(0.0),
        });
    }
    Ok(out)
}

/// Writes one `<shader>` block.
///
/// The node is named after the material, and the connection to `output surface` is what
/// makes the graph a closure rather than an orphan: the loader pre-binds `output` to the
/// graph's output node, and socket names are matched case-insensitively.
pub fn write_material(writer: &mut Writer, material: &Material) {
    // A tint multiplies the base colour rather than replacing it, so a document that writes
    // only a tint gets white underneath and the tint as the result.
    let base = material.tint.unwrap_or([1.0, 1.0, 1.0]);
    let attributes: Vec<(&str, String)> = vec![
        // The node is named, and the connection below refers to it by that name. The loader
        // resolves `from` and `to` against the node names inside the graph, so a node with
        // no name is unreachable however well formed its sockets are.
        ("name", format!("{}_bsdf", material.label)),
        ("base_color", Writer::color(base[0], base[1], base[2])),
        ("metallic", format!("{}", material.metallic)),
        ("roughness", format!("{}", material.roughness)),
        ("ior", format!("{}", material.ior)),
        ("transmission_weight", format!("{}", material.transmission)),
    ];
    writer.open("shader", &[("name", material.label.clone())]);
    writer.leaf("principled_bsdf", &attributes);
    writer.leaf(
        "connect",
        &[
            // The `from` half is `node_name socket_name`, and the node's name is the one
            // written above. A mismatch here is silent: the loader logs an unknown node and
            // leaves the graph unconnected, so the surface renders as the default closure
            // rather than as an error.
            ("from", format!("{}_bsdf bsdf", material.label)),
            ("to", "output surface".to_owned()),
        ],
    );
    writer.close("shader");
}

fn optional_number(fields: &FieldMap, name: &str) -> Option<f64> {
    read::field(fields, name).and_then(|value| read::number(value, name).ok())
}
