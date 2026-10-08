//! Meshes: points, indices, and the attributes the loader will read.
//!
//! # The encoding
//!
//! The loader takes three parallel arrays and reconstructs polygons from them:
//!
//! ```text
//! P      flat float3 per vertex
//! verts  flat index, consumed nverts[i] at a time
//! nverts vertex count of each polygon
//! ```
//!
//! Polygons are fan triangulated, so a triangle is `nverts="3"` and a quad is `nverts="4"`
//! with the same `verts`. This was confirmed by rendering: a scene holding two quads and a
//! triangle rendered all three.
//!
//! # The UV problem
//!
//! UVs are the one attribute that does not fit the pattern above, and getting it wrong is
//! silent. The loader reads `UV` into an `ATTR_STD_UV` and fills it **per triangle corner**,
//! indexing it by the corner's position in the fan rather than by vertex. A mesh of `n`
//! vertices therefore needs `3n` UV pairs, and they are consumed in triangle order.
//!
//! Our document stores UVs per vertex, like every other point attribute. Converting means
//! expanding per-vertex UVs into per-corner UVs using the same fan the loader will use. The
//! expansion is written out explicitly rather than delegated to the loader, because getting
//! it wrong produces a correctly-shaped mesh with a scrambled texture on it, and nothing
//! downstream reports it.

use super::LossyMode;
use super::attribute_map::{handle_unsupported_point_attribute, is_mappable};
use super::error::{EmitError, EmitResult, UnsupportedField};
use super::lossy::LossyReport;
use super::read;
use super::writer::Writer;
use definition::{FieldMap, Value};

/// One mesh's geometry, read out of the document.
pub struct Mesh {
    /// The name an instance refers to this mesh by.
    pub label: String,
    /// Flat `x y z` per vertex.
    pub positions: Vec<f64>,
    /// Flat `x y z` per vertex, when the document declared normals.
    pub normals: Option<Vec<f64>>,
    /// Flat `u v` per vertex, when the document declared coordinates.
    pub uvs: Option<Vec<f64>>,
    /// Indices into the vertex arrays, three per triangle.
    pub indices: Vec<i64>,
}

/// Reads every mesh in a scene.
///
/// A mesh whose `point_schema` names a field the loader cannot read is refused here in
/// [`LossyMode::Reject`], before any XML is written, so a rejected scene leaves no partial
/// file behind.
pub fn read_meshes(
    meshes: &[Value],
    mode: LossyMode,
    report: &mut LossyReport,
) -> EmitResult<Vec<Mesh>> {
    let mut out = Vec::with_capacity(meshes.len());
    for mesh in meshes {
        let fields = mesh.as_field_map().ok_or_else(|| EmitError::Malformed {
            detail: "a mesh is not a structure".to_owned(),
        })?;
        let label = read::text_field(fields, "label", "a mesh")?;

        // Topology is a variant, not a string: the document language gives it a tag so the
        // payload could grow without changing how the kind is named.
        let topology = fields
            .get("topology")
            .and_then(|value| value.variant_tag().map(str::to_owned))
            .or_else(|| {
                fields
                    .get("topology")
                    .and_then(|value| value.as_str().map(str::to_owned))
            })
            .ok_or_else(|| EmitError::Malformed {
                detail: format!("mesh '{label}' has no topology"),
            })?;
        if topology != "Triangles" {
            return Err(match mode {
                LossyMode::Reject => {
                    EmitError::Unsupported(UnsupportedField::Segments { mesh: label })
                }
                LossyMode::Degrade => {
                    report.record(UnsupportedField::Segments {
                        mesh: label.clone(),
                    });
                    continue;
                }
            });
        }

        let schema = read::field(fields, "point_schema");
        let (positions, normals, uvs) = read_points(&label, fields, schema, mode, report)?;
        let indices = read_indices(&label, fields, positions.len() / 3)?;

        out.push(Mesh {
            label,
            positions,
            normals,
            uvs,
            indices,
        });
    }
    Ok(out)
}

/// Reads a mesh's points, splitting them by which attribute the loader will read.
///
/// # Invariant
///
/// `positions.len() / 3` is the vertex count, and `normals` and `uvs` are either absent or
/// the same length. The loader indexes all three by the same vertex number, so a mismatch
/// would read past the end of the shorter array.
fn read_points(
    label: &str,
    fields: &FieldMap,
    schema: Option<&Value>,
    mode: LossyMode,
    report: &mut LossyReport,
) -> EmitResult<(Vec<f64>, Option<Vec<f64>>, Option<Vec<f64>>)> {
    let points = read::require(fields, "points", &format!("mesh '{label}'"))?;
    let elements = points.as_list().ok_or_else(|| EmitError::Malformed {
        detail: match points.as_packed() {
            // A compiled document carries points packed, against a layout this emitter does
            // not have. Saying so is better than reading the buffer as a tree and producing
            // geometry that looks fine and is wrong.
            Some(_) => format!(
                "mesh '{label}' carries packed points; emit the scene document, not a \
                 compiled one, or unpack it first"
            ),
            None => format!("mesh '{label}' has no point list"),
        },
    })?;
    if elements.is_empty() {
        return Err(EmitError::Malformed {
            detail: format!("mesh '{label}' has no points"),
        });
    }

    // The declared field names decide which arrays to read. A document that declares no
    // schema gets position alone, which is what `default_point_schema` says.
    let declared: Vec<String> = match schema.and_then(Value::as_field_map) {
        Some(schema) => schema.keys().cloned().collect(),
        None => vec!["position".to_owned()],
    };

    // Every declared field has to be accounted for, even the ones that are dropped: in
    // `Degrade` the point is to report them, and in `Reject` to stop. The mapping check
    // itself lives in `attribute_map` so that the table has exactly one reader.
    for name in &declared {
        if !is_mappable(name) {
            handle_unsupported_point_attribute(name, label, mode, report)?;
        }
    }

    let mut positions = Vec::with_capacity(elements.len() * 3);
    let mut normals = Vec::new();
    let mut uvs = Vec::new();
    let mut saw_normal = false;
    let mut saw_uv = false;

    for (index, point) in elements.iter().enumerate() {
        let point_fields = point.as_field_map().ok_or_else(|| EmitError::Malformed {
            detail: format!("mesh '{label}' point {index} is not a structure"),
        })?;
        let where_ = format!("mesh '{label}' point {index}");

        if !declared.iter().any(|name| name == "position") {
            return Err(EmitError::Malformed {
                detail: format!("{where_} has no 'position' to read"),
            });
        }
        let position = read::triple(
            read::require(point_fields, "position", &where_)?,
            &format!("{where_}'s position"),
        )?;
        positions.extend(position);

        if let Some(value) = read::field(point_fields, "normal") {
            if !saw_normal {
                normals.reserve(elements.len() * 3);
                saw_normal = true;
            }
            normals.extend(read::triple(value, &format!("{where_}'s normal"))?);
        }
        for name in ["uv", "texcoord", "uv_map"] {
            if let Some(value) = read::field(point_fields, name) {
                let pair = read::numbers(value, &format!("{where_}'s '{name}'"))?;
                if pair.len() != 2 {
                    return Err(EmitError::Malformed {
                        detail: format!(
                            "{where_}'s '{name}' has {} components, expected 2",
                            pair.len()
                        ),
                    });
                }
                if !saw_uv {
                    uvs.reserve(elements.len() * 2);
                    saw_uv = true;
                }
                uvs.extend(pair);
                break;
            }
        }
    }

    Ok((
        positions,
        saw_normal.then_some(normals),
        saw_uv.then_some(uvs),
    ))
}

/// Reads a mesh's triangle indices.
fn read_indices(label: &str, fields: &FieldMap, vertex_count: usize) -> EmitResult<Vec<i64>> {
    let Some(topology) = read::field(fields, "indices") else {
        return Err(EmitError::Malformed {
            detail: format!("mesh '{label}' has no 'indices'"),
        });
    };
    let raw = read::numbers(topology, &format!("mesh '{label}''s indices"))?;
    let mut indices = Vec::with_capacity(raw.len());
    for value in raw {
        let index = value as i64;
        if index < 0 || index as usize >= vertex_count {
            return Err(EmitError::Malformed {
                detail: format!(
                    "mesh '{label}' references vertex {index} of {vertex_count}, out of range"
                ),
            });
        }
        indices.push(index);
    }
    if indices.len() % 3 != 0 {
        return Err(EmitError::Malformed {
            detail: format!(
                "mesh '{label}' has {} indices, which is not a whole number of triangles",
                indices.len()
            ),
        });
    }
    Ok(indices)
}

/// Writes one mesh, with its material scope already open by the caller.
pub fn write_mesh(writer: &mut Writer, mesh: &Mesh) {
    let mut attributes: Vec<(&str, String)> = vec![("P", Writer::float_array(&mesh.positions))];

    // `nverts` is a single 3 per triangle because the emitter expands to triangles rather
    // than passing polygons through. The loader would accept polygons, but our topology is
    // already triangles and writing them as such removes a fan that nothing downstream
    // needs.
    let triangles = mesh.indices.len() / 3;
    let nverts: Vec<i64> = vec![3; triangles];
    attributes.push(("nverts", Writer::int_array(&nverts)));
    attributes.push(("verts", Writer::int_array(&mesh.indices)));

    if let Some(normals) = &mesh.normals {
        attributes.push((
            super::attribute_map::CYCLES_VERTEX_NORMAL,
            Writer::float_array(normals),
        ));
    }
    if mesh.uvs.is_some() {
        attributes.push((
            super::attribute_map::CYCLES_UV,
            Writer::float_array(&per_corner_uvs(mesh)),
        ));
    }

    writer.leaf("mesh", &attributes);
}

/// Expands per-vertex UVs into the per-corner array the loader reads.
///
/// The loader walks each polygon's corners and reads `UV` at `corner_index * 2`, where the
/// corner index counts through the fan, not through the vertex array. So corner `k` of
/// triangle `t` is vertex `indices[3t + k]`, and its UV is that vertex's pair.
fn per_corner_uvs(mesh: &Mesh) -> Vec<f64> {
    let Some(uvs) = &mesh.uvs else {
        return Vec::new();
    };
    let mut out = Vec::with_capacity(mesh.indices.len() * 2);
    for index in &mesh.indices {
        let vertex = *index as usize;
        out.push(uvs.get(vertex * 2).copied().unwrap_or(0.0));
        out.push(uvs.get(vertex * 2 + 1).copied().unwrap_or(0.0));
    }
    out
}
