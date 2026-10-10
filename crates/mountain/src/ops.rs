//! Editing a massif.
//!
//! # Why the vocabulary is this granular
//!
//! These are the agent's verbs. An agent authors a massif by writing rust that calls them, reads
//! [`Massif::validate`], and looks at what [`Massif::build`] produced. A coarse "make it look
//! like a mountain" op would hide exactly the decisions the agent is supposed to be making, so
//! each of these does one thing and leaves the invariant it may have broken to the checker.
//!
//! # Invariants
//!
//! No op here refuses an edit for making the massif invalid, and none of them repairs one. An
//! edit may legitimately pass through an invalid state — lowering a summit before raising the
//! spur that hangs off it is two edits and one of them is wrong on its own — so a check that
//! fired per call would make the two-step edit inexpressible. Only the ops that would leave a
//! **dangling index** refuse, and they refuse by returning `None`.

use crate::Massif;
use crate::line::{LineRef, Side, Station};
use crate::math::Vec2;
use crate::skeleton::{End, Ridge, RidgeId, RidgeNode, Valley, ValleyId, ValleyNode};
use crate::tessellation::{BeltRef, DiagonalOverride, DiagonalRule};

impl Massif {
    /// Adds a root ridge, returning its id.
    pub fn add_ridge(&mut self, ridge: Ridge) -> RidgeId {
        self.skeleton.ridges.push(ridge);
        RidgeId(self.skeleton.ridges.len() - 1)
    }

    /// Adds a ridge that grows out of a parent at a parameter, returning its id.
    ///
    /// # Invariants
    ///
    /// The child's start is an **end naming a station** and its position is not placed here. It is
    /// derived from the station when the massif is sampled, so placing it would be a second copy of
    /// the same fact and the copy is the one that goes stale.
    ///
    /// **The parent is split at `t` first**, and the child attaches to the piece after the split. This
    /// is the whole of what the op has to do to be usable, and it is what it did not do: a child that
    /// merely named its parent at an interior parameter asked for a junction the expansion cannot
    /// build, because **a belt changes line only at a line's end** — the parent's line has no end
    /// there, so no two ends weld, and the massif comes apart at every junction of a hierarchy. The
    /// author's intent is right and the op was the thing lying about it.
    ///
    /// The two pieces are made a junction the way a hand-written one is: the piece downstream names
    /// the upstream's far end, so the point is named from both sides and neither is taken for a tip.
    pub fn attach_ridge(&mut self, parent: RidgeId, t: f32, nodes: Vec<RidgeNode>) -> RidgeId {
        let stand = self.split_ridge(parent, t).unwrap_or(parent);
        self.skeleton.ridges.push(Ridge::child(stand, 0.0, nodes));
        RidgeId(self.skeleton.ridges.len() - 1)
    }

    /// Cuts a ridge in two at a parameter, returning the piece **after** the cut.
    ///
    /// # Invariants
    ///
    /// The ridge keeps its id and its nodes up to the cut; a new ridge takes the rest and the original
    /// far end. `t` is resolved by **arc length**, not by node index, because that is what a station
    /// means everywhere else in this crate; a node is inserted at the cut when `t` does not land on
    /// one, interpolated along the segment it falls in, so the two pieces share a point exactly rather
    /// than nearly.
    ///
    /// Returns `None` and changes nothing when the cut would leave a piece with no length — a ridge
    /// cannot be split at its own end, and a caller asking for that wants `attach_ridge` at the end it
    /// already has.
    pub fn split_ridge(&mut self, ridge: RidgeId, t: f32) -> Option<RidgeId> {
        let target = t.clamp(0.0, 1.0);
        let mut spans: Vec<f32> = Vec::new();
        let mut total = 0.0;
        let count = {
            let nodes = &self.skeleton.ridges.get(ridge.0)?.nodes;
            if nodes.len() < 2 {
                return None;
            }
            for pair in nodes.windows(2) {
                let span = crate::math::distance(pair[0].position, pair[1].position);
                spans.push(span);
                total += span;
            }
            nodes.len()
        };
        if total <= f32::EPSILON {
            return None;
        }
        // The cut, as a distance along the ridge.
        let want = total * target;
        let mut walked = 0.0;
        let mut cut = count;
        for (index, span) in spans.iter().enumerate() {
            if want <= walked + span {
                cut = index + 1;
                break;
            }
            walked += span;
        }
        // At an end already: there is nothing to cut, and a caller asking for that wants the end it
        // already has rather than a piece of no length.
        if cut == 0 || cut >= count {
            return None;
        }
        // A node in the middle of a segment is inserted, so the two pieces meet on a point rather than
        // on a place between two of them.
        let within = (want - walked) / spans[cut - 1].max(f32::EPSILON);
        if within > 1.0e-4 && within < 1.0 - 1.0e-4 {
            let (before, after) = {
                let nodes = &self.skeleton.ridges[ridge.0].nodes;
                (nodes[cut - 1], nodes[cut])
            };
            let mixed = RidgeNode {
                position: [
                    before.position[0] + (after.position[0] - before.position[0]) * within,
                    before.position[1] + (after.position[1] - before.position[1]) * within,
                ],
                elevation: before.elevation + (after.elevation - before.elevation) * within,
                sharpness: before.sharpness + (after.sharpness - before.sharpness) * within,
            };
            self.insert_ridge_node(ridge, cut, mixed)?;
        }
        let tail = self.skeleton.ridges[ridge.0].nodes.split_off(cut);
        let original_end = self.skeleton.ridges[ridge.0].end;
        self.skeleton.ridges[ridge.0].end = End::Free;
        let piece = Ridge::root(tail).with_ends(End::On(Station::ridge(ridge, 1.0)), original_end);
        self.skeleton.ridges.push(piece);
        Some(RidgeId(self.skeleton.ridges.len() - 1))
    }

    /// Adds a **tie ridge**: one whose two ends both stand on other lines.
    ///
    /// # Invariants
    ///
    /// This is what makes the ridge set a network rather than a tree, and it is the only place a
    /// cycle can come from. Nothing distinguishes a tie from any other ridge afterwards — its ends
    /// are `End::On` like a spur's — and that is deliberate: a tie is a connection and not a kind
    /// of line.
    pub fn tie_ridge(&mut self, from: Station, to: Station, nodes: Vec<RidgeNode>) -> RidgeId {
        self.skeleton
            .ridges
            .push(Ridge::root(nodes).with_ends(End::On(from), End::On(to)));
        RidgeId(self.skeleton.ridges.len() - 1)
    }

    /// Sets where a ridge's start stands.
    pub fn set_ridge_start(&mut self, ridge: RidgeId, end: End) -> Option<()> {
        self.skeleton.ridges.get_mut(ridge.0)?.start = end;
        Some(())
    }

    /// Moves the junction where a ridge's start stands along the line it names.
    pub fn set_attach_parameter(&mut self, ridge: RidgeId, t: f32) -> Option<()> {
        let named = self.skeleton.ridges.get(ridge.0)?.start.station()?;
        self.skeleton.ridges.get_mut(ridge.0)?.start = End::On(Station::new(named.line, t));
        Some(())
    }

    /// Sets where either end of a line stands.
    pub fn set_end(&mut self, line: LineRef, far: bool, end: End) -> Option<()> {
        self.skeleton.set_end(line, far, end)
    }

    /// Adds a valley, returning its id.
    pub fn add_valley(&mut self, valley: Valley) -> ValleyId {
        self.skeleton.valleys.push(valley);
        ValleyId(self.skeleton.valleys.len() - 1)
    }

    /// Removes a ridge and every ridge that hangs off it, returning how many went.
    ///
    /// # Invariants
    ///
    /// The **subtree** goes with it and the ids of the survivors shift down, so every id an
    /// author holds is stale afterwards. That is why this returns the count rather than nothing:
    /// the caller is being told that its ids are gone, not that they were renumbered quietly.
    /// The valleys that named a removed ridge keep naming it, and the coverage check reports
    /// them — a station is the author's statement about where a flank ends, and a removal that
    /// rewrote one would be deciding that for them.
    pub fn remove_ridge(&mut self, ridge: RidgeId) -> usize {
        if ridge.0 >= self.skeleton.ridges.len() {
            return 0;
        }
        let mut doomed = vec![ridge];
        let mut index = 0;
        while index < doomed.len() {
            let parent = doomed[index];
            for candidate in 0..self.skeleton.ridges.len() {
                let id = RidgeId(candidate);
                if doomed.contains(&id) {
                    continue;
                }
                // A ridge hangs off another when one of its ends **names** it. Reading the ends and
                // not a parent field is what makes a tie ridge behave: removing either ridge it ties
                // leaves the other one standing, and neither of them is the other's parent.
                let hangs = [
                    self.skeleton.ridges[candidate].start,
                    self.skeleton.ridges[candidate].end,
                ]
                .into_iter()
                .filter_map(|end| end.station())
                .any(|station| station.line == LineRef::Ridge(parent));
                if hangs {
                    doomed.push(id);
                }
            }
            index += 1;
        }
        doomed.sort();
        for id in doomed.iter().rev() {
            self.skeleton.ridges.remove(id.0);
        }
        doomed.len()
    }

    /// Removes a valley.
    pub fn remove_valley(&mut self, valley: ValleyId) -> Option<Valley> {
        if valley.0 >= self.skeleton.valleys.len() {
            return None;
        }
        Some(self.skeleton.valleys.remove(valley.0))
    }

    /// Moves one node of a line.
    pub fn move_node(&mut self, line: LineRef, node: usize, position: Vec2) -> Option<()> {
        match line {
            LineRef::Ridge(ridge) => {
                self.skeleton
                    .ridges
                    .get_mut(ridge.0)?
                    .nodes
                    .get_mut(node)?
                    .position = position;
            }
            LineRef::Valley(valley) => {
                self.skeleton
                    .valleys
                    .get_mut(valley.0)?
                    .nodes
                    .get_mut(node)?
                    .position = position;
            }
            LineRef::Footprint => {
                self.skeleton.footprint.get_mut(node)?.position = position;
            }
        }
        Some(())
    }

    /// Sets the elevation of one node of a line.
    pub fn set_elevation(&mut self, line: LineRef, node: usize, elevation: f32) -> Option<()> {
        match line {
            LineRef::Ridge(ridge) => {
                self.skeleton
                    .ridges
                    .get_mut(ridge.0)?
                    .nodes
                    .get_mut(node)?
                    .elevation = elevation;
            }
            LineRef::Valley(valley) => {
                self.skeleton
                    .valleys
                    .get_mut(valley.0)?
                    .nodes
                    .get_mut(node)?
                    .elevation = elevation;
            }
            LineRef::Footprint => {
                self.skeleton.footprint.get_mut(node)?.elevation = elevation;
            }
        }
        Some(())
    }

    /// Replaces one node of a ridge.
    pub fn set_ridge_node(&mut self, ridge: RidgeId, node: usize, value: RidgeNode) -> Option<()> {
        *self.skeleton.ridges.get_mut(ridge.0)?.nodes.get_mut(node)? = value;
        Some(())
    }

    /// Replaces one node of a valley.
    pub fn set_valley_node(
        &mut self,
        valley: ValleyId,
        node: usize,
        value: ValleyNode,
    ) -> Option<()> {
        *self
            .skeleton
            .valleys
            .get_mut(valley.0)?
            .nodes
            .get_mut(node)? = value;
        Some(())
    }

    /// Inserts a node into a ridge at an index.
    pub fn insert_ridge_node(
        &mut self,
        ridge: RidgeId,
        node: usize,
        value: RidgeNode,
    ) -> Option<()> {
        let nodes = &mut self.skeleton.ridges.get_mut(ridge.0)?.nodes;
        if node > nodes.len() {
            return None;
        }
        nodes.insert(node, value);
        Some(())
    }

    /// Inserts a node into a valley at an index.
    ///
    /// # Invariants
    ///
    /// A new valley node **must** be given both of its stations, because a node without a
    /// correspondence is a row of the mesh with nothing to build against. The value carries them,
    /// so there is no way to insert one without saying what it faces.
    pub fn insert_valley_node(
        &mut self,
        valley: ValleyId,
        node: usize,
        value: ValleyNode,
    ) -> Option<()> {
        let nodes = &mut self.skeleton.valleys.get_mut(valley.0)?.nodes;
        if node > nodes.len() {
            return None;
        }
        nodes.insert(node, value);
        Some(())
    }

    /// Removes a node from a line.
    ///
    /// # Invariants
    ///
    /// A line is left with at least **two** nodes: a one-node line has no direction, and the
    /// sampler would have to invent one. The refusal is `None` rather than a violation because
    /// there is no state to report — the edit did not happen.
    pub fn remove_node(&mut self, line: LineRef, node: usize) -> Option<()> {
        let count = self.skeleton.node_count(line);
        if count <= 2 {
            return None;
        }
        match line {
            LineRef::Ridge(ridge) => {
                let nodes = &mut self.skeleton.ridges.get_mut(ridge.0)?.nodes;
                if node >= nodes.len() {
                    return None;
                }
                nodes.remove(node);
            }
            LineRef::Valley(valley) => {
                let nodes = &mut self.skeleton.valleys.get_mut(valley.0)?.nodes;
                if node >= nodes.len() {
                    return None;
                }
                nodes.remove(node);
            }
            LineRef::Footprint => {
                if node >= self.skeleton.footprint.len() {
                    return None;
                }
                self.skeleton.footprint.remove(node);
            }
        }
        Some(())
    }

    /// Sets a polyline's sampling density multiplier.
    pub fn set_step_scale(&mut self, line: LineRef, step_scale: f32) -> Option<()> {
        match line {
            LineRef::Ridge(ridge) => self.skeleton.ridges.get_mut(ridge.0)?.step_scale = step_scale,
            LineRef::Valley(valley) => {
                self.skeleton.valleys.get_mut(valley.0)?.step_scale = step_scale
            }
            LineRef::Footprint => return None,
        }
        Some(())
    }

    /// Binds one side of a valley node to a station.
    ///
    /// # Invariants
    ///
    /// This is the op the whole design exists for: it is what makes a valley line and a ridge line
    /// into a pair that can be expanded, and it is a first-class edit rather than something
    /// derived on the author's behalf. Binding a side that is already bound **replaces** the
    /// station rather than adding a second one, because a node has one line on each side and a
    /// node with two would be a rib in two directions.
    pub fn bind(
        &mut self,
        valley: ValleyId,
        node: usize,
        side: Side,
        station: Station,
    ) -> Option<()> {
        self.skeleton
            .valleys
            .get_mut(valley.0)?
            .nodes
            .get_mut(node)?
            .set_station(side, station);
        Some(())
    }

    /// Slides a bound station along the line it names.
    ///
    /// # Invariants
    ///
    /// It keeps the **line** and changes only the parameter, which is the edit an author makes
    /// when a rib is at the right place on the wrong line's length. Rebinding to another line is
    /// [`Massif::bind`], and the two are separate because a slide is the one that cannot change
    /// which pair of flanks a belt is between.
    pub fn slide_station(
        &mut self,
        valley: ValleyId,
        node: usize,
        side: Side,
        t: f32,
    ) -> Option<()> {
        let entry = self
            .skeleton
            .valleys
            .get_mut(valley.0)?
            .nodes
            .get_mut(node)?;
        let mut station = entry.station(side);
        station.t = t;
        entry.set_station(side, station);
        Some(())
    }

    /// Sets which way every belt's strip steps.
    pub fn set_diagonal_rule(&mut self, rule: DiagonalRule) {
        self.tessellation.diagonal = rule;
    }

    /// Asks for the other diagonal in one cell of one belt.
    ///
    /// # Invariants
    ///
    /// The override is recorded against the belt's **name** — valley, side and segment — and not
    /// against a face index. Face indices move on every edit and a belt's name does not, so an
    /// override written now is still the same cell after the ridge above it is raised. It is
    /// dropped by [`Massif::forget_diagonals`] or when its segment is edited away, never silently
    /// re-aimed at whatever cell now holds that index.
    pub fn flip_diagonal(&mut self, belt: BeltRef, cell: usize) {
        let override_ = DiagonalOverride { belt, cell };
        if !self.tessellation.overrides.insert(override_) {
            self.tessellation.overrides.remove(&override_);
        }
    }

    /// Drops every diagonal override.
    pub fn forget_diagonals(&mut self) {
        self.tessellation.overrides.clear();
    }
}
