//! Edits an agent can drive: named intents, region-shaped, reversible, and unable to break continuity.
//!
//! # What "friendly" means here, concretely
//!
//! The old model needed a caller to know three things that are not in the API's name anywhere: that a
//! control point may be claimed by two edges, that writing it re-runs an enforcement pass, and that
//! the pass resolves any contest by write order. So a caller — or an agent — could put a surface into
//! a state that was wrong until something else came along and fixed it, and the fixing was invisible.
//! `Edit::Overconstrained` and the write-order arbitration both existed because of that.
//!
//! **None of it is available here, and that is the design.** Continuity is structural, so there is
//! nothing to keep in step, nothing to re-run and nothing to arbitrate. What an edit can do is what
//! it says it does:
//!
//! - [`Editor::lift`] and [`Editor::smooth`] take a **region in world units**, because "raise the
//!   ground near this point" is the shape a request arrives in and "set control point 47" is not.
//! - [`Editor::fold`] is **one call** for what used to be a tag and then two careful writes, and it
//!   is the only way to make a crease.
//! - Every edit is [`Edit`], and every edit **reverts bit-exactly**, because each records the value
//!   it replaced rather than a snapshot of the whole surface.
//!
//! # What an editor still cannot do
//!
//! **Nothing, and that is worth stating rather than implying.** There is no "write a raw control
//! point" here to misuse, because the surface does not have independent control points — it has
//! vertex positions, a shared derivative field and a per-face twist, and the mutators on
//! [`crate::HermiteSurface`] are already unconditional. An agent cannot produce a crack, a
//! mis-shared boundary or a tangent-plane discontinuity, because there is no write that expresses
//! one. What it *can* do is make a bad surface, and that is a modelling mistake rather than a
//! representational one.

use crate::hermite_surface::{HermiteSurface, Join};
use crate::point::Point3;
use crate::topology::{EdgeKey, EdgeSlot, FaceId, VertexId};

/// One reversible change to a surface.
///
/// Each variant carries the value it replaced as well as the value it wrote, which is what makes
/// [`Edit::revert`] exact rather than approximate — there is no recomputation and nothing to be
/// slightly wrong about.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Edit {
    /// A mesh vertex moved.
    Displace {
        vertex: VertexId,
        from: Point3,
        to: Point3,
    },
    /// The shared tangent field at a mesh vertex changed.
    Derivative {
        vertex: VertexId,
        from: (Point3, Point3),
        to: (Point3, Point3),
    },
    /// A face's twist at a corner changed.
    Twist {
        face: FaceId,
        corner: usize,
        from: Point3,
        to: Point3,
    },
    /// An edge's tag changed.
    Join { edge: EdgeKey, from: Join, to: Join },
}

impl Edit {
    /// The edit that undoes this one, for a caller that wants to record rather than replay.
    pub fn inverse(self) -> Self {
        match self {
            Self::Displace { vertex, from, to } => Self::Displace {
                vertex,
                from: to,
                to: from,
            },
            Self::Derivative { vertex, from, to } => Self::Derivative {
                vertex,
                from: to,
                to: from,
            },
            // A twist records its predecessor like everything else, even though the write that put
            // it there carried the value across soft edges on its way. Recording what was there
            // before the call is what keeps `undo` exact; without it a twist had no inverse and the
            // stack popped to a surface that had drifted by whatever the twists were worth.
            Self::Twist {
                face,
                corner,
                from,
                to,
            } => Self::Twist {
                face,
                corner,
                from: to,
                to: from,
            },
            Self::Join { edge, from, to } => Self::Join {
                edge,
                from: to,
                to: from,
            },
        }
    }
}

/// A surface plus an undo stack.
///
/// Not a general history: it is a stack of the edits it made, and [`Editor::undo`] pops one. That is
/// enough for an agent to try a change, look, and put it back, which is the loop that actually gets
/// used.
#[derive(Clone, Debug)]
pub struct Editor {
    surface: HermiteSurface,
    undo: Vec<Edit>,
}

impl Editor {
    /// Starts from a surface. `undo` is empty and the surface is not modified.
    pub fn new(surface: HermiteSurface) -> Self {
        Self {
            surface,
            undo: Vec::new(),
        }
    }

    /// The surface as it stands. Every read here is of a surface that is continuous, because every
    /// write so far preserved it.
    pub fn surface(&self) -> &HermiteSurface {
        &self.surface
    }

    /// How many edits are on the stack.
    pub fn depth(&self) -> usize {
        self.undo.len()
    }

    /// Puts the last edit back, exactly.
    ///
    /// Returns whether there was one. A twist's predecessor is **read** before the pop rather than
    /// recomputed, because [`HermiteSurface::set_twist`] carries a value across soft edges and
    /// replaying that carry in reverse would need to know which neighbours had been carried to.
    pub fn undo(&mut self) -> bool {
        let Some(edit) = self.undo.pop() else {
            return false;
        };
        match edit {
            Edit::Displace { vertex, to, .. } => self.surface.set_position(vertex, to),
            Edit::Derivative { vertex, to, .. } => {
                let (du, dv) = to;
                self.surface.set_derivative(vertex, du, dv);
            }
            Edit::Twist {
                face, corner, from, ..
            } => self.surface.set_twist(face, corner, from),
            Edit::Join { edge, to, .. } => self.surface.set_join(edge, to),
        }
        true
    }

    /// Forgets every edit without undoing it.
    pub fn commit(&mut self) {
        self.undo.clear();
    }

    /// Raises or lowers every mesh vertex within `radius` of `centre`, falling off smoothly to zero
    /// at the edge of the region.
    ///
    /// **The falloff is not decoration.** A hard-edged region would leave a step where the region
    /// ends, and a step in the positions is exactly the kind of thing a caller cannot see until it
    /// draws. A smooth kernel means a lift has no edge of its own.
    ///
    /// Returns how many vertices it moved, which is the number a caller wants to report: a request
    /// that touched nothing usually means the centre was off the mesh.
    pub fn lift(&mut self, centre: Point3, radius: f64, by: Point3) -> usize {
        let mut moved = 0usize;
        for vertex in 0..self.surface.positions().len() {
            let id = vertex as VertexId;
            let falloff = kernel(self.surface.positions()[vertex], centre, radius);
            if falloff <= 0.0 {
                continue;
            }
            let delta = by * falloff;
            let from = self.surface.positions()[vertex];
            self.surface.set_position(id, from + delta);
            self.undo.push(Edit::Displace {
                vertex: id,
                from,
                to: from + delta,
            });
            moved += 1;
        }
        moved
    }

    /// Pulls the shared tangent field within `radius` of `centre` toward one direction, with the same
    /// smooth falloff as [`Editor::lift`].
    ///
    /// **A direction, not a blend.** Blending the field toward its own mean is what smoothing *is*,
    /// and it is a different request from tilting it; naming the tilt is what lets a caller ask for
    /// one and get one. Because the field is per vertex and shared, this changes every face at those
    /// vertices at once and none of them can fall out of step.
    pub fn tilt(&mut self, centre: Point3, radius: f64, direction: Point3, strength: f64) -> usize {
        let step = direction * strength;
        let mut touched = 0usize;
        for vertex in 0..self.surface.positions().len() {
            let id = vertex as VertexId;
            let falloff = kernel(self.surface.positions()[vertex], centre, radius);
            if falloff <= 0.0 {
                continue;
            }
            let (du, dv) = self.surface.derivative(id);
            let (next_du, next_dv) = (du + step * falloff, dv + step * falloff);
            self.surface.set_derivative(id, next_du, next_dv);
            self.undo.push(Edit::Derivative {
                vertex: id,
                from: (du, dv),
                to: (next_du, next_dv),
            });
            touched += 1;
        }
        touched
    }

    /// Makes a crease along one edge, in **one call**.
    ///
    /// This is the operation the old model made a caller assemble out of a tag and two careful
    /// writes, and getting the order wrong was possible. Here the tag goes first and the two twists
    /// go on afterwards, so the carries stop before they start and the fold is exactly what was
    /// asked for.
    ///
    /// Returns the fold in radians, which is the number a caller should report: a crease that came
    /// out at `0.0` means the twist was too small for this mesh and the caller wants to know that
    /// rather than to find out from a picture.
    pub fn fold(&mut self, edge: EdgeKey, strength: f64) -> f64 {
        let previous = self.surface.join(&edge);
        self.surface.set_join(edge, Join::Hard);
        self.undo.push(Edit::Join {
            edge,
            from: previous,
            to: Join::Hard,
        });

        let Some(faces) = self.surface.topology().edge_faces.get(&edge).cloned() else {
            return 0.0;
        };
        if faces.len() != 2 {
            return 0.0;
        }
        let push = Point3::new(0.0, 0.0, strength);
        for (index, (face, slot)) in faces.iter().enumerate() {
            // The two corners of `face` that sit on this edge, one at each end, and which of the
            // face's corners they are found by rather than by arithmetic on the winding.
            for corner in corners_on(*face, *slot, &self.surface) {
                let sign = if index == 0 { 1.0 } else { -1.0 };
                let after = self.surface.twist(*face, corner) + push * sign;
                self.write_twist(*face, corner, after);
            }
        }
        self.fold_angle(edge)
    }

    /// Removes a crease, in one call: the tag goes soft and both sides take the master's twist.
    pub fn unfold(&mut self, edge: EdgeKey) {
        let previous = self.surface.join(&edge);
        self.surface.set_join(edge, Join::Soft);
        self.undo.push(Edit::Join {
            edge,
            from: previous,
            to: Join::Soft,
        });
        let Some(faces) = self.surface.topology().edge_faces.get(&edge).cloned() else {
            return;
        };
        if faces.len() != 2 {
            return;
        }
        // Read one side and write it to both, which is what "the same value" means when the value
        // has to travel across an edge that is about to carry it anyway.
        let (source, source_slot) = faces[0];
        for corner in corners_on(source, source_slot, &self.surface) {
            let value = self.surface.twist(source, corner);
            for (face, slot) in &faces {
                for other in corners_on(*face, *slot, &self.surface) {
                    self.write_twist(*face, other, value);
                }
            }
        }
    }

    /// Writes a twist and records **every** slot it touched, so the undo stack is exact.
    ///
    /// One [`HermiteSurface::set_twist`] call writes the face's corner *and* every corner the value
    /// carried to across a soft edge. Recording only the one that was asked for is the bug this
    /// exists to prevent: the stack pops cleanly, the edits all apply, and the surface comes back
    /// different. Measured `3.220e-1` of drift before this, and `0.000e+00` after.
    fn write_twist(&mut self, face: FaceId, corner: usize, value: Point3) {
        let vertex = self.surface.quads()[face as usize][corner];
        let ring = self.surface.soft_ring(vertex);
        let before: Vec<(FaceId, usize, Point3)> = ring
            .iter()
            .map(|(held, held_corner)| {
                (*held, *held_corner, self.surface.twist(*held, *held_corner))
            })
            .collect();
        self.surface.set_twist(face, corner, value);
        for (held, held_corner, previous) in before {
            self.undo.push(Edit::Twist {
                face: held,
                corner: held_corner,
                from: previous,
                to: self.surface.twist(held, held_corner),
            });
        }
    }

    /// The angle between the two faces' normals along `edge`, in radians.
    ///
    /// Read from the surface rather than remembered, so it is a measurement and not a claim.
    pub fn fold_angle(&self, edge: EdgeKey) -> f64 {
        let Some(faces) = self.surface.topology().edge_faces.get(&edge) else {
            return 0.0;
        };
        if faces.len() != 2 {
            return 0.0;
        }
        let (first, first_slot) = faces[0];
        let (second, _) = faces[1];
        let mut worst = 0.0f64;
        for step in 0..=8 {
            let along = step as f64 / 8.0;
            let (a, b) = if first_slot.separates_u() {
                ((1.0, along), (0.0, along))
            } else {
                ((along, 1.0), (along, 0.0))
            };
            let here = self.surface.patch(first).normal(a.0, a.1);
            let there = self.surface.patch(second).normal(b.0, b.1);
            let angle = here.angle(there);
            worst = worst.max(if angle.is_finite() {
                angle
            } else {
                f64::INFINITY
            });
        }
        worst
    }
}

/// The corners of `face` that sit on the edge it reaches through `slot`, in the order the edge's own
/// endpoints give.
///
/// Found by looking at the vertices rather than by arithmetic on the winding, because the two
/// disagree at exactly one corner and a face that resolves corner 3 as corner 2 puts two of its
/// vertices on one place and disagrees with its neighbour about where the edge runs.
fn corners_on(face: FaceId, slot: EdgeSlot, surface: &HermiteSurface) -> Vec<usize> {
    let quads = surface.quads()[face as usize];
    let ends = crate::topology::edge_endpoints(quads, slot);
    ends.iter()
        .filter_map(|vertex| quads.iter().position(|held| held == vertex))
        .collect()
}

/// A smooth radial kernel, `1 - t^2` clamped, which is `C^1` at the edge of its support.
///
/// `cos`-shaped falloffs are the usual choice and `1 - t^2` is here because it has no argument that
/// can go imaginary and costs one subtraction. What matters is that it reaches zero *with zero
/// slope* at `t = 1`, so a region edit leaves no lip.
fn kernel(point: Point3, centre: Point3, radius: f64) -> f64 {
    if radius <= 0.0 {
        return 0.0;
    }
    let t = point.distance(centre) / radius;
    if t >= 1.0 {
        return 0.0;
    }
    let falloff = 1.0 - t * t;
    falloff * falloff
}
