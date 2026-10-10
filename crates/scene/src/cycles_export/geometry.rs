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
use definition::{FieldMap, Schema, SchemaKind, Value};

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
    /// Indices into the vertex arrays, three per triangle, or `None` for a mesh whose corners
    /// are already its vertices in order.
    pub indices: Option<Vec<i64>>,
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
///
/// # Why packed points are read rather than refused
///
/// Every scene the gallery builds carries its points packed: `kit::mesh` writes
/// `Value::Packed` and its own documentation says why — "a generator produces the run rather
/// than a tree of maps, so the geometry a scene is built from never exists in the expensive
/// form at all". So packed is not a compiled document's shape that a scene author might avoid;
/// it is **the** shape a scene is written in, and an emitter that refuses it cannot render a
/// single gallery scene.
///
/// Reading goes through [`definition::PackedList::component`], which is a strided read of one
/// field of one element. That is what keeps this linear in the points read and not quadratic in
/// the ones there are, and it is the same reader a strided gpu pass uses, so the emitter and the
/// rasteriser agree on what a layout means rather than each having its own opinion.
fn read_points(
    label: &str,
    fields: &FieldMap,
    schema: Option<&Value>,
    mode: LossyMode,
    report: &mut LossyReport,
) -> EmitResult<(Vec<f64>, Option<Vec<f64>>, Option<Vec<f64>>)> {
    let points = read::require(fields, "points", &format!("mesh '{label}'"))?;

    // The declared field names decide which arrays to read.
    //
    // A `point_schema` is written by `Schema::to_value`, so it arrives as a **schema document** —
    // `{kind, attributes, fields}` — rather than as the bare field map a struct is. Reading it
    // with `as_field_map` therefore returns the schema's own keys and never a point field, and a
    // mesh whose points are perfectly readable comes back saying it declares no `position`.
    // `Schema::from_value` is the reader that pairs with the writer; the field names then come
    // out of the struct the schema describes.
    //
    // A mesh that declares no schema at all gets position alone, which is what
    // `default_point_schema` says.
    let declared: Vec<String> = match schema {
        Some(value) => read_point_schema_fields(label, value)?,
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

    if points.as_packed().is_some() {
        return read_packed_points(label, points, &declared);
    }

    let elements = points.as_list().ok_or_else(|| EmitError::Malformed {
        detail: format!("mesh '{label}' has no point list"),
    })?;
    if elements.is_empty() {
        return Err(EmitError::Malformed {
            detail: format!("mesh '{label}' has no points"),
        });
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
        for name in UV_FIELDS {
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

/// Reads the field names out of a mesh's `point_schema`.
///
/// # Invariants
///
/// The schema is read with [`Schema::from_value`], the reader that pairs with the
/// `Schema::to_value` that wrote it, and the names come out of the struct it describes. A schema
/// that is not a struct is refused here: a point is a struct of named fields by definition, and a
/// document whose `point_schema` says otherwise is not one whose points can be laid out.
fn read_point_schema_fields(label: &str, value: &Value) -> EmitResult<Vec<String>> {
    let schema = Schema::from_value(value).map_err(|error| EmitError::Malformed {
        detail: format!("mesh '{label}' has a point_schema that is not a schema: {error}"),
    })?;
    let SchemaKind::Struct { fields } = &schema.kind else {
        return Err(EmitError::Malformed {
            detail: format!(
                "mesh '{label}' declares a point_schema of {}, and a point is a structure of \
                 named fields",
                schema.kind_name()
            ),
        });
    };
    Ok(fields.iter().map(|(name, _)| name.clone()).collect())
}

/// The field names a point's texture coordinate may be written under, in the order they are
/// tried.
///
/// This is one list rather than several literals so that the packed and unpacked readers cannot
/// disagree about which spelling of a uv they accept — a divergence that would show as a mesh
/// that renders without texture coordinates on one path and with them on the other.
///
/// `texture` is first because that is what the workspace's own point schema calls it
/// (`kit::shaded_point_schema`); the rest are the spellings a document written by hand is likely
/// to use, since nothing in the workspace writes those.
const UV_FIELDS: &[&str] = &["texture", "uv", "texcoord", "uv_map"];

/// Reads the same three arrays out of a packed run.
///
/// # Invariants
///
/// The layout is read from the run itself rather than from the schema, because the run carries
/// the layout it was packed against and a layout derived separately is a second definition of
/// the same thing. A field the layout does not have is reported by name: it means the mesh's
/// declared schema and its packed layout disagree, which is a defect in the document rather
/// than something to paper over with a default.
fn read_packed_points(
    label: &str,
    points: &Value,
    declared: &[String],
) -> EmitResult<(Vec<f64>, Option<Vec<f64>>, Option<Vec<f64>>)> {
    let packed = points.as_packed().ok_or_else(|| EmitError::Malformed {
        detail: format!("mesh '{label}' has no packed run to read"),
    })?;
    let count = packed.len();
    if count == 0 {
        return Err(EmitError::Malformed {
            detail: format!("mesh '{label}' has no points"),
        });
    }

    if !declared.iter().any(|name| name == "position") {
        return Err(EmitError::Malformed {
            detail: format!("mesh '{label}' declares no 'position' to read"),
        });
    }

    let triple_of = |field: &str| -> EmitResult<Vec<f64>> {
        let mut values = Vec::with_capacity(count * 3);
        for index in 0..count {
            for component in 0..3 {
                let value = packed.component(index, field, component).ok_or_else(|| {
                    EmitError::Malformed {
                        detail: format!(
                            "mesh '{label}' point {index} has no '{field}' component {component}, \
                             and its packed layout declares {:?}",
                            packed.layout().field_names(),
                        ),
                    }
                })?;
                values.push(value);
            }
        }
        Ok(values)
    };

    let positions = triple_of("position")?;
    // A normal is optional, so its absence has to be distinguishable from a failure to read it:
    // `triple_of` fails when the layout declares the field and the bytes do not hold it, and the
    // two are different defects. The presence test is a single component read on the first point,
    // which is what decides it.
    let normals = packed
        .component(0, "normal", 0)
        .map(|_| triple_of("normal"))
        .transpose()?;
    let uvs = UV_FIELDS
        .iter()
        .find(|name| packed.component(0, name, 1).is_some())
        .map(|name| {
            let mut values = Vec::with_capacity(count * 2);
            for index in 0..count {
                for component in 0..2 {
                    // The presence test above read component 1 of point 0, so the only way this
                    // can be `None` is a run whose length disagrees with its own stride, which
                    // `PackedList::new` refuses to construct.
                    values.push(packed.component(index, name, component).unwrap_or_default());
                }
            }
            values
        });

    Ok((positions, normals, uvs))
}

/// Reads a mesh's triangle indices.
///
/// # Why a mesh with none is not an error
///
/// A mesh that writes no indices is a **non-indexed** triangle list: three vertices per face, in
/// order. That is how every scene in the gallery is built — `Shaded::push` appends a whole
/// element per corner, and `kit::mesh` writes no index array at all — so refusing it would mean
/// the emitter cannot render any of them.
///
/// The target format has a spelling for this that is genuinely smaller rather than a
/// convenience: `verts` is a list of corner numbers, and omitting it entirely means the corners
/// are the vertices in order. So this returns `None` for the non-indexed case and the writer
/// leaves `verts` off, rather than synthesising an identity index that would say the same thing
/// in more bytes.
fn read_indices(
    label: &str,
    fields: &FieldMap,
    vertex_count: usize,
) -> EmitResult<Option<Vec<i64>>> {
    let Some(topology) = read::field(fields, "indices") else {
        return Ok(None);
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
    Ok(Some(indices))
}

/// Writes one mesh, with its material scope already open by the caller.
pub fn write_mesh(writer: &mut Writer, mesh: &Mesh) {
    // Positions and normals both go into Cycles' axes. A normal is not a point, but the basis
    // change is a rotation, and a rotation carries a direction the same way it carries a point
    // minus the translation — so the same permutation is right for both, and reading it as its
    // own case would be a second definition of one mapping.
    let mut attributes: Vec<(&str, String)> =
        vec![("P", Writer::float_array(&into_cycles_axes(&mesh.positions)))];

    // `nverts` is a single 3 per triangle because the emitter expands to triangles rather
    // than passing polygons through. The loader would accept polygons, but our topology is
    // already triangles and writing them as such removes a fan that nothing downstream
    // needs.
    //
    // `verts` is written even for a mesh that has no index array of its own, because the loader
    // indexes into it per corner and a mesh without one crashes it: it reads `verts` for corner
    // zero of every polygon and dereferences whatever is there. So a non-indexed triangle list
    // becomes the identity, which is what its corners already are.
    let indices: Vec<i64> = match &mesh.indices {
        Some(indices) => indices.clone(),
        None => (0..(mesh.positions.len() / 3) as i64).collect(),
    };
    let nverts: Vec<i64> = vec![3; indices.len() / 3];
    attributes.push(("nverts", Writer::int_array(&nverts)));
    attributes.push(("verts", Writer::int_array(&indices)));

    if let Some(normals) = &mesh.normals {
        attributes.push((
            super::attribute_map::CYCLES_VERTEX_NORMAL,
            Writer::float_array(&into_cycles_axes(normals)),
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

/// Returns a flat run of `xyz` triples in Cycles' axes.
///
/// The run is the loader's own shape — one long list, three numbers per element — so it is
/// walked in threes rather than regrouped. A run whose length is not a multiple of three is
/// left alone rather than dropped: it is a document that is already malformed, and saying so
/// is the reader's job rather than this function's.
fn into_cycles_axes(triples: &[f64]) -> Vec<f64> {
    triples
        .chunks_exact(3)
        .flat_map(|triple| super::axes::position([triple[0], triple[1], triple[2]]))
        .collect()
}

/// Expands per-vertex UVs into the per-corner array the loader reads.
///
/// The loader walks each polygon's corners and reads `UV` at `corner_index * 2`, where the
/// corner index counts through the fan, not through the vertex array. So corner `k` of
/// triangle `t` is vertex `indices[3t + k]`, and its UV is that vertex's pair.
///
/// A mesh with no indices is already in corner order, so its per-vertex UVs are already the
/// per-corner array and are returned as they are. That is not a shortcut: the two arrays are the
/// same sequence of pairs, and copying it through an identity index would produce the same bytes.
fn per_corner_uvs(mesh: &Mesh) -> Vec<f64> {
    let Some(uvs) = &mesh.uvs else {
        return Vec::new();
    };
    let Some(indices) = &mesh.indices else {
        return uvs.clone();
    };
    let mut out = Vec::with_capacity(indices.len() * 2);
    for index in indices {
        let vertex = *index as usize;
        out.push(uvs.get(vertex * 2).copied().unwrap_or(0.0));
        out.push(uvs.get(vertex * 2 + 1).copied().unwrap_or(0.0));
    }
    out
}
