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
use crate::math::Vec3;
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

        let (mut high, mut low) = (0usize, 0usize);
        for step in walked {
            let corners = match step {
                Step::Top => {
                    let face = [belt.top[high], belt.top[high + 1], belt.bottom[low]];
                    high += 1;
                    face
                }
                Step::Bottom => {
                    let face = [belt.top[high], belt.bottom[low], belt.bottom[low + 1]];
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

/// Enumerates the belts of a skeleton, in a fixed order, with whatever stopped one being made.
///
/// # Invariants
///
/// The order is valley id, then left side before right, then segment index. Every consumer of
/// the belt list — the expander and the coverage check — reads them in that one order, so
/// neither can see a different mesh from the other.
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
        let valley_id = ValleyId(index);
        let line = LineRef::Valley(valley_id);
        if valley.nodes.len() < 2 {
            problems.push(Problem::ThinValley { valley: valley_id });
            continue;
        }
        for side in [Side::Left, Side::Right] {
            for segment in 0..valley.nodes.len() - 1 {
                let reference = BeltRef {
                    valley: valley_id,
                    side,
                    segment,
                };
                let from = valley.nodes[segment].station(side);
                let to = valley.nodes[segment + 1].station(side);
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
        }
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
    Some(if from.index <= to.index {
        (from.index..=to.index)
            .map(|index| Vertex { line, index })
            .collect()
    } else {
        (to.index..=from.index)
            .rev()
            .map(|index| Vertex { line, index })
            .collect()
    })
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
    corners: [Vertex; 3],
    sampling: &Sampling,
    attributes: FaceAttributes,
) {
    let points = corners.map(|vertex| sampling.point(vertex));
    let plan = points.map(|point| [point[0], point[2]]);
    let area = crate::math::signed_area(plan[0], plan[1], plan[2]);
    if area.abs() <= AREA_EPSILON {
        return;
    }
    let ordered = if area > 0.0 {
        corners
    } else {
        [corners[0], corners[2], corners[1]]
    };
    let indices = ordered.map(|vertex| {
        *table.entry(vertex).or_insert_with(|| {
            mesh.points.push(sampling.point(vertex));
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
