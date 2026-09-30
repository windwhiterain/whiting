//! Procedural geometry, as the points of a mesh.
//!
//! A mesh is geometry and nothing about where it is drawn, so a builder here produces
//! points and a point schema and stops. An instance places the result.
//!
//! # Invariants
//!
//! Every point a builder emits is a struct holding exactly the fields of the schema the
//! builder was given, in schema order. The schema is what a buffer is packed with and what
//! the vertex layout is derived from, so a point carrying a field the schema does not
//! declare is a compile error rather than a silently shifted buffer.

use definition::{FieldMap, Schema, SchemaKind, Value};
use graphics::layout::components_schema;

/// Builds the points of a mesh, against one point schema.
///
/// The schema is held rather than the field names, because the widths a buffer is packed
/// with come from it and a builder that named its own fields could disagree with it.
pub struct MeshBuilder {
    schema: Schema,
    points: Vec<Value>,
}

impl MeshBuilder {
    /// Starts a mesh whose points are described by `schema`.
    pub fn new(schema: Schema) -> Self {
        Self {
            schema,
            points: Vec::new(),
        }
    }

    /// Returns the point schema the mesh declares.
    pub fn schema(&self) -> &Schema {
        &self.schema
    }

    /// Returns how many points the mesh holds.
    pub fn len(&self) -> usize {
        self.points.len()
    }

    /// Returns whether the mesh holds no points, which the compiler refuses.
    pub fn is_empty(&self) -> bool {
        self.points.is_empty()
    }

    /// Consumes the builder, yielding the points it emitted.
    pub fn points(self) -> Vec<Value> {
        self.points
    }

    /// Adds one point, built from a field in schema order.
    ///
    /// The caller passes the components of each field in the order the schema sorts them
    /// into, which is the order the buffer lays them out in.
    pub fn point(&mut self, fields: &[&[f64]]) -> &mut Self {
        let SchemaKind::Struct { fields: declared } = &self.schema.kind else {
            panic!("a point schema is a struct, so a point is a struct of its fields");
        };
        assert_eq!(
            declared.len(),
            fields.len(),
            "a point carries one value per field its schema declares"
        );
        let point: FieldMap = declared
            .keys()
            .zip(fields)
            .map(|(name, components)| (name.clone(), list(components)))
            .collect();
        self.points.push(Value::structure(point));
        self
    }

    /// Adds a triangle from three positions, and derives a flat normal for all three.
    ///
    /// A flat normal is what a facet wants: the three corners of one face share a normal,
    /// so a sphere built from triangles is faceted rather than smooth. A smooth normal is
    /// the caller's to supply, through [`MeshBuilder::point`].
    pub fn triangle(&mut self, corners: [[f64; 3]; 3]) -> &mut Self {
        let normal = face_normal(corners);
        for corner in corners {
            self.point(&[&normal, &corner]);
        }
        self
    }

    /// Adds a quad from four corners wound counter clockwise, as two triangles.
    pub fn quad(&mut self, corners: [[f64; 3]; 4]) -> &mut Self {
        self.triangle([corners[0], corners[1], corners[2]]);
        self.triangle([corners[0], corners[2], corners[3]]);
        self
    }
}

/// Returns the schema of a point carrying a normal, a position and a texture coordinate.
///
/// The fields are sorted by name, so the buffer lays them out as `normal`, `position`,
/// `texture`, and the vertex layout reads them at locations zero, one and two in that
/// order. A shader written against this schema reads the same three locations.
pub fn shaded_point_schema() -> Schema {
    Schema::new(SchemaKind::structure([
        ("normal".to_owned(), components_schema(3)),
        ("position".to_owned(), components_schema(3)),
        ("texture".to_owned(), components_schema(2)),
    ]))
}

/// Returns the schema of a point carrying only a position.
pub fn position_point_schema() -> Schema {
    Schema::new(SchemaKind::structure([(
        "position".to_owned(),
        components_schema(3),
    )]))
}

/// Returns the normal of the triangle three corners describe.
///
/// A degenerate triangle has no normal, which is reported as the unit vector rather than
/// as a division by zero: a caller that built one wants a frame drawn, not a panic.
pub fn face_normal(corners: [[f64; 3]; 3]) -> [f64; 3] {
    let first = subtract(corners[1], corners[0]);
    let second = subtract(corners[2], corners[0]);
    let cross = [
        first[1] * second[2] - first[2] * second[1],
        first[2] * second[0] - first[0] * second[2],
        first[0] * second[1] - first[1] * second[0],
    ];
    let length = (cross[0] * cross[0] + cross[1] * cross[1] + cross[2] * cross[2]).sqrt();
    if length > f64::EPSILON {
        [cross[0] / length, cross[1] / length, cross[2] / length]
    } else {
        [0.0, 1.0, 0.0]
    }
}

/// Subtracts one vector from another.
pub fn subtract(left: [f64; 3], right: [f64; 3]) -> [f64; 3] {
    [left[0] - right[0], left[1] - right[1], left[2] - right[2]]
}

/// Returns a vector's unit length, or the unit y axis when it has none.
pub fn normalized(vector: [f64; 3]) -> [f64; 3] {
    let length = (vector[0] * vector[0] + vector[1] * vector[1] + vector[2] * vector[2]).sqrt();
    if length > f64::EPSILON {
        [vector[0] / length, vector[1] / length, vector[2] / length]
    } else {
        [0.0, 1.0, 0.0]
    }
}

/// Returns the dot product of two vectors.
pub fn dot(left: [f64; 3], right: [f64; 3]) -> f64 {
    left[0] * right[0] + left[1] * right[1] + left[2] * right[2]
}

/// Returns the cross product of two vectors.
pub fn cross(left: [f64; 3], right: [f64; 3]) -> [f64; 3] {
    [
        left[1] * right[2] - left[2] * right[1],
        left[2] * right[0] - left[0] * right[2],
        left[0] * right[1] - left[1] * right[0],
    ]
}

/// Wraps a run of components as the list a point field carries.
fn list(components: &[f64]) -> Value {
    Value::List(components.iter().copied().map(Value::Float).collect())
}
