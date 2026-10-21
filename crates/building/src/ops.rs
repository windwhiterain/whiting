//! The edit vocabulary: what an agent does to a building.
//!
//! # Contract of this module
//!
//! - **No op refuses an edit for being wrong.** An intermediate state that violates something is
//!   ordinary: a mass is added and only then attached, a roof is seated and only then given its
//!   planes. The ops that refuse are the ones that would leave a **dangling index** — a reference
//!   to a mass or a roof that is not there — and they refuse by returning `None`, which is the rule
//!   `mountain`'s ops follow for the same reason.
//! - **A junction is a named place, never a coincidence.** [`attach_mass`] splits the host's
//!   outline at the station first and then records the attachment, because an outline turns only at
//!   a node: a wing that merely named a point in the middle of a wall would ask for a junction the
//!   expansion cannot build. This is `mountain`'s `attach_ridge` doing to a face what it does to a
//!   ridge.
//! - **A re-split replaces, it does not merge.** The precedent is `mountain::derive_valleys`: it
//!   replaces every valley rather than merging, because "a derivation has no way to know which
//!   parts of an edited valley the author meant to keep" — it is the answer to "the rhythm is
//!   wrong", never to "the rhythm is nearly right". A split of an outline is the same operation:
//!   a second call at a new station inserts a second node, and the author who wants the old cut
//!   gone removes the node by index.
//! - **Every op is `O(log n)` or better in the thing it edits**, except the ones that must read a
//!   whole loop to answer at all (inserting a node at an arc-length station is one walk to find the
//!   edge and one insert), and none of them re-expands anything.
//!
//! # The verbs, for the shell phase
//!
//! - [`add_mass`] — returns the new id
//! - [`attach_mass`] — splits the host first, returns the new id
//! - [`split_outline`] — inserts a node, returns its index
//! - [`set_top`]
//! - [`add_roof`] — returns the new id
//! - [`remove_roof`] — returns the roof that went
//! - [`remove_mass`] — returns the mass that went, refuses while anything names it
//!
//! # The one thing every station means
//!
//! `station` is an **arc-length parameter over the closed loop, in `[0, 1]`**, the way
//! [`Attachment::station`] and [`EaveStation`](crate::roof::EaveStation) read it everywhere else in
//! this crate: a place on the loop, not a node of it. Both ops that take one normalise it into
//! `[0, 1)` before they use it, so the attachment a caller records and the node the split inserted
//! are the same place — not two readings of a parameter that happened to be written the same way.

use crate::grid::{FaceGrid, FaceRef, Rhythm};
use crate::mass::{Attachment, Join, Mass, MassId, PlanNode};
use crate::math::Vec2;
use crate::math::{distance, lerp};
use crate::member::{Anchor, MemberId};
use crate::openings::Opening;
use crate::parts::{Placement, Target, Template};
use crate::roof::{Roof, RoofId};
use crate::skeleton::BuildingSkeleton;

/// Adds a mass, returning its id.
///
/// # Invariants
///
/// **Pushing never renumbers.** An id is a list index, and this verb pushes at the end, so every
/// id an author already holds is still the mass it named. The same is true of [`add_roof`], and it
/// is why the two removals say in their own docs that the ids *after* the removed one shift down:
/// the operations are not symmetric about the middle of the list, and a caller that assumes they
/// are gets a document that names the wrong mass.
pub fn add_mass(skeleton: &mut BuildingSkeleton, mass: Mass) -> MassId {
    skeleton.masses.push(mass);
    MassId(skeleton.masses.len() - 1)
}

/// Adds a roof, returning its id.
///
/// # Invariants
///
/// The roof names the mass it sits on, and nothing here checks that the mass is there: an edit may
/// legitimately pass through a roof that is not seated yet, and [`remove_mass`] is what refuses a
/// mass something still names.
pub fn add_roof(skeleton: &mut BuildingSkeleton, roof: Roof) -> RoofId {
    skeleton.roofs.push(roof);
    RoofId(skeleton.roofs.len() - 1)
}

/// Attaches a mass to a host at a station, returning the new id.
///
/// # Invariants
///
/// **The host's outline is split at the station first**, and that is the whole of what the op has
/// to do to be usable, for the reason `mountain`'s `attach_ridge` gives: **an outline changes
/// direction only at a node.** A wing that merely named its host at an interior parameter asked
/// for a junction the expansion cannot build — the host's wall has no corner there, so the two
/// solids touch at a place neither of them has, and the author has to discover that from a
/// violation list instead of from the op that lied to them.
///
/// The station is normalised into `[0, 1)` before the split, so the recorded attachment and the
/// inserted node are one place read once. Nothing else is changed about the host: the split only
/// adds a node, so a host that already had a node there grows a degenerate one, which is a state
/// the author can see and remove.
///
/// Returns `None` and changes nothing when the host is not in the skeleton, or when its outline
/// has fewer than two nodes and therefore no edge to split — the two cases where the reference the
/// edit needs is not there.
pub fn attach_mass(
    skeleton: &mut BuildingSkeleton,
    host: MassId,
    station: f32,
    mut mass: Mass,
) -> Option<MassId> {
    let station = station.rem_euclid(1.0);
    split_outline(skeleton.masses.get_mut(host.0)?, station)?;
    mass.attachment = Some(Attachment {
        host,
        join: Join::Wing { station },
    });
    skeleton.masses.push(mass);
    Some(MassId(skeleton.masses.len() - 1))
}

/// Adds a mass that **stands on the top of `base`**, and returns the index it was given.
///
/// # Invariants
///
/// **The two masses are not split or welded**: a storey's base is the host's top, so the shells meet face to
/// face and each closes on its own. What the op records is the *relation*, which is the whole of what
/// `validate` reads when it decides that two overlapping plans are one building — the geometry the relation
/// claims (the base on the top, the plan inside the host's) is then checked, not assumed.
///
/// Returns `None` and changes nothing when the host is not in the skeleton.
pub fn stack_mass(skeleton: &mut BuildingSkeleton, base: MassId, mut mass: Mass) -> Option<MassId> {
    skeleton.masses.get(base.0)?;
    mass.attachment = Some(Attachment {
        host: base,
        join: Join::Storey,
    });
    skeleton.masses.push(mass);
    Some(MassId(skeleton.masses.len() - 1))
}

/// Inserts a node into a mass's outline at an arc-length station, returning its index.
///
/// # Invariants
///
/// - **The walk is over the closed loop**: the edge from the last node back to the first is
///   implied by [`Mass::outline`] and is part of the length here, so a station on it inserts at the
///   end of the list and still lands between the two nodes it names.
/// - **Position and elevation are interpolated along the one edge the station lands on**, the same
///   interpolation `Roof::eave_point` applies to a station of an eave loop, so a junction the two
///   ends share is a point they agree on rather than two points a tolerance apart.
/// - **It always inserts.** A station that lands exactly on an existing node still inserts one,
///   because refusing it would make "attach a wing at the corner" a different expression from
///   "attach a wing half way along the wall" and the author would have to know which case they were
///   in. The degenerate edge that leaves behind is a thing the author wrote on purpose, and
///   [`validate`](crate::validate) is where a zero-length edge is reported.
///
/// `None` only when the loop has fewer than two nodes: with no edge there is no edge to fall on.
/// `O(n)` in the loop's nodes — one walk to find the edge, one insert — which is the cost of
/// answering "where is this station" over a loop with no index to search.
pub fn split_outline(mass: &mut Mass, station: f32) -> Option<usize> {
    let nodes = &mass.outline();
    let count = nodes.len();
    if count < 2 {
        return None;
    }
    let (spans, total) = closed_spans(nodes);
    let target = station.rem_euclid(1.0) * total;
    // The cumulative lengths only grow, so the edge is a binary search rather than a walk; the
    // walk has already been paid for by `closed_spans`.
    let index = spans
        .partition_point(|length| *length <= target)
        .saturating_sub(1)
        .min(count - 1);
    let (low, high) = (spans[index], spans[index + 1]);
    let width = high - low;
    let within = if width > f32::EPSILON {
        ((target - low) / width).clamp(0.0, 1.0)
    } else {
        0.0
    };
    let from = nodes[index];
    let to = nodes[(index + 1) % count];
    let mixed = PlanNode::new(
        lerp(from.position, to.position, within),
        from.elevation + (to.elevation - from.elevation) * within,
    );
    let at = index + 1;
    // **The top section moves with the loop or it stops being a section.** The two are one node per position by
    // definition, so the new node takes the wall top the section has at its own station — the same interpolation
    // the foot's elevation gets — and a mass with a level top stays level, because `before` and `after` are equal
    // there.
    let crown = {
        let top = mass.crown();
        let before = top.outline[index].elevation;
        let after = top.outline[(index + 1) % count].elevation;
        before + (after - before) * within
    };
    mass.crown_mut().outline.insert(
        at,
        PlanNode::new(lerp(from.position, to.position, within), crown),
    );
    mass.outline_mut().insert(at, mixed);
    Some(at)
}

/// Sets a mass's wall top.
///
/// # Invariants
///
/// It moves the **wall top only**, and never the outline's own elevations: a mass on sloping ground
/// is authored by the outline, and a roof that sits on this mass reads its own sink from the top it
/// finds here, so changing the top moves the eave with it.
///
/// The `Option` is the verb list's uniformity rather than a refusal: the argument is already a
/// resolved `&mut Mass`, so there is no index that could dangle and nothing to report.
pub fn set_top(mass: &mut Mass, top: f32) -> Option<()> {
    mass.set_top(top);
    Some(())
}

/// Removes a roof, returning it.
///
/// # Invariants
///
/// **The ids after it shift down.** The caller is being told what went by having it handed back,
/// and every `RoofId` above this one is now a different roof; an author holding one has to re-read
/// it rather than trust it. `None` when the id is not there.
pub fn remove_roof(skeleton: &mut BuildingSkeleton, roof: RoofId) -> Option<Roof> {
    if roof.0 >= skeleton.roofs.len() {
        return None;
    }
    Some(skeleton.roofs.remove(roof.0))
}

/// Removes a mass, returning it.
///
/// # Invariants
///
/// **It refuses while anything names the mass**: a wing whose [`Attachment::host`] is this mass, or
/// a roof whose `host` is this mass. The refusal is the dangling-index rule and not a judgement
/// about the building — removing a host would leave a wing standing on nothing and a roof whose
/// authorization to touch the wall it sinks into has been withdrawn, and both would be repaired by
/// nothing. Removing those names first is two calls, and each of them hands back what it removed.
///
/// The ids after it shift down, for the reason [`remove_roof`] gives.
pub fn remove_mass(skeleton: &mut BuildingSkeleton, mass: MassId) -> Option<Mass> {
    skeleton.mass(mass)?;
    let named = !skeleton.wings_of(mass).is_empty() || !skeleton.roofs_of(mass).is_empty();
    if named {
        return None;
    }
    Some(skeleton.masses.remove(mass.0))
}

/// Returns the cumulative arc length at each node of a closed loop, and the total.
///
/// # Invariants
///
/// The edge from the last node back to the first is counted, because a station reads the loop as
/// closed and a walk that stopped one edge short would put every station after the last node in a
/// place the loop does not have.
///
/// This is the same walk as `Roof::eave_spans`, restated here because that method is on the frozen
/// `Roof` and a mass's outline is not one: reading the eave loop's lengths through a temporary
/// `Roof` would allocate a roof to ask a question about a loop. `derive` shares this one rather
/// than walking a second time, because two walks of the same loop is how two readings of one
/// station start to disagree.
pub(crate) fn closed_spans(nodes: &[PlanNode]) -> (Vec<f32>, f32) {
    let count = nodes.len();
    let mut spans = Vec::with_capacity(count + 1);
    let mut total = 0.0;
    spans.push(0.0);
    for index in 0..count {
        total += distance(nodes[index].position, nodes[(index + 1) % count].position);
        spans.push(total);
    }
    (spans, total)
}

/// Gives a face a rhythm, **replacing** whatever rhythm it had.
///
/// # Invariants
///
/// - **Replacement, never a merge** (D10's rule, and `mountain::derive_valleys`' precedent): both
///   families are laid out fresh from the face's own extents, and what was there is gone — returned
///   so a caller can keep it, which is also how a probe measures that the replacement happened
///   rather than accumulating.
/// - **The face decides the domain, and the rhythm is refused rather than stretched.** The columns
///   are laid out over the edge's plan length, and the rows over `[the lowest ground under it, the
///   wall top]`; [`Rhythm::stops`] answers `None` when it cannot span that, and this op passes the
///   refusal on. Nothing is clamped, and no bay is widened to make a facade come out.
/// - **A level is an elevation and a division is arc length.** The rows are laid out as rises and
///   written as elevations, because a storey line is level in the world whatever the ground under it
///   does; the columns stay as arc length, which is what a station means everywhere here.
/// - **A missing face is a refusal**, not a panic and not an entry in the map: a rhythm on a face the
///   document does not hold is a rhythm nothing can be authored against.
///
/// Returns how many bays the face now holds, or `None` when the face is not there or either rhythm
/// cannot be laid out on it.
pub fn split_face(
    skeleton: &mut BuildingSkeleton,
    face: FaceRef,
    columns: &Rhythm,
    rows: &Rhythm,
) -> Option<usize> {
    let length = skeleton.face_length(face)?;
    let (base, top) = skeleton.face_extent(face)?;
    let divisions = columns.stops(length)?;
    let levels: Vec<f32> = rows
        .stops(top - base)?
        .into_iter()
        .map(|rise| base + rise)
        .collect();
    let grid = FaceGrid::new(levels, divisions);
    let bays = grid.bays();
    skeleton.grids.replace(face, grid);
    Some(bays)
}

/// Removes a face's rhythm and returns it.
///
/// Nothing else is touched: a placement hanging in the grid is the caller's to keep or drop, which is
/// exactly what slots being indices rather than coordinates buys (D10) — the grid can be replaced
/// under a placement without the placement having to be rewritten.
pub fn clear_face(skeleton: &mut BuildingSkeleton, face: FaceRef) -> Option<FaceGrid> {
    skeleton.grids.take(face)
}

/// Cuts an opening into a bay, returning its index in the document's list.
///
/// # Invariants
///
/// - **The bay decides the domain.** The opening names a slot, and a slot the face's grid does not
///   hold is refused here (`None`) rather than recorded and reported later: an opening nothing can be
///   measured against is not an edit that half-happened, and the refusal is the same shape as
///   [`split_face`]'s.
/// - **Nothing is clamped.** An inset that would put the hole past its bay's own edge is refused by
///   [`Opening::extents`] rather than pulled back to fit — the rule everywhere else here.
/// - **Appending never renumbers.** The mesh's provenance names an opening by its index in the list,
///   so an edit that renumbered the ones already there would rewrite the meaning of every reveal that
///   has been built. [`remove_opening`] says in its own doc that it does renumber, because a list
///   removal does.
pub fn add_opening(skeleton: &mut BuildingSkeleton, opening: Opening) -> Option<usize> {
    let bay = skeleton.bay(opening.face, opening.row, opening.column)?;
    opening.extents(bay)?;
    Some(skeleton.openings.push(opening))
}

/// Removes an opening by index and returns it.
///
/// The indices **after** the removed one shift down, which is what a list removal does and what
/// [`remove_mass`] says of itself too: the operations are not symmetric about the middle of the list,
/// and a caller that assumes they are gets a document that names the wrong hole.
pub fn remove_opening(skeleton: &mut BuildingSkeleton, index: usize) -> Option<Opening> {
    skeleton.openings.remove(index)
}

/// Adds a template to the document and returns the index it was given.
///
/// Appending never renumbers, which matters here because a placement names its template by index: an
/// edit that renumbered the templates already there would move every part that names one.
pub fn add_template(skeleton: &mut BuildingSkeleton, template: Template) -> usize {
    skeleton.parts.add_template(template)
}

/// Places a template in a bay and returns the placement's index.
///
/// # Invariants
///
/// - **The document decides the domain.** A template index it does not hold, or a bay slot its grid
///   does not hold, is refused here (`None`) rather than recorded and reported later — the rule
///   [`add_opening`] follows, and for the same reason: there is no bay to measure anything against.
/// - **A part that does not fit its bay is still placed.** Fitting is a question about a template's
///   box against a bay's extents, and the answer belongs in a report
///   ([`PlacementOutsideBay`](crate::validate::Violation::PlacementOutsideBay)) while the author moves
///   the part; refusing it here would make a document unbuildable for a fault that is a matter of
///   centimetres. `O(log faces)` for the bay and `O(1)` for the rest.
pub fn place_part(skeleton: &mut BuildingSkeleton, placement: Placement) -> Option<usize> {
    let template = skeleton.parts.template(placement.template)?;
    // **The document decides the domain, for both targets.** A bay its grid does not hold, or a wall run it
    // does not hold, is refused here rather than recorded and reported later: there is nothing to measure the
    // part against.
    match placement.target {
        Target::Bay { face, row, column } => {
            skeleton.bay(face, row, column)?;
        }
        Target::Wall { wall } => {
            skeleton.walls.get(wall.0)?;
        }
    }
    // A mounted part is refused when its joint does not resolve — a socket the template does not name,
    // an anchor naming an opening in another wall, or a mount on a wall run, which has no bay to anchor
    // to. That is a domain question like the bay's own existence, and unlike a part that merely does not
    // fit it has no reading at all: there is nowhere to put the part. Fitting stays a reported fault.
    let at = skeleton.pin_of(&placement)?;
    placement.reach(template, at)?;
    Some(skeleton.parts.place(placement))
}

/// Removes a placement by index and returns it.
pub fn remove_placement(skeleton: &mut BuildingSkeleton, index: usize) -> Option<Placement> {
    skeleton.parts.remove(index)
}

/// Appends one document's skeleton to another, **re-basing every index it names**.
///
/// # Invariants
///
/// **A merge is a shift and not a copy**, because every reference in a document is an index into a list that is
/// about to move: a mass names its host, a roof and a slab name the mass they sit on, a member names the member
/// or the wall face it hangs from, an opening names the face it is cut into, and a rhythm names the face it was
/// laid on. `offset` is added to each of those and to nothing else — the geometry does not move, so a document
/// merged into another stands where it was authored, and a scene that wants it elsewhere translates its own
/// plans before merging rather than asking this op to do two things.
///
/// **A document that places parts is refused.** A placement names a template by index, a bay by slot, and a pin
/// inside that template; re-basing it is not a shift but a re-resolution against a grid that the base document
/// may have authored differently. Answering `false` is the honest reply, and the caller can place its parts
/// after the merge instead.
///
/// Returns whether the merge happened. Nothing is changed when it does not.
pub fn merge(base: &mut BuildingSkeleton, other: BuildingSkeleton) -> bool {
    if !other.parts.is_empty() {
        return false;
    }
    let mass_offset = base.masses.len();
    let member_offset = base.members.len();
    let rebase_mass = |host: MassId| MassId(host.0 + mass_offset);
    let rebase_member = |member: MemberId| MemberId(member.0 + member_offset);
    let rebase_anchor = |anchor: Anchor| match anchor {
        Anchor::Free { .. } => anchor,
        Anchor::OnMember { member, t } => Anchor::OnMember {
            member: rebase_member(member),
            t,
        },
        Anchor::OnFace { face, along, up } => Anchor::OnFace {
            face: rebase_face(face, mass_offset),
            along,
            up,
        },
    };

    // **The grids are re-keyed and not rebuilt**: a rhythm is a value that names a face, and the face is the
    // only part of it that a merge moves.
    for face in other.grids.faces() {
        if let Some(grid) = other.grids.get(face) {
            base.grids
                .replace(rebase_face(face, mass_offset), grid.clone());
        }
    }
    for opening in other.openings.iter() {
        let mut moved = opening.clone();
        moved.face = rebase_face(opening.face, mass_offset);
        base.openings.push(moved);
    }
    for mut mass in other.masses {
        if let Some(attachment) = mass.attachment {
            mass.attachment = Some(crate::mass::Attachment {
                host: rebase_mass(attachment.host),
                join: attachment.join,
            });
        }
        base.masses.push(mass);
    }
    for mut roof in other.roofs {
        roof.host = rebase_mass(roof.host);
        base.roofs.push(roof);
    }
    for mut slab in other.slabs {
        if let Some(host) = slab.host {
            slab.host = Some(rebase_mass(host));
        }
        base.slabs.push(slab);
    }
    for mut member in other.members {
        member.from = rebase_anchor(member.from);
        member.to = rebase_anchor(member.to);
        base.members.push(member);
    }
    // The site's own enclosures carry no indices into the lists above: a wall is a line and a height.
    base.walls.extend(other.walls);
    true
}

/// Returns a face with its mass re-based, which is the only field of it a merge moves.
fn rebase_face(face: crate::grid::FaceRef, mass_offset: usize) -> crate::grid::FaceRef {
    crate::grid::FaceRef {
        mass: MassId(face.mass.0 + mass_offset),
        ..face
    }
}

/// Moves a document's **plans** by an offset, leaving every elevation where it is.
///
/// # Invariants
///
/// **A translate moves geometry and no index.** It is the other half of [`merge`]: a merge appends a document
/// where it was authored, and a village that wants that document on the other side of a square translates it
/// first. Nothing that names a list moves with it — a host is still the same mass, a rhythm still the same
/// face — so the two ops commute with everything the expansion reads.
///
/// **Only the plan moves.** A roof's eaves and its crest points, a slab's outline, a member's free ends and the
/// site's walls are all read in plan and shifted by the same `offset`; a crest's elevation, a mass's node
/// elevations, a slab's two faces and a wall's height are untouched, because a document that moved up as well as
/// across would be a different building.
///
/// A member anchored on another member, on a wall face or on nothing but a fraction of a neighbour carries no
/// plan of its own: its two ends are derived from what they name, and that is moved for it.
pub fn translate(skeleton: &mut BuildingSkeleton, offset: Vec2) {
    let shift = |node: &mut PlanNode| {
        node.position[0] += offset[0];
        node.position[1] += offset[1];
    };
    // **Every section of a mass moves, not only its foot.** A mass drawn in several bands carries one loop per level, and
    // `Mass::outline_mut` is the foot's: a translation that used it would move the bottom of a battered or stepped mass
    // and leave the top where it was authored, which is a wall standing over a plan it does not share. Nothing in the
    // gallery translated such a mass when this was written, so the defect was invisible; the Dai hamlet turns houses,
    // and the rotation below is its twin.
    for mass in &mut skeleton.masses {
        for level in &mut mass.levels {
            for node in &mut level.outline {
                shift(node);
            }
        }
    }
    for roof in &mut skeleton.roofs {
        for node in &mut roof.eaves {
            shift(node);
        }
        for point in &mut roof.crest {
            point.position[0] += offset[0];
            point.position[1] += offset[1];
        }
    }
    for slab in &mut skeleton.slabs {
        for corner in &mut slab.outline {
            corner[0] += offset[0];
            corner[1] += offset[1];
        }
    }
    for member in &mut skeleton.members {
        let free = |anchor: &mut Anchor| {
            if let Anchor::Free { at } = anchor {
                at[0] += offset[0];
                at[2] += offset[1];
            }
        };
        free(&mut member.from);
        free(&mut member.to);
    }
    for wall in &mut skeleton.walls {
        shift(&mut wall.from);
        shift(&mut wall.to);
    }
}

/// Turns a whole document about a plan point, by `radians`, leaving every elevation where it is.
///
/// # Invariants
///
/// **It is [`translate`]'s twin and moves exactly the same fields**, for the same reason: a document whose parts were
/// turned and whose roofs were not is two documents. A plan point goes `(x, z) -> about + R (x - about)`, and an
/// elevation is untouched.
///
/// **Nothing that names a list changes.** A host is the same mass, a rhythm the same face, a member's `along` the same
/// distance from the same edge's start — a rotation turns an edge and keeps its index and its winding — so the two ops
/// commute with everything the expansion reads.
///
/// **A member's `up` is an elevation and does not turn**, which is what keeps a stair a stair: its two ends are two
/// heights, and turning the plan under them turns treads and stringer together.
pub fn rotate(skeleton: &mut BuildingSkeleton, about: Vec2, radians: f32) {
    let (sine, cosine) = radians.sin_cos();
    let turn = |at: &mut [f32; 2]| {
        let across = at[0] - about[0];
        let along = at[1] - about[1];
        at[0] = about[0] + across * cosine - along * sine;
        at[1] = about[1] + across * sine + along * cosine;
    };
    let shift = |node: &mut PlanNode| turn(&mut node.position);
    for mass in &mut skeleton.masses {
        for level in &mut mass.levels {
            for node in &mut level.outline {
                shift(node);
            }
        }
    }
    for roof in &mut skeleton.roofs {
        for node in &mut roof.eaves {
            shift(node);
        }
        for point in &mut roof.crest {
            turn(&mut point.position);
        }
    }
    for slab in &mut skeleton.slabs {
        for corner in &mut slab.outline {
            turn(corner);
        }
    }
    for member in &mut skeleton.members {
        let free = |anchor: &mut Anchor| {
            if let Anchor::Free { at } = anchor {
                let mut plan = [at[0], at[2]];
                turn(&mut plan);
                at[0] = plan[0];
                at[2] = plan[1];
            }
        };
        free(&mut member.from);
        free(&mut member.to);
        // **A member's `across` is a plan direction and turns with the body.** It is the direction its width is measured
        // along, so leaving it in the world frame turns every member's section against its own axis: measured on a
        // turned 竹楼 as `MemberDegenerate { length: 1.1 }` from the stairs, whose `across` then lay along the run it was
        // meant to be across.
        turn(&mut member.across);
    }
    for wall in &mut skeleton.walls {
        shift(&mut wall.from);
        shift(&mut wall.to);
    }
}
/// Returns `plan` wound positively, which is the order every outline in this crate is read in.
///
/// # Invariants
///
/// The signed area is `sum(x_i * y_{i+1} - x_{i+1} * y_i) / 2`, and a loop that winds the other way is reversed. **The first
/// node stays where it is**, so a run keeps the corner its neighbours were measured from — reversing in place is not the
/// same as rotating.
///
/// # Why it is a helper and not a habit
///
/// Which way a list of corners winds depends on the direction the run is *walked*, so the same four corners written for a
/// wall running east and one running west wind opposite ways. Round 72 wrote both by hand and the crate answered
/// `OffsetInverted` with areas `-15.81` and `-7.82`; the rule belongs in one place, next to the offsetting that reads it.
pub fn wound(mut plan: Vec<[f32; 2]>) -> Vec<[f32; 2]> {
    let mut twice = 0.0;
    for index in 0..plan.len() {
        let a = plan[index];
        let b = plan[(index + 1) % plan.len()];
        twice += a[0] * b[1] - b[0] * a[1];
    }
    if twice < 0.0 {
        plan.reverse();
    }
    plan
}
