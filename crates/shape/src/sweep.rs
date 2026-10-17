//! A spine plus a section law: the one primitive a trunk, a blade and an envelope are.
//!
//! # The one idea
//!
//! **A sweep is a spine and a law of the plane at each of its stations.** The section is a closed loop
//! in that plane, scaled along the spine by one [`Scalar`] and rotated in the plane by another, so
//! every shape a consumer asks for is a different section rather than a different code path:
//!
//! | what | spine | section | scale |
//! | --- | --- | --- | --- |
//! | a branch, a trunk, a culm | the branch's curve | [`Section::circle`] | the taper |
//! | a leaf blade | the midrib | [`Section::flat`] | the blade's width |
//! | an envelope, coarse | the trunk's curve | [`Section::circle`] | the crown's reach |
//! | an envelope, refined | the trunk's curve | [`Section::outline`] | the crown's reach |
//!
//! The last two rows are the superior's verdict made data — *a coarse envelope is a radius curve in the
//! curve's parameter space, refinable to a curve in the plane perpendicular to it* — and the point of
//! the table is that **refining an envelope is a change of section, not of type**. A sweep of an
//! outline is not a second primitive.
//!
//! # The section's plane, and its axes are stated
//!
//! `x` is the station frame's `normal` and `y` is its `binormal`, so a section wound counter-clockwise
//! in `(x, y)` faces along `+tangent` — [`crate::curve::Frame`]'s right-handed convention. Which way a
//! section's `x` points is therefore **the spine's** statement, made by
//! [`Curve::set_initial_normal`]: two sections on one plant cannot start their outlines at different
//! angles unless their spines were given different initial normals, and a caller that wants them to
//! agree writes one rule and applies it to both. That is the proposal's open question 2 answered, and
//! `sweep_probe` checks it against each spine's own frame rather than assuming it.
//!
//! # What each law means
//!
//! - `scale` multiplies **both** axes by one number, so a section keeps its aspect ratio as it tapers:
//!   a lens stays a lens, and a branch's cross-section stays round. A section that must change *shape*
//!   along the spine is an [`Section::outline`], not a second law here.
//! - `twist` is an angle in radians in the section's plane, positive from `x` toward `y`, and it is a
//!   law so a blade can roll along its own midrib.
//! - `capped` closes the two ends with a fan, which is the difference between a tube and a solid. Every
//!   section is a closed loop, so an uncapped sweep has two boundary loops and cannot be graded by
//!   [`crate::mesh::signed_volume`]; a capped one can.
//!
//! A station whose `scale` is `0` collapses its ring to a point. The caps are still built, so the
//! surface stays closed and only its geometry degenerates — that is a leaf's tip, and it is why the tip
//! needs no special case here.

use crate::curve::Curve;
use crate::loft::Loft;
use crate::scalar::Scalar;
use crate::section::Section;

/// A spine, a section, and the two laws that place the section at each station.
#[derive(Debug, Clone, PartialEq)]
pub struct Sweep {
    /// The curve the section is carried along.
    pub spine: Curve,
    /// The closed loop drawn in each station's plane.
    pub section: Section,
    /// The radius law: one number multiplying both axes of the section.
    pub scale: Scalar,
    /// The roll law: radians in the section's plane, from `x` toward `y`.
    pub twist: Scalar,
    /// Whether the two ends are closed by a fan.
    pub capped: bool,
}

impl Sweep {
    /// A sweep of `section` along `spine`: unscaled, untwisted, and capped.
    pub fn new(spine: Curve, section: Section) -> Self {
        Self {
            spine,
            section,
            scale: Scalar::constant(1.0),
            twist: Scalar::constant(0.0),
            capped: true,
        }
    }

    /// The loft of `stations` rings of `around` vertices each.
    ///
    /// # Invariants
    ///
    /// A ring's `u` is its station's arc-length fraction and slot `j`'s `v` is `j / around`, so a
    /// vertex's parameter is its station and its slot and nothing else — a consumer reads the pair from
    /// the same array as the position and cannot invent a second table.
    ///
    /// The section is sampled **once** and carried to every station. Its shape therefore cannot vary
    /// along the spine; only its scale and its twist can.
    pub fn sample(&self, stations: usize, around: usize) -> Loft {
        assert!(stations >= 2, "a sweep needs two rings to have a surface");
        assert!(
            around >= 3,
            "a section's loop needs three vertices to enclose an area"
        );
        let rings = self.spine.stations(stations);
        let loop_of_section = self.section.ring(around);
        let mut positions = Vec::with_capacity(stations * around);
        let mut uvs = Vec::with_capacity(stations * around);
        for station in &rings {
            let scale = self.scale.at(station.t);
            let twist = self.twist.at(station.t);
            let (sin, cos) = twist.sin_cos();
            for (slot, point) in loop_of_section.iter().enumerate() {
                let x = (point[0] * cos - point[1] * sin) * scale;
                let y = (point[0] * sin + point[1] * cos) * scale;
                let normal = station.frame.normal;
                let binormal = station.frame.binormal;
                positions.push([
                    station.position[0] + normal[0] * x + binormal[0] * y,
                    station.position[1] + normal[1] * x + binormal[1] * y,
                    station.position[2] + normal[2] * x + binormal[2] * y,
                ]);
                uvs.push([station.t, slot as f32 / around as f32]);
            }
        }
        let corners = corners(stations, around, self.capped);
        Loft::new(positions, uvs, corners, stations, around)
    }
}

/// Two triangles per grid cell, then a fan at each end when the sweep is capped.
///
/// # Invariants
///
/// The side winding is counter-clockwise seen from **outside** the solid, which
/// [`crate::mesh::signed_volume`] grades by being positive: for a ring counter-clockwise in `(x, y)`
/// the order `(slot, next, ahead_next)` faces outward, and reversing it turns every normal inward. The
/// fan reuses the end rings' own vertices, so `positions.len()` stays `rings * around` and a
/// degenerate ring costs degenerate triangles rather than degenerate indices.
fn corners(rings: usize, around: usize, capped: bool) -> Vec<[u32; 3]> {
    let capacity = 2 * (rings - 1) * around + if capped { 2 * (around - 2) } else { 0 };
    let mut corners = Vec::with_capacity(capacity);
    for ring in 0..rings - 1 {
        for slot in 0..around {
            let next = (slot + 1) % around;
            let start = (ring * around + slot) as u32;
            let ahead = ((ring + 1) * around + slot) as u32;
            let start_next = (ring * around + next) as u32;
            let ahead_next = ((ring + 1) * around + next) as u32;
            corners.push([start, start_next, ahead_next]);
            corners.push([start, ahead_next, ahead]);
        }
    }
    if capped {
        let last = (rings - 1) * around;
        for slot in 1..around - 1 {
            let next = slot + 1;
            corners.push([0, next as u32, slot as u32]);
            corners.push([last as u32, (last + slot) as u32, (last + next) as u32]);
        }
    }
    corners
}
