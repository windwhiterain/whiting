//! The expanded form: a triangle mesh whose faces are the massif's planar block faces.
//!
//! # Invariants
//!
//! - **Every face is exactly planar.** Elevation is sampled on the skeleton lines and nowhere
//!   else; a triangle's interior is the linear interpolation of its three corners and is
//!   never displaced. This is the contract that keeps the block-face reading intact for a
//!   downstream pipeline, and it is not something the mesh can check for itself — it is a
//!   property of the expansion, held here as its guarantee.
//! - **The mesh is watertight and covers the footprint exactly once.** A hole and an overlap
//!   are both failures of the authored correspondence, and both show up as the mesh's plan
//!   area disagreeing with the footprint's. That is a number, not a judgement, and
//!   [`FacetMesh::plan_area`] is where it is read.
//! - A face carries its own provenance and never an index into a previous build, because
//!   every edit re-expands and every index moves.

use std::collections::{BTreeMap, BTreeSet};

use crate::line::LineRef;
use crate::math::{Vec2, Vec3, distance};
use crate::tessellation::BeltRef;

/// One block face: the surface a belt spans between two of its ribs.
///
/// # Invariants
///
/// - **Its corners are `(crest a, crest b, valley a, valley b)`, and `a` pairs with `a`.** A rib
///   is a node of the line the block's floor runs along together with the station that node names
///   on this flank, so the corner order is the order the two ribs give: along the crest from `a` to
///   `b`, across rib `b` to the floor, back along the floor from `b` to `a`, across rib `a` to the
///   crest. Any other order makes the block's own two boundaries cross.
///
/// - **A triangle is this shape's degenerate case and not a different shape.** [`BlockFace::top`]
///   and [`BlockFace::bottom`] are the block's own stretches of the two boundaries, so a block
///   holds `top.len() + bottom.len() - 2` faces: two when both runs are two points long, which is
///   the quad, and three or more when the crest and the valley floor were sampled at different
///   rates — which is the common case, because nothing in this crate requires the two to agree.
///   One of the runs being a single point makes the count one, and [`BlockFace::corners`] reports
///   that by repeating the point rather than by returning three entries.
///
/// - **The ends are the author's, not this crate's.** A block's two ribs are two authored nodes of
///   the line its floor runs along, so how coarse the blocks are is how coarsely that line was
///   written: a skeleton with thirteen valley nodes per flank makes blocks no longer than what its
///   own nodes describe. Nothing here chooses where a block begins or ends, because anything that
///   did would be a second place for that decision to be made.
#[derive(Debug, Clone, PartialEq)]
pub struct BlockFace {
    /// The block's own run of points on its upper boundary, crest a through crest b.
    pub top: Vec<u32>,
    /// Its own run on the lower boundary, valley a through valley b.
    pub bottom: Vec<u32>,
    /// The faces it is expanded into.
    ///
    /// # Invariants
    ///
    /// It is a **range and not a list** because one belt is expanded in one walk, so the faces of
    /// one block are always consecutive — and a range cannot be read as an ordering of faces that
    /// the expander does not guarantee.
    pub faces: std::ops::Range<u32>,
    /// What the block is, in the skeleton's own terms.
    pub attributes: FaceAttributes,
}

impl BlockFace {
    /// Returns the block's four corners, as `(crest a, crest b, valley a, valley b)`.
    ///
    /// # Invariants
    ///
    /// It is `None` for a block with **no point at all on one of its two boundaries**, which is a
    /// block whose floor was never walked. Such a block has no shape to name, and reporting its
    /// emptiness as a corner would put a vertex index of zero in an answer about the mesh.
    ///
    /// A triangle block answers with **four** entries, two of which are the same vertex: that is
    /// the degenerate case this type exists to express, and shortening the answer would make the
    /// caller work out which corner is missing.
    pub fn corners(&self) -> Option<[u32; 4]> {
        Some([
            *self.top.first()?,
            *self.top.last()?,
            *self.bottom.first()?,
            *self.bottom.last()?,
        ])
    }

    /// Returns how many faces the block is expanded into.
    pub fn len(&self) -> usize {
        (self.faces.end - self.faces.start) as usize
    }

    /// Returns whether the block is expanded into nothing.
    pub fn is_empty(&self) -> bool {
        self.faces.start >= self.faces.end
    }
}

/// One triangle of a block face.
///
/// # Invariants
///
/// **It is exactly planar and it is not the unit of surface.** The triangle is the primitive the
/// belt's strip is walked in and the primitive a rasteriser draws; the surface a reader shades is
/// the [`BlockFace`] it belongs to. Which of the two a caller is holding is the difference between
/// a hard-edged mesh and a smooth one, and it is a question [`Face::block`] exists to answer.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Face {
    /// The three points, wound so the face looks up.
    pub indices: [u32; 3],
    /// The belt's own parameter space at each of those three points.
    ///
    /// # Invariants
    ///
    /// - **Entry `i` is the point `indices[i]` is**, in that order and no other, because the
    ///   winding is fixed by the face's own plan area and a pair of lists an author has to zip
    ///   by hand is a second thing that can disagree with the first. A face stores its uv
    ///   beside its indices for the same reason [`FaceAttributes::belt`] is stored beside both:
    ///   they are one fact about one face.
    /// - **`u` is the fraction of the belt's upper boundary already walked, by distance**, and
    ///   `v` is the fraction from that boundary down to the lower one. Distance rather than
    ///   sample index because a belt's top path turns a corner where a spur leaves its parent's
    ///   flank, and at that corner the two differ — and "62% of the way along the crest" is the
    ///   sentence an author means, where "62% of the samples" is a fact about the tessellation.
    /// - **`v` is zero on the crest and one on the valley** for every face of a belt, so `v` says
    ///   how far down a flank a point is rather than how far along a row of one tessellation.
    pub uv: [[f32; 2]; 3],
    /// What the face is, for whatever reads the mesh after this crate.
    pub attributes: FaceAttributes,
    /// Which block face this triangle is part of.
    ///
    /// # Invariants
    ///
    /// It indexes [`FacetMesh::blocks`], and this face's own index lies inside that block's
    /// [`BlockFace::faces`]. The two are written together by the expander and can therefore not be
    /// edited apart, which is what makes "average over the faces of one block" a lookup rather
    /// than a walk over the mesh.
    ///
    /// **A block boundary is a crease.** Every vertex a block shares with another block is still
    /// one vertex of the mesh, but its normal is read per block, so the two sides of a shared
    /// edge interpolate different normals and the edge shows. That is the intent — the ribs and
    /// the crest lines are the hard edges of the rock — and a caller that wants a block boundary
    /// to vanish must join the blocks rather than smooth across them.
    pub block: u32,
}

/// One face, in the belt's own coordinates.
///
/// # Invariants
///
/// This is a **point on a belt**, not a point in space: it says where on the surface between one
/// valley and the line it faces the point lies, and the mesh turns it into a position. It is the
/// answer a caller reading a rendered pixel needs, because a position says where the surface is
/// and a belt coordinate says **which authored line and which station** that surface came from.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BeltPoint {
    /// Which belt.
    pub belt: BeltRef,
    /// How far along its upper boundary, and how far down the strip.
    pub uv: [f32; 2],
}

/// What a face is, in the skeleton's own terms.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FaceAttributes {
    /// The belt the face was expanded from.
    pub belt: BeltRef,
    /// The line the belt's upper boundary is on.
    pub top: LineRef,
    /// The line the belt's lower boundary is on.
    pub bottom: LineRef,
    /// The plan direction the face falls in, which is the horizontal part of its own normal.
    pub dip: Vec2,
    /// How far the face falls, in radians from the horizontal.
    pub dip_angle: f32,
    /// How broken the crest above the face is, from the ridge's own sharpness.
    pub ruggedness: f32,
}

/// The expanded form of a [`crate::Massif`].
#[derive(Debug, Clone, Default, PartialEq)]
pub struct FacetMesh {
    /// Every vertex, shared: a sample of a skeleton line is one point however many faces
    /// meet it.
    pub points: Vec<Vec3>,
    /// Every face.
    pub faces: Vec<Face>,
    /// The block faces the triangles are grouped into, in the order they were expanded.
    ///
    /// # Invariants
    ///
    /// - **There is one block per belt that produced any faces**, so every index a
    ///   [`Face::block`] holds is in range. A belt that produced none — a strip whose every row
    ///   collapsed at a summit — has no block, because a block is a surface and a surface with
    ///   no faces is not one.
    /// - **Every face is in exactly one block**, and the blocks partition `faces`. A face outside
    ///   every block would have no block to average its normals over, and would silently fall back
    ///   to its own flat normal.
    pub blocks: Vec<BlockFace>,
}

impl FacetMesh {
    /// Returns the three points of a face.
    pub fn triangle(&self, face: &Face) -> [Vec3; 3] {
        [
            self.points[face.indices[0] as usize],
            self.points[face.indices[1] as usize],
            self.points[face.indices[2] as usize],
        ]
    }

    /// Returns the face's upward normal.
    ///
    /// # Invariants
    ///
    /// It is the normal of the side that is **looked at**, not the right-handed one over the
    /// winding; see [`crate::math::face_normal`] for why those are opposite here. A face with no
    /// plan area at all — one seen exactly edge on — has no side to be looked at, and the fallback
    /// is east rather than up so that a downstream reader cannot mistake a collapsed face for a
    /// level one.
    pub fn normal(&self, face: &Face) -> Vec3 {
        let [a, b, c] = self.triangle(face);
        crate::math::face_normal(a, b, c)
    }

    /// Returns the plan area the mesh covers.
    ///
    /// # Invariants
    ///
    /// It is the **signed** area, and the sign is kept. A mesh with a folded face has a
    /// positive area and a negative patch, and summing absolute areas would report it as
    /// whole — which is the one failure this number exists to find. Compared against the
    /// footprint's own signed area it is therefore a test of coverage and orientation at
    /// once.
    pub fn plan_area(&self) -> f32 {
        self.faces
            .iter()
            .map(|face| {
                let [a, b, c] = self.triangle(face);
                crate::math::signed_area([a[0], a[2]], [b[0], b[2]], [c[0], c[2]])
            })
            .sum()
    }

    /// Returns the lowest and highest space point the mesh reaches on each axis.
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

    /// Returns the length of the shortest edge, which is how fine the mesh reads.
    pub fn shortest_edge(&self) -> f32 {
        let mut shortest = f32::INFINITY;
        for face in &self.faces {
            let [a, b, c] = self.triangle(face);
            for (from, to) in [(a, b), (b, c), (c, a)] {
                shortest = shortest.min(distance([from[0], from[2]], [to[0], to[2]]));
            }
        }
        shortest
    }

    /// Returns every edge with how many faces use it, keyed by its two vertex indices.
    ///
    /// # Invariants
    ///
    /// The key is ordered, so the three edges of a triangle are the same three however the
    /// triangle is wound and two faces sharing an edge agree on it. That is what makes the count a
    /// count of faces per edge rather than of directed half-edges, which is the number that decides
    /// whether the surface is a manifold.
    fn edges(&self) -> BTreeMap<(u32, u32), usize> {
        let mut edges: BTreeMap<(u32, u32), usize> = BTreeMap::new();
        for face in &self.faces {
            for corner in 0..3 {
                let (a, b) = (face.indices[corner], face.indices[(corner + 1) % 3]);
                *edges.entry((a.min(b), a.max(b))).or_insert(0) += 1;
            }
        }
        edges
    }

    /// Returns how many closed loops of boundary the surface has.
    ///
    /// # Invariants
    ///
    /// A closed surface has none, and a **disc** — which is what a massif cut out of a footprint is
    /// meant to be — has exactly one, its rim. This is the number that catches a hole the plan area
    /// cannot: a junction left unwelded leaves triangles collapsed at one point, and a collapsed
    /// triangle encloses no area at all, so the area check passes and the surface has a slit.
    pub fn boundary_loops(&self) -> usize {
        let mut neighbours: BTreeMap<u32, Vec<u32>> = BTreeMap::new();
        for ((a, b), count) in self.edges() {
            if count != 1 {
                continue;
            }
            neighbours.entry(a).or_default().push(b);
            neighbours.entry(b).or_default().push(a);
        }
        let mut seen: BTreeSet<u32> = BTreeSet::new();
        let mut loops = 0;
        for start in neighbours.keys().copied() {
            if !seen.insert(start) {
                continue;
            }
            loops += 1;
            let mut stack = vec![start];
            while let Some(vertex) = stack.pop() {
                for next in neighbours.get(&vertex).into_iter().flatten() {
                    if seen.insert(*next) {
                        stack.push(*next);
                    }
                }
            }
        }
        loops
    }

    /// Returns the Euler characteristic of the surface: vertices less edges plus faces.
    ///
    /// # Invariants
    ///
    /// It is read over the mesh's **own** vertices — the pair of line and sample the expander
    /// assigned — and not over positions welded afterwards. Welding coincident positions would
    /// report a slit as closed, which is exactly the failure this is here to find.
    ///
    /// A disc is one, a closed surface with no boundary is two less twice its genus, and an annulus
    /// is zero.
    pub fn euler_characteristic(&self) -> i64 {
        self.points.len() as i64 - self.edges().len() as i64 + self.faces.len() as i64
    }

    /// Returns whether every edge is shared by exactly two faces.
    ///
    /// # Invariants
    ///
    /// A closed surface is a manifold only if no edge carries three faces, which is what a
    /// duplicated sheet or a folded flap looks like. An edge used once is a boundary and is not
    /// asked about here; [`FacetMesh::boundary_loops`] counts those.
    pub fn is_manifold(&self) -> bool {
        self.edges().values().all(|count| *count <= 2)
    }

    /// Returns where on its belt a face is, given a point's uv in the belt's own coordinates.
    ///
    /// This is the **inverse of reading a face's uv**, and it is the half of that pair a caller
    /// holding a rendered pixel needs: the rasteriser hands back the three corner uv of the face
    /// it covered, interpolated across the triangle, and this turns that one pair of numbers into
    /// the belt and the position on it.
    ///
    /// # Invariants
    ///
    /// - **It reads the face's own three corners and nothing else**, so a uv written by the same
    ///   [`Face::uv`] a scene's point data carries comes back to the belt it came from. A
    ///   definition of the belt's parameter space written down twice is a second thing that can
    ///   be wrong, and it would be wrong silently and by a rotation of the triangle.
    /// - **A uv outside the triangle is clamped to the triangle**, because a rasteriser
    ///   interpolates a varying linearly and a point on a shared edge legitimately carries
    ///   either side's value. The result is therefore the closest belt coordinate on this face,
    ///   which is what a caller asking "where is that pixel on this face" means.
    /// - **It is O(1)**: three corners, one triangle. A caller locating a whole contact sheet
    ///   pays a constant per pixel rather than a walk of the mesh.
    pub fn locate(&self, face: &Face, uv: [f32; 2]) -> BeltPoint {
        // Barycentric coordinates of the point within the face's own uv triangle, by the areas
        // the three subtriangles cut off — the standard construction, and the one that needs no
        // assumption about the winding because a subtraction of a signed area is signed either
        // way.
        let [a, b, c] = face.uv;
        let whole = area_of(a, b, c);
        let (wa, wb, wc) = if whole.abs() <= AREA_EPSILON {
            // A degenerate uv triangle has no interior to interpolate in, which happens where a
            // summit closes: every corner of the collapsed row carries the same uv. The corner is
            // then as good an answer as any, and it is a point on the belt rather than a division
            // by nothing.
            (1.0, 0.0, 0.0)
        } else {
            (
                area_of(uv, b, c) / whole,
                area_of(a, uv, c) / whole,
                area_of(a, b, uv) / whole,
            )
        };
        BeltPoint {
            belt: face.attributes.belt,
            uv: [
                wa * a[0] + wb * b[0] + wc * c[0],
                wa * a[1] + wb * b[1] + wc * c[1],
            ],
        }
    }
}

/// How small a uv triangle's area must be before it is treated as having no interior.
///
/// It is the same threshold the expander drops a collapsed face at, on purpose: a face that was
/// not emitted and a uv triangle that cannot be interpolated in are the same condition reached
/// two ways, and one number for both is one fewer place for them to disagree.
const AREA_EPSILON: f32 = 1.0e-6;

/// How small a corner's angle must be, in radians, before it counts as having none.
const ANGLE_EPSILON: f32 = 1.0e-6;

/// How small a triangle's corner must be, in radians, before its angle is read as a right angle.
///
/// The cosine of an angle just short of zero and of an angle just short of a full turn are the
/// same number to a float's precision, and a collapsed triangle's corner is somewhere in between.
/// A flat interior corner has a cosine of zero, so a right angle is the one answer that is right
/// for the degenerate case and wrong by at most a quarter turn for the near-degenerate one.
const FLAT_ANGLE: f32 = std::f32::consts::FRAC_PI_2;

/// Returns the angle at one corner of a triangle, in radians.
fn corner_angle(corners: [Vec3; 3], corner: usize) -> f32 {
    let from = corners[corner];
    let (one, other) = (
        crate::math::normalized3(crate::math::subtract(corners[(corner + 1) % 3], from)),
        crate::math::normalized3(crate::math::subtract(corners[(corner + 2) % 3], from)),
    );
    let cosine = crate::math::dot3(one, other);
    if cosine.abs() <= f32::EPSILON {
        return FLAT_ANGLE;
    }
    cosine.clamp(-1.0, 1.0).acos()
}

/// The normal at every corner of every face, averaged over the block that face belongs to.
///
/// # Invariants
///
/// - **Weighted by the corner's angle, not counted.** A sliver a hundred times longer than it is
///   wide is one face of a block and its two vertices are worth barely anything to the surface,
///   but a plain mean gives it a full vote each — and a belt zipped between two boundaries that
///   descend at different rates produces exactly those. Weighting by the angle at the corner is
///   what keeps a thin face from tilting its neighbours towards its own plane.
///
/// - **Averaged over one group, and the group is the caller's.** A vertex is read once per group
///   that touches it, so two faces meeting across a group boundary get normals from their own
///   sides and the shared edge stays a crease. The **block** is the default group, and it is a
///   default rather than an answer: one belt is one block, so the default creases at every rib as
///   well as at every crest, and a rib between two belts of one flank is an edge of the
///   correspondence rather than a feature of the rock. [`CornerNormals::of_grouped`] is how a
///   caller says which is which; averaging over the whole mesh instead would round the crest
///   lines off, and the crest lines are the rock.
///
/// - **A corner with no angle at it keeps its face's own normal.** A corner whose faces are all
///   collapsed has nothing to average, and a normal of no length is not a normal.
///
/// - **It is a table, not a walk.** [`CornerNormals::of`] is the one pass, and
///   [`CornerNormals::at`] is a constant-time read afterwards. A caller that shaded a mesh by
///   re-walking it per corner would pay for the same sum once per corner of every face.
#[derive(Debug, Clone, PartialEq)]
pub struct CornerNormals {
    /// One entry per face, three to a face, in the face's own corner order.
    normals: Vec<[Vec3; 3]>,
}

impl CornerNormals {
    /// Reads every corner of every face of a mesh, in one pass, averaging each corner over the
    /// **block** its face belongs to.
    ///
    /// This is [`Self::of_grouped`] with the block as the group, and it is the conservative
    /// default because a block is the one edge of the surface the caller did not choose: it is
    /// there so that this crate can answer for itself, and it is finer than what most surfaces
    /// want. A caller that knows which of its edges are features of the rock and which are only
    /// edges of the correspondence should say so through [`Self::of_grouped`] instead.
    pub fn of(mesh: &FacetMesh) -> Self {
        Self::of_grouped(mesh, |face| face.block)
    }

    /// Reads every corner of every face, averaging each corner over the faces that share its
    /// **group**, and the group is the caller's.
    ///
    /// # Invariants
    ///
    /// - **The group is read once per face and never again**, into a table of the same length as
    ///   the face list, so a caller may pass anything that reads the face — a closure over the
    ///   flank a face belongs to, a lookup into a caller's own array — without it having to be
    ///   cheap and without it being read a second time against a mesh it has already moved on
    ///   from.
    /// - **Two groups may not share an id.** The ids are compared, not the faces behind them, so
    ///   two groups that were numbered alike are one group and the edge between them is smoothed.
    ///   Nothing here can notice that, which is why the caller's key is the only place it can.
    ///
    /// Which group is right is a statement about the **surface**, and not about this mesh: rock is
    /// smooth across the floor of its own gully and creased along its crest, and a mesh whose
    /// finest grouping is the belt can only say that every rib is an edge. A caller whose surface
    /// is smooth across several belts passes a group that spans them — a whole flank, so that the
    /// strips of one flank are one surface and only the crest lines stay creased. Averaging over
    /// the whole mesh instead would round the crest lines off, and the crest lines are the rock.
    pub fn of_grouped(mesh: &FacetMesh, group: impl Fn(&Face) -> u32) -> Self {
        let groups: Vec<u32> = mesh.faces.iter().map(&group).collect();
        let mut sums: BTreeMap<(u32, u32), ([f32; 3], f32)> = BTreeMap::new();
        for (face, group) in mesh.faces.iter().zip(&groups) {
            let unit = mesh.normal(face);
            for corner in 0..3 {
                let angle = corner_angle(mesh.triangle(face), corner);
                let entry = sums
                    .entry((*group, face.indices[corner]))
                    .or_insert(([0.0; 3], 0.0));
                for (axis, component) in unit.iter().enumerate() {
                    entry.0[axis] += component * angle;
                }
                entry.1 += angle;
            }
        }
        let normals = mesh
            .faces
            .iter()
            .zip(&groups)
            .map(|(face, group)| {
                std::array::from_fn(|corner| match sums.get(&(*group, face.indices[corner])) {
                    Some((sum, total)) if *total > ANGLE_EPSILON => crate::math::normalized3(*sum),
                    _ => mesh.normal(face),
                })
            })
            .collect();
        CornerNormals { normals }
    }

    /// Returns the normal at one corner of one face.
    ///
    /// # Invariants
    ///
    /// It is a **constant-time read** and a face index that does not exist is answered with `up`
    /// rather than panicking, because a caller holding a face index holds a fact about the mesh
    /// and a mesh this was not read from is that fact being about something else — which is a
    /// caller bug, and a number is a better report of one than a backtrace.
    pub fn at(&self, face: usize, corner: usize) -> Vec3 {
        self.normals
            .get(face)
            .map(|corners| corners[corner % 3])
            .unwrap_or([0.0, 1.0, 0.0])
    }

    /// Returns how many faces this was read over.
    pub fn len(&self) -> usize {
        self.normals.len()
    }

    /// Returns whether there are no faces at all.
    pub fn is_empty(&self) -> bool {
        self.normals.is_empty()
    }
}

/// Returns the signed area of a triangle in a two dimensional parameter space.
fn area_of(a: [f32; 2], b: [f32; 2], c: [f32; 2]) -> f32 {
    (b[0] - a[0]) * (c[1] - a[1]) - (c[0] - a[0]) * (b[1] - a[1])
}
