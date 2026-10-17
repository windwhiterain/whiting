//! The expanded mesh: closed planar-facet solids, every face naming the element that made it.
//!
//! # Invariants
//!
//! - **A face is wound so that its right-handed normal points out of the solid.** Unlike
//!   `mountain`'s mesh — a height field over the plan, whose faces all look up and which therefore
//!   negates the right-handed normal — a building has vertical walls, and "up" is not a direction
//!   a wall's outer face can be described by. The contract here is `cross(b - a, c - a)` pointing
//!   away from the solid's interior, and it is enforced globally by
//!   [`signed_volume_of`](BuildingMesh::signed_volume_of) being **positive**: an inside-out
//!   building is a named violation rather than a frame nobody can explain
//!   (`gallery::built` records the same failure as "a wall that is not drawn").
//! - **Points are welded by authored identity, never by coordinate.** Two faces built from one
//!   authored corner share one point index, which is what lets a boundary be counted at all: a
//!   mesh of loose facets has no edges to be manifold about.
//! - **A face's [`Element`] is also its shading group.** One authored surface — one wall face, one
//!   roof plane — is one planar surface, so its facets are smooth across and hard at its edges,
//!   and that is a structural fact here rather than a smoothing pass downstream.

use std::collections::HashMap;

use crate::mass::MassId;
use crate::math::{Vec3, cross3, dot3, subtract};
use crate::roof::RoofId;

/// Which surface of a mass a face belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum MassSurface {
    /// The face seen from outside, one per outline edge.
    OuterWall {
        /// Index of the outline edge.
        edge: usize,
    },
    /// The face seen from inside, one per outline edge.
    InnerWall {
        /// Index of the outline edge.
        edge: usize,
    },
    /// The ring of wall top between the outer and inner faces, one per outline edge.
    WallTop {
        /// Index of the outline edge.
        edge: usize,
    },
    /// The floor's upper surface: the inner loop's polygon at the outline's own elevations, facing
    /// up.
    FloorTop,
    /// A surface of the wall's own thickness inside an opening: a jamb, a head or a sill.
    ///
    /// # Invariants
    ///
    /// One element per side per hole, because each side is its own plane and its own shading group —
    /// a reveal is masonry seen at ninety degrees to the wall it pierces, and smoothing a normal
    /// across that corner would round off the one edge that says how thick the wall is. `opening` is
    /// the hole's index in the document's own list ([`Openings`](crate::openings::Openings)), so a
    /// face traces back to the opening an author wrote and not merely to the wall it is in.
    Reveal {
        /// Index of the outline edge the hole is cut into.
        edge: usize,
        /// Index of the opening in the document's list.
        opening: usize,
        /// Which side of the reveal.
        side: crate::openings::RevealSide,
    },
    /// The slab's underside: the whole outline one slab thickness below it, facing down.
    ///
    /// # Invariants
    ///
    /// A **disc across the whole outline**, not a ring between the outer and inner loops: a ring
    /// would put a third face on the inner wall's bottom edge and the solid would not be manifold.
    /// Measured 2026-10-16 in `expand` while closing the shell — the ring version came out with one
    /// boundary loop and Euler characteristic 1.
    ///
    /// It is its own variant rather than sharing one with [`FloorTop`](Self::FloorTop) because the
    /// two are genuinely different planes, one slab thickness apart, and a face's element is also its
    /// shading group: one group spanning both would be reported as a non-planar surface by
    /// [`validate`](crate::validate) — measured at `departure = slab/2` on the first merged tip —
    /// and would smooth a shading normal across the two.
    SlabSoffit,
    /// The wall ring's **base**: the annulus between the outline and the inner loop, at the plate's bottom plane.
    ///
    /// # Invariants
    ///
    /// **It exists because a mass is becoming a volume, and a volume closes itself.** Once the plate is its own solid
    /// ([`Slab`](crate::mesh::Element::Slab)), the mass's faces are the wall ring and nothing else, and a ring is open
    /// at both ends: the top is [`WallTop`](Self::WallTop) and the base is this. It is called a base rather than a
    /// floor because the floor is the plate's own top face, one plane up, and the two are different elements of
    /// different solids — the ring's base is hidden inside the plate, the plate's top is the floor a person walks on.
    WallBase,
    /// The face of a courtyard wall seen from **inside the courtyard**, one per void edge.
    VoidWall {
        /// Which of the mass's voids, in the order it authored them.
        void: usize,
        /// Index of the void loop's edge.
        edge: usize,
    },
    /// The face of a courtyard wall seen from **the rooms**, one per void edge.
    VoidInner {
        /// Which of the mass's voids.
        void: usize,
        /// Index of the void loop's edge.
        edge: usize,
    },
    /// The strip of wall top between a courtyard's two loops, one per void edge.
    VoidTop {
        /// Which of the mass's voids.
        void: usize,
        /// Index of the void loop's edge.
        edge: usize,
    },
    /// A courtyard's paving: the plate's own top surface **inside** the courtyard wall.
    ///
    /// # Invariants
    ///
    /// **It is its own element and not part of [`FloorTop`](Self::FloorTop), for the reason this file already
    /// records for the soffit:** the plate's top cannot be one disc across a courtyard, because the courtyard wall
    /// meets it along an edge, and a disc continuing on both sides of that edge would put a third face on the wall's
    /// bottom edge — a non-manifold junction. So the plate's top is **a ring from the room-side loop out to the
    /// courtyard's outer loop** ([`FloorTop`](Self::FloorTop)) **plus the paving inside the courtyard's own loop**
    /// (this variant), two regions of one plane told apart by the wall between them.
    VoidFloor {
        /// Which of the mass's voids.
        void: usize,
    },
}

/// Which surface of a roof a face belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum RoofSurface {
    /// The upper face of a plane.
    Skin {
        /// Index of the plane.
        plane: usize,
    },
    /// The underside of a plane, one thickness below the skin.
    Soffit {
        /// Index of the plane.
        plane: usize,
    },
    /// The band around the skin's boundary: one quad per boundary edge of the skin, extruded down
    /// by the roof's thickness.
    ///
    /// # Invariants
    ///
    /// The band is derived from the **skin's own boundary** rather than from the eave loop, which is
    /// what closes a gable's ends as well as its eaves with one rule and no case analysis. `edge` is
    /// therefore a position in that boundary's sorted edge list — **not** an index into
    /// [`Roof::eaves`](crate::roof::Roof), which it equals only for a roof whose whole boundary is
    /// eave.
    Fascia {
        /// Position in the skin's sorted boundary-edge list.
        edge: usize,
    },
}

/// The authored element a face came from: its provenance, and its shading group.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Element {
    /// A face of a mass.
    Mass(MassId, MassSurface),
    /// A face of a roof.
    Roof(RoofId, RoofSurface),
    /// A face of a part: which placement stands it, and what it was inside its template.
    ///
    /// # Invariants
    ///
    /// The two halves answer the two questions a part's face can be asked. `placement` is the index of
    /// the placement in the document, which is also the index its [`SolidId::Part`] carries, so a face
    /// traces back to the authored placement rather than to the template it happens to be a copy of —
    /// the identity a scene needs when the same door leaf stands in four doorways. `surface` is the
    /// **template's own** element, unchanged, so a face still knows whether it is a wall, a slab, or a
    /// reveal, and a report can say "the second leaf's inner wall" without a second vocabulary.
    Part {
        /// The placement, by its index in the document's placements.
        placement: usize,
        /// What the face was inside the template.
        surface: PartSurface,
    },
    /// A face of a wall.
    Wall(crate::wall::WallId, crate::wall::WallSurface),
    /// A face of a slab.
    Slab(crate::slab::SlabId, crate::slab::SlabSurface),
}

/// What a part's face was inside the template it was copied from.
///
/// It is [`Element`] without the part, which is what keeps the recursion out of the type: a template
/// is a skeleton of masses and roofs, and a template holding a part is not a thing this crate builds,
/// so one level is the whole of it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum PartSurface {
    /// A face of one of the template's masses.
    Mass(MassId, MassSurface),
    /// A face of one of the template's roofs.
    Roof(RoofId, RoofSurface),
    /// A face of one of the template's walls.
    ///
    /// A `Template` holds masses and roofs and no walls, so nothing builds this today; the variant
    /// exists because the map from an element to a template's own element has to be total, and a
    /// `panic!` in that map would be a crash for a type the document can express.
    Wall(crate::wall::WallId, crate::wall::WallSurface),
    /// A face of one of the template's slabs.
    ///
    /// A `Template` holds masses and roofs and no slabs, so nothing builds this today; it exists for the
    /// same reason as the wall variant above — the map from an element to a template's own element has to
    /// be total, and a `panic!` there would be a crash for a type the document can express.
    Slab(crate::slab::SlabId, crate::slab::SlabSurface),
}

impl Element {
    /// Returns the solid this element belongs to.
    pub fn solid(&self) -> SolidId {
        match self {
            Self::Mass(id, _) => SolidId::Mass(*id),
            Self::Roof(id, _) => SolidId::Roof(*id),
            Self::Part { placement, .. } => SolidId::Part(*placement),
            Self::Wall(id, _) => SolidId::Wall(*id),
            Self::Slab(id, _) => SolidId::Slab(*id),
        }
    }
}

/// One closed solid the mesh is made of.
///
/// # Invariants
///
/// One variant per kind of body a document holds, and the index is that body's own index in the
/// document's list of its kind. A **part** is a solid of its own rather than a surface of its host's:
/// it is closed, it is oriented outward, and the two-level watertightness rule reads it exactly as it
/// reads a mass — which is what lets a door leaf stand in a hole without the wall and the leaf having
/// to agree about a single vertex.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum SolidId {
    /// A mass.
    Mass(MassId),
    /// A roof.
    Roof(RoofId),
    /// A part, by its placement's index in the document's placements.
    Part(usize),
    /// A wall, by its index in the document's walls.
    Wall(crate::wall::WallId),
    /// A slab, by its index in the document's slabs.
    Slab(crate::slab::SlabId),
}

/// One triangle, and the element that produced it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BuildingFace {
    /// The three point indices, wound so the right-handed normal points out.
    pub indices: [u32; 3],
    /// What the face came from.
    pub element: Element,
}

/// A building as geometry: welded points, triangles, and each triangle's provenance.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct BuildingMesh {
    /// The welded points.
    pub points: Vec<Vec3>,
    /// The triangles.
    pub faces: Vec<BuildingFace>,
}

impl BuildingMesh {
    /// Builds an empty mesh.
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns a face's three corners.
    pub fn triangle(&self, face: &BuildingFace) -> [Vec3; 3] {
        face.indices.map(|index| self.points[index as usize])
    }

    /// Returns a face's normal, pointing out of the solid it belongs to.
    pub fn normal(&self, face: &BuildingFace) -> Vec3 {
        let [a, b, c] = self.triangle(face);
        crate::math::normalized3(cross3(subtract(b, a), subtract(c, a)))
    }

    /// Returns the box the mesh stands in, as (lowest, highest) corners.
    pub fn bounds(&self) -> (Vec3, Vec3) {
        let mut low = [f32::INFINITY; 3];
        let mut high = [f32::NEG_INFINITY; 3];
        for point in &self.points {
            for axis in 0..3 {
                low[axis] = low[axis].min(point[axis]);
                high[axis] = high[axis].max(point[axis]);
            }
        }
        (low, high)
    }

    /// Returns the box one solid stands in, as (lowest, highest) corners.
    ///
    /// # Invariants
    ///
    /// It reads the points the solid's **own faces** name, not the mesh's whole point list, which is
    /// the difference that matters as soon as a document holds more than one body: a part's box
    /// against its host's says where the part was placed, and the mesh's own box says nothing about
    /// either. `O(faces of that solid)`, the same pass the other `_of` readings make.
    pub fn bounds_of(&self, solid: SolidId) -> (Vec3, Vec3) {
        let mut low = [f32::INFINITY; 3];
        let mut high = [f32::NEG_INFINITY; 3];
        for face in self.faces_of(solid) {
            for corner in self.faces[face].indices {
                let point = self.points[corner as usize];
                for axis in 0..3 {
                    low[axis] = low[axis].min(point[axis]);
                    high[axis] = high[axis].max(point[axis]);
                }
            }
        }
        (low, high)
    }

    /// Returns the signed plan area of the whole mesh.
    ///
    /// # Invariants
    ///
    /// **On a closed mesh this is identically zero**, and that is a fact about closed surfaces rather
    /// than a defect: every face's plan projection is cancelled by another face's, so the sum of the
    /// directed plan areas vanishes — the discrete form of the divergence theorem's boundary term.
    /// It is therefore a check on a **subset** ([`plan_area_of`](BuildingMesh::plan_area_of): a roof's
    /// skin against its eave loop, the plan-facing faces of a mass) and never a statement about a
    /// whole solid. Measured 2026-10-16: every solid the census prints reports `0.0000`, and all that
    /// says is that the solid is closed.
    pub fn plan_area(&self) -> f32 {
        self.plan_area_of(&(0..self.faces.len()).collect::<Vec<_>>())
    }

    /// Returns the signed plan area of a set of faces.
    ///
    /// # Invariants
    ///
    /// The sign carries the winding: faces wound to look up contribute positively and faces wound
    /// down negatively. This is what makes it a check — over a subset the author meant to **tile** a
    /// region (a roof's skin over its eave loop), a total that does not match the authored area means
    /// a face is missing, doubled, or wound the other way. That comparison is the design's
    /// *coverage*, and it is [`validate`](crate::validate)'s business rather than this function's.
    pub fn plan_area_of(&self, faces: &[usize]) -> f32 {
        let mut total = 0.0;
        for index in faces {
            let [a, b, c] = self.triangle(&self.faces[*index]);
            total += crate::math::signed_area([a[0], a[2]], [b[0], b[2]], [c[0], c[2]]);
        }
        total
    }

    /// Returns the signed volume enclosed by a set of faces.
    ///
    /// # Invariants
    ///
    /// **Positive means the faces point out of the solid.** The divergence theorem over the closed
    /// surface, so a set of faces that is not closed gives a number that means nothing; the closure
    /// itself is [`boundary_loops_of`](BuildingMesh::boundary_loops_of)'s business and the two are
    /// read together. This is the check that turns an inside-out building into a report instead of
    /// a frame in which every wall is culled.
    pub fn signed_volume_of(&self, faces: &[usize]) -> f32 {
        let mut total = 0.0;
        for index in faces {
            let [a, b, c] = self.triangle(&self.faces[*index]);
            total += dot3(a, cross3(b, c));
        }
        total / 6.0
    }

    /// Returns the directed edges of a face set that have no opposite.
    ///
    /// An empty answer means the set is closed. The edges are directed the way their face winds,
    /// so a caller can walk a boundary rather than only count it.
    pub fn boundary_edges_of(&self, faces: &[usize]) -> Vec<(u32, u32)> {
        let mut used: HashMap<(u32, u32), usize> = HashMap::new();
        for index in faces {
            for edge in edges_of(&self.faces[*index].indices) {
                *used.entry(edge).or_insert(0) += 1;
            }
        }
        let mut boundary = Vec::new();
        for (edge, count) in &used {
            let opposite = (edge.1, edge.0);
            let matched = used.get(&opposite).copied().unwrap_or(0);
            let unmatched = count.saturating_sub(matched);
            for _ in 0..unmatched {
                boundary.push(*edge);
            }
        }
        boundary.sort_unstable();
        boundary
    }

    /// Returns how many closed loops a face set's boundary falls into; zero means it is closed.
    pub fn boundary_loops_of(&self, faces: &[usize]) -> usize {
        let boundary = self.boundary_edges_of(faces);
        if boundary.is_empty() {
            return 0;
        }
        let mut parent: HashMap<u32, u32> = HashMap::new();
        for (from, to) in &boundary {
            parent.entry(*from).or_insert(*from);
            parent.entry(*to).or_insert(*to);
        }
        fn find(parent: &mut HashMap<u32, u32>, index: u32) -> u32 {
            let mut root = index;
            while parent[&root] != root {
                root = parent[&root];
            }
            let mut walk = index;
            while parent[&walk] != root {
                let next = parent[&walk];
                parent.insert(walk, root);
                walk = next;
            }
            root
        }
        let pairs: Vec<(u32, u32)> = boundary.clone();
        for (from, to) in pairs {
            let (a, b) = (find(&mut parent, from), find(&mut parent, to));
            parent.insert(a, b);
        }
        let keys: Vec<u32> = parent.keys().copied().collect();
        let roots: std::collections::BTreeSet<u32> = keys
            .into_iter()
            .map(|index| find(&mut parent, index))
            .collect();
        roots.len()
    }

    /// Returns `V - E + F` over a set of faces.
    ///
    /// # Invariants
    ///
    /// Euler's characteristic is only meaningful on a closed surface that shares its vertices, so
    /// this reads the **welded** indices and not the corners a face happens to list. A sphere is
    /// `2`: every closed building solid is one, and a value that is not 2 means either a hole
    /// (`1` for a disc), a handle, or unwelded seams.
    pub fn euler_characteristic_of(&self, faces: &[usize]) -> i64 {
        let mut vertices: std::collections::BTreeSet<u32> = std::collections::BTreeSet::new();
        let mut edges: std::collections::BTreeSet<(u32, u32)> = std::collections::BTreeSet::new();
        for index in faces {
            let corners = self.faces[*index].indices;
            for corner in corners {
                vertices.insert(corner);
            }
            for (from, to) in edges_of(&corners) {
                edges.insert(if from < to { (from, to) } else { (to, from) });
            }
        }
        vertices.len() as i64 - edges.len() as i64 + faces.len() as i64
    }

    /// Returns whether every edge of a face set is used by exactly two of its faces.
    ///
    /// # Invariants
    ///
    /// This is **edge-manifoldness**, and it is all this function claims: an edge shared by three
    /// or more faces, or by one, fails. A vertex whose faces form more than one fan — two solids
    /// meeting at a point — passes, and is not what a building's checks rest on.
    pub fn is_manifold_of(&self, faces: &[usize]) -> bool {
        let mut used: HashMap<(u32, u32), usize> = HashMap::new();
        for index in faces {
            for (from, to) in edges_of(&self.faces[*index].indices) {
                let key = if from < to { (from, to) } else { (to, from) };
                *used.entry(key).or_insert(0) += 1;
            }
        }
        used.values().all(|count| *count == 2)
    }

    /// Returns the face indices of one solid.
    pub fn faces_of(&self, solid: SolidId) -> Vec<usize> {
        self.faces
            .iter()
            .enumerate()
            .filter_map(|(index, face)| (face.element.solid() == solid).then_some(index))
            .collect()
    }

    /// Returns a solid's signed volume; positive means it points outward.
    pub fn solid_signed_volume(&self, solid: SolidId) -> f32 {
        self.signed_volume_of(&self.faces_of(solid))
    }

    /// Returns a solid's signed plan area.
    pub fn solid_plan_area(&self, solid: SolidId) -> f32 {
        self.plan_area_of(&self.faces_of(solid))
    }

    /// Returns how many boundary loops a solid has; zero means it is closed.
    pub fn solid_boundary_loops(&self, solid: SolidId) -> usize {
        self.boundary_loops_of(&self.faces_of(solid))
    }

    /// Returns a solid's Euler characteristic; a closed solid is 2.
    pub fn solid_euler_characteristic(&self, solid: SolidId) -> i64 {
        self.euler_characteristic_of(&self.faces_of(solid))
    }

    /// Returns whether a solid is edge-manifold.
    pub fn solid_is_manifold(&self, solid: SolidId) -> bool {
        self.is_manifold_of(&self.faces_of(solid))
    }

    /// Returns per-corner normals, smooth within an authored surface and hard between them.
    pub fn corner_normals(&self) -> CornerNormals {
        CornerNormals::of(self)
    }
}

/// The three directed edges of a triangle, in its winding order.
fn edges_of(corners: &[u32; 3]) -> [(u32, u32); 3] {
    [
        (corners[0], corners[1]),
        (corners[1], corners[2]),
        (corners[2], corners[0]),
    ]
}

/// Per-corner normals, grouped by the authored surface each corner belongs to.
///
/// # Invariants
///
/// The grouping key is the face's [`Element`], so two facets of one roof plane share their normals
/// at a shared vertex and a facet of the next plane does not: the crease falls where the author put
/// an edge, which is what makes a roof read as planes rather than as a faceted sheet. Accumulation
/// is area-weighted — an unweighted mean of a fan of slivers would be dominated by the smallest
/// triangle it happens to hold.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct CornerNormals {
    per_face: Vec<[Vec3; 3]>,
}

impl CornerNormals {
    /// Builds the per-corner normals of a mesh.
    pub fn of(mesh: &BuildingMesh) -> Self {
        let mut accum: HashMap<(u32, Element), Vec3> = HashMap::new();
        for face in &mesh.faces {
            let [a, b, c] = mesh.triangle(face);
            let normal = cross3(subtract(b, a), subtract(c, a));
            for index in face.indices {
                let entry = accum.entry((index, face.element)).or_insert([0.0; 3]);
                entry[0] += normal[0];
                entry[1] += normal[1];
                entry[2] += normal[2];
            }
        }
        let per_face = mesh
            .faces
            .iter()
            .map(|face| {
                face.indices.map(|index| {
                    let sum = accum
                        .get(&(index, face.element))
                        .copied()
                        .unwrap_or([0.0, 1.0, 0.0]);
                    crate::math::normalized3(sum)
                })
            })
            .collect();
        Self { per_face }
    }

    /// Returns the normal at one corner of one face.
    pub fn at(&self, face: usize, corner: usize) -> Vec3 {
        self.per_face[face][corner.min(2)]
    }

    /// Returns how many faces the normals cover.
    pub fn len(&self) -> usize {
        self.per_face.len()
    }

    /// Returns whether there are no normals.
    pub fn is_empty(&self) -> bool {
        self.per_face.is_empty()
    }
}
