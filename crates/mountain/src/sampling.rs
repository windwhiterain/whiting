//! Turning the authored lines into the vertex runs the mesh is built from.
//!
//! # Why this is a pass of its own
//!
//! A station names a point on an edge, and a mesh vertex cannot be a point on an edge: it has
//! to be a vertex, or two faces built from the same station would build it twice at two
//! slightly different elevations. Sampling therefore makes every parameter that anything
//! names an actual sample, and everything downstream works in indices and never in
//! parameters. See [SPEC.align/mountain.md](../../../.agents/SPEC.align/mountain.md).

use std::collections::HashMap;

use crate::line::{LineRef, Node, Station};
use crate::math::{Vec2, Vec3, lift};
use crate::skeleton::{Skeleton, arc_lengths, elevation_at, point_at};
use crate::tessellation::Tessellation;

/// How far apart two parameters may be and still be one sample.
///
/// # Invariants
///
/// It is a **parameter** tolerance and not a distance, so it means "this is the same station"
/// and not "these are the same place". A station whose parameter is merged into a neighbour's
/// is still resolved, because resolution accepts a sample within this of the name and a merge
/// is by definition within it.
pub const PARAMETER_EPSILON: f32 = 1.0e-5;

/// How far apart two plan points may be and still be welded into one corner.
///
/// # Invariants
///
/// It is a **positional** tolerance, because the thing being decided is whether two lines
/// meet at a point. It is taken against a massif measured in hundreds of units, where a
/// hundredth is far below one pixel at any framing that shows the whole of it.
pub const WELD: f32 = 1.0e-2;

/// One vertex of a sampled line.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Sample {
    /// Arc-length parameter along the line.
    pub t: f32,
    /// Plan position.
    pub position: Vec2,
    /// Elevation.
    pub elevation: f32,
    /// Interpolated flank width, and zero on a line that carries none.
    pub half_width: f32,
    /// Interpolated crest sharpness, and zero on a line that carries none.
    pub sharpness: f32,
}

impl Sample {
    /// Returns the sample as a space point.
    pub fn point(&self) -> Vec3 {
        lift(self.position, self.elevation)
    }
}

/// A vertex of the mesh: one sample of one line.
///
/// # Invariants
///
/// The pair is the identity of a vertex. Two faces that name the same pair are naming the
/// same vertex, which is what makes the mesh watertight by construction rather than by two
/// runs of arithmetic agreeing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Vertex {
    /// The line the vertex is a sample of.
    pub line: LineRef,
    /// The sample's index in that line's run.
    pub index: usize,
}

/// A line's sampled run.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct SampledLine {
    /// The samples, in parameter order.
    pub samples: Vec<Sample>,
}

impl SampledLine {
    /// Returns the sample a parameter names, if the run holds one.
    ///
    /// # Invariants
    ///
    /// A **binary search** over a run in parameter order, accepting only a sample within
    /// [`PARAMETER_EPSILON`] of the name. A nearest-match with no bound would silently land
    /// on a distant sample for a parameter that was never sampled, and a rib would then be
    /// built where the author did not ask for one — a wrong mesh that no violation names.
    pub fn index_at(&self, t: f32) -> Option<usize> {
        let after = self.samples.partition_point(|sample| sample.t < t);
        [after.checked_sub(1), Some(after)]
            .into_iter()
            .flatten()
            .filter(|index| *index < self.samples.len())
            .find(|index| (self.samples[*index].t - t).abs() <= PARAMETER_EPSILON)
    }
}

/// Why a station could not be resolved to a vertex.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Unresolved {
    /// Nothing samples a point at that parameter on that line.
    NoSuchSample {
        /// The station that could not be resolved.
        station: Station,
    },
    /// Two lines a path has to walk between do not meet.
    Disjoint {
        /// The line the path was leaving.
        from: LineRef,
        /// The line the path was joining.
        to: LineRef,
    },
}

/// Every line's samples, and the plan index a corner is resolved through.
#[derive(Debug, Clone, Default)]
pub struct Sampling {
    ridges: Vec<SampledLine>,
    valleys: Vec<SampledLine>,
    footprint: SampledLine,
    /// Where each line's two ends stand, resolved. `None` means the end stands where its own node
    /// does; `Some(vertex)` means it stands at **that** vertex, which belongs to another line.
    ends: Vec<(Option<Vertex>, Option<Vertex>)>,
    welded: HashMap<(i64, i64), Vec<Vertex>>,
}

impl Sampling {
    /// Samples every line of a skeleton.
    ///
    /// # Invariants
    ///
    /// Every parameter that any station or line end names is a sample of the line it names. That is
    /// what lets a station be looked up rather than interpolated, and it is why two lines that name
    /// one point end up sharing one vertex: an end that stands on a station resolves to *that
    /// station's* sample, so eight ridges starting on one ridge's summit are one vertex and not
    /// eight that happen to coincide. A mesh built the other way has a slit at every junction.
    pub fn of(skeleton: &Skeleton, tessellation: Tessellation) -> Self {
        let step = tessellation.step.max(0.0);
        let mut ridges = Vec::with_capacity(skeleton.ridges.len());
        for (index, ridge) in skeleton.ridges.iter().enumerate() {
            let line = LineRef::Ridge(crate::skeleton::RidgeId(index));
            let named = station_parameters(skeleton, line);
            ridges.push(sample_line(
                &skeleton.nodes(line),
                step * ridge.step_scale.max(f32::EPSILON),
                &named,
            ));
        }

        let mut valleys = Vec::with_capacity(skeleton.valleys.len());
        for (index, valley) in skeleton.valleys.iter().enumerate() {
            let line = LineRef::Valley(crate::skeleton::ValleyId(index));
            let named = station_parameters(skeleton, line);
            valleys.push(sample_line(
                &skeleton.nodes(line),
                step * valley.step_scale.max(f32::EPSILON),
                &named,
            ));
        }

        let footprint = if skeleton.footprint.is_empty() {
            SampledLine::default()
        } else {
            let line = LineRef::Footprint;
            let named = station_parameters(skeleton, line);
            sample_line(&skeleton.nodes(line), step, &named)
        };

        let mut sampling = Self {
            ridges,
            valleys,
            footprint,
            ends: Vec::new(),
            welded: HashMap::new(),
        };
        sampling.ends = sampling.resolve_ends(skeleton);
        sampling.weld();
        sampling
    }

    /// Resolves every line's two ends to the vertex each stands at.
    ///
    /// # Invariants
    ///
    /// An end that names a station resolves to the **vertex of the line the station names**, which
    /// is what makes a junction one vertex rather than several that coincide. A chain of ends — one
    /// naming a line whose own end names a third — is followed to the far end of it, because
    /// stopping at the middle would put a copy of the middle line's vertex at the far line's
    /// coordinates and the junction would be two vertices again.
    fn resolve_ends(&self, skeleton: &Skeleton) -> Vec<(Option<Vertex>, Option<Vertex>)> {
        let count = self.count();
        let mut raw = vec![(None, None); count];
        for line in skeleton.lines() {
            for far in [false, true] {
                let Some(station) = skeleton.end(line, far).station() else {
                    continue;
                };
                let Some(vertex) = self.own_vertex(station) else {
                    continue;
                };
                let slot = self.slot(line);
                if far {
                    raw[slot].1 = Some(vertex);
                } else {
                    raw[slot].0 = Some(vertex);
                }
            }
        }
        let mut settled = raw.clone();
        for slot in 0..count {
            for far in [false, true] {
                let mut current = if far { raw[slot].1 } else { raw[slot].0 };
                // At most one step per line, so the walk cannot spin on a cycle of ends. A cycle
                // of ends is not a junction but an inconsistency, and `validate` reports it.
                for _ in 0..count {
                    let Some(vertex) = current else { break };
                    let samples = self.run(vertex.line).samples.len();
                    let at_far = vertex.index + 1 == samples;
                    if vertex.index != 0 && !at_far {
                        break;
                    }
                    let next = if at_far {
                        raw[self.slot(vertex.line)].1
                    } else {
                        raw[self.slot(vertex.line)].0
                    };
                    match next {
                        Some(followed) if followed != vertex => current = Some(followed),
                        _ => break,
                    }
                }
                if far {
                    settled[slot].1 = current;
                } else {
                    settled[slot].0 = current;
                }
            }
        }
        settled
    }

    /// Returns how many lines have a slot in the end table.
    fn count(&self) -> usize {
        self.ridges.len() + self.valleys.len() + 1
    }

    /// Returns a line's slot in the end table.
    fn slot(&self, line: LineRef) -> usize {
        match line {
            LineRef::Ridge(ridge) => ridge.0,
            LineRef::Valley(valley) => self.ridges.len() + valley.0,
            LineRef::Footprint => self.ridges.len() + self.valleys.len(),
        }
    }

    /// Returns the vertex a station names, ignoring end resolution.
    fn own_vertex(&self, station: Station) -> Option<Vertex> {
        self.run(station.line)
            .index_at(station.t)
            .map(|index| Vertex {
                line: station.line,
                index,
            })
    }

    /// Returns where an end of a line stands, if it stands at another line's vertex.
    fn end_vertex(&self, line: LineRef, far: bool) -> Option<Vertex> {
        let (start, end) = self.ends[self.slot(line)];
        if far { end } else { start }
    }

    /// Returns a line's run.
    pub fn run(&self, line: LineRef) -> &SampledLine {
        match line {
            LineRef::Ridge(ridge) => &self.ridges[ridge.0],
            LineRef::Valley(valley) => &self.valleys[valley.0],
            LineRef::Footprint => &self.footprint,
        }
    }

    /// Returns a sample.
    pub fn sample(&self, vertex: Vertex) -> &Sample {
        &self.run(vertex.line).samples[vertex.index]
    }

    /// Returns a vertex's space point.
    pub fn point(&self, vertex: Vertex) -> Vec3 {
        self.sample(vertex).point()
    }

    /// Returns how many samples a line has.
    pub fn len(&self, line: LineRef) -> usize {
        self.run(line).samples.len()
    }

    /// Returns whether no line has a sample.
    pub fn is_empty(&self) -> bool {
        self.ridges.iter().all(|run| run.samples.is_empty())
            && self.valleys.iter().all(|run| run.samples.is_empty())
            && self.footprint.samples.is_empty()
    }

    /// Returns the vertex a station resolves to.
    ///
    /// # Invariants
    ///
    /// A station landing on a line's **end** resolves to the vertex that end stands at. That is the
    /// whole of how a junction becomes one vertex: a rib naming a ridge's summit and the ridge's
    /// own start both resolve to the same sample of whichever line owns the summit, so the triangles
    /// around it share it and the ring closes.
    pub fn vertex(&self, station: Station) -> Result<Vertex, Unresolved> {
        let index = self
            .run(station.line)
            .index_at(station.t)
            .ok_or(Unresolved::NoSuchSample { station })?;
        Ok(self.settled(station.line, index))
    }

    /// Returns the vertex a sample index of a line settles to.
    fn settled(&self, line: LineRef, index: usize) -> Vertex {
        let samples = self.run(line).samples.len();
        let far = samples > 0 && index + 1 == samples;
        if index == 0 || far {
            if let Some(vertex) = self.end_vertex(line, far) {
                return vertex;
            }
        }
        Vertex { line, index }
    }

    /// Returns the vertex an authored node of a skeleton line resolves to.
    ///
    /// # Invariants
    ///
    /// A line's own nodes are samples by construction, so this is exact rather than nearest. It is
    /// what lets a belt's lower boundary be the valley the author drew instead of a re-sample of it
    /// — and it is where a line's two ends become another line's vertices, so a lower boundary that
    /// starts at a summit starts at the summit's own vertex.
    pub fn node_vertex(&self, skeleton: &Skeleton, line: LineRef, node: usize) -> Option<Vertex> {
        let nodes = skeleton.nodes(line);
        if node >= nodes.len() {
            return None;
        }
        if node == 0
            && let Some(vertex) = self.end_vertex(line, false)
        {
            return Some(vertex);
        }
        if node + 1 == nodes.len()
            && let Some(vertex) = self.end_vertex(line, true)
        {
            return Some(vertex);
        }
        let (span, total) = arc_lengths(&nodes);
        if total <= f32::EPSILON {
            return Some(self.settled(line, 0));
        }
        self.run(line)
            .index_at(span[node] / total)
            .map(|index| self.settled(line, index))
    }

    /// Returns a line's vertices between two of its samples, in walk order.
    ///
    /// # Invariants
    ///
    /// Every vertex run in this crate is built here, and that is what keeps end resolution from
    /// being something each caller has to remember: a run that reaches a line's end reaches the
    /// vertex that end stands at, whichever line that vertex belongs to.
    pub fn run_between(&self, line: LineRef, from: usize, to: usize) -> Vec<Vertex> {
        run_indices(from, to)
            .into_iter()
            .map(|index| self.settled(line, index))
            .collect()
    }

    /// Returns the sampled run between two stations, as vertices in walk order.
    ///
    /// # Invariants
    ///
    /// The run begins at the first station's point and ends at the second's, and its interior
    /// vertices are the line's own samples — so a belt's upper boundary is exactly the crest
    /// the author drew and not a chord across it. When the two stations name the same line
    /// the run is that line's samples between them, **in whichever direction the pair asks
    /// for**: the two valleys facing one ridge are free to subdivide it differently, and a
    /// run that only went forwards would forbid one of them.
    ///
    /// When the two name different lines, the run is followed through the point where they
    /// meet — a junction, a ridge tip meeting the rim, or a valley mouth. Two lines that do
    /// not meet is [`Unresolved::Disjoint`] rather than a path across the gap.
    pub fn path(&self, from: Station, to: Station) -> Result<Vec<Vertex>, Unresolved> {
        let missing = |station: Station| Unresolved::NoSuchSample { station };
        if from.line == to.line {
            let start = self.run(from.line).index_at(from.t).ok_or(missing(from))?;
            let end = self.run(to.line).index_at(to.t).ok_or(missing(to))?;
            return Ok(self.run_between(from.line, start, end));
        }

        let end = self.vertex(to)?;
        // The corner is the point where the two lines actually meet, and it is found by asking
        // which pair of ends weld rather than by measuring the plan. A walk from a station to a
        // station on another line has to leave the first at one of its ends and join the second at
        // one of its, and the only place it can do both is a point the two lines share. Aiming at
        // the nearer end instead is a fan-shaped rule: it is right while every crest runs outwards
        // from one summit, and it turns a walk back on its own summit as soon as a crest is one
        // segment of a longer chain.
        let aim = self.point(end);
        let (leaving, joining) =
            self.corner(from.line, to.line, aim)
                .ok_or(Unresolved::Disjoint {
                    from: from.line,
                    to: to.line,
                })?;

        let head = self.run(from.line).index_at(from.t).ok_or(missing(from))?;
        let tail = self.run(to.line).index_at(to.t).ok_or(missing(to))?;
        let mut path: Vec<Vertex> = self.run_between(from.line, head, leaving.index);
        // The corner is reached twice — once as the leaving line's end and once as the joining
        // line's sample — and at a junction those are the same vertex, so the second reading is
        // dropped. Kept, every belt at a corner would open with a zero-width row.
        let corner = self.settled(to.line, joining.index);
        let mut arc: Vec<Vertex> = self
            .arc_vertices(to.line, joining.index, tail)
            .into_iter()
            .map(|index| self.settled(to.line, index))
            .collect();
        if arc.first() == Some(&corner) {
            arc.remove(0);
        }
        path.extend(arc);
        Ok(path)
    }

    /// Returns the pair of samples where two lines meet, as the walk between them needs it.
    ///
    /// # Invariants
    ///
    /// A path that changes line can only do so at a point **both** lines own, and the only points
    /// a line offers to another are its two ends — a junction, a tip reaching the rim, a mouth. So
    /// the corner is found by asking which end of the leaving line has a sample welded onto the
    /// joining line, which is a fact about the document. Choosing the end nearer the point being
    /// aimed at is a measurement of the plan, and it is right for exactly one shape: a fan whose
    /// crests all run away from one summit. On a chain — a crest that is one segment of a longer
    /// ridge, which is what a hierarchy is made of — the walk often has to leave by the end that is
    /// *further* from its aim, and the measurement then turns it back on the summit and reports a
    /// corner that does not exist as `Disjoint`.
    ///
    /// When both ends weld the two lines close a lens, and the nearer one is the corner the walk
    /// reaches first. That is the rim's case, where a loop meets a line at one point and the seam
    /// makes that point both ends.
    ///
    /// **This is the corner rule for the whole crate.** [`crate::validate`] reads it too, rather
    /// than measuring the same thing a second way: two rules about where two lines meet is two
    /// rules that can disagree, and the disagreement shows up as a skeleton that builds and is
    /// reported broken, or the other way round.
    pub fn corner(&self, from: LineRef, to: LineRef, aim: Vec3) -> Option<(Vertex, Vertex)> {
        let last = self.run(from).samples.len().checked_sub(1)?;
        let mut found: Option<(Vertex, Vertex, f32)> = None;
        for index in [0, last] {
            let leaving = Vertex { line: from, index };
            let Some(joining) = self.welded_vertex(to, self.point(leaving)) else {
                continue;
            };
            let plan = self.point(leaving);
            let square = (plan[0] - aim[0]).powi(2) + (plan[2] - aim[2]).powi(2);
            if found.as_ref().is_none_or(|(_, _, best)| square < *best) {
                found = Some((leaving, joining, square));
            }
        }
        found.map(|(leaving, joining, _)| (leaving, joining))
    }

    /// Returns the parameters at which two lines meet, for a walk that leaves the first and joins
    /// the second. [`Sampling::corner`]'s answer, in the form the coverage check reads.
    pub fn corner_parameters(&self, from: LineRef, to: LineRef, aim: Vec3) -> Option<(f32, f32)> {
        let (leaving, joining) = self.corner(from, to, aim)?;
        Some((self.sample(leaving).t, self.sample(joining).t))
    }

    /// Returns whether a line is a loop.
    ///
    /// # Invariants
    ///
    /// The footprint is the only loop, and it is one because it is the boundary of the massif's
    /// domain rather than a line inside it. Everything else has two ends, which is what makes a
    /// ridge tip and a valley mouth places where a path changes line.
    pub fn is_closed(&self, line: LineRef) -> bool {
        line == LineRef::Footprint
    }

    /// Returns the sample indices of an arc between two of a line's samples.
    ///
    /// # Invariants
    ///
    /// On a closed line the arc is the **shorter** of the two, which is what stops the wedge that
    /// straddles the rim's seam from being walked the whole way round it. The run may therefore
    /// cross the seam, and it is the two pieces of the loop that the wrap joins — the sample at
    /// parameter one is the same vertex as the sample at parameter zero, so the second piece
    /// starts one past it rather than repeating it.
    fn arc_vertices(&self, line: LineRef, from: usize, to: usize) -> Vec<usize> {
        if !self.is_closed(line) {
            return run_indices(from, to);
        }
        let run = self.run(line);
        let last = run.samples.len() - 1;
        if crate::line::arc_runs_forwards(run.samples[from].t, run.samples[to].t, true) {
            if from <= to {
                (from..=to).collect()
            } else {
                (from..=last).chain(1..=to).collect()
            }
        } else if to <= from {
            (to..=from).rev().collect()
        } else {
            (0..=from).rev().chain((to..last).rev()).collect()
        }
    }

    /// Returns the vertex of a line that coincides with a corner point.
    fn welded_vertex(&self, line: LineRef, corner: Vec3) -> Option<Vertex> {
        let candidates = self.welded.get(&weld_key([corner[0], corner[2]]))?;
        // A welded point holds several lines' samples, because that is exactly what a corner
        // is. The one wanted is on the named line.
        candidates
            .iter()
            .copied()
            .find(|vertex| vertex.line == line)
    }

    /// Builds the plan-position index a corner is resolved through.
    ///
    /// # Invariants
    ///
    /// One hash lookup per corner rather than a scan of every line for a point that matches.
    /// Corners are the only place a position has to be looked up at all, so this is the whole of
    /// the index's work.
    fn weld(&mut self) {
        let mut entries: Vec<(Vec2, Vertex)> = Vec::new();
        let lines: Vec<LineRef> = self
            .ridges
            .iter()
            .enumerate()
            .map(|(index, _)| LineRef::Ridge(crate::skeleton::RidgeId(index)))
            .chain(
                self.valleys
                    .iter()
                    .enumerate()
                    .map(|(index, _)| LineRef::Valley(crate::skeleton::ValleyId(index))),
            )
            .chain(std::iter::once(LineRef::Footprint))
            .collect();
        for line in lines {
            for (index, sample) in self.run(line).samples.iter().enumerate() {
                entries.push((sample.position, Vertex { line, index }));
            }
        }
        for (position, vertex) in entries {
            self.welded
                .entry(weld_key(position))
                .or_default()
                .push(vertex);
        }
    }
}

/// Returns the plan-position key a vertex is welded under.
fn weld_key(position: Vec2) -> (i64, i64) {
    const SCALE: f32 = 1024.0;
    (
        (position[0] * SCALE).round() as i64,
        (position[1] * SCALE).round() as i64,
    )
}

/// Returns the indices from one sample to another, in that direction and inclusive.
fn run_indices(from: usize, to: usize) -> Vec<usize> {
    if from <= to {
        (from..=to).collect()
    } else {
        (to..=from).rev().collect()
    }
}

/// Returns every parameter some station names on a line.
fn station_parameters(skeleton: &Skeleton, line: LineRef) -> Vec<f32> {
    let mut named = Vec::new();
    for valley in &skeleton.valleys {
        for node in &valley.nodes {
            for station in [node.left, node.right] {
                if station.line == line {
                    named.push(station.t.clamp(0.0, 1.0));
                }
            }
        }
    }
    named
}

/// Samples one line: its nodes, every named parameter, and a subdivision between them.
///
/// # Invariants
///
/// A named parameter is a sample **exactly**, never a nearby one. The breakpoints are sorted
/// and deduplicated before the positions are read, so a parameter landing on a node, on
/// another parameter, or on a uniform step is one sample and every station naming it resolves
/// to it. Sampling each anchor separately and concatenating would put two vertices at one
/// point, which is the T-junction the correspondence exists to avoid.
///
/// **"Exactly" is positional and not only parametric.** A break that lands on one of the
/// line's own nodes is read from that node, because a parameter is a rounded `arc / total` and
/// the amount `edge_at` recovers from it does not come back as exactly zero or exactly one. An
/// interpolated read there is a ten-thousandth of a unit away from the node, which is nothing
/// on its own and is a missing shared vertex when a second line has the same authored point as
/// one of its own nodes.
pub fn sample_line(nodes: &[Node], step: f32, named: &[f32]) -> SampledLine {
    if nodes.len() < 2 {
        return SampledLine {
            samples: nodes
                .iter()
                .map(|node| Sample {
                    t: 0.0,
                    position: node.position,
                    elevation: node.elevation,
                    half_width: node.half_width,
                    sharpness: node.sharpness,
                })
                .collect(),
        };
    }
    let (span, total) = arc_lengths(nodes);
    if total <= f32::EPSILON {
        return SampledLine {
            samples: vec![Sample {
                t: 0.0,
                position: nodes[0].position,
                elevation: nodes[0].elevation,
                half_width: nodes[0].half_width,
                sharpness: nodes[0].sharpness,
            }],
        };
    }

    let mut breaks: Vec<f32> = span.iter().map(|along| along / total).collect();
    breaks.extend(named.iter().map(|t| crate::math::clamp(*t, 0.0, 1.0)));
    if step > f32::EPSILON {
        for index in 0..nodes.len() - 1 {
            let (low, high) = (span[index] / total, span[index + 1] / total);
            let divisions = (((high - low) * total / step).ceil() as usize).max(1);
            for division in 1..divisions {
                breaks.push(low + (high - low) * division as f32 / divisions as f32);
            }
        }
    }
    breaks.sort_by(f32::total_cmp);
    breaks.dedup_by(|left, right| (*left - *right).abs() <= PARAMETER_EPSILON);

    SampledLine {
        samples: breaks
            .into_iter()
            .map(|t| {
                let (index, amount) = edge_at(&span, total, t);
                let from = nodes[index];
                let to = nodes[index + 1];
                // A break that lands on a node is read **from that node** and not interpolated.
                // `edge_at` recovers the amount from a parameter that is itself a rounded
                // `span / total`, so at a node the amount lands a rounding error short of one
                // and the lerp returns a point about a ten-thousandth of a unit away from the
                // node it was supposed to be reading.
                //
                // That matters because a node is a vertex, and a vertex has to be one point and
                // not two nearly-equal ones. The case that breaks without this is a crest tip
                // that is also a rim vertex: both are nodes of their own line, both are read
                // back a hair off, and whether the two errors land in the same cell of the weld
                // index is arithmetic rather than intent. A massif whose crest tips meet its rim
                // is then a mesh with a crack at every tip, or a `Disjoint` from the walk that
                // was supposed to close the rim — decided by which side of a cell boundary a
                // float happened to fall on.
                let node = if amount <= PARAMETER_EPSILON {
                    Some(from)
                } else if amount >= 1.0 - PARAMETER_EPSILON {
                    Some(to)
                } else {
                    None
                };
                Sample {
                    t,
                    position: node.map_or_else(|| point_at(nodes, &span, t), |node| node.position),
                    elevation: node.map_or_else(
                        || elevation_at(nodes, &span, total, t),
                        |node| node.elevation,
                    ),
                    half_width: from.half_width + (to.half_width - from.half_width) * amount,
                    sharpness: from.sharpness + (to.sharpness - from.sharpness) * amount,
                }
            })
            .collect(),
    }
}

/// Returns the edge a parameter falls on and how far along it.
fn edge_at(span: &[f32], total: f32, t: f32) -> (usize, f32) {
    let target = crate::math::clamp(t, 0.0, 1.0) * total;
    let last = span.len() - 2;
    let index = span
        .partition_point(|length| *length <= target)
        .saturating_sub(1)
        .min(last);
    let (low, high) = (span[index], span[index + 1]);
    let width = high - low;
    let amount = if width > f32::EPSILON {
        (target - low) / width
    } else {
        0.0
    };
    (index, crate::math::clamp(amount, 0.0, 1.0))
}
