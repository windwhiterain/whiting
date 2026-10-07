//! The authored control structure: ridges, valleys and the footprint.
//!
//! This is the document an agent edits. Everything downstream — sampling, expansion,
//! validation — reads it and never writes to it, so an edit is always a change to a value
//! the author can see whole.

use crate::line::{LineRef, Node, Side, Station};
use crate::math::{Vec2, Vec3, distance, lift};

/// Index of a ridge in a [`Skeleton`]'s list.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct RidgeId(pub usize);

/// Index of a valley in a [`Skeleton`]'s list.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ValleyId(pub usize);

/// One node of a ridge polyline.
///
/// # Invariants
///
/// The attributes interpolate linearly along an edge, so a sample anywhere on an edge has a
/// flank width and a sharpness that are the two endpoints' mixed by the same amount their
/// positions are. A sample that took its attributes from the nearest node instead would
/// step at every node, which on a crest is a run of facets where the crest was meant to
/// turn.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RidgeNode {
    /// Plan position.
    pub position: Vec2,
    /// Elevation.
    pub elevation: f32,
    /// How far the flank reaches, as a hint to sampling and valley derivation.
    pub half_width: f32,
    /// `0.0` for a rounded crest, `1.0` for a knife-edge arete.
    pub sharpness: f32,
}

impl RidgeNode {
    /// Builds a node with the given position and elevation and no flank detail.
    pub fn new(position: Vec2, elevation: f32) -> Self {
        Self {
            position,
            elevation,
            half_width: 1.0,
            sharpness: 0.5,
        }
    }

    /// Sets the flank width hint.
    pub fn with_half_width(mut self, half_width: f32) -> Self {
        self.half_width = half_width;
        self
    }

    /// Sets the crest sharpness.
    pub fn with_sharpness(mut self, sharpness: f32) -> Self {
        self.sharpness = sharpness;
        self
    }
}

/// Where a child ridge joins its parent.
///
/// # Invariants
///
/// The child's first node sits on the parent polyline at plan arc-length parameter `t`, and
/// its position and elevation are read from the parent rather than stored. The join is
/// therefore structural: it cannot come adrift when the parent moves, and moving the
/// junction is editing one number.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Attach {
    /// The ridge this one grows out of.
    pub parent: RidgeId,
    /// Where on the parent, as the parent's plan arc-length parameter.
    pub t: f32,
}

/// A ridge line: one crest, summit-first.
#[derive(Debug, Clone, PartialEq)]
pub struct Ridge {
    /// The authored nodes, summit-first.
    pub nodes: Vec<RidgeNode>,
    /// Sampling density multiplier for this polyline.
    pub step_scale: f32,
    /// Where this ridge joins its parent, and `None` for a root ridge.
    pub attach: Option<Attach>,
}

impl Ridge {
    /// Builds a root ridge from its nodes.
    pub fn root(nodes: Vec<RidgeNode>) -> Self {
        Self {
            nodes,
            step_scale: 1.0,
            attach: None,
        }
    }

    /// Builds a ridge that grows out of a parent at a parameter.
    pub fn child(parent: RidgeId, t: f32, nodes: Vec<RidgeNode>) -> Self {
        Self {
            nodes,
            step_scale: 1.0,
            attach: Some(Attach { parent, t }),
        }
    }

    /// Sets the sampling density multiplier.
    pub fn with_step_scale(mut self, step_scale: f32) -> Self {
        self.step_scale = step_scale;
        self
    }
}

/// One node of a valley polyline.
///
/// # Invariants
///
/// `left` and `right` are the correspondence: the two lines this node sits between, and
/// therefore the two ribs that reach up the flanks from it. They are authored rather than
/// searched for, which is what makes expansion a walk. Both are read as the *sides of the
/// valley node's own direction*, so they swap if the valley is authored the other way round.
///
/// The node's own `position` and `elevation` are authored — a valley line is a control line
/// the author moves, not a value derived from the ridges — while the two stations are
/// derived positions on the lines they name. See [`Station`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ValleyNode {
    /// Plan position.
    pub position: Vec2,
    /// Elevation.
    pub elevation: f32,
    /// The line on this node's left, read along the valley's direction.
    pub left: Station,
    /// The line on this node's right, read along the valley's direction.
    pub right: Station,
}

impl ValleyNode {
    /// Builds a valley node between two lines.
    pub fn new(position: Vec2, elevation: f32, left: Station, right: Station) -> Self {
        Self {
            position,
            elevation,
            left,
            right,
        }
    }

    /// Returns the station on one side.
    pub fn station(&self, side: Side) -> Station {
        match side {
            Side::Left => self.left,
            Side::Right => self.right,
        }
    }

    /// Sets the station on one side.
    pub fn set_station(&mut self, side: Side, station: Station) {
        match side {
            Side::Left => self.left = station,
            Side::Right => self.right = station,
        }
    }
}

/// A valley line: the low control line between two ridges.
///
/// # Invariants
///
/// A valley is deliberately not a tree. The valley network is the hydrological dual of the
/// ridge tree and has its own branching, so forcing one structure on both would couple two
/// things the author controls separately. It carries no attributes beyond elevation and its
/// nodes' correspondence.
#[derive(Debug, Clone, PartialEq)]
pub struct Valley {
    /// The authored nodes, in the order the sides are read in.
    pub nodes: Vec<ValleyNode>,
    /// Sampling density multiplier for this polyline.
    pub step_scale: f32,
}

impl Valley {
    /// Builds a valley from its nodes.
    pub fn new(nodes: Vec<ValleyNode>) -> Self {
        Self {
            nodes,
            step_scale: 1.0,
        }
    }

    /// Sets the sampling density multiplier.
    pub fn with_step_scale(mut self, step_scale: f32) -> Self {
        self.step_scale = step_scale;
        self
    }
}

/// One node of the footprint.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FootprintNode {
    /// Plan position.
    pub position: Vec2,
    /// Elevation.
    pub elevation: f32,
}

impl FootprintNode {
    /// Builds a footprint node.
    pub fn new(position: Vec2, elevation: f32) -> Self {
        Self {
            position,
            elevation,
        }
    }
}

/// The control structure of a massif.
///
/// # Invariants
///
/// The footprint is stored **open**: its nodes are the corners of a closed loop and the edge
/// from the last back to the first is implied. Sampling closes it by repeating the first
/// node, so the loop's parameter `1.0` and its parameter `0.0` are the same point and a rib
/// that reaches one reaches both. Stored closed instead, the rim would have a seam that a
/// station list tiling `[0, 1]` would leave uncovered.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Skeleton {
    /// The ridge forest.
    pub ridges: Vec<Ridge>,
    /// The valley lines.
    pub valleys: Vec<Valley>,
    /// The closed boundary of the massif.
    pub footprint: Vec<FootprintNode>,
}

impl Skeleton {
    /// Builds a skeleton from its three parts.
    pub fn new(ridges: Vec<Ridge>, valleys: Vec<Valley>, footprint: Vec<FootprintNode>) -> Self {
        Self {
            ridges,
            valleys,
            footprint,
        }
    }

    /// Returns how many nodes a line holds.
    pub fn node_count(&self, line: LineRef) -> usize {
        match line {
            LineRef::Ridge(ridge) => self.ridges[ridge.0].nodes.len(),
            LineRef::Valley(valley) => self.valleys[valley.0].nodes.len(),
            LineRef::Footprint => self.footprint.len(),
        }
    }

    /// Returns one node of a line, in the form sampling reads it.
    ///
    /// The footprint's node after its last is its first, which is what closes the loop. Every
    /// other line returns `None` past its last node.
    pub fn node(&self, line: LineRef, index: usize) -> Option<Node> {
        match line {
            LineRef::Ridge(ridge) => self.ridges.get(ridge.0)?.nodes.get(index).map(Node::from),
            LineRef::Valley(valley) => self.valleys.get(valley.0)?.nodes.get(index).map(Node::from),
            LineRef::Footprint => {
                if self.footprint.is_empty() {
                    return None;
                }
                let index = if index == self.footprint.len() {
                    0
                } else {
                    index
                };
                self.footprint.get(index).map(Node::from)
            }
        }
    }

    /// Returns a line's nodes, with the footprint's loop closed.
    pub fn nodes(&self, line: LineRef) -> Vec<Node> {
        let mut nodes: Vec<Node> = (0..self.node_count(line))
            .filter_map(|index| self.node(line, index))
            .collect();
        if line == LineRef::Footprint && !nodes.is_empty() {
            nodes.push(nodes[0]);
        }
        nodes
    }

    /// Returns the plan position a station names, or `None` for a parameter off the line.
    pub fn station_position(&self, station: Station) -> Option<Vec2> {
        let nodes = self.nodes(station.line);
        if nodes.len() < 2 {
            return None;
        }
        let (span, _) = arc_lengths(&nodes);
        Some(point_at(&nodes, &span, station.t))
    }

    /// Returns the space position a station names, with its elevation.
    pub fn station_point(&self, station: Station) -> Option<Vec3> {
        let nodes = self.nodes(station.line);
        if nodes.len() < 2 {
            return None;
        }
        let (span, total) = arc_lengths(&nodes);
        let t = station.t.clamp(0.0, 1.0);
        let position = point_at(&nodes, &span, t);
        let elevation = elevation_at(&nodes, &span, total, t);
        Some(lift(position, elevation))
    }

    /// Returns the children of a ridge, ordered by where on the parent they join.
    ///
    /// # Invariants
    ///
    /// The order is the **parameter order and not the list order**, and ties are broken by
    /// the child's own id. Two children joining at one parameter is a legal skeleton — a
    /// crest that splits in two — and a walk that read them in list order would visit them
    /// in whatever order they happened to be authored, which is a mesh that changes when an
    /// unrelated line is inserted.
    pub fn children_of(&self, parent: RidgeId) -> Vec<(f32, RidgeId)> {
        let mut children: Vec<(f32, RidgeId)> = self
            .ridges
            .iter()
            .enumerate()
            .filter_map(|(index, ridge)| {
                let attach = ridge.attach?;
                (attach.parent == parent).then_some((attach.t, RidgeId(index)))
            })
            .collect();
        children.sort_by(|left, right| {
            left.0
                .total_cmp(&right.0)
                .then_with(|| left.1.cmp(&right.1))
        });
        children
    }

    /// Returns whether a ridge has any child joining it.
    pub fn has_children(&self, parent: RidgeId) -> bool {
        !self.children_of(parent).is_empty()
    }

    /// Returns the ids of every ridge, in list order.
    pub fn ridge_ids(&self) -> Vec<RidgeId> {
        (0..self.ridges.len()).map(RidgeId).collect()
    }

    /// Returns the ids of every valley, in list order.
    pub fn valley_ids(&self) -> Vec<ValleyId> {
        (0..self.valleys.len()).map(ValleyId).collect()
    }

    /// Returns every line this skeleton holds, in a fixed order.
    ///
    /// # Invariants
    ///
    /// The order is the [`LineRef`] order and not the order the lines were authored in, so
    /// two skeletons with the same lines list them alike and every walk over the lines is
    /// reproducible. That is what `build()`'s determinism rests on.
    pub fn lines(&self) -> Vec<LineRef> {
        let mut lines: Vec<LineRef> = self
            .ridge_ids()
            .into_iter()
            .map(LineRef::Ridge)
            .chain(self.valley_ids().into_iter().map(LineRef::Valley))
            .collect();
        if !self.footprint.is_empty() {
            lines.push(LineRef::Footprint);
        }
        lines
    }

    /// Returns the total plan length of a line.
    pub fn line_length(&self, line: LineRef) -> f32 {
        let nodes = self.nodes(line);
        arc_lengths(&nodes).1
    }

    /// Returns which side of a line, **at a parameter**, a plan point lies on.
    ///
    /// # Invariants
    ///
    /// The side is taken against the line's direction at that parameter and not against the
    /// line as a whole. A valley that bends round a ridge tip has its two sides meeting
    /// there, so a side read from the line's average direction would call both of the ribs at
    /// the tip the same side and report the ridge as half covered.
    ///
    /// The parameter is passed in rather than searched for, so this is `O(log n)` in the
    /// line's nodes. Searching for the nearest point instead would be `O(n)` and would be
    /// asked once per rib.
    pub fn side_of(&self, line: LineRef, t: f32, point: Vec2) -> Side {
        let nodes = self.nodes(line);
        if nodes.len() < 2 {
            return Side::Left;
        }
        let (span, _) = arc_lengths(&nodes);
        let direction = direction_at(&nodes, &span, t);
        let at = point_at(&nodes, &span, t);
        if crate::math::cross(direction, crate::math::difference(point, at)) >= 0.0 {
            Side::Left
        } else {
            Side::Right
        }
    }

    /// Returns the distance from a plan point to a station's point.
    pub fn distance_to(&self, station: Station, point: Vec2) -> Option<f32> {
        self.station_position(station).map(|at| distance(at, point))
    }
}

/// Returns a line's cumulative plan arc length at each node, and the total.
pub fn arc_lengths(nodes: &[Node]) -> (Vec<f32>, f32) {
    let mut span = Vec::with_capacity(nodes.len());
    let mut total = 0.0;
    span.push(0.0);
    for pair in nodes.windows(2) {
        total += distance(pair[0].position, pair[1].position);
        span.push(total);
    }
    (span, total)
}

/// Returns the plan point at an arc-length parameter.
pub fn point_at(nodes: &[Node], span: &[f32], t: f32) -> Vec2 {
    let (index, amount) = locate(nodes, span, t);
    crate::math::lerp(nodes[index].position, nodes[index + 1].position, amount)
}

/// Returns the elevation at an arc-length parameter.
pub fn elevation_at(nodes: &[Node], span: &[f32], _total: f32, t: f32) -> f32 {
    let (index, amount) = locate(nodes, span, t);
    nodes[index].elevation + (nodes[index + 1].elevation - nodes[index].elevation) * amount
}

/// Returns the direction of the edge a parameter falls on.
pub fn direction_at(nodes: &[Node], span: &[f32], t: f32) -> Vec2 {
    let (index, _) = locate(nodes, span, t);
    crate::math::normalized(crate::math::difference(
        nodes[index + 1].position,
        nodes[index].position,
    ))
}

/// Returns the parameter of the point on a line nearest a plan point, by projection on each
/// edge.
pub fn parameter_of(nodes: &[Node], span: &[f32], total: f32, point: Vec2) -> f32 {
    if total <= f32::EPSILON || nodes.len() < 2 {
        return 0.0;
    }
    let mut best = (f32::INFINITY, 0.0);
    for index in 0..nodes.len() - 1 {
        let (from, to) = (nodes[index].position, nodes[index + 1].position);
        let edge = crate::math::difference(to, from);
        let length_squared = crate::math::dot(edge, edge);
        let amount = if length_squared > f32::EPSILON {
            crate::math::clamp(
                crate::math::dot(crate::math::difference(point, from), edge) / length_squared,
                0.0,
                1.0,
            )
        } else {
            0.0
        };
        let at = crate::math::lerp(from, to, amount);
        let separation = distance(at, point);
        if separation < best.0 {
            let along = span[index] + (span[index + 1] - span[index]) * amount;
            best = (separation, along / total);
        }
    }
    best.1
}

/// Returns the edge index a parameter falls on and how far along that edge it is.
///
/// # Invariants
///
/// A **binary search** over the cumulative lengths, which are sorted because a polyline's arc
/// length only grows. Every parameter read on a line goes through here — a position, a
/// direction, an elevation — so a linear scan would put the node count of a line into the
/// cost of every sample the expander reads.
fn locate(nodes: &[Node], span: &[f32], t: f32) -> (usize, f32) {
    if nodes.len() < 2 {
        return (0, 0.0);
    }
    let total = span[span.len() - 1];
    if total <= f32::EPSILON {
        return (0, 0.0);
    }
    let target = crate::math::clamp(t, 0.0, 1.0) * total;
    let last = nodes.len() - 2;
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
