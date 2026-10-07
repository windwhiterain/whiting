//! The expansion: the authored lines become a facet mesh, by walking the correspondence.
//!
//! # The shape of it
//!
//! Every valley node names the line on each of its two sides, so a valley is a run of ribs and
//! the surface between two consecutive ribs is one **belt**. A belt is bounded above by the
//! line its two stations name and below by the valley's own run between the two nodes, and the
//! mesh between them is the [strip](crate::zip) that walks both. There is no triangulation
//! step, no constraint to insert, and nothing to search: the correspondence is the mesh's
//! topology, written down by the author. See
//! [SPEC/mountain.md](../../../.agents/SPEC/mountain.md) R4.1.

use std::collections::{BTreeSet, HashMap};

use crate::facet::{Face, FaceAttributes, FacetMesh};
use crate::line::{LineRef, Side, Station};
use crate::math::{Vec3, span};
use crate::sampling::{Sampling, Unresolved, Vertex};
use crate::skeleton::{RidgeId, Skeleton, ValleyId};
use crate::tessellation::{BeltRef, Tessellation};
use crate::zip::{Step, steps};

/// The surface between one valley segment and the line it faces.
#[derive(Debug, Clone, PartialEq)]
pub struct Belt {
    /// The belt's name, which an override is recorded under.
    pub reference: BeltRef,
    /// The upper boundary, from the first station to the second.
    pub top: Vec<Vertex>,
    /// The lower boundary, being the valley's own vertices between the two nodes.
    pub bottom: Vec<Vertex>,
}

/// Why a skeleton could not be expanded.
#[derive(Debug, Clone, PartialEq)]
pub struct BuildError {
    /// Everything that was wrong, so an author sees all of it rather than the first.
    pub problems: Vec<Problem>,
}

/// One reason an expansion could not be made.
#[derive(Debug, Clone, PartialEq)]
pub enum Problem {
    /// A line has too few nodes to have a direction.
    TooFewNodes {
        /// The line.
        line: LineRef,
        /// How many nodes it has.
        nodes: usize,
    },
    /// There is no footprint, so the massif has no domain.
    NoFootprint,
    /// A station names a point nothing samples.
    Unresolved(Unresolved),
    /// A ridge has too few nodes to sample.
    ThinRidge {
        /// The ridge.
        ridge: RidgeId,
    },
    /// A valley has too few nodes to have a segment.
    ThinValley {
        /// The valley.
        valley: ValleyId,
    },
}

impl std::fmt::Display for BuildError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            formatter,
            "the skeleton does not expand: {} problem(s)",
            self.problems.len()
        )?;
        for problem in &self.problems {
            write!(formatter, "; {problem:?}")?;
        }
        Ok(())
    }
}

impl std::error::Error for BuildError {}

impl BuildError {
    /// Returns the problems as an error, or nothing when there were none.
    pub fn check(problems: Vec<Problem>) -> Result<(), Self> {
        if problems.is_empty() {
            Ok(())
        } else {
            Err(Self { problems })
        }
    }
}

/// Expands a skeleton into a facet mesh.
///
/// # Invariants
///
/// **Deterministic.** The same skeleton and tessellation always build the same mesh, vertex for
/// vertex and face for face: the belts are enumerated in id order, the steps of a strip are a
/// function of its two boundaries, and the only map read is the vertex table, whose insertion
/// order is the enumeration order. An agent iterates by diffing two meshes, and nondeterminism
/// would make every diff the whole mesh.
///
/// **Watertight, where the correspondence is complete.** Two faces meet along a rib because
/// they were built from the same two stations and therefore from the same two vertices, never
/// because two independent triangulations agreed. A hole is then not a triangulation failure
/// but an omission in the correspondence, which [`crate::validate`] names.
pub fn build(skeleton: &Skeleton, tessellation: Tessellation) -> Result<FacetMesh, BuildError> {
    let sampling = Sampling::of(skeleton, tessellation.clone());
    let (belts, mut problems) = enumerate(skeleton, &sampling);
    if skeleton.footprint.len() < 3 {
        problems.push(Problem::NoFootprint);
    }
    for line in skeleton.lines() {
        let nodes = skeleton.node_count(line);
        if nodes < 2 {
            problems.push(Problem::TooFewNodes { line, nodes });
        }
    }
    BuildError::check(problems)?;

    let mut mesh = FacetMesh::default();
    let mut table: HashMap<Vertex, u32> = HashMap::new();
    for belt in &belts {
        let top: Vec<Vec3> = belt.top.iter().map(|v| sampling.point(*v)).collect();
        let bottom: Vec<Vec3> = belt.bottom.iter().map(|v| sampling.point(*v)).collect();
        if top.is_empty() || bottom.is_empty() {
            continue;
        }
        let walked = steps(
            &top,
            &bottom,
            tessellation.diagonal,
            &forced_cells(&tessellation, belt.reference),
        );

        let ruggedness = mean_sharpness(&sampling, &belt.top);
        let top_line = belt.top[0].line;
        let bottom_line = belt.bottom[0].line;

        // The belt's own coordinates, normalised once per boundary rather than per face: `u` is
        // how far along the crest, by distance, and `v` is how far down the strip. See
        // `Face::uv`, which is where what these two mean is written down.
        let along_top = fractions(&top);
        let along_bottom = fractions(&bottom);

        let (mut high, mut low) = (0usize, 0usize);
        for step in walked {
            let corners = match step {
                Step::Top => {
                    let face = [
                        crest(belt.top[high], along_top[high]),
                        crest(belt.top[high + 1], along_top[high + 1]),
                        floor(belt.bottom[low], along_bottom[low]),
                    ];
                    high += 1;
                    face
                }
                Step::Bottom => {
                    let face = [
                        crest(belt.top[high], along_top[high]),
                        floor(belt.bottom[low], along_bottom[low]),
                        floor(belt.bottom[low + 1], along_bottom[low + 1]),
                    ];
                    low += 1;
                    face
                }
            };
            push_face(
                &mut mesh,
                &mut table,
                corners,
                &sampling,
                FaceAttributes {
                    belt: belt.reference,
                    top: top_line,
                    bottom: bottom_line,
                    dip: [0.0, 0.0],
                    dip_angle: 0.0,
                    ruggedness,
                },
            );
        }
    }
    Ok(mesh)
}

/// Returns every belt of a skeleton, in the one order they are enumerated in.
///
/// # Invariants
///
/// - **The order is valley id, then left side before right, then segment index**, and it is
///   written down **here**, once, rather than restated by each consumer. [`enumerate`] walks it,
///   and a caller that numbers the belts for any reason walks it too — so an id assigned from
///   this list and a mesh built from [`enumerate`] are the same numbering by construction rather
///   than by two loops that have been kept in step. A caller that wrote the triple loop out
///   itself would be right today and wrong the first time the order changed for a reason.
/// - **A belt a valley too short to have is absent**, because there is none to enumerate: a
///   valley of one node has no segment, and a belt of no segment is not a belt with an empty
///   range.
pub fn belt_references(skeleton: &Skeleton) -> Vec<BeltRef> {
    let mut references = Vec::new();
    for (index, valley) in skeleton.valleys.iter().enumerate() {
        if valley.nodes.len() < 2 {
            continue;
        }
        for side in [Side::Left, Side::Right] {
            for segment in 0..valley.nodes.len() - 1 {
                references.push(BeltRef {
                    valley: ValleyId(index),
                    side,
                    segment,
                });
            }
        }
    }
    references
}

/// Enumerates the belts of a skeleton, in a fixed order, with whatever stopped one being made.
///
/// # Invariants
///
/// The order is valley id, then left side before right, then segment index, and it is
/// [`belt_references`]'s rather than this function's — see there. Every consumer of the belt list
/// reads them in that one order, so none can see a different mesh from another.
pub fn enumerate(skeleton: &Skeleton, sampling: &Sampling) -> (Vec<Belt>, Vec<Problem>) {
    let mut belts = Vec::new();
    let mut problems = Vec::new();
    for (index, ridge) in skeleton.ridges.iter().enumerate() {
        if ridge.nodes.len() < 2 {
            problems.push(Problem::ThinRidge {
                ridge: RidgeId(index),
            });
        }
    }
    for (index, valley) in skeleton.valleys.iter().enumerate() {
        if valley.nodes.len() < 2 {
            problems.push(Problem::ThinValley {
                valley: ValleyId(index),
            });
        }
    }
    for reference in belt_references(skeleton) {
        let valley_id = reference.valley;
        let line = LineRef::Valley(valley_id);
        let segment = reference.segment;
        let from = skeleton.valleys[valley_id.0].nodes[segment].station(reference.side);
        let to = skeleton.valleys[valley_id.0].nodes[segment + 1].station(reference.side);
        let top = match sampling.path(from, to) {
            Ok(path) => path,
            Err(unresolved) => {
                problems.push(Problem::Unresolved(unresolved));
                continue;
            }
        };
        let bottom = match valley_run(skeleton, sampling, line, segment) {
            Some(run) => run,
            None => {
                problems.push(Problem::Unresolved(Unresolved::NoSuchSample {
                    station: Station::new(line, 0.0),
                }));
                continue;
            }
        };
        belts.push(Belt {
            reference,
            top,
            bottom,
        });
    }
    (belts, problems)
}

/// Returns the valley's own vertices between two consecutive nodes.
fn valley_run(
    skeleton: &Skeleton,
    sampling: &Sampling,
    line: LineRef,
    segment: usize,
) -> Option<Vec<Vertex>> {
    let from = sampling.node_vertex(skeleton, line, segment)?;
    let to = sampling.node_vertex(skeleton, line, segment + 1)?;
    // A node that stands at another line's vertex has no index on this line to walk from, so the
    // run is asked for by **sample index on the line the vertex belongs to** and the ends are
    // substituted by the sampler. That is what keeps a valley's first vertex the summit's own.
    let count = sampling.run(line).samples.len();
    let start = if from.line == line { from.index } else { 0 };
    let finish = if to.line == line {
        to.index
    } else {
        count.saturating_sub(1)
    };
    Some(sampling.run_between(line, start, finish))
}

/// Returns the cells of a belt the author has overridden.
fn forced_cells(tessellation: &Tessellation, belt: BeltRef) -> BTreeSet<usize> {
    tessellation
        .overrides
        .iter()
        .filter(|override_| override_.belt == belt)
        .map(|override_| override_.cell)
        .collect()
}

/// Returns the mean crest sharpness along a belt's upper boundary.
fn mean_sharpness(sampling: &Sampling, top: &[Vertex]) -> f32 {
    if top.is_empty() {
        return 0.0;
    }
    let total: f32 = top
        .iter()
        .map(|vertex| sampling.sample(*vertex).sharpness)
        .sum();
    total / top.len() as f32
}

/// One point of a strip, carrying the belt coordinate it is written at.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Corner {
    /// Which sample of which line this is.
    vertex: Vertex,
    /// The belt's own `(u, v)`: how far along the boundary, and how far down the strip.
    uv: [f32; 2],
}

/// Returns one point of a strip at the crest, where `v` is zero.
fn crest(vertex: Vertex, u: f32) -> Corner {
    Corner {
        vertex,
        uv: [u, 0.0],
    }
}

/// Returns one point of a strip at the valley, where `v` is one.
fn floor(vertex: Vertex, u: f32) -> Corner {
    Corner {
        vertex,
        uv: [u, 1.0],
    }
}

/// Returns, for each point of a boundary, the fraction of the boundary's **length** already walked.
///
/// # Invariants
///
/// - **By distance, not by index.** A belt's top path turns a corner where a spur leaves its
///   parent's flank, and at that corner the two disagree: the path walks a different line, and a
///   different line's samples are spaced by its own `step_scale`. An index fraction would make
///   `u` a fact about the tessellation, and the same belt re-tessellated would answer a
///   different `u` for the same piece of crest.
/// - **A boundary of one point is all zero**, rather than a division by a length of nothing. The
///   strip it belongs to has no interior to walk, so the number is never read.
fn fractions(boundary: &[Vec3]) -> Vec<f32> {
    if boundary.len() < 2 {
        return vec![0.0; boundary.len()];
    }
    let mut walked = Vec::with_capacity(boundary.len());
    let mut total = 0.0f32;
    for index in 1..boundary.len() {
        total += span(boundary[index - 1], boundary[index]);
        walked.push(total);
    }
    let scale = if total > 0.0 { 1.0 / total } else { 0.0 };
    let mut fractions = Vec::with_capacity(boundary.len());
    fractions.push(0.0);
    fractions.extend(walked.into_iter().map(|value| value * scale));
    fractions
}

/// Appends a face, wound to look up, and drops one that has collapsed.
///
/// # Invariants
///
/// The winding is fixed **by the face's own plan area** rather than by the order the strip
/// produced its corners: a strip walking one belt one way and the next the other would
/// otherwise leave half the massif's faces back to front, and a back face is one a rasterizer
/// culls. One unconditional rule instead of a case per boundary direction.
///
/// A face with no plan area is a belt row that collapsed — a corner where both stations name
/// one point, which is exactly what closes a summit and a spur junction. It is dropped rather
/// than emitted, because a degenerate triangle is a vertex the rasterizer still reads and a
/// normal it cannot compute.
fn push_face(
    mesh: &mut FacetMesh,
    table: &mut HashMap<Vertex, u32>,
    corners: [Corner; 3],
    sampling: &Sampling,
    attributes: FaceAttributes,
) {
    let points = corners.map(|corner| sampling.point(corner.vertex));
    let plan = points.map(|point| [point[0], point[2]]);
    let area = crate::math::signed_area(plan[0], plan[1], plan[2]);
    if area.abs() <= AREA_EPSILON {
        return;
    }
    // The winding is fixed here rather than by the order the strip produced the corners in, and
    // **the uv is flipped with it**. They are one list about one face: permuting the indices
    // without permuting the uv would leave every reversed face reporting a point on the far side
    // of the belt, which is exactly the kind of answer that looks plausible.
    let corners = if area > 0.0 {
        corners
    } else {
        [corners[0], corners[2], corners[1]]
    };
    let indices = corners.map(|corner| {
        *table.entry(corner.vertex).or_insert_with(|| {
            mesh.points.push(sampling.point(corner.vertex));
            (mesh.points.len() - 1) as u32
        })
    });
    let [a, b, c] = indices.map(|index| mesh.points[index as usize]);
    let normal = crate::math::face_normal(a, b, c);
    let mut attributes = attributes;
    attributes.dip = crate::math::normalized([-normal[0], -normal[2]]);
    attributes.dip_angle = normal[1].clamp(-1.0, 1.0).acos();
    mesh.faces.push(Face {
        indices,
        uv: corners.map(|corner| corner.uv),
        attributes,
    });
}

/// How small a face's plan area must be to be a collapse rather than a face.
///
/// # Invariants
///
/// It is an **area** and not a length, so it scales with the square of the massif, and it is
/// set far below the smallest face a sampled flank produces: two samples a hundredth of a unit
/// apart on a flank a unit wide are an area of a hundredth of a square unit, and a collapse is
/// exactly zero.
const AREA_EPSILON: f32 = 1.0e-6;

/// Returns the plan direction a face with this normal falls in.
///
/// # Invariants
///
/// It is the horizontal part of the normal, negated: a plane leaning to the east has a normal
/// leaning to the west, and the direction it falls is the east. The negation is the whole of
/// the definition and getting it the wrong way round gives every face a dip direction uphill,
/// which is a field that reads as plausible and is exactly backwards.
pub fn fall_line(normal: Vec3) -> [f32; 2] {
    crate::math::normalized([-normal[0], -normal[2]])
}
