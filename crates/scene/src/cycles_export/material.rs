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
//! socket at emit time or it does not exist.
//!
//! **`custom_data` is the only place a material's parameters are written**, so it is the only
//! place they are read from. `tint`, `metallic`, `roughness` and `emissive` each land in a
//! `principled_bsdf` socket and are folded into it; a field with no such socket is recorded
//! rather than dropped, because a tint that silently failed to apply is the same invisible-wrong-
//! image failure this module is arranged to prevent.
//!
//! **Everything that is written is written as a node attribute, not as a connection**, and that
//! is not a style choice: a socket whose name contains a space cannot be connected at all — see
//! [`UnsupportedField::MaterialImage`]. An emitter reaching for `<connect>` would fail on exactly
//! the fields whose names are the most descriptive.
//!
//! # `reflect` has a socket and is still not written
//!
//! `principled_bsdf` carries `Specular IOR Level`, which sounds like ours, and the mapping was
//! written and rendered before this line existed. It is wrong: ours is how much sky a surface
//! shows **at a grazing angle** and that socket is the reflectance it has at **normal incidence**,
//! with Fresnel supplying the rise towards grazing on its own. Measured on `landscape`, writing
//! it drops the correlation against the rasterised frame from `+0.6223` to `+0.5733` on the whole
//! frame and from `+0.6249` to `+0.5919` below the sky — the document's zero is not a dielectric,
//! and writing it takes away the specular the sun and the sky were contributing.
//!
//! so the field stays in the report, where its absence is visible, rather than being written and
//! read as honoured. That is the difference between an approximation and a mistake: both are
//! defensible, and only the measurement says which one this is.
//!
//! # Why the dielectric parameters are not read
//!
//! `ior` and `transmission_weight` are always written and are never read, because the document
//! has no field for either and never had. They are the dielectric defaults stated once in the
//! XML rather than left to whatever the node's own defaults happen to be, so a reader of the
//! document does not have to know them to know what was meant.

use super::LossyMode;
use super::error::{EmitError, EmitResult, UnsupportedField};
use super::lossy::LossyReport;
use super::read;
use super::writer::Writer;
use definition::Value;

/// The field a material's colour multiplier travels in.
///
/// The name is written by the gallery rather than declared here: `custom_data` is an open map
/// that the gallery populates and this reads, and nothing in the scene schema decides what may
/// be in it.
const TINT: &str = "tint";

/// The field a material's metalness travels in.
const METALLIC: &str = "metallic";

/// The field a material's roughness travels in.
const ROUGHNESS: &str = "roughness";

/// The field a material's own colour travels in, the one no light reaches.
const EMISSIVE: &str = "emissive";

/// The field a material's image travels under.
const IMAGE: &str = "image";

/// The roughness a material that names none is drawn with.
const DEFAULT_ROUGHNESS: f64 = 0.5;

/// The metalness a material that names none is drawn with.
const DEFAULT_METALLIC: f64 = 0.0;

/// The index of refraction a material that is not glass is drawn with.
const DEFAULT_IOR: f64 = 1.45;

/// The transmission weight a material that is not glass is drawn with.
const DEFAULT_TRANSMISSION: f64 = 0.0;

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
    /// The colour the surface emits on its own, when the document wrote one.
    pub emissive: Option<[f64; 3]>,
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
        let mut emissive = None;
        let mut roughness = DEFAULT_ROUGHNESS;
        let mut metallic = DEFAULT_METALLIC;
        if let Some(custom) = read::field(fields, "custom_data") {
            if let Some(custom_fields) = custom.as_field_map() {
                for (name, value) in custom_fields {
                    match name.as_str() {
                        TINT => tint = Some(read::triple(value, &format!("{context}'s tint"))?),
                        ROUGHNESS => roughness = parameter(value, ROUGHNESS, &context)?,
                        METALLIC => metallic = parameter(value, METALLIC, &context)?,
                        EMISSIVE => {
                            emissive = Some(read::triple(value, &format!("{context}'s emissive"))?)
                        }
                        // An image is not a number and is refused by name rather than folded, so
                        // that the reason a textured surface came out flat is the one thing in
                        // the report about it. The variant carries the reason.
                        IMAGE => match mode {
                            LossyMode::Reject => {
                                return Err(EmitError::Unsupported(
                                    UnsupportedField::MaterialImage {
                                        material: label.clone(),
                                    },
                                ));
                            }
                            LossyMode::Degrade => report.record(UnsupportedField::MaterialImage {
                                material: label.clone(),
                            }),
                        },
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
            roughness,
            metallic,
            ior: DEFAULT_IOR,
            transmission: DEFAULT_TRANSMISSION,
            emissive,
        });
    }
    Ok(out)
}

/// Writes one `<shader>` block.
///
/// The node is named after the material, and the connection to `output surface` is what
/// makes the graph a closure rather than an orphan: the loader pre-binds `output` to the
/// graph's output node, and socket names are matched case-insensitively.
///
/// a material declaring no `emissive` writes none, because the node's own default is then what
/// the document meant and restating it would claim a decision nobody made. `reflect` is a
/// separate decision and is not made here — see the module documentation for the measurement
/// that settled it.
pub fn write_material(writer: &mut Writer, material: &Material) {
    // A tint multiplies the base colour rather than replacing it, so a document that writes
    // only a tint gets white underneath and the tint as the result.
    let base = material.tint.unwrap_or([1.0, 1.0, 1.0]);
    let mut attributes: Vec<(&str, String)> = vec![
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
    if let Some(emissive) = material.emissive {
        attributes.push((
            "emission_color",
            Writer::color(emissive[0], emissive[1], emissive[2]),
        ));
        // The colour is not a light on its own: `Emission Strength` keeps the node's own default
        // of zero unless the document says otherwise, so a surface naming a colour and no
        // strength emits nothing.
    }
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

/// Reads one custom-data parameter, naming the material and the field when it is not a number.
///
/// # Invariants
///
/// A malformed parameter is an error rather than the default, because a field that arrived as
/// the wrong type was written by an author who expected it to be read, and answering with the
/// default produces a picture that is wrong without saying so.
fn parameter(value: &Value, name: &str, context: &str) -> EmitResult<f64> {
    read::number(value, &format!("{context}'s {name}"))
}
