//! A building's solids as a scene draws them.
//!
//! This is the one place this crate joins `building` to a scene, the way [`patchmesh`](crate::patchmesh)
//! is the one place it joins `mountain` to `patch`: neither crate may know about the other, so the
//! conversion lives above them.
//!
//! # Invariants
//!
//! - **The normals are the mesh's own corner normals.** They come from the authored surface
//!   grouping — smooth across one wall face or one roof plane, hard at the edges between them — and
//!   a scene that recomputed them from the triangle order instead would shade a roof as a faceted
//!   sheet, undoing the one shading rule the crate states (its D13).
//! - **One mesh per solid**, because a material belongs to an instance and not to a face: walls and
//!   roofs are drawn by different materials, so they are different meshes. [`points_of`] is how a
//!   scene draws one of them, [`points_where`] is how it draws any set an author names.
//! - **The texture coordinate is the plan position.** A material that reads it sees a projection
//!   from above, which is enough for a flat colour and is not a texture atlas; mapping a facade
//!   properly needs the bay grid, which is P2. Said here rather than left for a scene to discover as
//!   a stretched image.

use building::{BuildingMesh, Element, SolidId};
use definition::PackedList;

use crate::geometry::Shaded;

/// Builds the points of every face an element filter keeps.
pub fn points_where(built: &BuildingMesh, keep: impl Fn(&Element) -> bool) -> PackedList {
    let normals = built.corner_normals();
    let mut points = Shaded::with_capacity(built.faces.len() * 3);
    for (index, face) in built.faces.iter().enumerate() {
        if !keep(&face.element) {
            continue;
        }
        let triangle = built.triangle(face);
        for corner in 0..3 {
            let position = triangle[corner];
            let normal = normals.at(index, corner);
            points.push(
                [position[0] as f64, position[1] as f64, position[2] as f64],
                [normal[0] as f64, normal[1] as f64, normal[2] as f64],
                [position[0] as f64, position[2] as f64],
            );
        }
    }
    points.finish()
}

/// Builds the points of one solid.
pub fn points_of(built: &BuildingMesh, solid: SolidId) -> PackedList {
    points_where(built, |element| element.solid() == solid)
}

/// Returns every solid the mesh is made of, in the mesh's own face order.
pub fn solids(built: &BuildingMesh) -> Vec<SolidId> {
    let mut solids: Vec<SolidId> = Vec::new();
    for face in &built.faces {
        let solid = face.element.solid();
        if !solids.contains(&solid) {
            solids.push(solid);
        }
    }
    solids
}
